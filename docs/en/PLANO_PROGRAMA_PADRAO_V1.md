# v1 plan — Generate with CREXE and keep as the default application

[Português](../PLANO_PROGRAMA_PADRAO_V1.md) · [Generate with CREXE](GENERATE_WITH_CREXE.md)

**Status: minimal implementation added; full graphical acceptance remains pending.** This plan was written on November 25, 2026, against base revision `28e5356` and adopted for this delivery. Commands are described in the [updated guide](GENERATE_WITH_CREXE.md); this document preserves decisions and acceptance criteria without claiming a completed graphical confirmation. The [validation record (Portuguese)](../validation/2026-11-25-kept-reader.md) separates passed checks from unverified steps.

## Recommendation

A Windows MVP can fit before publication if it includes the system's final default-app confirmation. Generate a reader once, choose it for the extension, then open other documents of that type by double-clicking, without a model request or compilation.

The second menu entry is small. Keeping a working reader and integrating Windows' choice requires more work. Registering `open-generating` as the default command is insufficient: changes to provider settings, reader contract or cache can trigger generation again.

The release gate is the complete Windows flow, including cancellation and a second document. A menu item that only opens Settings, without a persistent working reader, does not complete the feature.

## Windows constraint

Windows requires default selection through system UI. Registry-based replacement of protected preferences is unsupported. CREXE should register a candidate and guide the user to choose it, without modifying `UserChoice`, recreating private hashes or bypassing UCPD. [Current Microsoft guidance](https://learn.microsoft.com/en-us/windows/apps/develop/windows-integration/default-apps-platform)

Since Windows 10, the registration flags of `SHOpenWithDialog` cannot set the default. The Explorer interaction **Open with → Choose another app → Always** is a manual alternative, not a guarantee provided by that API. [OPENASINFO](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/ns-shlobj_core-openasinfo) · [Explorer instructions](https://support.microsoft.com/en-us/accessibility/windows/use-a-screen-reader-to-explore-and-navigate-file-explorer-in-windows)

Use `ms-settings:defaultapps`, with `registeredAppUser` and a URI-escaped registered name on supported Windows 11 builds. Older systems fall back to the general page and instructions to select the extension. [Settings deep link](https://learn.microsoft.com/en-us/windows/apps/develop/launch/launch-default-apps-settings) · [Select by file type](https://support.microsoft.com/en-us/windows/apps/change-default-apps-in-windows)

## Implemented components

| Component | Source | Responsibility |
|---|---|---|
| CLI and flow control | `src/lib.rs`, `src/reader.rs` | Explicit keep action, separate launcher and management commands. |
| Accepted revision | `src/orchestration.rs` | A callback receives the accepted revision path; keeping defers launch until the persistent payload exists. |
| Storage and launcher | `src/kept_reader.rs` | Validate the revision, copy source/recipe/payload, register a candidate through a callback and publish the index; open without a provider or compiler. |
| Installed root | `src/installation.rs` | Infer the root from the installed executable when Explorer does not inherit `CREXE_HOME`; require the matching installed engine during preparation. |
| Windows menu | `src/installation/windows.rs` | Separate verbs, single selection and quoted executable/document paths. |
| Candidate and Settings | `src/installation/reader_defaults.rs` | Per-extension ProgID/capabilities, ownership, rollback and Settings URI; no `UserChoice` writes. |
| Observable launch | `src/executor.rs`, `src/presentation.rs` | Native document argument and window observation; the confirmation dialog does not prove default selection. |

Generation passes the accepted revision to storage through the preparation callback. The store also acquires the cache lock and verifies that the revision is still the accepted one before copying it.

## MVP flow

1. Keep **Generate with CREXE** and add **Generate with CREXE and Keep as Default Application**. Classic verbs may appear under **Show more options** on Windows 11.
2. Accept one regular local document with a supported extension. The choice applies to all files with that extension, not just the selected file.
3. Generate or reuse the reader, build and check it with synthetic fixtures. Supply the real document only at launch.
4. Preserve the accepted revision in persistent reader storage, register it as a candidate and open the document. Build completion alone must not be reported as successful application startup.
5. After observing a usable reader window, show **Leitor mantido — CREXE** with **Sim/Não** (Yes/No) to offer opening Windows default-app Settings. If a usable window was not confirmed, retain the reader and print the `reader-defaults choose` command. The user completes the actual choice in Windows.
6. Double-click another `.xyz`: the kept reader opens that document with no LLM, compiler or generation credentials.
7. Closing Settings leaves a registered candidate, not a claimed successful default. If the user later selects another application, CREXE respects it.

In the MVP, the new explicit UI action offers the confirmation dialog whenever a usable reader window is observed, including when reusing a reader. The CLI without `--ui` prints the command for opening Settings. The optimization that skipped confirmation when the reader appeared to be the default was removed: Shell association queries can return a fallback candidate for a new extension before any user choice. Ordinary double-clicks only open the reader and do not show this invitation. The engine does not infer confirmed user choice from registration.

## Minimal implementation commands and separation

Commands in the minimal implementation:

```text
crexe open-generating --keep-as-default --ui -- "document.xyz"
crexe open-kept-reader --extension xyz --ui -- "another document.xyz"
crexe reader-defaults list
crexe reader-defaults choose --extension xyz
crexe reader-defaults forget --extension xyz
```

`--keep-as-default` is Windows-only and incompatible with `--no-run` or `--export`. The execution-only command must not load a provider, create a generation session, build or fall back to `open-generating`. Missing artifacts, incompatible contracts, invalid manifests or missing runtimes produce a local error and instructions for explicit preparation.

A new keep action reuses a valid reader already kept for that extension, even if the model changed. This version has neither automatic updates nor a direct replacement command. To deliberately prepare another reader, forget the current binding and request preparation again. Old objects remain on disk.

## Persistent readers

Implemented layout below `installation::reader_root()`:

```text
reader-apps/
  objects/<manifest-sha256>/
    manifest.json
    app/...
    source/...
    recipe/original.crexe
    recipe/resolved.yaml
  extensions/<extension-sha256>.json
  extensions/<extension-sha256>.lock
  state/<extension-sha256>/
```

The internal manifest records the schema, reader contract, extension, OS/architecture, profile, entrypoint, secret environment variable names to filter, and file hashes. Its hash identifies the object and incorporates those file hashes. It contains neither the selected document's name/path nor credential values. Runtime requirements come from the profile; a .NET payload includes the application's published files, without bundling the .NET runtime.

The store validates the accepted revision, source and inventory, copies to staging, verifies the executable and completed object, registers the candidate, then atomically replaces the index. Failures preserve the previous index; if the first index publication fails after registration, it attempts to remove the new candidate. The public CREXB manifest was not extended; native-binary validation is shared.

Preparation and removal hold an extension lock. Conflicting operations fail with a retry instruction; opening the application does not retain that lock for the process lifetime. Ordinary reader preparation keeps its existing coordination.

Opening reads the extension index and verifies the object without deriving identity from provider/model settings. The document is passed as a native argument, never shell text, and the handler does not pass it to `ShellExecute`. A separate working directory lives under `state/`; the engine filters inherited credentials using common names and names saved in the manifest, without resolving a credential to open the document.

Clearing `readers/` and the generation cache does not remove `reader-apps/`. Objects, source, recipe and state survive even `forget`; this version has no automatic collection of those files, avoiding deletion of data used by applications that are still running.

## Registration and recovery

Register per user without elevation: an owned ProgID, launcher command, friendly name **CREXE — Leitor .xyz**, `OpenWithProgids`, `Capabilities/FileAssociations` and `HKCU\Software\RegisteredApplications`. Advertise only extensions with accepted readers. Candidate registration is distinct from the OS default. [Capability structure](https://learn.microsoft.com/en-us/windows/win32/shell/default-programs) · [Application registration](https://learn.microsoft.com/en-us/windows/win32/shell/app-registration)

The legacy Default Programs article is used for registration structure, not its obsolete default-setting procedures. Current deep-link documentation supports per-user registration. The local graphical fixture showed the candidate in Settings with the correct title; actual selection and double-click remain pending in the validation record.

Registration checks ownership before changes and captures previous values in memory for rollback on error. Ownership metadata lives in the registry; there is no `registrations.json` or durable transaction journal. A lock at `%LOCALAPPDATA%\CREXE\registry.lock`, independent of `CREXE_HOME`, serializes registry changes across installations belonging to the same user. Operations are idempotent and enumerate only candidates owned by this installation. Failure to open Settings retains the prepared reader and shows manual instructions; it does not generate again.

`unassociate` removes both menus and reader candidate registrations owned by the installation while preserving kept indices and objects. `forget` moves the index to a recoverable location before removing the candidate; a registry failure attempts to restore the index and keeps a copy if restoration also fails. After success the active index is removed; objects and state remain on disk. Neither action changes `UserChoice`, other applications' choices, or shared extension keys with unrelated content. If the reader was the default, the user must select another application in Windows; the earlier default is not restored automatically. [ProgID removal](https://learn.microsoft.com/en-us/windows/win32/shell/how-to-register-a-file-type-for-a-new-application)

Preserve existing rejection of missing extensions and `.crexe`/`.crexp`/`.crexb`. Limit the new action to documents by explicitly rejecting executable, script, installer and system-shortcut types. This restriction does not change the original action's byte-preview behavior. Declare and test the restricted set; do not promise default handling for every Windows extension.

## Privacy and scope

Only the document extension reaches the LLM. Do not send its name, path, content, size, timestamps or hash during planning, generation or repair. Keep one base-planning request and the existing normal code-attempt budget. Fixtures and dummy files remain synthetic.

The kept-reader launcher must not initialize LLM integration. Existing read-only and preview limits remain. Persistence adds neither a sandbox nor editing, full proprietary-format support or a proof of generated-code safety.

Out of scope: macOS/Linux default associations, modern COM context-menu integration, extensionless detection, byte-based classification, silent default replacement, Store publication, corporate policies and multiple selectable readers for one extension. Existing generation remains available on its supported platforms.

## Implementation status

1. **Persistence, launcher, menu and registration:** implemented, with automated storage, command, registry ownership and rollback tests.
2. **Generation and reuse:** the Windows roundtrip with a controlled provider passed with one planning and one generation request, opening other documents without further requests and surviving removal of the generation cache/configuration.
3. **First window and Settings:** the WinForms fixture showed the first document and Settings with the correct candidate. The next binary also showed the reader and the complete Yes/No invitation. This does not prove default selection or cancellation.
4. **Remaining graphical acceptance:** accept/cancel the system choice, double-click a second document, select another application and remove the candidate, recording the Windows version/build. Available automation could not safely operate Settings or the invitation controls.
5. **Delivery:** the final local suite passed 148 tests plus 9 generation tests run explicitly. Record the last formatting/clippy check after the timeout-test adjustment and check CI. Do not claim full graphical support for both Windows 10 and 11 based only on these local checks.

## Release acceptance

| Area | Required evidence |
|---|---|
| Privacy | Controlled provider captures all requests; real-document markers never appear. One plan plus normal code attempts. |
| Reuse | Other files of the same extension open with no requests, including after provider/model changes or unavailable credentials. |
| Cache independence | Remove only the test generation cache; the kept reader still opens without reconstruction. |
| Arguments | Spaces, Unicode, initial hyphens and metacharacters remain arguments. Wrong extension, missing file and directory do not trigger generation. |
| Integrity | Interrupted staging, changed files, links and missing runtime fail locally while preserving the previous accepted reader/index. |
| Concurrency | Coordinated preparation, concurrent document windows and updates that preserve running readers. |
| Registry | Isolated HKCU fixtures verify commands, ownership, rollback and idempotent removal without changing actual defaults. |
| Windows UI | New extension, accepted choice, cancellation, another app selected, reopening and removal; record OS build and actual double-click evidence. |
| Regression | Formatting, clippy, unit/CLI/reader/association tests and CI on all three hosts. |
| Publication | PT/EN docs describe Windows confirmation and later execution without LLM, not universal automatic association. |

Registry fixtures and a `ShellExecute` smoke test do not replace actual selection in Windows UI. Do not advertise the full flow until this manual acceptance passes.

## Demonstration

Double-click an unknown document → Windows has no app → choose the new CREXE menu → generate and read → confirm the Windows default → double-click a second document with the same extension → read its actual content immediately.

This is a moderate, bounded addition with a clear v1 benefit. System confirmation is part of the experience; the essential engineering is keeping the generated program usable beyond its generation session.
