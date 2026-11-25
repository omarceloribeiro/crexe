# CREXE — Creative Executable

This is the English edition of the project guide. The native engine interface currently uses Portuguese labels, shown in parentheses where controls are described; the website translation does not change that interface. Links to the development plan, implementation records, platform guides, and historical archives retain their original language.

A `.crexe` file describes an application's intent. The engine generates a project, compiles it, tests when the profile provides tests, stores a revision in the cache, and opens the program. Editing the intent evolves the latest approved revision; reopening without changes reuses the validated revision.

With **Generate with CREXE** (Gerar com CREXE), an ordinary document can also be the starting point: select a file and the engine generates or reuses a reader, passing that document to the application when it opens.

**Under development on the `feature/v1` branch; not yet a public release.** The original executor is preserved at the `baseline/pre-v1` tag. See the [plan](../PLANO_V1.md) and [implementation log](../IMPLEMENTACAO_V1.md) to distinguish present functionality from outstanding criteria.

## CREXP and CREXB

The preview includes **CREXP — Creative Executable Package**, a ZIP of Markdown SDD documents used for generation, and **CREXB — Creative Executable Bundle**, preserving the recipe and accepted snapshot for opening on a compatible target without AI or recompilation. These are minimal subsets: runtimes and libraries are still required, and applications are not sandboxed.

See the [CREXP guide](publicacoes/crexp.md), [CREXB guide](publicacoes/crexb.md), and [complete roadmap](publicacoes/roadmap-formatos.md). The [original handoffs and JSON references](../formatos/originais/README.md) are preserved unabridged for download. The site and maintained guides have Portuguese and English editions; the complete handoffs remain in their original Portuguese, clearly labelled as such.

## Getting started

With Rust/Cargo and native build tools, from the repository root:

```sh
cargo build --release --locked
cargo test --locked
```

On Windows, `scripts\windows\build-executor.cmd` also works from another directory. Building the engine uses the Visual C++ linker. The distributed binary does not require the user to have Rust; generated applications need their profile's SDK.

The MSVC build statically includes the C runtime; CI inspects the executable to avoid dependencies on Visual C++ redistributable DLLs. Windows' own DLLs remain necessary. `scripts/package-engine.py` prepares development snapshots containing the binary, examples, sample configuration, license, code provenance, and SHA-256. CI packages are for validation and are not yet stable releases. [Distribution guide](../DISTRIBUICAO_V1.md).

Create `calculadora.crexe` containing only:

```text
Generate a simple calculator with the four arithmetic operations and a green background.
```

With Ollama and `gemma4:12b` available locally, and the .NET SDK installed:

```powershell
.\target\release\crexe.exe doctor
.\target\release\crexe.exe doctor --check
.\target\release\crexe.exe inspect calculadora.crexe
.\target\release\crexe.exe calculadora.crexe
```

The preferred Windows profile uses C#/.NET 8 and Windows Forms. There are also C++/Win32, Objective-C++/Cocoa, C/GTK3, and host-specific OpenGL profiles. For text/Markdown without an explicit profile, the model can choose among profiles with available prerequisites, giving a reason for technology changes. The engine supplies projects and commands, restores registered libraries, compiles, and runs the self-test before publishing the cache. There are 3 total attempts by default, configurable from 1 to 10. Program quality still depends on the model; an approved build does not prove every requested behavior. [Profiles and requirements](PERFIS_V1.md).

`doctor` reports engine/host architecture, profile, requirements, and paths. With `--check`, it queries the local SDK before generation, without calling AI. `inspect` shows the profile requirements alongside the resolved intent; it does not execute SDKs.

## Generate with CREXE

The [complete feature guide](GENERATE_WITH_CREXE.md) covers calls, cache, privacy, SDKs, acceptance checks, and a demonstration walkthrough. It is also available [in Portuguese](../GERAR_COM_CREXE.md).

To open a document without writing a recipe, use **Select a file and generate a program…** (Selecionar arquivo e gerar programa…) in the engine window. On Windows, after `install` and `associate`, **Gerar com CREXE** is also available in a file's context menu; on Windows 11 it may appear under **Show more options**. On macOS and Linux, this preview's graphical entry point is the file picker in the engine window; it does not claim Finder, GNOME or Dolphin menu integration.

From the extracted package directory, the CLI equivalent is:

```powershell
.\crexe.exe open-generating -- "meeting notes.md"
```

On macOS/Linux, use `./crexe open-generating -- "meeting notes.md"`. For an engine built from the checkout, the executable is in `target/release`. Paths resolve from the caller's directory; `--` accepts names starting with a hyphen. The command takes one regular local file with a valid extension.

When preparing a new reader, **one LLM call formulates its intent from the file extension**. The normal code generation then follows, with build, self-tests and bounded repairs. Planning has its own request count: the default allows one plan and up to three code attempts, subject to token and time limits. The application opens the selected document without asking for it again. Another document of the same compatible type reuses the approved reader: a valid cache means zero additional model calls and no rebuild. Changing only a document's contents does not change the reader's identity.

Valid unknown extensions are accepted. Files with no extension or an invalid extension receive an unsupported-type message, without a model call or reading bytes to identify the type. Selecting `.crexe`, `.crexp`, or `.crexb` in this action also returns a message: recipes, packages, and bundles must be opened through the normal execution workflow.

The document's name, path and body do not enter model requests, generation history or the source ZIP. The application receives its path as a separate argument and reads the data locally at runtime. The saved active configuration supplies provider, model and limits; selecting a document does not save or discard pending settings edits. Cancelling the picker makes no AI call.

**Keep as the default program (Windows).** Use the new menu or `crexe open-generating --keep-as-default --ui -- "document.xyz"`. Check the reader and confirm the choice in Windows; another document with the same extension then opens by double-click without generation. The program is preserved outside the cache and does not require provider credentials to reopen. [Usage, removal and acceptance criteria](GENERATE_WITH_CREXE.md).

This preview's contract is **read-only**, preserving the original: actual content or a readable error, an explicit empty-file state and bounded reading. Unknown formats may produce a text view or a labelled hexadecimal preview. Full interpretation of proprietary formats is not promised. This contract guides generated code; it provides neither a sandbox nor a security guarantee. A provider/model and the profile's SDK remain necessary for the first reader, and remote providers may charge for planning, generation and repairs.

## Provider and credentials

Open `crexe configure` (or the executable without arguments) to select the provider/model and **Maximum attempts** (Tentativas máximas), and save the API key in the system vault. **Advanced** (Avançado) allows limiting total output tokens; the screen shows when that cap reduces effective attempts. The screen shows the active file and allows importing a TOML file for review. `config.example.toml` is an example: editing it does not change the active configuration. Installing/reinstalling preserves your choices; interactive installers open the screen when finished.

Available presets are **Ollama**, **OpenAI**, and **DeepSeek**. For DeepSeek, select the `deepseek` profile, enter the API key, and save. The preset uses `deepseek-flash`, with reasoning disabled by default; the option can be changed under **Advanced** (Avançado). Older installations also offer the new profile on the screen, without changing the active provider until you save.

The active file is stored at `%APPDATA%\CREXE\config.toml` on Windows, `~/Library/Application Support/CREXE/config.toml` on macOS, or `${XDG_CONFIG_HOME:-$HOME/.config}/crexe/config.toml` on Linux. Alternatively, use `--config caminho.toml`. Without a file, defaults are **local Ollama**, without a key. The CLI remains available on hosts without a desktop.

The provider, endpoint, and credential reference belong to local configuration. Legacy recipes' `generator` fields do not control those values; the CLI reports this migration. An Ollama failure never automatically switches to a paid service.

```powershell
# Explicit OpenAI selection; may consume API credits.
.\target\release\crexe.exe calculadora.crexe --provider openai

# Explicitly selected secrets file; it is not searched for beside the recipe.
.\target\release\crexe.exe calculadora.crexe --provider openai --env-file .env

# DeepSeek: after saving the profile through the screen, or using --config with your TOML.
.\target\release\crexe.exe calculadora.crexe --provider deepseek
```

Advanced environment-based configuration remains available: the example references `crexe_openai_api_key_env`, in the environment or in a line of the explicitly selected `.env` file. A key saved through the screen takes precedence over old variables; an explicit `--env-file` may override it. TOML stores only a destination-bound reference, never the key. See [configuration, vault, and limits](CONFIGURACAO.md).

The DeepSeek profile references `crexe_deepseek_api_key`, also compatible with `--env-file .env`. Users manage API credit with the provider; the engine does not buy credits or switch services automatically.

## Install and open with a double-click

```powershell
.\target\release\crexe.exe install
& "$env:LOCALAPPDATA\CREXE\releases\v1\crexe.exe" associate
```

The association points to `%LOCALAPPDATA%\CREXE\releases\v1\crexe.exe` and enables the “Creative Executable / Gerando seu programa…” window (“Generating your program…”). It shows the provider, model, and “Tentativa 1 de 3 — até 2 correções” (“Attempt 1 of 3 — up to 2 repairs”), separately from the current phase. The progress window waits for the application's window to open (up to 15 seconds), restores minimized windows, and requests focus from Windows. If the system denies focus, the taskbar signals the opening. Failures appear in a message; closing the progress window cancels preparation. Terminal invocation preserves logs; `--ui` explicitly enables the presentation.

Repeated clicks during preparation notify the existing execution without generating or compiling again. If the intent/options changed, the progress window directs the user to finish or cancel first. After the application opens, another click can open another instance from cache.

Windows may require choosing CREXE as the default application for `.crexe`. The generic **Gerar com CREXE** action does not change other documents' default applications. The new **Gerar com CREXE e Manter como Programa Padrão** option preserves a reader and offers confirmation in Windows Default apps settings; later opens use the kept program without an LLM or compilation. `unassociate` removes the associations, reader registrations and both menus owned by this installation and preserves other applications' choices. On Linux/macOS, the CLI has its own installation paths and modules selected by `cfg`; ordinary documents can be selected through the engine window. The `.crexe` association and file-manager menus on those platforms are not implemented yet. [Platform guide](../platforms/README.md).

`CREXE_HOME` selects an absolute root for installation, configuration, and cache, useful for tests and portable installations.

## Editing the intent

Use **Edit** (Editar) in the Windows context menu (after registering the current association) or `crexe edit "calculadora.crexe"`. The integrated editor works on Windows, macOS, and Linux with a graphical session. Each opening has its own window, with the path, change indicator, **Save** (Salvar), **Reload** (Recarregar), and **Close** (Fechar). It offers selection, copy/paste, undo/redo, and `Ctrl+S` (`Cmd+S` on macOS).

Saving changes only the document. To generate or update the program, open the `.crexe` file again. The editor does not query providers or load credentials. It accepts empty or incomplete text, preserves UTF-8, BOM, and LF/CRLF, and limits the file to 1 MiB. Writes are atomic; external changes block saving and retain the window's text for recovery. Reloading and closing ask for confirmation when there are local changes.

Opening an empty file, a whitespace-only file, or a BOM-only file through the Windows graphical flow shows **“O arquivo está vazio. Abrir para edição?”** (“The file is empty. Open for editing?”), with **Yes** (Sim) and **Cancel** (Cancelar). Yes opens the same editor without starting generation after saving. The CLI directs users to `crexe edit`; without a graphical session, the command reports the limitation.

## Formats, cache, and ZIP

- [Legacy YAML](../../examples/yaml/calculator_native.crexe): explicit targets and commands. The original example uses MinGW/Win32, Clang/Cocoa, or GCC/GTK3; these compilers are not installed automatically.
- [Markdown](../../examples/markdown/regra-de-tres.crexe): front matter with `crexe: 1`, optional metadata, and a free-form body.
- [Plain text](../../examples/plaintext/calculadora.crexe): only the intent. The default name comes from the file.

```powershell
crexe exec calculadora.crexe --no-run --export calculadora-fontes.zip
crexe calculadora.crexe --rebuild --max-repairs 2
crexe calculadora.crexe --fresh
```

Every approved revision includes `source.zip`. In the .NET profile it contains sources, `.csproj`, instructions, and build, run, test, and publish scripts. The ZIP can be recompiled without CREXE, with the specified prerequisites. .NET publish is runtime-dependent; it does not install or publish online. Export requires a new destination. Legacy YAML includes build/run scripts; it does not invent a missing publish operation.

The `CREXE-PROJECT.json` manifest stores sources and operations used by the CLI and scripts. With a revision or extracted ZIP, these operations neither call AI nor require a key:

```powershell
crexe build ./projeto-extraido --output ./projeto-recompilado
crexe test ./projeto-extraido
crexe run ./projeto-recompilado
crexe publish ./projeto-extraido --output ./projeto-publicado
crexe export ./projeto-extraido --output ./fontes.zip
```

Build/test/publish use a new temporary directory; build/publish deliver a new output directory and preserve the source project. Export packages current sources without recompiling them. A ZIP from older versions without a manifest must be regenerated to use these commands; its scripts remain usable. [Operations contract](PROJETOS_V1.md).

For `.crexe` recipes, the cache has immutable revisions, hashes, and an index keyed by the recipe's canonical path. Each change sends the previous request, approved sources, and current request to AI. It receives instructions to preserve unaffected behavior, and may justify a complete recreation or profile change when necessary. Repairs use the latest attempt and its diagnostics. Binaries, logs, and build directories are not included in that context. For **Generate with CREXE**, the reader recipe is internal and independent of the opened document; the cache identifies the compatible reader, not each file's contents.

The index advances only after successful builds/tests. Failure or cancellation preserves the baseline. `--rebuild` forces a new generation from it; `--fresh` generates without it. Corrupted approved sources or context exceeding the limit produce diagnostics, without silent discarding. Old cache entries are adopted only with verifiable identity and inventory. Copying or moving a `.crexe` file starts an independent history; `--cache-dir` selects another local root. Changing only the attempt limit does not invalidate the approved revision.

## Current limits

Builds and applications use the **host with the user's permissions**. The temporary directory organizes the work and does not isolate files, networks, or processes. Docker and WSL are not requirements. Windows Sandbox is prohibited at this stage. Use recipes and code you are willing to run locally; there is no security guarantee for external programs.

There is path validation, response/file/log limits, subprocess control, timeout, and cancellation, without inheritance of configured credentials. This does not prevent a malicious native program from accessing the host. `allowNetwork: false` is rejected because the engine does not provide network isolation.

Tools and limits on calls, output tokens, and time are defined in local configuration; a recipe cannot expand them. Outstanding criteria include validation with real models and application use across the full native matrix, compatibility triage, dependencies beyond the engine's catalog, and release preparation. Check the implementation log before distributing.

## Documentation and license

- [Public crexe.org site](../../src/site/README.md): landing, examples, and downloads; local build and preview independent of the engine. [Publication and download counts](DOWNLOADS_AND_PUBLICATION.md).
- [CREXE library](publicacoes/README.md): white papers, guides, articles, and manifesto; current implementation and future vision identified in each text.
- [Implemented format contract](FORMATOS_V1.md)
- [Generate with CREXE: usage, architecture, and validation](GENERATE_WITH_CREXE.md)
- [Configuration](CONFIGURACAO.md), [contribution](CONTRIBUTING.md), and [security limits](SECURITY.md)
- [RFCs and historical vision](../crexe_handoff_docs/README.md), being reconciled with the implementation
- [Baseline and historical files](../archive/README.md)
- [Local Ollama analysis](../ANALISE_OLLAMA_LOCAL_2026-09-23.md)

The engine uses [MIT](../../LICENSE), allowing ports, modifications and open or proprietary integrations while preserving its required notices. **You choose the generated application's license, including a proprietary license, and may sell it.** CREXE does not require publishing the application's source, royalties or attribution for its designated output materials and official examples, which are [MIT-0](../../LICENSE-GENERATED). The ZIP includes that notice without replacing the application's `LICENSE`. Terms of dependencies, assets, third-party recipes and the model/provider remain applicable. See [licensing and ecosystem organization](LICENCIAMENTO_E_ECOSSISTEMA.md).
