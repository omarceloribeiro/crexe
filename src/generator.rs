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
    let messages = json!([
        {"role": "system", "content": format!("{system}\nReturn only a JSON object with files: an array of objects with path and content (strings). Include every required source file. No prose outside JSON.")},
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
        bail!("Provider returned HTTP {status}; check model availability, service and credentials. No automatic fallback was attempted.");
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
        Kind::Openai => {
            let choice = &response["choices"][0];
            if choice["finish_reason"] != "stop" || choice["message"]["refusal"].is_string() { bail!("OpenAI response incomplete or refused; no files were accepted"); }
            choice["message"]["content"].as_str()
        }
    }.context("Provider response has no message content")?;
    serde_json::from_str(&super::extract_json(content))
        .context("Provider did not return the required files JSON object")
}

#[cfg(test)]
mod tests {
    use super::*;
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
