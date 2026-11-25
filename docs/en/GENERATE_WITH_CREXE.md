# Generate with CREXE

[Português](../GERAR_COM_CREXE.md) · [Main guide](README.md)

**Status: v1 implementation under development.** An ordinary document can start the generation of a local reader. The engine prepares the program from the file type and passes the document to that program only when it opens. This guide describes the implemented behavior, preview limitations, and checks required before publication. Native interface labels currently appear in Portuguese.

## Use the CLI or engine window

From the extracted Windows package directory:

```powershell
.\crexe.exe configure
.\crexe.exe doctor --check
.\crexe.exe open-generating -- "meeting notes.md"
```

On macOS/Linux, replace `.\crexe.exe` with `./crexe`. In a compiled checkout, use `target/release/crexe` or `target\release\crexe.exe`. The examples below use `crexe` assuming the executable is on PATH.

```sh
# One file per invocation; -- also accepts names starting with a hyphen.
crexe open-generating -- "-meeting notes.md"

# Explicit profile selection; the model does not choose another profile.
crexe open-generating --profile dotnet-console -- "sample.unknown-type"

# Generate, build, test, and export sources; do not open the final reader.
crexe open-generating --no-run --export reader-sources.zip -- "notes.md"

# Limit compiler/test repairs; planning still uses one call.
crexe open-generating --max-repairs 0 -- "notes.md"
```

The export destination must be a new file. The ZIP contains the reader project, not a copy of the selected document. `--no-run` can still query the model, compile, and test when no approved reader exists.

The command accepts regular local files with any extension, including no extension. Relative paths resolve from the caller's directory. Names containing spaces, accents, or a leading hyphen are separate arguments; they do not become shell commands. A `.crexe` file follows the existing recipe execution workflow.

Open the engine without arguments or run `crexe configure`. Under **Gerar com CREXE**, click **Selecionar arquivo e gerar programa…** (Select a file and generate a program). The picker allows one file at a time and does not filter extensions. Cancelling ends the selection without generation or a model call. Opening uses the saved configuration; unsaved settings edits remain intact and are not applied to the reader.

On Windows, `install` and `associate` register **Gerar com CREXE** in file context menus. On Windows 11, also check **Show more options** (Mostrar mais opções). This action preserves each document's default application. On macOS/Linux, the engine window's picker is this preview's graphical alternative; Finder, GNOME, and Dolphin context menu integration is not implemented. The CLI also works without a graphical session when using a console profile.

## SDKs and profile selection

The provider, model, and credentials come from [local configuration](CONFIGURACAO.md). Ollama, OpenAI, and DeepSeek reuse the engine's existing transports. The engine does not automatically switch providers after a failure.

| Host or selection | Reader profile | Prerequisites |
|---|---|---|
| Windows, default preference | `dotnet-winforms` | .NET 8 **SDK** on PATH; Windows Forms |
| Windows, native alternative | `cpp-win32` | MinGW-w64 `g++`, matching the engine architecture |
| macOS, default | `objc-cocoa` | Apple Clang, Command Line Tools, and Cocoa SDK |
| Linux, default | `c-gtk` | GCC, GNU Make, `pkg-config`, and GTK3 headers/libraries |
| Console, explicit selection | `dotnet-console` | .NET 8 SDK; available on all three hosts |

On Windows, the initial preference changes to `cpp-win32` only when `dotnet` cannot be found and `g++` is available. Finding `dotnet` does not prove SDK 8 is installed: preflight checks that requirement. A .NET runtime without the SDK is insufficient for generation. Use `--profile` to select another option deliberately; the reader retains that profile through planning, generation, and repairs. A failure does not automatically trigger another profile attempt.

```sh
crexe doctor --check --profile dotnet-winforms
crexe doctor --check --profile cpp-win32
```

Run the command for your intended host and profile. Preflight queries local tools before the first planning call, without calling the model; the engine does not install SDKs. The [complete profile guide](PERFIS_V1.md) covers additional requirements and distribution limitations. A cached reader still needs the libraries/runtime used by its executable.

## Implemented plan and architecture

1. **Validate the input and environment.** Resolve the local path, reject directories/devices, identify the type, and fix the profile. Check the budget and SDK before the first call.
2. **Coordinate reader preparation.** Acquire a local lock for the type/configuration combination. Another compatible opening waits for that preparation.
3. **Plan once.** Send only the type identifier, profile, and reader contract. The response must be JSON with a single `prompt` field; it is not a `files[]` response. There is one planning attempt in that preparation, without automatic retries or fallback.
4. **Persist the internal intent.** Validate the prompt and atomically save the planning record and internal recipe, with identity and checksum. The recipe's name and profile are controlled by the engine.
5. **Generate and verify the program.** Use the existing `files[]` pipeline, engine project, build, and self-test. Add the reading checks described below. Build/test failures may request corrections within the remaining budget.
6. **Publish and open.** Advance history only after approval. Release shared preparation and start the executable with the document path as one native argument. Each opening receives its own file.

Planning, generation, and repairs use the **same session**, sharing request, output-token, and time budgets and the same resolved credential. First preparation requires at least two available requests and enough tokens for the plan and one full generation. Planning reserves up to 2,048 output tokens, also capped by the configured per-call limit.

| Situation | Model calls |
|---|---|
| First reader, code approved without repair | 1 planning + 1 generation |
| First reader, one repair needed | 1 planning + 1 generation + 1 repair, if budget allows |
| Another document of the same type, compatible approved reader | 0 |
| Only document contents changed | 0, while the reader remains valid |
| Planning failure | No automatic second planning attempt |
| Failure after a valid plan was saved, followed by an explicit new opening | Reuse the plan and try generation with the new invocation's budget |

The default three-request budget allows planning, generation, and one repair. `--max-repairs` can reduce corrections; it cannot expand the local budget. An HTTP or provider-response failure ends the operation without hidden retries. Remote providers may charge for every request, including those followed by a failure elsewhere in the workflow.

The reader cache considers the normalized extension or absence of an extension, provider/model generation parameters, profile, engine OS/architecture, and contract version. Opaque extensions receive an identifier derived from their bytes rather than entering the prompt as free text. The document's name, directory, and contents are not part of that identity. Changing only the request/total-token caps does not invalidate an approved reader.

Internal state lives in `readers/<identity>/` under the managed installation root; approved revisions use the engine cache/history. `CREXE_HOME` selects an isolated absolute root for tests. Opening documents does not require creating, editing, or distributing these internal recipes.

Two simultaneous openings for the same reader share one preparation and each open their own document after success. If preparation fails or is cancelled, waiting callers receive an error without starting new calls. Another attempt requires a new explicit invocation. The lock is released before waiting for the application to close, allowing other documents to open while it remains running.

## Privacy and v1 reader limits

The selected document's path, name, and body are not included in planning, generation, repairs, recipe history, or the source ZIP. Its path stays local and reaches the executable only at launch. Names and paths may appear in the local interface to identify the file. Prepublication checks use synthetic test files without reading the selected document.

The contract requires a **read-only** reader: immediately open the selected file, show its actual contents or a readable error, and preserve the original. Minimal editing, Save As, and complete interpretation of proprietary formats are outside this delivery.

- The preview reads up to **65,536 bytes (64 KiB)**, plus one byte to detect truncation.
- Valid UTF-8 without NUL bytes is displayed as text; other data receives a labelled hexadecimal preview. Cutting through a UTF-8 sequence can also result in a hexadecimal preview.
- Empty files have an explicit indication; content above the limit must show a truncation notice.
- Known formats may receive simple presentation, such as Markdown headings or EML subject/body. Opaque formats may provide only text or bytes without claiming full interpretation.
- The contract prohibits executing document scripts, fetching external resources, and writing the original.

Before approving a revision, the engine runs `--crexe-reader-check PATH` against two different text files, an empty file, binary bytes, and a file above the limit. The JSON response must contain `kind`, `content`, and `truncated`, with the expected bytes; the engine also checks that the test files were preserved. Generated code must use the same bounded reading function in the interface and this headless mode.

These checks verify the observable contract; they are neither a sandbox nor proof of the complete interface or generated code's security. The executable runs on the host with the user's permissions. Preventing the engine workflow from sending the document does not provide filesystem/network isolation for a native program. See [security limitations](SECURITY.md).

## Automated verification and local evidence

Run the commands below on the publication candidate revision. Record automated checks, real-model generation, and graphical workflows separately for each operating system.

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --lib
cargo test --locked --test cli
cargo test --locked --test open_generating -- --ignored --nocapture
```

The last command requires .NET 8 SDK and uses a controlled local server, without a real LLM, paid API, or window interaction. It covers cache, arbitrary/missing extensions, text/empty/binary/truncated inputs, repairs sharing the budget, privacy/export, concurrent success/failure, invalid prerequisites, `.crexe` delegation, and rejection of a reader with fixed placeholder content. Unix has one additional case for a path containing non-UTF-8 bytes. Transport unit tests cover all three providers and the planning contract.

The [CI configuration](../../.github/workflows/ci.yml) includes the reader suite on its runners; the presence of that step does not replace checking a run's result for the candidate commit.

## Manual acceptance: CLI, context menu, and picker

Use synthetic documents and a new directory. Record commit/package, host, profile, SDK, provider/model, results, and timing. For a verifiable first generation, use an unprepared type or an isolated test installation without deleting personal caches.

| Workflow | Action and expected observation |
|---|---|
| CLI | Open a file with spaces/accents and another with a leading hyphen using `--`; contents and title match the selected file. |
| Reuse | Open another file with the same extension and different contents; it shows the second contents without planning, generation, or rebuilding. |
| Changed data | Change only the second document's contents and reopen; the preview changes while the reader remains cached. |
| Limits | Open empty, binary, and larger-than-64-KiB files; check empty, hex, and truncation indications. Compare original hashes before/after. |
| Windows Explorer | After installation/association, right-click and select **Gerar com CREXE**; on Windows 11, check **Show more options**. The reader receives the document without a second selection and the default application remains unchanged. |
| Engine picker | Open `configure`, select the document, and follow progress until the reader opens. The engine window remains available; after preparation, select another document. |
| Pending settings | Edit a preference without saving and use the picker; opening uses saved settings and the pending edit remains in the form. |
| Picker cancellation | Open and cancel the picker; no generation starts. |
| Failure before generation | In a test environment without the SDK or with insufficient budget, open a new type; check the error and absence of provider requests. |
| Concurrency/failure | Open two documents of the same type during preparation; check that one preparation is shared. On failure, both receive diagnostics without an automatic new generation from the waiting caller. |

On Windows, also follow the [installation check outside the development environment](../VALIDACAO_INSTALACAO_WINDOWS.md) (Portuguese). For each platform, a real-provider run and interaction with the reader remain separate checks; mocks do not replace that evidence.

## Video walkthrough

1. **Show the intent.** Create `calculator.crexe` containing “Generate a simple calculator with the four arithmetic operations and a green background.” Show the text file and double-click it after registering the association.
2. **Show generation and behavior.** Record the provider/model, phases, and time until the application opens. Calculate **6 × 7 = 42**. Close and reopen to demonstrate cache reuse.
3. **Evolve the intent.** Use **Editar** (Edit) or `crexe edit calculator.crexe`, change the requested background to **blue**, save, and reopen. Show the blue background and repeat **6 × 7 = 42**, checking that the function is still available.
4. **Start from an ordinary document.** Create synthetic text named `sample.new-demo-type` with two recognizable lines. Show its contents and try a double-click. If Windows offers to choose an application because the extension is unknown, cancel that dialog; this behavior depends on the host's associations.
5. **Use Generate with CREXE.** Right-click the same file; on Windows 11, open **Show more options** when needed. Select **Gerar com CREXE**, show the single planning step and generation, and verify actual contents in the reader.
6. **Show the second opening.** Open another document with the same extension and different contents through the same action. Verify the new contents and cache use, distinguishing its timing from first generation.
7. **Show the graphical alternative.** Open the engine window, use **Selecionar arquivo e gerar programa…**, and select a document. Also demonstrate cancelling without generation.

Record the actual latency of planning, generation, build, and opening. If waits are cut or footage is accelerated, label this on screen and give the measured total time. Clearly distinguish first use from cache. Do not present a simulated provider response as LLM generation or a hexadecimal preview as complete format support.
