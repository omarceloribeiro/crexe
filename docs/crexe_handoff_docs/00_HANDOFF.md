# CREXE Project Handoff

**Date:** 2026-09-23  
**Project:** CREXE — Creative Executable  
**Current working package:** `crexe_v1_rust_executor_responses_gpt55pro.zip`  
**Continuation target:** Codex Local Desktop

---

## 1. Project definition

CREXE is a **Creative Executable** format. A `.crexe` file is a human-readable generative software recipe that can generate, build, cache, and execute a native application locally.

Core phrase:

> Download intent. Execute creativity.

The project is exploring a new software distribution primitive:

> download a text recipe, generate software, compile locally, execute natively.

---

## 2. Strategic positioning

The name originally suggested:

> Compile-time Regenerative Executable

The stronger positioning is now:

> **CREXE = Creative Executable**

CREXE is not only about shipping software. It is also about user creativity:

- run a `.crexe`
- edit a `.crexe`
- design/customize behavior
- regenerate software
- fork/share recipes

---

## 3. Current technical architecture

Current implementation: **Rust executor**.

Current runtime supports:

- `.crexe` YAML input
- CLI execution
- direct `.crexe` invocation
- Windows drag-and-drop / Open With association
- OS target selection
- OpenAI online LLM generation
- Chat Completions transport
- Responses API transport
- cache by fingerprint
- build/run commands
- generated executable allowlist
- generated executable path fix
- CMD-only Windows scripts, no PowerShell dependency

---

## 4. Current command model

The executor supports both:

```cmd
crexe.exe exec examples\calculator_native.crexe
```

and direct invocation:

```cmd
crexe.exe examples\calculator_native.crexe
```

The second form is required because Windows uses it for:

- dragging `.crexe` onto `crexe.exe`
- “Open with” association
- double-click when `.crexe` is associated

---

## 5. Windows script requirement

The user explicitly wants **CMD**, not PowerShell.

Preserve `.cmd` scripts such as:

```text
build-executor.cmd
run-example.cmd
run-example-rebuild.cmd
run-example-chat.cmd
run-example-responses.cmd
associate-crexe.cmd
unassociate-crexe.cmd
clean-cache.cmd
test-drag-openwith.cmd
test-associated-from-examples.cmd
test-openwith-cache-hit.cmd
```

Do not replace these with `.ps1` scripts.

---

## 6. Best known working base

The best stable base before Responses support was:

```text
crexe_v1_rust_executor_cmd_openwith_policyfix3_minimal_pathfix.zip
```

Responses support was added in:

```text
crexe_v1_rust_executor_responses_gpt55pro.zip
```

When continuing, preserve the style and behavior of `policyfix3_minimal_pathfix`.

---

## 7. Bugs fixed so far

### 7.1 `max_tokens` rejected by newer models

Error:

```text
Unsupported parameter: 'max_tokens' is not supported with this model.
Use 'max_completion_tokens' instead.
```

Fix for Chat Completions:

- retry with `max_completion_tokens`
- optionally remove `temperature` if rejected

### 7.2 Open With / drag-and-drop support

Windows calls:

```cmd
crexe.exe "C:\path\file.crexe"
```

So the executor must accept direct `.crexe` invocation, not only the `exec` subcommand.

### 7.3 Generated executable blocked by policy

Error:

```text
Command not allowed by policy: build\CrexeCalculator.exe
```

Fix:

- allow generated executables inside the CREXE workspace/cache during `run`
- keep allowlist protection for external commands

### 7.4 Relative generated executable path failing

Error:

```text
Failed to start command: build\CrexeCalculator.exe
O sistema não pode encontrar o caminho especificado. (os error 3)
```

Fix:

- before `Command::new(...)`, convert generated executable path to absolute path
- minimal fix was applied in `resolve_run_command`
- avoid large rewrites of the run flow unless necessary

### 7.5 Avoid PowerShell

A package accidentally switched to PowerShell scripts. Rejected. Keep CMD scripts.

---

## 8. Generator transports

### 8.1 Chat Completions

For models that support:

```text
/v1/chat/completions
```

CREXE config:

```yaml
generator:
  mode: "online"
  provider: "openai-compatible"
  transport: "chat_completions"
  baseUrl: "https://api.openai.com/v1"
  apiKeyEnv: "OPENAI_API_KEY"
  model: "gpt-5.4"
  temperature: 0.2
  maxOutputTokens: 6000
  tokenParameter: "auto"
  sendTemperature: true
```

### 8.2 Responses API

Required for `gpt-5.5-pro`.

CREXE config:

```yaml
generator:
  mode: "online"
  provider: "openai"
  transport: "responses"
  baseUrl: "https://api.openai.com/v1"
  apiKeyEnv: "OPENAI_API_KEY"
  model: "gpt-5.5-pro"
  maxOutputTokens: 6000
  reasoning:
    effort: "high"
  sendTemperature: false
```

`gpt-5.5-pro` supports Responses and Batch, not Chat Completions.

---

## 9. Responses API implementation notes

For `transport: responses`, call:

```text
POST /v1/responses
```

Payload shape:

```json
{
  "model": "gpt-5.5-pro",
  "input": [
    {
      "role": "system",
      "content": [
        { "type": "input_text", "text": "..." }
      ]
    },
    {
      "role": "user",
      "content": [
        { "type": "input_text", "text": "..." }
      ]
    }
  ],
  "max_output_tokens": 6000,
  "reasoning": { "effort": "high" }
}
```

Response extraction order:

1. `output_text`
2. concatenate `output[].content[].text`

The extracted text should then be parsed as JSON matching the CREXE output contract.

---

## 10. CREXE output contract

The model must return only JSON:

```json
{
  "files": [
    { "path": "relative/path", "content": "file content" }
  ],
  "hints": {
    "entrypoint": "optional",
    "notes": "optional"
  }
}
```

No markdown. No explanation. No text outside JSON.

---

## 11. Current example app

Example:

```text
examples/calculator_native.crexe
```

Purpose:

- generate a native calculator
- Windows target: Win32 API in C
- macOS target: Cocoa Objective-C
- Linux target: GTK3 C

Core idea:

> Single prompt, multiple native outputs.

---

## 12. Windows target assumptions

Windows target uses:

```cmd
cl /nologo /O2 /W3 /DUNICODE /D_UNICODE src\main.c /Fe:build\CrexeCalculator.exe user32.lib gdi32.lib comctl32.lib
```

This requires MSVC Build Tools / Visual Studio Developer Command Prompt or an environment where `cl.exe` is available.

---

## 13. Cache design

Workspace root:

```text
%LOCALAPPDATA%\CREXE\cache\<fingerprint>
```

Fingerprint should include:

- CREXE file hash
- effective inputs
- runtime version
- generator provider
- generator model
- generator transport
- relevant generation parameters

Cache hit:

- skip generation
- skip build
- run existing generated executable

Cache miss or `--rebuild`:

- generate
- build
- write manifest and `.build_ok`
- run

---

## 14. Security baseline

The runtime should remain strict:

- no write outside workspace
- commands restricted by allowlist
- generated executable allowed only if inside workspace/cache
- build/run separated
- no arbitrary network during build/run
- LLM network only through runtime generation layer

---

## 15. Recommended next steps for Codex

### Immediate tests

1. Build the latest ZIP.
2. Test `run-example-responses.cmd` with `gpt-5.5-pro`.
3. Test `run-example-chat.cmd` with a Chat Completions-compatible model.
4. Test:
   ```cmd
   cd examples
   calculator_native.crexe
   ```
5. Test drag-and-drop onto `crexe.exe`.
6. Test extension association:
   ```cmd
   associate-crexe.cmd
   ```

### Code cleanup

Split `main.rs` into modules:

- `cli.rs`
- `spec.rs`
- `template.rs`
- `llm_chat.rs`
- `llm_responses.rs`
- `cache.rs`
- `executor.rs`
- `policy.rs`

Add unit tests for:

- target selector
- template rendering
- direct invocation parser
- path normalization
- `extract_responses_text`

---

## 16. Explicit constraints

Do not change without reason:

- do not convert CMD scripts to PowerShell
- do not break `crexe.exe file.crexe`
- do not break `crexe.exe exec file.crexe`
- do not remove Chat Completions support
- do not remove Responses support
- do not let generated code write outside workspace
- do not allow arbitrary external command execution
- do not hide execution failures behind shell wrappers
- keep the latest working path fix minimal

---

## 17. North star

CREXE is not only a runtime.

It is a candidate for a new computing primitive:

> software as regenerable creative intent.
