use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use regex::Regex;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use serde_yaml::{Mapping, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const RUNTIME_VERSION: &str = "1.0-rust";

#[derive(Parser, Debug)]
#[command(name = "crexe")]
#[command(about = "CREXE v1 Rust executor", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate + build + run a .crexe file
    Exec {
        /// Path to .crexe file
        file: PathBuf,
        /// Override inputs: KEY=VALUE
        #[arg(long = "set")]
        set: Vec<String>,
        /// Ignore cache and regenerate/rebuild
        #[arg(long = "rebuild", default_value_t = false)]
        rebuild: bool,
    },
}

#[derive(Debug, Clone)]
struct RuntimeVars {
    os: String,
    arch: String,
    exe_ext: String,
    user_home: String,
}

#[derive(Debug)]
struct CacheState {
    workspace: PathBuf,
    fingerprint: String,
    manifest_path: PathBuf,
    build_ok_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct LlmResponse {
    choices: Vec<LlmChoice>,
}

#[derive(Debug, Deserialize)]
struct LlmChoice {
    message: LlmMessage,
}

#[derive(Debug, Deserialize)]
struct LlmMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct GeneratedOutput {
    files: Vec<GeneratedFile>,
    #[allow(dead_code)]
    hints: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GeneratedFile {
    path: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct Manifest<'a> {
    fingerprint: &'a str,
    crexe_path: String,
    runtime_version: &'a str,
    generated_at: String,
    inputs: BTreeMap<String, String>,
}

fn main() -> Result<()> {
    let raw_args: Vec<String> = env::args().collect();

    // Windows drag-and-drop / "Open with" support:
    //
    // Explorer calls:
    //   crexe.exe "C:\path\file.crexe"
    //
    // Standard CLI still works:
    //   crexe.exe exec "C:\path\file.crexe"
    if let Some((file, set, rebuild)) = try_parse_direct_crexe_invocation(&raw_args)? {
        let overrides = parse_set_args(&set)?;
        exec_crexe(&file, overrides, rebuild)?;
        return Ok(());
    }

    let cli = Cli::parse();

    match cli.command {
        Commands::Exec { file, set, rebuild } => {
            let overrides = parse_set_args(&set)?;
            exec_crexe(&file, overrides, rebuild)?;
        }
    }

    Ok(())
}

fn try_parse_direct_crexe_invocation(args: &[String]) -> Result<Option<(PathBuf, Vec<String>, bool)>> {
    if args.len() < 2 {
        return Ok(None);
    }

    let first = PathBuf::from(&args[1]);

    // If first user argument is not a .crexe file, let clap handle normal commands.
    if !looks_like_crexe_file(&first) {
        return Ok(None);
    }

    let mut set_values = Vec::new();
    let mut rebuild = false;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--rebuild" => {
                rebuild = true;
                i += 1;
            }
            "--set" => {
                let Some(value) = args.get(i + 1) else {
                    bail!("--set requires KEY=VALUE");
                };
                set_values.push(value.clone());
                i += 2;
            }
            other => {
                bail!(
                    "Unexpected argument after .crexe file: {}. Supported: --set KEY=VALUE, --rebuild",
                    other
                );
            }
        }
    }

    Ok(Some((first, set_values, rebuild)))
}

fn looks_like_crexe_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("crexe"))
        .unwrap_or(false)
}

fn exec_crexe(file: &Path, overrides: BTreeMap<String, String>, rebuild: bool) -> Result<()> {
    let crexe_path = file.canonicalize().with_context(|| format!("CREXE not found: {}", file.display()))?;
    let raw = fs::read_to_string(&crexe_path)?;
    let spec: Value = serde_yaml::from_str(&raw).context("Invalid YAML")?;

    let inputs = resolve_inputs(&spec, overrides)?;
    let runtime = detect_runtime_vars();

    let mut ctx = BTreeMap::new();
    for (k, v) in &inputs {
        ctx.insert(k.clone(), v.clone());
    }
    ctx.insert("OS".to_string(), runtime.os.clone());
    ctx.insert("ARCH".to_string(), runtime.arch.clone());
    ctx.insert("EXE_EXT".to_string(), runtime.exe_ext.clone());
    ctx.insert("USER_HOME".to_string(), runtime.user_home.clone());

    let target = select_target(&spec, &ctx)?;
    println!("Target: {} (OS={}, ARCH={})", target, runtime.os, runtime.arch);

    let cache = get_cache_state(&crexe_path, raw.as_bytes(), &spec, &inputs)?;
    println!("Workspace: {}", cache.workspace.display());

    if !rebuild && is_cache_hit(&cache)? {
        println!("Cache hit: skipping generate/build");
        let run_cmd = get_cmd(&spec, &["targets", &target, "run", "cmd"])?;
        run_command(&spec, &run_cmd, &cache.workspace, "run", &runtime.os, &ctx)?;
        return Ok(());
    }

    let (system_prompt, user_prompt) = build_prompt(&spec, &target, &ctx)?;
    let generated = call_llm(&spec, &system_prompt, &user_prompt)?;
    apply_generated_files(&cache.workspace, &generated.files, &ctx)?;

    let build_steps = get_build_steps(&spec, &target)?;
    for cmd in build_steps {
        run_command(&spec, &cmd, &cache.workspace, "build", &runtime.os, &ctx)?;
    }

    write_manifest(&cache, &crexe_path, &inputs)?;

    let run_cmd = get_cmd(&spec, &["targets", &target, "run", "cmd"])?;
    run_command(&spec, &run_cmd, &cache.workspace, "run", &runtime.os, &ctx)?;

    Ok(())
}

fn parse_set_args(items: &[String]) -> Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    for item in items {
        let Some((k, v)) = item.split_once('=') else {
            bail!("--set must be KEY=VALUE, got: {}", item);
        };
        map.insert(k.trim().to_string(), v.trim().to_string());
    }
    Ok(map)
}

fn detect_runtime_vars() -> RuntimeVars {
    let os = match env::consts::OS {
        "windows" => "windows",
        "macos" => "macos",
        "linux" => "linux",
        other => other,
    }
    .to_string();

    let exe_ext = if os == "windows" { ".exe" } else { "" }.to_string();
    let user_home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());

    RuntimeVars {
        os,
        arch: env::consts::ARCH.to_string(),
        exe_ext,
        user_home,
    }
}

fn resolve_inputs(spec: &Value, overrides: BTreeMap<String, String>) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    if let Some(inputs) = get_mapping(spec, &["inputs"]) {
        for (k, v) in inputs {
            let key = k.as_str().ok_or_else(|| anyhow!("inputs keys must be strings"))?;
            let default = get_path(v, &["default"])
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            out.insert(key.to_string(), default);
        }
    }
    for (k, v) in overrides {
        out.insert(k, v);
    }
    Ok(out)
}

fn select_target(spec: &Value, ctx: &BTreeMap<String, String>) -> Result<String> {
    let fallback = get_path(spec, &["selectors", "target", "fallback"])
        .and_then(Value::as_str)
        .unwrap_or("linux")
        .to_string();

    let Some(rules) = get_path(spec, &["selectors", "target", "rules"]).and_then(Value::as_sequence) else {
        return Ok(fallback);
    };

    for rule in rules {
        let when = get_path(rule, &["when"]).and_then(Value::as_str).unwrap_or("");
        let use_target = get_path(rule, &["use"]).and_then(Value::as_str);
        if let Some(use_target) = use_target {
            if eval_condition(when, ctx) {
                return Ok(use_target.to_string());
            }
        }
    }

    Ok(fallback)
}

fn eval_condition(expr: &str, ctx: &BTreeMap<String, String>) -> bool {
    // Supports only: {{KEY}} == 'value' and {{KEY}} != 'value'
    let re = Regex::new(r#"^\s*\{\{([A-Za-z_][A-Za-z0-9_]*)\}\}\s*(==|!=)\s*'([^']*)'\s*$"#).unwrap();
    let Some(caps) = re.captures(expr) else {
        return false;
    };
    let key = caps.get(1).unwrap().as_str();
    let op = caps.get(2).unwrap().as_str();
    let expected = caps.get(3).unwrap().as_str();
    let actual = ctx.get(key).map(String::as_str).unwrap_or("");
    match op {
        "==" => actual == expected,
        "!=" => actual != expected,
        _ => false,
    }
}

fn build_prompt(spec: &Value, target: &str, ctx: &BTreeMap<String, String>) -> Result<(String, String)> {
    let system_raw = get_path(spec, &["prompt", "system"])
        .and_then(Value::as_str)
        .unwrap_or("");

    let user_raw = get_path(spec, &["prompt", "user_by_target", target])
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Missing prompt.user_by_target.{}", target))?;

    let system = render_template(system_raw, spec, ctx);
    let user = render_template(&render_template(user_raw, spec, ctx), spec, ctx);
    Ok((system, user))
}

fn render_template(text: &str, spec: &Value, ctx: &BTreeMap<String, String>) -> String {
    let re = Regex::new(r#"\{\{([A-Za-z_][A-Za-z0-9_\.]*?)\}\}"#).unwrap();
    re.replace_all(text, |caps: &regex::Captures| {
        let key = caps.get(1).unwrap().as_str();
        if let Some(v) = ctx.get(key) {
            return v.clone();
        }
        if key.contains('.') {
            let parts: Vec<&str> = key.split('.').collect();
            if let Some(v) = get_path(spec, &parts).and_then(Value::as_str) {
                return v.to_string();
            }
        }
        caps.get(0).unwrap().as_str().to_string()
    })
    .to_string()
}

fn call_llm(spec: &Value, system_prompt: &str, user_prompt: &str) -> Result<GeneratedOutput> {
    let generator = get_path(spec, &["generator"]).unwrap_or(&Value::Null);
    let provider = get_path(generator, &["provider"]).and_then(Value::as_str).unwrap_or("openai-compatible");

    if provider != "openai-compatible" && provider != "openai" {
        bail!("Unsupported provider: {}", provider);
    }

    let transport = get_path(generator, &["transport"])
        .and_then(Value::as_str)
        .unwrap_or("chat_completions");

    match transport {
        "chat_completions" | "chat-completions" | "chat" => {
            call_llm_chat_completions(spec, system_prompt, user_prompt)
        }
        "responses" | "response" => {
            call_llm_responses(spec, system_prompt, user_prompt)
        }
        other => bail!("Unsupported generator.transport: {}", other),
    }
}

fn call_llm_chat_completions(spec: &Value, system_prompt: &str, user_prompt: &str) -> Result<GeneratedOutput> {
    let generator = get_path(spec, &["generator"]).unwrap_or(&Value::Null);
    let provider = get_path(generator, &["provider"]).and_then(Value::as_str).unwrap_or("openai-compatible");

    let base_url = get_path(generator, &["baseUrl"]).and_then(Value::as_str).unwrap_or("https://api.openai.com/v1");
    let api_key_env = get_path(generator, &["apiKeyEnv"]).and_then(Value::as_str).unwrap_or("OPENAI_API_KEY");
    let api_key = env::var(api_key_env).with_context(|| format!("Missing API key env var: {}", api_key_env))?;
    let model = get_path(generator, &["model"]).and_then(Value::as_str).unwrap_or("gpt-4o-mini");
    let temperature = get_path(generator, &["temperature"]).and_then(Value::as_f64).unwrap_or(0.2);
    let max_tokens = get_path(generator, &["maxOutputTokens"]).and_then(Value::as_i64).unwrap_or(6000);
    let timeout = timeout_seconds(spec, "generate");

    println!("LLM: provider={}, transport=chat_completions, baseUrl={}, model={}", provider, base_url, model);

    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout))
        .build()?;

    let token_param = get_path(generator, &["tokenParameter"])
        .and_then(Value::as_str)
        .unwrap_or("auto");

    let mut use_max_completion_tokens =
        token_param == "max_completion_tokens"
        || (token_param == "auto" && model.to_ascii_lowercase().starts_with("gpt-5"));

    let mut send_temperature = get_path(generator, &["sendTemperature"])
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let mut last_error = String::new();

    for _attempt in 0..3 {
        let mut payload = json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ]
        });

        if send_temperature {
            payload["temperature"] = json!(temperature);
        }

        if use_max_completion_tokens {
            payload["max_completion_tokens"] = json!(max_tokens);
        } else {
            payload["max_tokens"] = json!(max_tokens);
        }

        let resp = client
            .post(&url)
            .bearer_auth(&api_key)
            .json(&payload)
            .send()?;

        let status = resp.status();
        let body = resp.text().unwrap_or_default();

        if status.is_success() {
            let llm: LlmResponse = serde_json::from_str(&body)?;
            let content = llm
                .choices
                .first()
                .ok_or_else(|| anyhow!("LLM response has no choices"))?
                .message
                .content
                .clone();

            return parse_generated_output_from_text(&content);
        }

        last_error = format!("LLM HTTP {}: {}", status, truncate(&body, 700));
        let body_lower = body.to_ascii_lowercase();

        if status.as_u16() == 400
            && body_lower.contains("max_tokens")
            && body_lower.contains("max_completion_tokens")
            && !use_max_completion_tokens
        {
            println!("LLM retry: switching max_tokens -> max_completion_tokens");
            use_max_completion_tokens = true;
            continue;
        }

        if status.as_u16() == 400
            && body_lower.contains("temperature")
            && (body_lower.contains("unsupported") || body_lower.contains("not support"))
            && send_temperature
        {
            println!("LLM retry: removing temperature");
            send_temperature = false;
            continue;
        }

        bail!("{}", last_error);
    }

    bail!("{}", last_error)
}

fn call_llm_responses(spec: &Value, system_prompt: &str, user_prompt: &str) -> Result<GeneratedOutput> {
    let generator = get_path(spec, &["generator"]).unwrap_or(&Value::Null);
    let provider = get_path(generator, &["provider"]).and_then(Value::as_str).unwrap_or("openai");

    let base_url = get_path(generator, &["baseUrl"]).and_then(Value::as_str).unwrap_or("https://api.openai.com/v1");
    let api_key_env = get_path(generator, &["apiKeyEnv"]).and_then(Value::as_str).unwrap_or("OPENAI_API_KEY");
    let api_key = env::var(api_key_env).with_context(|| format!("Missing API key env var: {}", api_key_env))?;
    let model = get_path(generator, &["model"]).and_then(Value::as_str).unwrap_or("gpt-5.5-pro");
    let max_output_tokens = get_path(generator, &["maxOutputTokens"]).and_then(Value::as_i64).unwrap_or(6000);
    let timeout = timeout_seconds(spec, "generate");

    println!("LLM: provider={}, transport=responses, baseUrl={}, model={}", provider, base_url, model);

    let url = format!("{}/responses", base_url.trim_end_matches('/'));
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout))
        .build()?;

    let mut payload = json!({
        "model": model,
        "input": [
            {
                "role": "system",
                "content": [
                    { "type": "input_text", "text": system_prompt }
                ]
            },
            {
                "role": "user",
                "content": [
                    { "type": "input_text", "text": user_prompt }
                ]
            }
        ],
        "max_output_tokens": max_output_tokens
    });

    // Optional reasoning config for models that support it:
    // generator:
    //   reasoning:
    //     effort: "high"
    if let Some(reasoning) = get_path(generator, &["reasoning"]) {
        if let Ok(reasoning_json) = serde_json::to_value(reasoning) {
            payload["reasoning"] = reasoning_json;
        }
    }

    // Some models may reject temperature in Responses. Default is false for Responses.
    let send_temperature = get_path(generator, &["sendTemperature"])
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if send_temperature {
        let temperature = get_path(generator, &["temperature"]).and_then(Value::as_f64).unwrap_or(0.2);
        payload["temperature"] = json!(temperature);
    }

    let resp = client
        .post(&url)
        .bearer_auth(&api_key)
        .json(&payload)
        .send()?;

    let status = resp.status();
    let body = resp.text().unwrap_or_default();

    if !status.is_success() {
        bail!("LLM HTTP {}: {}", status, truncate(&body, 1000));
    }

    let value: JsonValue = serde_json::from_str(&body)
        .with_context(|| format!("Invalid Responses API JSON. First chars: {}", truncate(&body, 500)))?;

    let text = extract_responses_text(&value)?;
    parse_generated_output_from_text(&text)
}

fn parse_generated_output_from_text(text: &str) -> Result<GeneratedOutput> {
    let json_text = extract_json(text);
    let out: GeneratedOutput = serde_json::from_str(&json_text)
        .with_context(|| format!("LLM did not return valid JSON. First chars: {}", truncate(&json_text, 500)))?;

    if out.files.is_empty() {
        bail!("LLM output contains empty files[]");
    }

    Ok(out)
}

fn extract_responses_text(value: &JsonValue) -> Result<String> {
    if let Some(text) = value.get("output_text").and_then(JsonValue::as_str) {
        if !text.trim().is_empty() {
            return Ok(text.to_string());
        }
    }

    let mut result = String::new();

    if let Some(output) = value.get("output").and_then(JsonValue::as_array) {
        for item in output {
            if let Some(content) = item.get("content").and_then(JsonValue::as_array) {
                for part in content {
                    if let Some(text) = part.get("text").and_then(JsonValue::as_str) {
                        result.push_str(text);
                    }
                }
            }
        }
    }

    if !result.trim().is_empty() {
        return Ok(result);
    }

    bail!("Could not extract text from Responses API output: {}", truncate(&value.to_string(), 1000))
}

fn extract_json(text: &str) -> String {
    let trimmed = text.trim();
    let re = Regex::new(r#"(?is)```(?:json)?\s*(.*?)\s*```"#).unwrap();
    if let Some(caps) = re.captures(trimmed) {
        return caps.get(1).unwrap().as_str().trim().to_string();
    }
    trimmed.to_string()
}

fn apply_generated_files(workspace: &Path, files: &[GeneratedFile], ctx: &BTreeMap<String, String>) -> Result<()> {
    fs::create_dir_all(workspace)?;
    let workspace_abs = workspace.canonicalize().unwrap_or_else(|_| workspace.to_path_buf());

    for file in files {
        let rel = render_template(&file.path, &Value::Null, ctx);
        let out_path = workspace.join(rel);

        if out_path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            bail!("Refusing to write path containing '..': {}", out_path.display());
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let parent_abs = out_path
            .parent()
            .unwrap_or(workspace)
            .canonicalize()
            .unwrap_or_else(|_| workspace_abs.clone());

        if !parent_abs.starts_with(&workspace_abs) {
            bail!("Refusing to write outside workspace: {}", out_path.display());
        }

        fs::write(&out_path, &file.content)?;
        println!("Wrote: {}", out_path.strip_prefix(workspace).unwrap_or(&out_path).display());
    }

    Ok(())
}

fn get_build_steps(spec: &Value, target: &str) -> Result<Vec<Vec<String>>> {
    let seq = get_path(spec, &["targets", target, "build", "steps"])
        .and_then(Value::as_sequence)
        .ok_or_else(|| anyhow!("Missing targets.{}.build.steps", target))?;

    let mut out = Vec::new();
    for step in seq {
        let cmd_val = if let Some(v) = get_path(step, &["cmd"]) { v } else { step };
        out.push(value_to_cmd(cmd_val)?);
    }
    Ok(out)
}

fn get_cmd(spec: &Value, path: &[&str]) -> Result<Vec<String>> {
    let v = get_path(spec, path).ok_or_else(|| anyhow!("Missing command at {}", path.join(".")))?;
    value_to_cmd(v)
}

fn value_to_cmd(v: &Value) -> Result<Vec<String>> {
    if let Some(s) = v.as_str() {
        // Minimal split for convenience. Prefer YAML list commands for reliable quoting.
        return Ok(s.split_whitespace().map(|x| x.to_string()).collect());
    }
    let seq = v.as_sequence().ok_or_else(|| anyhow!("Command must be string or list"))?;
    let mut out = Vec::new();
    for item in seq {
        let s = item.as_str().ok_or_else(|| anyhow!("Command list items must be strings"))?;
        out.push(s.to_string());
    }
    Ok(out)
}


fn is_inside(root: &Path, child: &Path) -> bool {
    let Ok(root) = fs::canonicalize(root) else {
        return false;
    };

    let Ok(child) = fs::canonicalize(child) else {
        return false;
    };

    child.starts_with(root)
}

fn normalize_generated_command_path(cmd0: &str, cwd: &Path) -> PathBuf {
    let p = Path::new(cmd0);

    if p.is_absolute() {
        return p.to_path_buf();
    }

    cwd.join(p)
}

fn is_generated_run_target(cwd: &Path, cmd0: &str) -> bool {
    let candidate = normalize_generated_command_path(cmd0, cwd);

    if candidate.exists() && is_inside(cwd, &candidate) {
        return true;
    }

    // Let build/<app> reach resolve_run_command/fallback logic instead of being blocked by allowlist.
    let build_dir = cwd.join("build");
    candidate.starts_with(&build_dir)
}


fn run_command(
    spec: &Value,
    cmd_raw: &[String],
    cwd: &Path,
    phase: &str,
    os_name: &str,
    ctx: &BTreeMap<String, String>,
) -> Result<()> {
    let mut cmd: Vec<String> = cmd_raw
        .iter()
        .map(|s| render_template(s, spec, ctx))
        .collect();

    if phase == "run" {
        cmd = resolve_run_command(cmd, cwd)?;
    }

    if cmd.is_empty() {
        bail!("Empty command");
    }

    check_allowlist(spec, &cmd, os_name, phase, cwd)?;

    let printable = cmd.join(" ");
    println!("$ (cwd={}) {}", cwd.display(), printable);

    let timeout = timeout_seconds(spec, phase);
    let mut child = Command::new(&cmd[0])
        .args(&cmd[1..])
        .current_dir(cwd)
        .spawn()
        .with_context(|| format!("Failed to start command: {}", cmd[0]))?;

    // v1: wait for the process. GUI processes usually keep running until closed.
    // Timeout is not force-killed in this simple implementation to avoid killing user GUI unexpectedly.
    let status = child.wait()?;
    if !status.success() {
        bail!("Command failed: {}", printable);
    }

    let _ = timeout; // kept for future timeout implementation
    Ok(())
}

fn resolve_run_command(cmd: Vec<String>, cwd: &Path) -> Result<Vec<String>> {
    if cmd.is_empty() {
        bail!("Empty command");
    }

    let first = Path::new(&cmd[0]);
    let first_name = first
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    // Do not rewrite shell commands.
    if ["cmd", "powershell", "sh", "bash"].contains(&first_name.as_str()) {
        return Ok(cmd);
    }

    let expected = normalize_generated_command_path(&cmd[0], cwd);

    if expected.exists() {
        let mut fixed = cmd.clone();

        // Windows/Rust fix:
        // Command::new("build\\CrexeCalculator.exe") may fail even with current_dir set.
        // Always execute generated apps using an absolute path.
        let absolute = fs::canonicalize(&expected).unwrap_or(expected);
        fixed[0] = absolute.display().to_string();

        return Ok(fixed);
    }

    let build_dir = cwd.join("build");
    if build_dir.exists() {
        let mut candidates = Vec::new();
        for entry in fs::read_dir(&build_dir)? {
            let path = entry?.path();
            if path.is_file() && is_probable_executable(&path) {
                candidates.push(path);
            }
        }

        if candidates.len() == 1 {
            let mut fixed = cmd.clone();
            println!(
                "Run fallback: expected '{}' but using '{}'",
                expected.display(),
                candidates[0].display()
            );
            fixed[0] = candidates[0].display().to_string();
            return Ok(fixed);
        }
    }

    Ok(cmd)
}

fn is_probable_executable(path: &Path) -> bool {
    if env::consts::OS == "windows" {
        return path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("exe"))
            .unwrap_or(false);
    }

    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    !matches!(ext, "c" | "cpp" | "h" | "hpp" | "m" | "mm" | "o" | "obj" | "txt" | "json" | "log")
}


fn check_allowlist(spec: &Value, cmd: &[String], os_name: &str, phase: &str, cwd: &Path) -> Result<()> {
    if phase == "run" && is_generated_run_target(cwd, &cmd[0]) {
        return Ok(());
    }
    let Some(list) = get_path(spec, &["policies", "commandAllowlist", os_name]).and_then(Value::as_sequence) else {
        return Ok(());
    };

    let allowed: Vec<String> = list
        .iter()
        .filter_map(Value::as_str)
        .map(|s| s.to_ascii_lowercase())
        .collect();

    if allowed.is_empty() {
        return Ok(());
    }

    let exe = Path::new(&cmd[0])
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&cmd[0])
        .to_ascii_lowercase();

    if !allowed.iter().any(|a| a == &exe) {
        bail!("Command not allowed by policy: {}", cmd[0]);
    }

    Ok(())
}

fn get_cache_state(crexe_path: &Path, crexe_bytes: &[u8], spec: &Value, inputs: &BTreeMap<String, String>) -> Result<CacheState> {
    let fp = compute_fingerprint(crexe_bytes, spec, inputs)?;

    let os_key = match env::consts::OS {
        "windows" => "windows",
        "macos" => "macos",
        _ => "linux",
    };

    let default_root = default_cache_root();
    let root_str = get_path(spec, &["workspace", "cache", "root", os_key])
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_root.to_string_lossy().to_string());

    let layout = get_path(spec, &["workspace", "cache", "layout"])
        .and_then(Value::as_str)
        .unwrap_or("{{fingerprint}}");

    let root = expand_path(&root_str);
    let rel = layout.replace("{{fingerprint}}", &fp);
    let workspace = root.join(rel);

    let manifest = get_path(spec, &["workspace", "cache", "markers", "manifest"])
        .and_then(Value::as_str)
        .unwrap_or("crexe.manifest.json");
    let build_ok = get_path(spec, &["workspace", "cache", "markers", "build_ok"])
        .and_then(Value::as_str)
        .unwrap_or(".build_ok");

    let _ = crexe_path;

    Ok(CacheState {
        workspace: workspace.clone(),
        fingerprint: fp,
        manifest_path: workspace.join(manifest),
        build_ok_path: workspace.join(build_ok),
    })
}

fn compute_fingerprint(crexe_bytes: &[u8], spec: &Value, inputs: &BTreeMap<String, String>) -> Result<String> {
    let mut crexe_hasher = Sha256::new();
    crexe_hasher.update(crexe_bytes);
    let crexe_sha = format!("{:x}", crexe_hasher.finalize());

    let generator = get_path(spec, &["generator"]).unwrap_or(&Value::Null);
    let payload = json!({
        "runtimeVersion": RUNTIME_VERSION,
        "crexeSha256": crexe_sha,
        "inputs": inputs,
        "generator": {
            "provider": get_path(generator, &["provider"]).and_then(Value::as_str),
            "baseUrl": get_path(generator, &["baseUrl"]).and_then(Value::as_str),
            "model": get_path(generator, &["model"]).and_then(Value::as_str),
            "temperature": get_path(generator, &["temperature"]).and_then(Value::as_f64),
            "maxOutputTokens": get_path(generator, &["maxOutputTokens"]).and_then(Value::as_i64),
        }
    });

    let bytes = serde_json::to_vec(&payload)?;
    let mut h = Sha256::new();
    h.update(bytes);
    Ok(format!("{:x}", h.finalize()))
}

fn default_cache_root() -> PathBuf {
    match env::consts::OS {
        "windows" => env::var("LOCALAPPDATA")
            .or_else(|_| env::var("APPDATA"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("CREXE")
            .join("cache"),
        "macos" => PathBuf::from(env::var("HOME").unwrap_or_else(|_| ".".to_string()))
            .join("Library")
            .join("Caches")
            .join("CREXE"),
        _ => env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".cache"))
            .join("crexe"),
    }
}

fn expand_path(s: &str) -> PathBuf {
    let mut out = s.to_string();
    if out.starts_with('~') {
        if let Ok(home) = env::var("HOME").or_else(|_| env::var("USERPROFILE")) {
            out = out.replacen('~', &home, 1);
        }
    }

    // Minimal Windows env expansion for %LOCALAPPDATA% style variables.
    let re = Regex::new(r#"%([A-Za-z_][A-Za-z0-9_]*)%"#).unwrap();
    out = re
        .replace_all(&out, |caps: &regex::Captures| {
            env::var(caps.get(1).unwrap().as_str()).unwrap_or_else(|_| caps.get(0).unwrap().as_str().to_string())
        })
        .to_string();

    PathBuf::from(out)
}

fn is_cache_hit(cache: &CacheState) -> Result<bool> {
    if !cache.manifest_path.exists() || !cache.build_ok_path.exists() {
        return Ok(false);
    }
    let text = fs::read_to_string(&cache.manifest_path)?;
    let v: serde_json::Value = serde_json::from_str(&text)?;
    Ok(v.get("fingerprint").and_then(|x| x.as_str()) == Some(cache.fingerprint.as_str()))
}

fn write_manifest(cache: &CacheState, crexe_path: &Path, inputs: &BTreeMap<String, String>) -> Result<()> {
    fs::create_dir_all(&cache.workspace)?;
    let manifest = Manifest {
        fingerprint: &cache.fingerprint,
        crexe_path: crexe_path.display().to_string(),
        runtime_version: RUNTIME_VERSION,
        generated_at: Utc::now().to_rfc3339(),
        inputs: inputs.clone(),
    };
    fs::write(&cache.manifest_path, serde_json::to_string_pretty(&manifest)?)?;
    fs::write(&cache.build_ok_path, "ok")?;
    Ok(())
}

fn timeout_seconds(spec: &Value, phase: &str) -> u64 {
    get_path(spec, &["policies", "timeoutSeconds", phase])
        .and_then(Value::as_i64)
        .unwrap_or(180)
        .max(1) as u64
}

fn get_path<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cur = v;
    for p in path {
        let key = Value::String((*p).to_string());
        cur = cur.as_mapping()?.get(&key)?;
    }
    Some(cur)
}

fn get_mapping<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Mapping> {
    get_path(v, path)?.as_mapping()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
