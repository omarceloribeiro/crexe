//! Engine-owned project conventions; no model-supplied build command is executed.
use anyhow::{bail, Result};
use serde_json::json;
use serde_yaml::Value;

pub(crate) fn resolve(name: &str, prompt: &str, requested: Option<&str>) -> Result<Value> {
    let profile = requested.unwrap_or(if cfg!(windows) {
        "dotnet-winforms"
    } else {
        "dotnet-console"
    });
    if requested.is_none() && !cfg!(windows) {
        bail!("Automatic desktop profiles for this OS are not implemented yet; use legacy YAML with an explicit native toolchain, or --profile dotnet-console for a console application");
    }
    if !matches!(profile, "dotnet-winforms" | "dotnet-console") {
        bail!("Unknown intent profile '{profile}'; available: dotnet-winforms (Windows), dotnet-console. Use legacy YAML for other explicit toolchains.");
    }
    if profile == "dotnet-winforms" && !cfg!(windows) {
        bail!("dotnet-winforms requires a Windows host");
    }
    super::executor::resolve_tool("dotnet")?;
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
    secret_names: &[String],
) -> Result<()> {
    if super::get_path(spec, &["engine_profile"]).is_some() {
        // Resolve the target SDK before spending model tokens. Build remains authoritative.
        super::executor::run(
            &["dotnet".into(), "--list-sdks".into()],
            workspace,
            Some(std::time::Duration::from_secs(15)),
            true,
            secret_names,
        )?;
    }
    Ok(())
}
