# v1 plan — Generate with CREXE and keep as the default application

[Português](../PLANO_PROGRAMA_PADRAO_V1.md) · [Generate with CREXE](GENERATE_WITH_CREXE.md)

**Status: implementation proposal; this feature is not implemented yet.** Analysis dated November 25, 2026, against revision `28e5356`. This document does not install readers or change Windows associations.

## Recommendation

A Windows MVP can fit before publication if it includes the system's final default-app confirmation. Generate a reader once, choose it for the extension, then open other documents of that type by double-clicking, without a model request or compilation.

The second menu entry is small. Keeping a working reader and integrating Windows' choice requires more work. Registering `open-generating` as the default command is insufficient: changes to provider settings, reader contract or cache can trigger generation again.

The release gate is the complete Windows flow, including cancellation and a second document. A menu item that only opens Settings, without a persistent working reader, does not complete the feature.

## Windows constraint

Windows requires default selection through system UI. Registry-based replacement of protected preferences is unsupported. CREXE should register a candidate and guide the user to choose it, without modifying `UserChoice`, recreating private hashes or bypassing UCPD. [Current Microsoft guidance](https://learn.microsoft.com/en-us/windows/apps/develop/windows-integration/default-apps-platform)

Since Windows 10, the registration flags of `SHOpenWithDialog` cannot set the default. The Explorer interaction **Open with → Choose another app → Always** is a manual alternative, not a guarantee provided by that API. [OPENASINFO](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/ns-shlobj_core-openasinfo) · [Explorer instructions](https://support.microsoft.com/en-us/accessibility/windows/use-a-screen-reader-to-explore-and-navigate-file-explorer-in-windows)

Use `ms-settings:defaultapps`, with `registeredAppUser` and a URI-escaped registered name on supported Windows 11 builds. Older systems fall back to the general page and instructions to select the extension. [Settings deep link](https://learn.microsoft.com/en-us/windows/apps/develop/launch/launch-default-apps-settings) · [Select by file type](https://support.microsoft.com/en-us/windows/apps/change-default-apps-in-windows)

## Existing implementation

Line references apply to the reviewed revision:

| Existing component | Source | Planned use |
|---|---|---|
| Per-user, single-file menu with quoted `%1` and `--` | `src/installation/windows.rs:58`, `:149` | Add a separate verb without changing the existing action. |
| Stable installed engine path | `src/installation.rs:43` | Registry commands use the installed engine, not an extracted temporary package. |
| Extension-only planning and saved plan | `src/reader.rs:82`, `:154`, `:218` | Preserve privacy and the single planning request. |
| Provider/model and contract in reader identity | `src/reader.rs:196` | Default execution must not recalculate its identity from current generation settings. |
| Accepted revision after build and checks | `src/orchestration.rs:458`, `:509` | Keep only accepted output; failures preserve the previous reader. |
| Preparation callback before launch | `src/orchestration.rs:557` | Prepared is not the same as successfully started. |
| Native document argument | `src/executor.rs:116`, `:190` | Keep direct process launch and `OsString`. |
| Hashed revisions and atomic publication | `src/cache.rs:55`, `:96` | Reuse bounded copy and validation in persistent storage. |
| Accepted CREXB payload export | `src/crexb.rs:343` | Reuse helpers where appropriate; current bundle launch (`:975`) does not accept a document. |
| Isolated registry fixtures | `src/installation/windows.rs:228` | Automated tests need not alter the user's real defaults. |

## MVP flow

1. Keep **Generate with CREXE** and add **Generate with CREXE and Keep as Default Application**. Classic verbs may appear under **Show more options** on Windows 11.
2. Accept one regular local document with a supported extension. The choice applies to all files with that extension, not just the selected file.
3. Generate or reuse the reader, build and check it with synthetic fixtures. Supply the real document only at launch.
4. Preserve the accepted revision in persistent reader storage, register it as a candidate and open the document. Build completion alone must not be reported as successful application startup.
5. Explain that the reader is ready for `.xyz` and offer **Choose as default in Windows**. The user can inspect the reader before completing the system selection.
6. Double-click another `.xyz`: the kept reader opens that document with no LLM, compiler or generation credentials.
7. Closing Settings leaves a registered candidate, not a claimed successful default. If the user later selects another application, CREXE respects it.

If the reader is already installed and effectively selected, open it directly. Query the OS to establish that state; the existence of our registration is insufficient. [QueryCurrentDefault](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-iapplicationassociationregistration-querycurrentdefault)

## Proposed commands and separation

These commands are proposed, not available today:

```text
crexe open-generating --keep-as-default --ui -- "document.xyz"
crexe open-kept-reader --ui -- "another document.xyz"
crexe reader-defaults list
crexe reader-defaults forget --extension xyz
```

`--keep-as-default` is Windows-only and incompatible with `--no-run`. The execution-only command must not load a provider, create a generation session, build or fall back to `open-generating`. Missing artifacts, incompatible contracts, invalid manifests or missing runtimes produce a local error and instructions for explicit preparation.

Separate reader preparation from launch. Return a typed `PreparedReader` containing the accepted revision, profile, target and contract. The persistence layer consumes this result rather than selecting an attempt directory. Preserve preparation locks and concurrent-open behavior.

## Persistent readers

Proposed layout below `installation::root()`:

```text
reader-apps/
  objects/<payload-hash>/
    manifest.json
    app/...
    recipe/...
  extensions/<extension-hash>.json
  registrations.json
```

The versioned manifest records the normalized extension, profile, target, contract, relative entrypoint, runtime requirement and file hashes. The extension index is independent of the currently configured provider. Do not store document names, paths or secrets. Preserve the full runtime payload, not just one executable.

Stage and validate a copy of the accepted revision, reject unsafe paths and links/reparse points, then atomically publish the index. Clearing generation cache must not delete a kept reader. Engine upgrades preserve this directory. Updating a reader is explicit and switches the index only after checks succeed; keep in-use files until safe later cleanup.

The launcher verifies the payload and launches only the entrypoint allowed by the contract. Pass the new document as a native argument, never shell text or a template. Do not call `ShellExecute` on the document inside the association handler, which would recurse. Retain secret filtering in the child environment without resolving generation credentials; locally recorded variable names may support that filtering, but never store their values.

CREXB payload/runtime helpers may be reused internally. Do not extend the public bundle schema or allow arbitrary archive-supplied arguments solely for this feature. The document argument originates only from the local launcher invocation. Concurrent opens share immutable files without an exclusive lock held for the application's lifetime.

## Registration and recovery

Register per user without elevation: an owned ProgID, launcher command, friendly name **CREXE — .xyz Reader**, `OpenWithProgids`, `Capabilities/FileAssociations` and `HKCU\Software\RegisteredApplications`. Advertise only extensions with accepted readers. Candidate registration is distinct from the OS default. [Capability structure](https://learn.microsoft.com/en-us/windows/win32/shell/default-programs) · [Application registration](https://learn.microsoft.com/en-us/windows/win32/shell/app-registration)

The legacy Default Programs article is used for registration structure, not its obsolete default-setting procedures. Current deep-link documentation supports per-user registration. The first implementation spike must verify that a newly invented extension appears in Settings with the correct reader.

Track owned entries, check ownership before changes and support repeatable operations. Registration failure must preserve a previous reader and avoid keys pointing to nonexistent files. Failure to open Settings retains the preparation and shows manual instructions; it must not generate again.

Extend `unassociate` to remove both menus and reader registrations owned by the installation. Preserve shared extension keys, other applications and `UserChoice`. Keep payloads until explicit `forget`; if the reader remains the effective default, guide the user to another app in Settings. Do not promise automatic restoration of a former default. [ProgID removal](https://learn.microsoft.com/en-us/windows/win32/shell/how-to-register-a-file-type-for-a-new-application)

Preserve existing rejection of missing extensions and `.crexe`/`.crexp`/`.crexb`. Limit the new action to documents by explicitly rejecting executable, script, installer and system-shortcut types. This restriction does not change the original action's byte-preview behavior. Declare and test the restricted set; do not promise default handling for every Windows extension.

## Privacy and scope

Only the document extension reaches the LLM. Do not send its name, path, content, size, timestamps or hash during planning, generation or repair. Keep one base-planning request and the existing normal code-attempt budget. Fixtures and dummy files remain synthetic.

The kept-reader launcher must not initialize LLM integration. Existing read-only and preview limits remain. Persistence adds neither a sandbox nor editing, full proprietary-format support or a proof of generated-code safety.

Out of scope: macOS/Linux default associations, modern COM context-menu integration, extensionless detection, byte-based classification, silent default replacement, Store publication, corporate policies and multiple selectable readers for one extension. Existing generation remains available on its supported platforms.

## Implementation order

1. Prove candidate registration, Settings selection and double-click with a fixture reader and invented extension in a Windows test user/VM. Validate Windows 11 and Windows 10 fallback before claiming both.
2. Implement persistent manifests, atomic promotion and provider-independent execution. This is the core feature.
3. Integrate the accepted preparation result while preserving privacy, budgets, checks and concurrency.
4. Add the menu, capabilities, Settings entry point and distinct prepared/pending/confirmed states. A generic process-exit callback is not proof that its UI opened.
5. Cover removal and recovery, update PT/EN guides/site and deliver a separate commit from the download counter.

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
