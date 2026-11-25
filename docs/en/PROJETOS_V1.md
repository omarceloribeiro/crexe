# V1 project operations

`exec` transforms an intent into a validated project. The operations below take a **directory containing CREXE-PROJECT.json**, such as a revision or an extracted ZIP. They do not take a `.crexe` file or start AI generation/repair.

| Command | Result |
|---|---|
| `crexe build pasta --output nova-pasta` | Copies sources into a temporary workspace, compiles, runs defined tests, and delivers the project/artifacts in a new directory. |
| `crexe test pasta` | Recompiles and runs defined tests; fails if there is no test operation. |
| `crexe run pasta` | Executes the existing artifact declared in the manifest; does not compile or select another executable. |
| `crexe publish pasta --output nova-pasta` | Compiles, tests when defined, and runs publish if it exists. Produces local files; does not deploy. |
| `crexe export pasta --output novo.zip` | Packages current sources, the manifest, and scripts, without build/test. Does not claim that subsequent edits have been validated. |

Existing destinations are rejected. Build/test/publish preserve the source project, including on failure. The diagnostic workspace is retained when an operation fails. Edit sources in the extracted ZIP or a working copy, preserving the cached revision.

The version 1 manifest records the name, OS, engine architecture, profile, source list, and argument vectors for each operation. Build/run are mandatory; test/publish are optional. The engine generates BAT or SH scripts from those same lists. Reserved metadata and scripts are written by the engine, not replaced by the model's response.

Execution operations check the recorded OS and architecture; they do not perform implicit cross-compilation. Export can be used to inspect/share sources on another OS. The manifest is reviewable local data, not a trust signature. Changing commands does not bypass the CLI's local tool policy.

In the .NET profile, publish uses `dotnet publish --self-contained false`: the recipient still needs a compatible runtime. The ZIP includes sources and instructions, without an SDK, package cache, or credentials. Using the scripts without CREXE requires preparing the specified SDK; they execute directly on the host and do not apply the engine's TOML policy.

Missing manifests in older projects are not inferred from discovered executables. Scripts from an older ZIP remain available; generate a project with the current engine to use the new operations.
