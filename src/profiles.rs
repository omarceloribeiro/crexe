//! Engine-owned project conventions; no model-supplied build command is executed.
use anyhow::{bail, Result};
use serde_json::json;
use serde_yaml::Value;

mod native;

pub(crate) fn default_profile() -> &'static str {
    if cfg!(windows) {
        // Finding a command does not prove SDK compatibility; preflight verifies it.
        if super::executor::resolve_tool("dotnet").is_err()
            && super::executor::resolve_tool("g++").is_ok()
        {
            "cpp-win32"
        } else {
            "dotnet-winforms"
        }
    } else if cfg!(target_os = "macos") {
        "objc-cocoa"
    } else if cfg!(target_os = "linux") {
        "c-gtk"
    } else {
        "dotnet-console"
    }
}

#[derive(serde::Serialize)]
pub(crate) struct Requirements {
    pub language: &'static str,
    pub tools: &'static [&'static str],
    pub sdk: &'static str,
}

pub(crate) fn requirements(profile: &str) -> Result<Requirements> {
    let (language, tools, sdk): (_, &[_], _) = match profile {
        "dotnet-winforms" => ("C#", &["dotnet"], ".NET 8 SDK / Windows Forms"),
        "dotnet-console" => ("C#", &["dotnet"], ".NET 8 SDK"),
        "dotnet-opengl" => (
            "C# / OpenGL",
            &["dotnet"],
            ".NET 8 SDK / OpenTK 4.9.4 / Windows OpenGL 3.3 driver",
        ),
        "cpp-win32" => (
            "C++17",
            &["g++"],
            "MinGW-w64 / Win32, matching the engine architecture",
        ),
        "objc-cocoa-opengl" => (
            "Objective-C++17 / OpenGL",
            &["clang++"],
            "Apple Clang / Cocoa / deprecated OpenGL framework / OpenGL 3.2 core driver",
        ),
        "objc-cocoa" => (
            "Objective-C++17",
            &["clang++"],
            "Apple Clang / Cocoa SDK, matching the engine architecture",
        ),
        "c-gtk-opengl" => (
            "C17 / OpenGL",
            &["gcc", "make", "pkg-config"],
            "GTK3 and libepoxy development headers / GtkGLArea / OpenGL 3.3 driver",
        ),
        "c-gtk" => (
            "C17",
            &["gcc", "make", "pkg-config"],
            "GTK3 development headers and libraries, matching the engine architecture",
        ),
        _ => bail!("Unknown intent profile '{profile}'"),
    };
    Ok(Requirements {
        language,
        tools,
        sdk,
    })
}

pub(crate) fn resolve(name: &str, prompt: &str, requested: Option<&str>) -> Result<Value> {
    let profile = requested.unwrap_or_else(|| default_profile());
    if matches!(
        profile,
        "cpp-win32" | "objc-cocoa" | "c-gtk" | "objc-cocoa-opengl" | "c-gtk-opengl"
    ) {
        return native::resolve(name, prompt, profile);
    }
    if !matches!(
        profile,
        "dotnet-winforms" | "dotnet-console" | "dotnet-opengl"
    ) {
        bail!("Unknown intent profile '{profile}'; available: dotnet-winforms, dotnet-console, dotnet-opengl, cpp-win32, objc-cocoa, objc-cocoa-opengl, c-gtk, c-gtk-opengl");
    }
    if matches!(profile, "dotnet-winforms" | "dotnet-opengl") && !cfg!(windows) {
        bail!("{profile} requires a Windows host");
    }
    let opengl = profile == "dotnet-opengl";
    let gui = profile != "dotnet-console";
    let output = if gui { "WinExe" } else { "Exe" };
    let framework = if gui { "net8.0-windows" } else { "net8.0" };
    let forms = if profile == "dotnet-winforms" {
        "<UseWindowsForms>true</UseWindowsForms>"
    } else {
        ""
    };
    let packages = if opengl {
        "<ItemGroup><PackageReference Include=\"OpenTK\" Version=\"[4.9.4]\" /></ItemGroup><PropertyGroup><RestorePackagesWithLockFile>true</RestorePackagesWithLockFile><RestoreLockedMode Condition=\"Exists('packages.lock.json')\">true</RestoreLockedMode></PropertyGroup>"
    } else {
        ""
    };
    let project = format!("<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><OutputType>{output}</OutputType><TargetFramework>{framework}</TargetFramework>{forms}<ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable><AssemblyName>CrexeApp</AssemblyName></PropertyGroup>{packages}</Project>\n");
    let mut system = format!(
        "You develop a complete small {profile} application for {os} {arch}, using .NET 8 and only the standard library. Display name: {name}. Return JSON files[] with complete sources. Organize UI and logic in separate .cs files when useful. The engine supplies CrexeApp.csproj; do not generate project files, scripts or build commands. Use a single explicit Program.Main entry point. It MUST check for --crexe-self-test before opening any window; that branch runs meaningful deterministic assertions of the app logic, exits 0 on success, nonzero on failure, and does not access external services or devices. No network operations on startup. No global installation. Desktop requirements unsupported by this profile must be stated in a README, not silently replaced. The engine compiles and can return diagnostics for correction. C# arithmetic uses normal C# syntax. For WinForms, use System.Windows.Forms and System.Drawing; call ApplicationConfiguration.Initialize() before creating the main form. Never use WPF/XAML in this profile.",
        os=std::env::consts::OS, arch=std::env::consts::ARCH
    );
    if opengl {
        system = format!("Develop a Windows C# .NET 8 application with OpenTK 4.9.4 and desktop OpenGL 3.3 core. Display name: {name}. Use OpenTK.Windowing.Desktop.GameWindow, NativeWindowSettings.ClientSize and OpenTK.Graphics.OpenGL4. The engine supplies CrexeApp.csproj and restores the exact NuGet package; do not return project files, package lockfiles or scripts. Return complete sources in files[]. Use inline vertex/fragment shaders, separate UI/rendering and application logic, and one explicit Program.Main. Before creating ANY window, handle --crexe-self-test with meaningful deterministic logic assertions and no graphics, device or external service access, exiting 0/nonzero. Handle --crexe-graphics-check by creating a context, compiling shaders, rendering one frame and exiting 0/nonzero. In normal startup validate the context/version and show a clear error if the driver cannot provide OpenGL 3.3; never silently replace 3D with an unrelated 2D UI. No network on startup or global installs. Support mouse/keyboard interaction and resizing. Only standard .NET and the supplied OpenTK dependency are allowed.");
    }
    // Commands own their process groups and terminate descendants at completion.
    // Do not share a persistent compiler/server with another concurrent build.
    let mut spec = serde_yaml::to_value(json!({
        "version": "1.0", "meta": {"name": name},
        "engine_profile": profile, "engine_project": project,
        "selectors": {"target": {"fallback": "host"}},
        "prompt": {"system": system, "user_by_target": {"host": prompt}},
        "policies": {"timeoutSeconds": {"build": 180, "test": 30}, "commandAllowlist": { std::env::consts::OS: ["dotnet"] }},
        "targets": {"host": {
            "build": {"steps": [{"cmd": ["dotnet", "build", "CrexeApp.csproj", "--disable-build-servers", "--configuration", "Release", "--output", "build", "--nologo"]}]},
            "test": {"steps": [{"cmd": ["dotnet", "build/CrexeApp.dll", "--crexe-self-test"]}]},
            "run": {"cmd": ["build/CrexeApp{{EXE_EXT}}"]}
        }}
    }))?;
    if opengl {
        spec["targets"]["host"]["build"]["steps"] = serde_yaml::to_value(json!([
            {"cmd": ["dotnet", "restore", "CrexeApp.csproj", "--disable-build-servers", "--use-lock-file", "--nologo"]},
            {"cmd": ["dotnet", "build", "CrexeApp.csproj", "--disable-build-servers", "--no-restore", "--configuration", "Release", "--output", "build", "--nologo"]}
        ]))?;
    }
    Ok(spec)
}

pub(crate) fn validate_files(spec: &Value, files: &[super::GeneratedFile]) -> Result<()> {
    if super::get_path(spec, &["engine_profile"]).is_none() {
        return Ok(());
    }
    for file in files {
        let path = file.path.replace('\\', "/").to_lowercase();
        if path.ends_with(".csproj")
            || path.ends_with(".props")
            || path.ends_with(".targets")
            || path.ends_with("nuget.config")
            || path.ends_with("packages.lock.json")
            || path.ends_with("crexebuild.mk")
        {
            bail!(
                "Project and dependency files are owned by the engine: {}",
                file.path
            );
        }
    }
    Ok(())
}

pub(crate) fn available(
    name: &str,
    prompt: &str,
    workspace: &std::path::Path,
    settings: &super::config::Settings,
) -> Result<Vec<String>> {
    let candidates: &[&str] = if cfg!(windows) {
        &[
            "dotnet-winforms",
            "dotnet-console",
            "cpp-win32",
            "dotnet-opengl",
        ]
    } else if cfg!(target_os = "macos") {
        &["objc-cocoa", "objc-cocoa-opengl", "dotnet-console"]
    } else {
        &["c-gtk", "c-gtk-opengl", "dotnet-console"]
    };
    let mut profiles = Vec::new();
    let mut failures = Vec::new();
    for candidate in candidates {
        match resolve(name, prompt, Some(candidate))
            .and_then(|spec| preflight(&spec, workspace, settings))
        {
            Ok(()) => profiles.push((*candidate).into()),
            Err(error) => failures.push(format!("{candidate}: {error}")),
        }
    }
    if profiles.is_empty() {
        bail!(
            "Nenhum perfil disponível; verifique os SDKs e ferramentas locais. {}",
            failures.join("; ")
        );
    }
    Ok(profiles)
}

pub(crate) fn selection_prompt(
    name: &str,
    prompt: &str,
    allowed: &[String],
    preferred: &str,
) -> Result<String> {
    let mut contracts = Vec::new();
    for profile in allowed {
        let spec = resolve(name, prompt, Some(profile))?;
        contracts.push(json!({"profile": profile, "contract": super::get_path(&spec, &["prompt", "system"]).and_then(Value::as_str)}));
    }
    Ok(format!("Implement an application for {} / {} only. Prefer profile {preferred}, especially for small changes to the baseline. Choose exactly one of these available profiles and follow ONLY its contract. Return profile, change_kind (incremental or recreate), reason and the FULL files[] in a single JSON object. A change of technology requires a reason. Do not invent dependencies or build commands. Profile contracts:\n{}", std::env::consts::OS, std::env::consts::ARCH, serde_json::to_string(&contracts)?))
}

pub(crate) fn scaffold(spec: &Value, workspace: &std::path::Path) -> Result<()> {
    if let Some(project) = super::get_path(spec, &["engine_project"]).and_then(Value::as_str) {
        std::fs::write(workspace.join("CrexeApp.csproj"), project)?;
    }
    Ok(())
}

pub(crate) fn preflight(
    spec: &Value,
    workspace: &std::path::Path,
    settings: &super::config::Settings,
) -> Result<()> {
    let Some(profile) = super::get_path(spec, &["engine_profile"]).and_then(Value::as_str) else {
        return Ok(());
    };
    if profile.starts_with("dotnet-") {
        settings.execution.authorize("dotnet")?;
        let sdks = super::executor::output(
            &["dotnet".into(), "--list-sdks".into()],
            workspace,
            &settings.secret_names,
        )?;
        if !sdks.lines().any(|line| line.starts_with("8.")) {
            bail!("This profile requires the .NET 8 SDK; install it before generating. A runtime alone is insufficient.");
        }
    } else {
        native::preflight(profile, workspace, settings)?;
    }
    Ok(())
}

pub(crate) fn materialize(
    spec: &Value,
    workspace: &std::path::Path,
    files: &[super::GeneratedFile],
    ctx: &std::collections::BTreeMap<String, String>,
) -> Result<Value> {
    native::materialize(spec, workspace, files, ctx)
}
