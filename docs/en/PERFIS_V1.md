# Intent profiles and prerequisites

All profiles compile on the user's host, in a temporary directory. The engine does not install SDKs, emulate another OS, or use Windows Sandbox. The new native profiles are **experimental** until they complete the real-generation and GUI-use trials specified in the plan.

| Profile | Host | SDK/tools | Output |
|---|---|---|---|
| `dotnet-winforms` | Windows | .NET 8 SDK on PATH | C#, Windows Forms, engine-supplied project |
| `dotnet-console` | Windows/Linux/macOS | .NET 8 SDK on PATH | C#, console; explicit selection |
| `cpp-win32` | Windows | MinGW-w64 `g++` on PATH, target compatible with the engine | Multiple `.cpp/.h` files, Unicode Win32, executable with statically linked GCC runtime |
| `objc-cocoa` | macOS | Apple Clang/Xcode Command Line Tools and Cocoa SDK | Multiple `.mm/.h` files, ARC, native executable |
| `c-gtk` | Linux | `gcc`, GNU Make, `pkg-config`, and GTK3 development files | Multiple `.c/.h` files, engine-supplied Makefile |
| `dotnet-opengl` | Windows | .NET 8 SDK and OpenGL 3.3 driver | C#, OpenTK 4.9.4, project and NuGet lockfile |
| `objc-cocoa-opengl` | macOS | Apple Clang, Cocoa, and OpenGL framework; 3.2 core context | Objective-C++, `NSOpenGLView` |
| `c-gtk-opengl` | Linux | GCC, Make, GTK3, and libepoxy through `pkg-config`; 3.3 context | C, `GtkGLArea`, engine-supplied Makefile |

During automatic selection, the model receives the catalog of profiles that passed SDK and local-policy checks. The initial preference is Windows Forms/.NET on Windows, GTK3 on Linux, and Cocoa on macOS; subsequent revisions prefer the baseline profile. It may change technology with a reason in the same generation response, without an additional planning call. “3D buttons” is not a fixed selection rule. `--profile name`, Markdown's `profile` metadata, and YAML targets fix the technology and prevent model-driven changes.

Before calling the model, .NET requires an installed SDK 8 and the native profiles check the compiler target. GTK also checks `pkg-config --exists gtk+-3.0`. Link/build failures remain possible and produce diagnostics. This basic check does not detect every third-party SDK, driver, or prompt requirement.

`dotnet-opengl` uses [`OpenTK` 4.9.4](https://www.nuget.org/packages/OpenTK/4.9.4), with an exact version in the project, `dotnet restore --use-lock-file`, and subsequent restores in locked mode. The approved lockfile accompanies incremental revisions and ZIPs. The engine restores libraries per project; it does not install SDKs/compilers. GTK3/libepoxy and Apple frameworks are host prerequisites; the engine does not download or lock system packages.

On Linux, `pkg-config` also checks libepoxy; the contract uses [`GtkGLArea`](https://docs.gtk.org/gtk3/class.GLArea.html) and requires an error diagnostic when context creation fails. On macOS, [`NSOpenGLView`](https://developer.apple.com/documentation/appkit/nsopenglview?language=objc) and OpenGL are deprecated. The OpenGL contracts require checking the context/driver at startup and reporting incompatibility, as well as providing `--crexe-graphics-check` to create a context, compile shaders, and render one frame. Logic self-tests remain display-free; an approved build does not prove graphics support on the destination.

When generating a program to open it, the engine also runs `--crexe-graphics-check` before approving the revision; when reopening from cache, it repeats the check before starting the interface. `--no-run` keeps the flow display-free while running logic tests. The graphics test has its own test timeout and diagnostics; its implementation is part of the generated sources, just like the logic self-test.

`crexe doctor` shows the profile, language, SDK requirement, and tool paths without executing them. `crexe doctor --check` runs the same local probes used at the start of generation; `--profile name` selects another profile. Queries respect the local tool policy, have a 15-second timeout each, and do not call providers, read keys, install tools, or compile an application. The command fails when an SDK is missing or the policy prevents the query. Finding a path in plain `doctor` output does not mean the SDK has been validated.

`doctor` and `inspect` distinguish the engine's architecture from the architecture reported by the OS. Windows queries [IsWow64Process2](https://learn.microsoft.com/en-us/windows/win32/api/wow64apiset/nf-wow64apiset-iswow64process2), loaded dynamically; macOS queries [sysctl.proc_translated](https://developer.apple.com/documentation/apple-silicon/about-the-rosetta-translation-environment) to recognize Rosetta and uses `uname` in the native process; Linux uses `uname`. If detection fails, the host is reported as unknown. This observation describes the environment visible to the OS, not proof of physical hardware outside a VM or container. The build and `ARCH` continue to use the engine's architecture; a different detected architecture does not enable cross-compilation. Real emulation scenarios still require validation.

Compilation commands belong to the engine. C++/Cocoa receive the explicit list of generated sources, without limiting the project to one file. GTK receives a Makefile that resolves flags on the compiling machine, using the [build process documented by GTK](https://docs.gtk.org/gtk3/compiling.html). Headers and resources may accompany sources; these profiles do not include a general external dependency manager.

All profiles request `--crexe-self-test` before starting the UI and run that test before accepting the cache. This test is produced by the model and does not replace independent tests. `--no-run` still compiles and tests. The approved revision records the effective profile and commands; caching, repairs, and export use that profile. Older responses containing only `files` keep the current profile.

The ZIP includes sources, the manifest, scripts, and build files. For C++/Cocoa/GTK, publish recompiles a `CrexeApp-publish` executable; it does not create an installer, a GTK library package, a signature/notarization, or an `.app` bundle. In .NET, publish is runtime-dependent. Required system libraries and SDKs must be present on the destination; there is no promise of a universal binary or ready-to-use distribution between arbitrary machines.

## Automated verification

`cargo test --locked` tests the engine without additional application SDKs. The native trial is separate:

```sh
cargo test --locked --test native_profiles -- --ignored --nocapture
```

It receives two fixed sources from a local mock Ollama server, compiles, runs the display-free logic test, exports/recompiles the ZIP through the scripts, and publishes to another directory. The cache is also opened with an empty PATH and the server shut down. It does not use a real model or interact with windows.

CI prepares the SDK only on disposable runners: [MSYS2 UCRT64](https://github.com/msys2/setup-msys2) on Windows, GTK3 packages on Ubuntu, and the existing Apple SDK on macOS. These CI steps are not end-user installers. Results and limitations are recorded in the [implementation log](../IMPLEMENTACAO_V1.md).

The OpenGL migration trial uses controlled sources and verifies context continuity, a profile change, a subsequent increment, caching, the lockfile where applicable, export, and recompilation:

```sh
cargo test --locked --test opengl_profiles -- --ignored --nocapture
```

Set `CREXE_TEST_GRAPHICS=1` in a graphical session with a compatible driver to also run rendering. Without that variable, the test compiles the OpenGL sources and runs only logic checks. CI includes this trial on all three hosts; inclusion in the workflow does not mean that a remote run of this increment has already passed.
