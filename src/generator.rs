//! Provider adapters return data only; build and file operations belong to CREXE.
use super::{
    config::{Kind, Settings},
    GeneratedOutput,
};
use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::{
    io::Read,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

const MAX_RESPONSE: u64 = 20 * 1024 * 1024;
type CredentialSnapshot = Arc<OnceLock<Result<Option<String>, String>>>;

pub(crate) struct Session<'a> {
    settings: &'a Settings,
    started: Instant,
    requests: u8,
    reserved_tokens: u32,
    credential: CredentialSnapshot,
}

impl<'a> Session<'a> {
    pub fn new(settings: &'a Settings) -> Self {
        Self {
            settings,
            started: Instant::now(),
            requests: 0,
            reserved_tokens: 0,
            credential: Arc::new(OnceLock::new()),
        }
    }
    pub fn generate(&mut self, system: &str, user: &str) -> Result<GeneratedOutput> {
        let policy = &self.settings.execution;
        if self.requests >= policy.max_provider_requests {
            bail!("Local provider request budget exhausted");
        }
        if system.len().saturating_add(user.len()) > policy.max_prompt_bytes {
            bail!("Prompt exceeds local max_prompt_bytes; no provider request was made");
        }
        let reserved = self
            .reserved_tokens
            .saturating_add(self.settings.provider.max_output_tokens);
        if reserved > policy.max_output_tokens_total {
            bail!("Local output-token budget exhausted; no provider request was made");
        }
        let remaining = Duration::from_secs(policy.generation_timeout_seconds)
            .checked_sub(self.started.elapsed())
            .filter(|v| v.as_secs() > 0)
            .context("Local generation time budget exhausted")?;
        self.requests += 1;
        self.reserved_tokens = reserved;
        let mut settings = self.settings.clone();
        settings.provider.timeout_seconds =
            settings.provider.timeout_seconds.min(remaining.as_secs());
        generate(&settings, system, user, self.credential.clone())
    }
}

fn generate(
    settings: &Settings,
    system: &str,
    user: &str,
    credential: CredentialSnapshot,
) -> Result<GeneratedOutput> {
    if super::executor::cancelled() {
        bail!("Cancelled");
    }
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let settings = settings.clone();
    let system = system.to_owned();
    let user = user.to_owned();
    std::thread::spawn(move || {
        let _ = send.send(request(&settings, &system, &user, &credential));
    });
    loop {
        if super::executor::cancelled() {
            bail!("Cancelled; generation was not published");
        }
        match receive.recv_timeout(Duration::from_millis(100)) {
            Ok(result) => return result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => bail!("Provider request worker stopped unexpectedly"),
        }
    }
}

fn session_key(settings: &Settings, credential: &CredentialSnapshot) -> Result<Option<String>> {
    // Resolve on the first request, in its worker thread. Repairs use the same
    // credential even if configuration rotates/deletes its vault entry meanwhile.
    credential
        .get_or_init(|| settings.api_key().map_err(|e| format!("{e:#}")))
        .clone()
        .map_err(anyhow::Error::msg)
}

fn request(
    settings: &Settings,
    system: &str,
    user: &str,
    credential: &CredentialSnapshot,
) -> Result<GeneratedOutput> {
    let p = &settings.provider;
    let key = session_key(settings, credential)?;
    let client = Client::builder()
        .timeout(Duration::from_secs(p.timeout_seconds))
        .connect_timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let example = if p.kind == Kind::Deepseek {
        "\nJSON structure example: {\"files\":[{\"path\":\"src/example.txt\",\"content\":\"complete file contents\"}]}. Use the actual paths required by this project."
    } else {
        ""
    };
    let messages = json!([
        {"role": "system", "content": format!("{system}\nReturn only a JSON object with files: an array of objects with path and content (strings). Include every required source file. No prose outside JSON.{example}")},
        {"role": "user", "content": user}
    ]);
    let (endpoint, payload) = match p.kind {
        Kind::Ollama => (
            "api/chat",
            json!({
                "model": p.model, "messages": messages, "stream": false,
                "format": {"type": "object", "required": ["files"], "properties": {"files": {"type": "array", "minItems": 1, "items": {"type": "object", "required": ["path", "content"], "properties": {"path": {"type": "string"}, "content": {"type": "string"}}}}}},
                "think": p.thinking, "keep_alive": p.keep_alive,
                "options": {"num_ctx": p.context_tokens, "num_predict": p.max_output_tokens, "temperature": 0.2}
            }),
        ),
        Kind::Openai => {
            let mut payload = json!({"model": p.model, "messages": messages, "response_format": {"type": "json_object"}});
            let reasoning = p.model.starts_with("gpt-5")
                || p.model.starts_with("o1")
                || p.model.starts_with("o3")
                || p.model.starts_with("o4");
            payload[if reasoning {
                "max_completion_tokens"
            } else {
                "max_tokens"
            }] = json!(p.max_output_tokens);
            if !reasoning {
                payload["temperature"] = json!(0.2);
            }
            ("chat/completions", payload)
        }
        Kind::Deepseek => {
            let thinking = p.thinking.unwrap_or(false);
            let mut payload = json!({
                "model": p.model, "messages": messages, "stream": false,
                "max_tokens": p.max_output_tokens,
                "response_format": {"type": "json_object"},
                "thinking": {"type": if thinking { "enabled" } else { "disabled" }}
            });
            if !thinking {
                payload["temperature"] = json!(0.2);
            }
            ("chat/completions", payload)
        }
    };
    println!(
        "Generating with provider={}, model={}",
        settings.profile, p.model
    );
    let url = format!("{}/{endpoint}", p.base_url.trim_end_matches('/'));
    let mut request = client.post(url).json(&payload);
    if let Some(key) = key {
        request = request.bearer_auth(key);
    }
    let response = request.send().map_err(|error| {
        if error.is_timeout() {
            anyhow::anyhow!("Provider timed out; no automatic fallback was attempted")
        } else {
            anyhow::anyhow!("Provider connection failed; check the configured service and endpoint")
        }
    })?;
    let status = response.status();
    // Do not echo arbitrary HTTP error bodies: gateways can include Authorization.
    if !status.is_success() {
        bail!(p.kind.http_error(status));
    }
    let mut body = Vec::new();
    response
        .take(MAX_RESPONSE + 1)
        .read_to_end(&mut body)
        .context("Cannot read provider response")?;
    if body.len() as u64 > MAX_RESPONSE {
        bail!("Provider response exceeded 20 MiB");
    }
    decode(p.kind, &body)
}

fn decode(kind: Kind, body: &[u8]) -> Result<GeneratedOutput> {
    let response: Value = serde_json::from_slice(body).context("Invalid provider response JSON")?;
    let content = match kind {
        Kind::Ollama => {
            if response["done"] != true || response["done_reason"] != "stop" { bail!("Ollama response incomplete (token limit or interrupted generation); increase local limits or choose another local model"); }
            response["message"]["content"].as_str()
        }
        Kind::Openai | Kind::Deepseek => {
            let choice = &response["choices"][0];
            if choice["finish_reason"] != "stop" || choice["message"]["refusal"].is_string()
                || choice["message"]["tool_calls"].as_array().is_some_and(|calls| !calls.is_empty()) {
                bail!("{} response incomplete or refused; no files were accepted", kind.label());
            }
            choice["message"]["content"].as_str()
        }
    }.context("Provider response has no message content")?;
    if content.trim().is_empty() {
        bail!("Provider returned empty content; no files were accepted");
    }
    let generated = serde_json::from_str(&super::extract_json(content))
        .context("Provider did not return the required files JSON object")?;
    if kind == Kind::Deepseek {
        // Whitelist metadata only; never log arbitrary content, reasoning or HTTP bodies.
        let mut metadata = serde_json::Map::new();
        if let Some(model) = response["model"].as_str().filter(|s| {
            s.len() <= 256
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-._:/".contains(c))
        }) {
            metadata.insert("model".into(), json!(model));
        }
        for field in [
            "prompt_tokens",
            "completion_tokens",
            "total_tokens",
            "prompt_cache_hit_tokens",
            "prompt_cache_miss_tokens",
        ] {
            if let Some(n) = response["usage"][field].as_u64() {
                metadata.insert(field.into(), json!(n));
            }
        }
        println!("DeepSeek response metadata: {}", Value::Object(metadata));
    }
    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpListener};

    fn deepseek_exchange(
        status: u16,
        body: String,
        thinking: Option<bool>,
        delay: Duration,
    ) -> (Result<GeneratedOutput>, Value) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buf = [0; 4096];
            let start = loop {
                let n = stream.read(&mut buf).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buf[..n]);
                if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    break i + 4;
                }
            };
            let header = String::from_utf8_lossy(&bytes[..start]).to_ascii_lowercase();
            assert!(header.starts_with("post /chat/completions http/1.1"));
            assert!(header.contains("authorization: bearer synthetic-deepseek-only"));
            let length: usize = header
                .lines()
                .find_map(|s| s.strip_prefix("content-length:"))
                .unwrap()
                .trim()
                .parse()
                .unwrap();
            while bytes.len() < start + length {
                let n = stream.read(&mut buf).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buf[..n]);
            }
            let payload: Value = serde_json::from_slice(&bytes[start..start + length]).unwrap();
            std::thread::sleep(delay);
            let _ = write!(
                stream,
                "HTTP/1.1 {status} Result\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            payload
        });
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let env_file = temp.path().join(".env");
        std::fs::write(
            &path,
            include_str!("../config.example.toml")
                .replace("https://api.deepseek.com", &url)
                .replace("crexe_deepseek_api_key", "CREXE_DEEPSEEK_SYNTHETIC_UNIT"),
        )
        .unwrap();
        std::fs::write(
            &env_file,
            "CREXE_DEEPSEEK_SYNTHETIC_UNIT=synthetic-deepseek-only",
        )
        .unwrap();
        let mut settings =
            Settings::load(Some(&path), Some("deepseek"), None, Some(&env_file)).unwrap();
        settings.provider.thinking = thinking;
        settings.provider.timeout_seconds = 1;
        // Model names must not trigger OpenAI-specific parameter inference.
        settings.provider.model = "gpt-5-synthetic".into();
        let mut session = Session::new(&settings);
        let result = session.generate("Return source files", "Test program");
        assert_eq!(session.requests, 1);
        (result, server.join().unwrap())
    }

    #[test]
    fn deepseek_wire_contract_modes_and_multiple_files() {
        let files = json!({"files":[{"path":"src/main.rs","content":"mod other; fn main() {}"},{"path":"src/other.rs","content":"// module"}]}).to_string();
        for thinking in [None, Some(false), Some(true)] {
            let body = format!(
                "\n\n{}",
                json!({"model":"deepseek-flash", "usage":{"prompt_tokens":20,"completion_tokens":30,"total_tokens":50},
                "choices":[{"finish_reason":"stop","message":{"content":files,"reasoning_content":"never-source"}}]})
            );
            let (result, payload) = deepseek_exchange(200, body, thinking, Duration::ZERO);
            assert_eq!(result.unwrap().files.len(), 2);
            assert_eq!(
                payload["thinking"]["type"],
                if thinking == Some(true) {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            assert_eq!(payload["max_tokens"], 6000);
            assert_eq!(payload["stream"], false);
            assert_eq!(payload["response_format"]["type"], "json_object");
            assert_eq!(
                payload["temperature"],
                if thinking == Some(true) {
                    Value::Null
                } else {
                    json!(0.2)
                }
            );
            for absent in [
                "max_completion_tokens",
                "think",
                "options",
                "reasoning_effort",
            ] {
                assert!(payload.get(absent).is_none());
            }
            assert!(payload["messages"][0]["content"]
                .as_str()
                .unwrap()
                .contains("JSON structure example"));
        }
    }

    #[test]
    fn deepseek_errors_are_bounded_sanitized_and_not_retried() {
        for (status, message) in [
            (401, "Credencial"),
            (402, "Saldo"),
            (400, "Parâmetros"),
            (422, "Parâmetros"),
            (429, "Limite"),
            (500, "indisponível"),
            (503, "indisponível"),
        ] {
            let (result, _) = deepseek_exchange(
                status,
                "private-body-synthetic-deepseek-only".into(),
                None,
                Duration::ZERO,
            );
            let error = format!("{:#}", result.unwrap_err());
            assert!(error.contains(message));
            assert!(error.contains(&status.to_string()));
            assert!(!error.contains("private-body") && !error.contains("synthetic-deepseek-only"));
        }
        let (result, _) = deepseek_exchange(200, "{}".into(), None, Duration::from_millis(1250));
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    #[test]
    fn deepseek_rejects_empty_reasoning_only_malformed_and_truncated_outputs() {
        let good = json!({"files":[{"path":"main.c","content":"int main(){}"}]}).to_string();
        for content in [
            Value::Null,
            json!(""),
            json!(" \n\t"),
            json!("{\"files\":"),
            json!("not-json"),
        ] {
            let body = json!({"choices":[{"finish_reason":"stop","message":{"content":content,"reasoning_content":good}}]}).to_string();
            assert!(decode(Kind::Deepseek, body.as_bytes()).is_err());
        }
        for reason in [
            "length",
            "content_filter",
            "tool_calls",
            "aborted",
            "insufficient_system_resource",
        ] {
            let body = json!({"choices":[{"finish_reason":reason,"message":{"content":good}}]})
                .to_string();
            assert!(decode(Kind::Deepseek, body.as_bytes()).is_err());
        }
        for extra in [
            json!({"content":good,"refusal":"refused"}),
            json!({"content":good,"tool_calls":[{}]}),
        ] {
            let body = json!({"choices":[{"finish_reason":"stop","message":extra}]}).to_string();
            assert!(decode(Kind::Deepseek, body.as_bytes()).is_err());
        }
    }
    #[test]
    fn repairs_keep_the_credential_captured_by_the_first_request() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let env = temp.path().join(".env");
        std::fs::write(
            &path,
            include_str!("../config.example.toml")
                .replace("crexe_openai_api_key_env", "CREXE_SYNTHETIC_SNAPSHOT_TEST"),
        )
        .unwrap();
        std::fs::write(&env, "CREXE_SYNTHETIC_SNAPSHOT_TEST=synthetic-session-key").unwrap();
        let settings = Settings::load(Some(&path), Some("openai"), None, Some(&env)).unwrap();
        let snapshot = Arc::new(OnceLock::new());
        assert_eq!(
            session_key(&settings, &snapshot).unwrap().as_deref(),
            Some("synthetic-session-key")
        );
        // Simulate a credential source that became unavailable after request one.
        let expired = Settings::load(Some(&path), Some("openai"), None, None).unwrap();
        assert!(expired.api_key().is_err());
        assert_eq!(
            session_key(&expired, &snapshot).unwrap().as_deref(),
            Some("synthetic-session-key")
        );
    }
    #[test]
    fn truncated_or_refused_responses_never_become_projects() {
        let content = json!({"files": [{"path": "main.c", "content": "int main(){}"}]}).to_string();
        for reason in ["length", "content_filter", "tool_calls"] {
            assert!(decode(Kind::Openai, &serde_json::to_vec(&json!({"choices": [{"finish_reason": reason, "message": {"content": content}}]})).unwrap()).is_err());
        }
        assert!(decode(
            Kind::Ollama,
            &serde_json::to_vec(
                &json!({"done": true, "done_reason": "length", "message": {"content": content}})
            )
            .unwrap()
        )
        .is_err());
        assert_eq!(
            decode(
                Kind::Ollama,
                &serde_json::to_vec(
                    &json!({"done": true, "done_reason": "stop", "message": {"content": content}})
                )
                .unwrap()
            )
            .unwrap()
            .files
            .len(),
            1
        );
    }
}
