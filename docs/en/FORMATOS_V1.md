# Format contract — implementation under development

All formats use `.crexe`, UTF-8 (optional BOM), and LF or CRLF. Documents larger than 1 MiB and recognized but invalid structures fail before calling the model. An empty file (including whitespace or a BOM alone) offers editing in the Windows graphical flow and guidance to use `crexe edit` in the CLI. The editor allows empty or incomplete content to be saved; validation takes place when executing it.

## Plain text

`generate a rule-of-three calculator` is a complete input. No section or keyword is mandatory. The name comes from the filename without its extension. The entire text forms the intent. A sentence containing a colon remains plain text.

Automatic selection offers the model the supported profiles whose SDKs are available on the host, including OpenGL. It prefers the baseline profile, or the system default when there is no previous revision. A change requires a reason. `--profile` allows a fixed choice, including `dotnet-console` for a console application. The host is not emulated and global dependencies are not installed. See the [matrix and SDKs](PERFIS_V1.md).

## Markdown

```markdown
---
crexe: 1
name: Rule of three
---
# My intent
A simple calculator for A/B = C/X.
```

The marker identifies structured Markdown. `name` and `profile` are optional. The entire body is the intent; headings have no mandatory meaning. Unclosed front matter, a different version, or unknown fields cause an error. Markdown without the marker is accepted as a free-form prompt.

## Legacy YAML

A reserved key at the root (`version`, `targets`, `selectors`, or `prompt_core`), or a JSON object, identifies a structured recipe. It requires `version: 1.0`; errors do not trigger a fallback to plain text. The adapter preserves inputs, selectors, `prompt_core`, templates, and targets from the original example.

`build.steps`, `run.cmd`, and optional `test.steps` use argument lists. The build requires at least one command; the expected artifact must exist in the workspace. Another executable is not selected automatically. Unresolved templates and invalid inputs fail before generation. The legacy `generator` is ignored with a warning: move provider/model choices to local configuration. `workspace.cache.root` does not control the cache. Custom markers/layout and unimplemented network restrictions are rejected.

The main fields require mappings. Operations use 1 to 32 steps; malformed test/publish operations are not ignored. `policies` accepts only `allowNetwork`, `timeoutSeconds`, and `commandAllowlist`, with validated types. This is not yet normative validation of every possible field in the historical RFCs; capabilities must not be inferred from those documents.

## Projects and repairs

Providers return `files: [{path, content}]`, with optional `profile`, `change_kind` (`incremental` or `recreate`), and `reason` for compatibility with older responses. Automatic selection requests these metadata; changing profiles requires a reason and accepts only profiles offered by the engine. The engine validates the entire list before writing, including relative paths, duplicates, file/directory collisions, links, and limits. Each attempt compiles in a new subdirectory; previous sources/artifacts do not contaminate the next one.

The .NET profile supplies `.csproj`, registered dependencies, and commands; the model supplies source files. The build is followed by the `--crexe-self-test` requested from the generator. Failures return current sources and diagnostics to the model, which must return a complete snapshot. There are 1–10 total attempts, with a default of 3, subject to the local budget. No agent SDK or provider sandbox is required.

Each document has an independent history keyed by its canonical path. The approved revision stores the resolved request, profile, parameters, ancestor, and source inventory with hashes in `CREXE-GENERATION.json`. The index in `cache/.documents` points only to approved revisions. A new request receives the prompt and text sources declared in the previous manifest, including projects and dependencies; compiled and temporary files are excluded. The index changes only after approval, and generation identity includes its ancestor. `--fresh` explicitly disregards this history; `--rebuild` uses it.

The ZIP contains the source list and engine-supplied files, excluding restore caches, binaries, and credentials. Build/run scripts use BAT on Windows and shell on other systems; the .NET profile also includes test/publish. Arguments incompatible with safe BAT export produce an explicit error.

This contract describes the current code. Historical RFCs represent a vision, not a guarantee of implemented capabilities. Full normative reconciliation is part of the release criteria.
