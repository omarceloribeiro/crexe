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
        "cpp-win32" => (
            "C++17",
            &["g++"],
            "MinGW-w64 / Win32, matching the engine architecture",
        ),
        "objc-cocoa" => (
            "Objective-C++17",
            &["clang++"],
            "Apple Clang / Cocoa SDK, matching the engine architecture",
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
    if matches!(profile, "cpp-win32" | "objc-cocoa" | "c-gtk") {
        return native::resolve(name, prompt, profile);
    }
    if !matches!(profile, "dotnet-winforms" | "dotnet-console") {
        bail!("Unknown intent profile '{profile}'; available: dotnet-winforms, dotnet-console, cpp-win32, objc-cocoa, c-gtk");
    }
    if profile == "dotnet-winforms" && !cfg!(windows) {
        bail!("dotnet-winforms requires a Windows host");
    }
    let gui = profile == "dotnet-winforms";
    let output = if gui { "WinExe" } else { "Exe" };
    let framework = if gui { "net8.0-windows" } else { "net8.0" };
    let forms = if gui {
        "<UseWindowsForms>true</UseWindowsForms>"
    } else {
        ""
    };
    let project = format!("<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><OutputType>{output}</OutputType><TargetFramework>{framework}</TargetFramework>{forms}<ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable><AssemblyName>CrexeApp</AssemblyName></PropertyGroup></Project>\n");
    let system = format!(
        "You develop a complete small {profile} application for {os} {arch}, using .NET 8 and only the standard library. Display name: {name}. Return JSON files[] with complete sources. Organize UI and logic in separate .cs files when useful. The engine supplies CrexeApp.csproj; do not generate project files, scripts or build commands. Use a single explicit Program.Main entry point. It MUST check for --crexe-self-test before opening any window; that branch runs meaningful deterministic assertions of the app logic, exits 0 on success, nonzero on failure, and does not access external services or devices. No network operations on startup. No global installation. Desktop requirements unsupported by this profile must be stated in a README, not silently replaced. The engine compiles and can return diagnostics for correction. C# arithmetic uses normal C# syntax. For WinForms, use System.Windows.Forms and System.Drawing; call ApplicationConfiguration.Initialize() before creating the main form. Never use WPF/XAML in this profile.",
        os=std::env::consts::OS, arch=std::env::consts::ARCH
    );
    Ok(serde_yaml::to_value(json!({
        "version": "1.0", "meta": {"name": name},
        "engine_profile": profile, "engine_project": project,
        "selectors": {"target": {"fallback": "host"}},
        "prompt": {"system": system, "user_by_target": {"host": prompt}},
        "policies": {"timeoutSeconds": {"build": 180, "test": 30}, "commandAllowlist": { std::env::consts::OS: ["dotnet"] }},
        "targets": {"host": {
            "build": {"steps": [{"cmd": ["dotnet", "build", "CrexeApp.csproj", "--configuration", "Release", "--output", "build", "--nologo"]}]},
            "test": {"steps": [{"cmd": ["dotnet", "build/CrexeApp.dll", "--crexe-self-test"]}]},
            "run": {"cmd": ["build/CrexeApp{{EXE_EXT}}"]}
        }}
    }))?)
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
