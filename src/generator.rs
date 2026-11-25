//! Provider adapters return data only; build and file operations belong to CREXE.
use super::{
    config::{Kind, Settings},
    GeneratedOutput,
};
use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::{
    io::Read,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

const MAX_RESPONSE: u64 = 20 * 1024 * 1024;
const MAX_READER_PROMPT_BYTES: usize = 16 * 1024;
type CredentialSnapshot = Arc<OnceLock<Result<Option<String>, String>>>;

#[derive(Clone, Copy)]
enum Contract {
    Files,
    ReaderPlan,
}

impl Contract {
    fn instruction(self, kind: Kind) -> &'static str {
        match (self, kind) {
            (Self::Files, Kind::Deepseek) => "Return only a JSON object with files: an array of objects with path and content (strings). Include every required source file. No prose outside JSON.\nJSON structure example: {\"files\":[{\"path\":\"src/example.txt\",\"content\":\"complete file contents\"}]}. Use the actual paths required by this project.",
            (Self::Files, _) => "Return only a JSON object with files: an array of objects with path and content (strings). Include every required source file. No prose outside JSON.",
            (Self::ReaderPlan, _) => "Return only a JSON object with exactly one field: prompt (a nonempty string, at most 16384 UTF-8 bytes). JSON structure example: {\"prompt\":\"Build a simple, efficient local document viewer following the supplied reader contract.\"}. Formulate a prose specification for program generation; do not output source files, tool calls or executable commands. No prose outside JSON.",
        }
    }

    fn schema(self) -> Value {
        match self {
            Self::Files => {
                json!({"type": "object", "required": ["files"], "properties": {"profile": {"type": "string"}, "change_kind": {"type": "string", "enum": ["incremental", "recreate"]}, "reason": {"type": "string"}, "files": {"type": "array", "minItems": 1, "items": {"type": "object", "required": ["path", "content"], "properties": {"path": {"type": "string"}, "content": {"type": "string"}}}}}})
            }
            Self::ReaderPlan => {
                json!({"type": "object", "required": ["prompt"], "additionalProperties": false, "properties": {"prompt": {"type": "string", "minLength": 1, "maxLength": MAX_READER_PROMPT_BYTES}}})
            }
        }
    }

    fn output_tokens(self, configured: u32) -> u32 {
        match self {
            Self::Files => configured,
            Self::ReaderPlan => configured.min(2048),
        }
    }

    fn response_limit(self) -> u64 {
        match self {
            Self::Files => MAX_RESPONSE,
            // Includes the provider envelope and JSON escaping of the prompt.
            Self::ReaderPlan => 256 * 1024,
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ReaderPlan {
    prompt: String,
}

pub(crate) fn reader_planning_tokens(settings: &Settings) -> u32 {
    Contract::ReaderPlan.output_tokens(settings.provider.max_output_tokens)
}

/// An explicit aggregate token limit always wins. Otherwise the one planning
/// request has its own reservation, in addition to the normal code budget.
pub(crate) fn reader_output_budget(settings: &Settings) -> u32 {
    settings
        .execution
        .max_output_tokens_total
        .unwrap_or_else(|| {
            settings
                .execution
                .output_budget(settings.provider.max_output_tokens)
                .saturating_add(reader_planning_tokens(settings))
        })
}

pub(crate) struct Session {
    settings: Settings,
    started: Instant,
    requests: u8,
    source_requests: u8,
    reserved_tokens: u32,
    credential: CredentialSnapshot,
    reader_plan_attempted: bool,
}

impl Session {
    pub fn new(settings: &Settings) -> Self {
        Self {
            settings: settings.clone(),
            started: Instant::now(),
            requests: 0,
            source_requests: 0,
            reserved_tokens: 0,
            credential: Arc::new(OnceLock::new()),
            reader_plan_attempted: false,
        }
    }
    pub fn generate(&mut self, system: &str, user: &str) -> Result<GeneratedOutput> {
        self.generate_response(system, user, Contract::Files)
    }

    #[cfg(test)]
    pub fn requests(&self) -> u8 {
        self.requests
    }

    fn output_budget(&self) -> u32 {
        if self.reader_plan_attempted {
            reader_output_budget(&self.settings)
        } else {
            self.settings
                .execution
                .output_budget(self.settings.provider.max_output_tokens)
        }
    }

    /// Planning has a separate request allowance, but an explicit aggregate
    /// token cap can still reduce the remaining full source-generation requests.
    pub fn remaining_requests(&self) -> u8 {
        let tokens = self.settings.provider.max_output_tokens;
        let remaining_tokens = self.output_budget().saturating_sub(self.reserved_tokens);
        let by_tokens = remaining_tokens.checked_div(tokens).unwrap_or(0);
        self.settings
            .execution
            .max_provider_requests
            .saturating_sub(self.source_requests)
            .min(by_tokens.min(u8::MAX as u32) as u8)
    }

    /// One planning attempt per session, including failures. The caller supplies
    /// only a type identifier and the reader contract, never document data.
    /// Planning has one separate request and a short derived token allowance.
    /// Explicit aggregate token limits, elapsed time and credentials are shared.
    pub fn plan_reader(&mut self, system: &str, user: &str) -> Result<String> {
        if self.reader_plan_attempted {
            bail!("Reader planning was already attempted; no second provider request was made");
        }
        self.reader_plan_attempted = true;
        let plan: ReaderPlan = self.generate_response(system, user, Contract::ReaderPlan)?;
        let prompt = plan.prompt.trim();
        if prompt.is_empty()
            || prompt.len() > MAX_READER_PROMPT_BYTES
            || prompt
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        {
            bail!("Reader plan must contain a nonempty prompt of at most 16384 UTF-8 bytes without control characters");
        }
        Ok(prompt.to_owned())
    }

    fn generate_response<T: DeserializeOwned + Send + 'static>(
        &mut self,
        system: &str,
        user: &str,
        contract: Contract,
    ) -> Result<T> {
        let policy = &self.settings.execution;
        if policy.max_provider_requests == 0
            || (matches!(contract, Contract::Files)
                && self.source_requests >= policy.max_provider_requests)
        {
            bail!("Local provider request budget exhausted");
        }
        if system.len().saturating_add(user.len()) > policy.max_prompt_bytes {
            bail!("Prompt exceeds local max_prompt_bytes; no provider request was made");
        }
        let reserved = self
            .reserved_tokens
            .saturating_add(contract.output_tokens(self.settings.provider.max_output_tokens));
        if reserved > self.output_budget() {
            bail!("Local output-token budget exhausted; no provider request was made");
        }
        let remaining = Duration::from_secs(policy.generation_timeout_seconds)
            .checked_sub(self.started.elapsed())
            .filter(|v| v.as_secs() > 0)
            .context("Local generation time budget exhausted")?;
        self.requests += 1;
        if matches!(contract, Contract::Files) {
            self.source_requests += 1;
        }
        let detail = match contract {
            Contract::Files if self.reader_plan_attempted => format!(
                "Provider: {} ({})\nModelo: {}\nGerando leitor — tentativa {}",
                self.settings.profile,
                self.settings.provider.kind.label(),
                self.settings.provider.model,
                self.source_requests,
            ),
            Contract::Files => format!(
                "Provider: {} ({})\nModelo: {}\nTentativa {} de {} — até {} correções",
                self.settings.profile,
                self.settings.provider.kind.label(),
                self.settings.provider.model,
                self.source_requests,
                policy.attempts(self.settings.provider.max_output_tokens, None),
                policy.attempts(self.settings.provider.max_output_tokens, None).saturating_sub(1)
            ),
            Contract::ReaderPlan => format!(
                "Provider: {} ({})\nModelo: {}\nPlanejando o leitor — uma única solicitação, sem repetição automática",
                self.settings.profile,
                self.settings.provider.kind.label(),
                self.settings.provider.model,
            ),
        };
        super::presentation::details(&detail);
        self.reserved_tokens = reserved;
        let mut settings = self.settings.clone();
        settings.provider.timeout_seconds =
            settings.provider.timeout_seconds.min(remaining.as_secs());
        generate(&settings, system, user, self.credential.clone(), contract)
    }
}

fn generate<T: DeserializeOwned + Send + 'static>(
    settings: &Settings,
    system: &str,
    user: &str,
    credential: CredentialSnapshot,
    contract: Contract,
) -> Result<T> {
    if super::executor::cancelled() {
        bail!("Cancelled");
    }
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let settings = settings.clone();
    let system = system.to_owned();
    let user = user.to_owned();
    std::thread::spawn(move || {
        let _ = send.send(request(&settings, &system, &user, &credential, contract));
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

fn request<T: DeserializeOwned>(
    settings: &Settings,
    system: &str,
    user: &str,
    credential: &CredentialSnapshot,
    contract: Contract,
) -> Result<T> {
    let p = &settings.provider;
    let key = session_key(settings, credential)?;
    let client = Client::builder()
        .timeout(Duration::from_secs(p.timeout_seconds))
        .connect_timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let messages = json!([
        {"role": "system", "content": format!("{system}\n{}", contract.instruction(p.kind))},
        {"role": "user", "content": user}
    ]);
    let max_output_tokens = contract.output_tokens(p.max_output_tokens);
    let (endpoint, payload) = match p.kind {
        Kind::Ollama => (
            "api/chat",
            json!({
                "model": p.model, "messages": messages, "stream": false,
                "format": contract.schema(),
                "think": p.thinking, "keep_alive": p.keep_alive,
                "options": {"num_ctx": p.context_tokens, "num_predict": max_output_tokens, "temperature": 0.2}
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
            }] = json!(max_output_tokens);
            if !reasoning {
                payload["temperature"] = json!(0.2);
            }
            ("chat/completions", payload)
        }
        Kind::Deepseek => {
            let thinking = p.thinking.unwrap_or(false);
            let mut payload = json!({
                "model": p.model, "messages": messages, "stream": false,
                "max_tokens": max_output_tokens,
                "response_format": {"type": "json_object"},
                "thinking": {"type": if thinking { "enabled" } else { "disabled" }}
            });
            if !thinking {
                payload["temperature"] = json!(0.2);
            }
            ("chat/completions", payload)
        }
    };
    let operation = match contract {
        Contract::Files => "Generating",
        Contract::ReaderPlan => "Planning reader",
    };
    println!(
        "{operation} with provider={}, model={}",
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
    let limit = contract.response_limit();
    response
        .take(limit + 1)
        .read_to_end(&mut body)
        .context("Cannot read provider response")?;
    if body.len() as u64 > limit {
        bail!("Provider response exceeded the local {limit}-byte response limit");
    }
    decode_response(p.kind, &body, contract)
}

#[cfg(test)]
fn decode(kind: Kind, body: &[u8]) -> Result<GeneratedOutput> {
    decode_response(kind, body, Contract::Files)
}

fn decode_response<T: DeserializeOwned>(kind: Kind, body: &[u8], contract: Contract) -> Result<T> {
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
                bail!("{} response incomplete or refused; no output was accepted", kind.label());
            }
            choice["message"]["content"].as_str()
        }
    }.context("Provider response has no message content")?;
    if content.trim().is_empty() {
        bail!("Provider returned empty content; no output was accepted");
    }
    let generated = serde_json::from_str(&super::extract_json(content)).map_err(|_| {
        anyhow::anyhow!(match contract {
            Contract::Files => "Provider did not return the required files JSON object",
            Contract::ReaderPlan =>
                "Provider did not return the required reader prompt JSON object",
        })
    })?;
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

    fn reader_settings(directory: &std::path::Path, kind: Kind, url: &str) -> Settings {
        let path = directory.join("reader-config.toml");
        let env_file = directory.join("reader.env");
        std::fs::write(
            &path,
            include_str!("../config.example.toml")
                .replace("http://127.0.0.1:11434", url)
                .replace("https://api.openai.com/v1", url)
                .replace("https://api.deepseek.com", url)
                .replace("crexe_openai_api_key_env", "CREXE_READER_OPENAI_SYNTHETIC")
                .replace("crexe_deepseek_api_key", "CREXE_READER_DEEPSEEK_SYNTHETIC"),
        )
        .unwrap();
        std::fs::write(
            &env_file,
            "CREXE_READER_OPENAI_SYNTHETIC=synthetic-reader-only\nCREXE_READER_DEEPSEEK_SYNTHETIC=synthetic-reader-only\n",
        )
        .unwrap();
        let profile = match kind {
            Kind::Ollama => "local",
            Kind::Openai => "openai",
            Kind::Deepseek => "deepseek",
        };
        Settings::load(Some(&path), Some(profile), None, Some(&env_file)).unwrap()
    }

    fn reader_body(kind: Kind, content: Value, finish: &str) -> String {
        match kind {
            Kind::Ollama => json!({"done":true,"done_reason":finish,"message":{"content":content}}),
            Kind::Openai | Kind::Deepseek => {
                json!({"choices":[{"finish_reason":finish,"message":{"content":content}}]})
            }
        }
        .to_string()
    }

    fn reader_request(stream: &mut std::net::TcpStream, kind: Kind) -> Value {
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
        let endpoint = if kind == Kind::Ollama {
            "/api/chat"
        } else {
            "/chat/completions"
        };
        assert!(header.starts_with(&format!("post {endpoint} http/1.1")));
        assert_eq!(
            header.contains("authorization: bearer synthetic-reader-only"),
            kind != Kind::Ollama
        );
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
        serde_json::from_slice(&bytes[start..start + length]).unwrap()
    }

    fn reader_exchange(kind: Kind, status: u16, body: String) -> (Result<String>, Value) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let payload = reader_request(&mut stream, kind);
            let _ = write!(
                stream,
                "HTTP/1.1 {status} Result\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            payload
        });
        let temp = tempfile::tempdir().unwrap();
        let settings = reader_settings(temp.path(), kind, &url);
        let mut session = Session::new(&settings);
        let result = session.plan_reader(
            "Plan a local reader. Receive only the type identifier.",
            r#"{"extension":"invented-reader-test"}"#,
        );
        assert_eq!(session.requests, 1);
        assert_eq!(session.reserved_tokens, 2048);
        // Failure, refusal and malformed output must not permit a hidden retry.
        let second = session.plan_reader("same", "same").unwrap_err().to_string();
        assert!(second.contains("already attempted"));
        assert_eq!(session.requests, 1);
        (result, server.join().unwrap())
    }

    #[test]
    fn reader_planning_uses_one_prompt_request_for_each_provider() {
        let prompt =
            "Build a simple efficient reader that reads its first native path argument locally.";
        for kind in [Kind::Ollama, Kind::Openai, Kind::Deepseek] {
            let (result, payload) = reader_exchange(
                kind,
                200,
                reader_body(kind, json!(json!({"prompt": prompt}).to_string()), "stop"),
            );
            assert_eq!(result.unwrap(), prompt);
            let instruction = payload["messages"][0]["content"].as_str().unwrap();
            assert!(instruction.contains("exactly one field: prompt"));
            assert!(!instruction.contains("\"files\""));
            assert!(!instruction.contains("required source file"));
            assert_eq!(
                payload["messages"][1]["content"],
                r#"{"extension":"invented-reader-test"}"#
            );
            if kind == Kind::Ollama {
                assert_eq!(payload["format"]["required"], json!(["prompt"]));
                assert_eq!(payload["format"]["additionalProperties"], false);
                assert!(payload["format"]["properties"].get("files").is_none());
                assert_eq!(payload["options"]["num_predict"], 2048);
            } else {
                assert_eq!(payload["response_format"]["type"], "json_object");
                assert_eq!(payload["max_tokens"], 2048);
            }
        }
    }

    #[test]
    fn one_reader_plan_preserves_code_attempts_on_every_provider() {
        for kind in [Kind::Ollama, Kind::Openai, Kind::Deepseek] {
            for (max_requests, token_limited_attempts) in [(1, None), (3, None), (3, Some(1))] {
                let code_attempts = token_limited_attempts.unwrap_or(max_requests);
                let listener = TcpListener::bind("127.0.0.1:0").unwrap();
                let url = format!("http://{}", listener.local_addr().unwrap());
                let server = std::thread::spawn(move || {
                    let mut requests = Vec::new();
                    for index in 0..=code_attempts {
                        let (mut stream, _) = listener.accept().unwrap();
                        requests.push(reader_request(&mut stream, kind));
                        let content = if index == 0 {
                            json!({"prompt":"Build a simple local reader"})
                        } else {
                            json!({"files":[{"path":"main.c","content":"int main(void) { return 0; }"}]})
                        };
                        let body = reader_body(kind, json!(content.to_string()), "stop");
                        write!(
                            stream,
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        )
                        .unwrap();
                    }
                    requests
                });
                let temp = tempfile::tempdir().unwrap();
                let mut settings = reader_settings(temp.path(), kind, &url);
                settings.execution.max_provider_requests = max_requests;
                settings.execution.max_output_tokens_total = token_limited_attempts.map(|count| {
                    reader_planning_tokens(&settings)
                        + u32::from(count) * settings.provider.max_output_tokens
                });
                let mut session = Session::new(&settings);
                let plan = session.plan_reader("Plan a reader", "eml").unwrap();
                assert_eq!(session.requests(), 1);
                assert_eq!(session.remaining_requests(), code_attempts);
                for remaining in (0..code_attempts).rev() {
                    assert_eq!(
                        session
                            .generate("Generate the reader", &plan)
                            .unwrap()
                            .files
                            .len(),
                        1
                    );
                    assert_eq!(session.remaining_requests(), remaining);
                }
                let error = session.generate("Generate the reader", &plan).unwrap_err();
                assert!(format!("{error:#}").contains("budget exhausted"));
                assert!(session.plan_reader("Plan again", "eml").is_err());
                assert_eq!(session.requests(), code_attempts + 1);
                assert_eq!(session.source_requests, code_attempts);
                let requests = server.join().unwrap();
                assert_eq!(requests.len(), usize::from(code_attempts) + 1);
                for (index, request) in requests.iter().enumerate() {
                    let instruction = request["messages"][0]["content"].as_str().unwrap();
                    assert_eq!(
                        instruction.contains("exactly one field: prompt"),
                        index == 0
                    );
                }
            }
        }
    }

    #[test]
    fn reader_planning_rejects_wrong_empty_oversize_and_control_outputs_once() {
        for output in [
            json!({"files":[{"path":"main.c","content":"int main(){}"}]}),
            json!({"prompt":true}),
            json!({"prompt":" \r\n\t"}),
            json!({"prompt":"reader", "files":[]}),
            json!({"prompt":"reader\u{0000}"}),
            json!({"prompt":"x".repeat(MAX_READER_PROMPT_BYTES + 1)}),
            json!({"prompt":"é".repeat(MAX_READER_PROMPT_BYTES / 2 + 1)}),
        ] {
            let (result, _) = reader_exchange(
                Kind::Ollama,
                200,
                reader_body(Kind::Ollama, json!(output.to_string()), "stop"),
            );
            assert!(result.is_err());
        }
        let prompt = "x".repeat(MAX_READER_PROMPT_BYTES);
        let (result, _) = reader_exchange(
            Kind::Ollama,
            200,
            reader_body(
                Kind::Ollama,
                json!(json!({"prompt":prompt}).to_string()),
                "stop",
            ),
        );
        assert_eq!(result.unwrap().len(), MAX_READER_PROMPT_BYTES);
    }

    #[test]
    fn reader_planning_provider_failures_never_retry_or_accept_partial_plans() {
        for kind in [Kind::Ollama, Kind::Openai, Kind::Deepseek] {
            let (result, _) =
                reader_exchange(kind, 429, "private-response-synthetic-reader-only".into());
            let error = format!("{:#}", result.unwrap_err());
            assert!(error.contains("429"));
            assert!(
                !error.contains("private-response") && !error.contains("synthetic-reader-only")
            );
            for content in [
                Value::Null,
                json!(""),
                json!("not-json"),
                json!("{\"prompt\":"),
            ] {
                assert!(
                    reader_exchange(kind, 200, reader_body(kind, content, "stop"))
                        .0
                        .is_err()
                );
            }
            let good = json!(json!({"prompt":"Build the local reader"}).to_string());
            assert!(
                reader_exchange(kind, 200, reader_body(kind, good, "length"))
                    .0
                    .is_err()
            );
        }
        let (result, _) = reader_exchange(Kind::Ollama, 200, " ".repeat(256 * 1024 + 1));
        assert!(result.unwrap_err().to_string().contains("response limit"));
    }

    #[test]
    fn reader_planning_respects_request_prompt_time_and_output_budgets() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let temp = tempfile::tempdir().unwrap();
        let mut settings = reader_settings(temp.path(), Kind::Ollama, &url);
        settings.execution.max_provider_requests = 0;
        let mut session = Session::new(&settings);
        assert!(session
            .plan_reader("reader", "type")
            .unwrap_err()
            .to_string()
            .contains("request budget"));
        assert_eq!(session.requests, 0);
        settings.execution.max_provider_requests = 1;
        let mut session = Session::new(&settings);
        let oversized = "x".repeat(settings.execution.max_prompt_bytes + 1);
        assert!(session
            .plan_reader(&oversized, "type")
            .unwrap_err()
            .to_string()
            .contains("max_prompt_bytes"));
        assert_eq!(session.requests, 0);
        settings.execution.max_output_tokens_total = Some(
            settings
                .execution
                .output_budget(settings.provider.max_output_tokens),
        );
        let mut session = Session::new(&settings);
        session.started =
            Instant::now() - Duration::from_secs(settings.execution.generation_timeout_seconds + 1);
        assert!(session
            .plan_reader("reader", "type")
            .unwrap_err()
            .to_string()
            .contains("time budget"));
        assert_eq!(session.requests, 0);
        let mut session = Session::new(&settings);
        session.reserved_tokens = settings
            .execution
            .output_budget(settings.provider.max_output_tokens);
        assert!(session
            .plan_reader("reader", "type")
            .unwrap_err()
            .to_string()
            .contains("output-token budget"));
        assert_eq!(session.requests, 0);
        assert!(
            matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
    }

    #[test]
    fn planning_preserves_source_attempts_but_respects_aggregate_token_cap() {
        let temp = tempfile::tempdir().unwrap();
        let mut settings = reader_settings(temp.path(), Kind::Ollama, "http://127.0.0.1:1");
        settings.execution.max_provider_requests = 3;
        let mut session = Session::new(&settings);
        assert_eq!(session.remaining_requests(), 3);
        session.requests = 1;
        session.reader_plan_attempted = true;
        session.reserved_tokens =
            Contract::ReaderPlan.output_tokens(settings.provider.max_output_tokens);
        assert_eq!(session.requests(), 1);
        assert_eq!(session.remaining_requests(), 3);
        settings.execution.max_output_tokens_total = Some(
            Contract::ReaderPlan.output_tokens(settings.provider.max_output_tokens)
                + settings.provider.max_output_tokens,
        );
        let mut capped = Session::new(&settings);
        capped.requests = 1;
        capped.reader_plan_attempted = true;
        capped.reserved_tokens =
            Contract::ReaderPlan.output_tokens(settings.provider.max_output_tokens);
        assert_eq!(capped.remaining_requests(), 1);
        capped.source_requests = 1;
        capped.reserved_tokens += settings.provider.max_output_tokens;
        assert_eq!(capped.remaining_requests(), 0);
    }

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
