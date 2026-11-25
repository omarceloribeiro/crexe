# Local engine configuration

The native engine interface currently uses Portuguese labels. This English documentation gives the original labels in parentheses; translating the website does not change the engine's interface language.

The version 1 TOML file selects a profile in `providers`. `--provider` selects another local profile; `--model` overrides that profile's model. `--config` explicitly selects the file. A recipe cannot select an endpoint, a credential variable, or a paid fallback.

Use `crexe configure`, the executable without arguments, or the package's `configure.cmd` / `configure.command` / `configure.sh` launcher. The native screen lets you select the profile, edit the model, test the connection, query models, save the API key, and import the default provider from another TOML file. `configure --import caminho.toml` also loads an import for review; only **Save** (Salvar) applies the change. Importing preserves local execution rules and other profiles, and does not copy vault references from another configuration. Switching profiles on the screen discards unsaved edits to the previous profile.

Defaults and types are defined in `config.example.toml`; unknown fields are rejected. The example is not the active configuration. Initial installation creates the active file if absent; reinstalling preserves an existing file. The screen and `doctor` show the same path. Writes are atomic, with locking and external-edit detection: use **Reload** (Recarregar) if there is a conflict. Closing without saving does not change the file; an ongoing generation retains the settings it started with.

The screen queries `GET /api/tags` on Ollama or `GET /models` on OpenAI/DeepSeek only through **Test connection / refresh models** (Testar conexão / atualizar modelos). It does not generate anything, has a 15-second timeout, and allows entering a model even without a listing. A successful query does not guarantee that every listed model accepts CREXE's generation contract. Network and vault access run outside the UI thread.

Presets come from `config.example.toml`; they also appear when opening older configurations. Selecting a missing preset writes it only when saving. The CLI's `--provider deepseek` requires that profile to exist in the active TOML file, or uses defaults when no active file exists. Updating/reinstalling does not automatically change the selected provider.

## Attempts and output budget

**Maximum attempts** (Tentativas máximas) on the main screen controls `execution.max_provider_requests`: 1 to 10, default 3, including initial generation. Only compilation/test failures allow automatic repairs; authentication, balance, connection, and invalid-response failures stop execution. `--max-repairs 0..9` is optional: when provided, it reduces the number of repairs within the configured limit. `--max-repairs 0` allows only the initial proposal.

Without `execution.max_output_tokens_total`, the output budget is derived from `max_provider_requests × providers.<profile>.max_output_tokens`. A cap saved in older configurations remains effective. Under **Advanced** (Avançado), **Limit total output tokens** (Limitar total de tokens de saída) lets you edit that cap or return to automatic calculation. For example, 10 attempts with 6000 tokens per call use a budget of 60000; an explicit cap of 18000 reduces the effective limit to 3. If the cap cannot accommodate one call, execution reports the problem before contacting the provider.

The generation window shows the provider, model, current attempt, and effective total, with the phase in a separate field. These values are captured at startup, including CLI overrides. Changing attempts or the output cap does not invalidate an approved program or modify an execution already underway. Saving preserves other execution rules, comments, and local profiles.

## Keys in the user's credential vault

**Save** (Salvar) stores the entered key in Windows Credential Manager (local persistence), macOS Keychain, or Secret Service on Linux. TOML receives only `credential.id` and `credential.binding`, bound to the configuration path, profile, and endpoint. Changing the endpoint requires a new key or explicit removal. The screen does not populate the field with the old key; it shows the reference's status. A locked/missing vault causes an error, without saving plain text or silently using another key.

A replacement writes another entry, publishes the configuration, and only then removes the old CREXE-owned entry. If saving fails, it keeps the previous configuration and attempts to remove the temporary entry. Copying TOML containing a reference to another machine or path requires setting up the key again. Changing only the key/reference does not change cache identity. Generation resolves the key on the first call and keeps it in memory for repairs; later replacing/removing it in the vault does not interrupt those calls. Revoking the key at the remote service can still prevent its use.

Precedence when a vault reference exists: the corresponding variable in an explicit `--env-file`, then the vault. Old process/Windows variables do not override the saved key. Without a reference, previous compatibility is retained: process variable, explicit `--env-file`, Windows user variable. Removing a saved key reactivates that legacy resolution when `api_key_env` is configured.

Literal secrets are not part of the TOML schema. Loading `.env` is confined to engine memory, without exporting its contents to the process or its children. The referenced name must match the name to the left of `=`. CLI/doctor/inspect do not require a display; hosts without a desktop/vault can continue using the environment or `--env-file`. Linux requires an unlocked Secret Service session to save keys through the screen.

| Platform | Default file |
|---|---|
| Windows | `%APPDATA%\CREXE\config.toml` |
| Linux | `${XDG_CONFIG_HOME:-$HOME/.config}/crexe/config.toml` |
| macOS | `~/Library/Application Support/CREXE/config.toml` |
| Explicit `CREXE_HOME` | `<CREXE_HOME>/config.toml` |

## Adapters

Ollama uses [`POST /api/chat`](https://docs.ollama.com/api/chat), a JSON schema for `files`, `stream: false`, `think`, `keep_alive`, `num_ctx`, and `num_predict`. The adapter receives a complete response; streaming and per-file generation are possible future developments, without a promise that they would fix syntax by themselves. `done: true` and `done_reason: stop` are required.

OpenAI uses [Chat Completions](https://developers.openai.com/api/reference/cli/resources/chat/subresources/completions/methods/create), JSON mode, and Bearer authentication when a variable is configured. It uses `max_completion_tokens` for recognized reasoning families and `max_tokens` for others. Refusals and finish reasons other than `stop` do not become files. The example model is `gpt-4o-mini`; changing the model is a local choice.

DeepSeek uses [Chat Completions](https://api-docs.deepseek.com/api/create-chat-completion/) at `https://api.deepseek.com`, `max_tokens`, JSON mode, and a complete response (`stream: false`). The `deepseek-flash` preset has a 300-second timeout and a maximum output of 6000 tokens. `thinking = false` (or absent) explicitly sends reasoning as disabled; `true` enables the mode and omits temperature. The screen offers this control under **Advanced** (Avançado). Reasoning may increase latency and usage; it is not required by the CREXE contract. The API key is saved in the vault or resolved through `api_key_env = "crexe_deepseek_api_key"`.

Empty, truncated, refused, or tool-call-containing DeepSeek responses are not accepted. Only the final content becomes files; reasoning is not included in sources, logs, or the ZIP. Logs of valid JSON responses may include the returned model and numeric token counts supplied by the API. HTTP 401 indicates an invalid key; 402 insufficient balance; 400/422 invalid parameters; 429 a request limit; and 500/503 unavailability. These failures do not trigger automatic retries. Compilation/test repairs remain subject to local limits.

`crexe_deepseek_api_key` and `DEEPSEEK_API_KEY` are removed from child-process environments even in older configurations or when another provider is active, in addition to configured custom names. For DeepSeek, changing the key reference/variable retains cache identity; absent `thinking` is equivalent to `false`. Model, endpoint, and effective reasoning mode distinguish generations. Legacy Ollama/OpenAI identities remain preserved.

HTTPS is required outside loopback. URLs with credentials, a query, or a fragment are rejected. HTTP redirects are disabled. HTTP error bodies are not reproduced in logs. No adapter buys credits or selects another provider after a failure.

## Applied limits

| Resource | Current implementation |
|---|---|
| Document | Up to 1 MiB |
| HTTP call | `timeout_seconds`, 1–1800 s; connection up to 15 s |
| Model output | `max_output_tokens`, 128–65536; HTTP response up to 20 MiB |
| Ollama | Context 512–131072, larger than output and also leaving room for the prompt |
| Proposals | 1 to 10 total attempts (default 3); optional `--max-repairs 0..9` restricts repairs |
| Local budget | Up to `execution.max_provider_requests` calls; output derived from attempts × tokens per call, or explicit `max_output_tokens_total`; full prompt up to `max_prompt_bytes` bytes (default 131072) |
| Generation deadline | `execution.generation_timeout_seconds` (default 1800 s) from the start of preparation; reduces subsequent call timeouts to the remaining time |
| Sources | 1–128 files; 4 MiB per file; 16 MiB per response |
| Build/test | Timeout per command; 2 MiB per log stream; a tail of up to 64 KiB per stream in diagnostics |
| Revision | Up to 4096 files / 2 GiB |
| Final app | No default timeout; the YAML run timeout applies if explicit |

Closing the window or pressing Ctrl+C stops orchestration and its own subprocess groups. A request already delivered to the server may continue briefly at the provider; the engine does not publish its result after cancellation. The Ollama service is not stopped.

The output reservation uses the maximum requested per call even if the model responds with less. It does not measure input tokens and is not a monetary API cap. The generation deadline prevents new calls after the limit; build/test have their own per-command deadlines (defaults 300/60 s), which a recipe can only reduce. It is not one overall deadline that interrupts the final application.

Incremental context is explicit: the previous request, approved sources, and current request, following [message-based state management](https://developers.openai.com/api/docs/guides/conversation-state). It does not depend on files or conversations retained by the provider. The prompt limit includes profile contracts and this context; exceeding it stops generation and preserves the baseline. Repairs send the snapshot of the latest attempt, without repeating the original approved snapshot.

## Local execution policy

`execution.allowed_tools` authorizes executables on PATH or absolute paths defined by the user. Comparison uses the tool's resolved path, not only its name. The YAML allowlist may restrict this list but cannot expand it. Artifacts built inside the workspace may be executed as the application/self-test.

Shells and shell scripts require `allow_shells = true` and explicit inclusion of the tool in `allowed_tools`. Legacy recipes that used `cmd`, `sh`, or PowerShell require this local configuration; do not change the policy for every received document. ZIP scripts are an alternative outside the CLI and execute directly with the user's permissions.

This control applies to commands started by the engine. Compilers, projects, scripts, and applications continue to run on the host; the policy does not restrict their own subprocesses or file/network access. Automatic revision/diagnostic retention management is not yet available. The model-generated self-test requires independent validation to serve as strong functional evidence.

In YAML, `allowNetwork` requires a real boolean; `false` remains rejected because it requires unavailable isolation. Unknown policies are rejected. Timeouts require positive integer seconds; `generate` may reduce the provider's per-call timeout. An empty allowlist for the OS blocks tools instead of allowing any command. Invalid build/test/publish structures and unresolved templates in those operations are rejected before generation.
