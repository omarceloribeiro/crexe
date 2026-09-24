//! Small host-native profiles. Platform SDKs remain explicit prerequisites.
use crate::{config::Settings, get_path, GeneratedFile};
use anyhow::{bail, Context, Result};
use serde_json::json;
use serde_yaml::Value;
use std::{collections::BTreeMap, fs, path::Path};

const SOURCES: &str = "$CREXE_SOURCES";
pub(super) const MAKEFILE: &str = "CrexeBuild.mk";

pub(super) fn resolve(name: &str, prompt: &str, profile: &str) -> Result<Value> {
    let (os, language, build): (&str, &str, Vec<&str>) = match profile {
        "cpp-win32" => (
            "windows", "C++17 and the Win32 API (Unicode). Use wWinMain, Windows headers, user32/gdi32. Separate application logic from UI into .cpp/.h files. Check --crexe-self-test in the command line before creating windows.",
            vec!["g++", "-std=c++17", "-O2", "-municode", "-mwindows", "-static", SOURCES, "-o", "CrexeApp.exe", "-luser32", "-lgdi32"],
        ),
        "objc-cocoa" => (
            "macos", "Objective-C++17 and Cocoa/AppKit with ARC. Use .mm/.h sources, one main(int argc, char **argv). Separate application logic and UI. Check --crexe-self-test before starting NSApplication. Build is a native executable, not a signed .app bundle.",
            vec!["clang++", "-std=c++17", "-O2", "-fobjc-arc", "-arch", if cfg!(target_arch = "aarch64") { "arm64" } else { std::env::consts::ARCH }, SOURCES, "-framework", "Cocoa", "-o", "CrexeApp"],
        ),
        "c-gtk" => (
            "linux", "C17 and GTK3 (not GTK4), using .c/.h sources and one main(int argc, char **argv). Separate application logic from UI. Check --crexe-self-test before gtk_init or gtk_application_new so tests work without a display. The engine supplies the Makefile and pkg-config build commands.",
            vec!["make", "-f", MAKEFILE],
        ),
        _ => bail!("Unknown native profile"),
    };
    if os != std::env::consts::OS {
        bail!(
            "Profile {profile} requires a {os} host; this engine runs on {}",
            std::env::consts::OS
        );
    }
    let system = format!("Develop a complete small native application with {language} Display name: {name}. Return complete sources in files[]. No shell scripts, project/build files or external dependencies beyond the named SDK and standard library. The --crexe-self-test branch MUST exercise meaningful deterministic application logic, exit 0 on success and nonzero on failure, and avoid external services/devices. Do not open a window in that branch. No network on startup or global installations. State unsupported requirements explicitly in README; do not silently substitute unrelated behavior. Build diagnostics can be returned for a complete corrected snapshot.");
    let mut spec = serde_yaml::to_value(json!({
        "version": "1.0", "meta": {"name": name}, "engine_profile": profile,
        "selectors": {"target": {"fallback": "host"}},
        "prompt": {"system": system, "user_by_target": {"host": prompt}},
        "policies": {"timeoutSeconds": {"build": 180, "test": 30}},
        "targets": {"host": {
            "build": {"steps": [{"cmd": build}]},
            "test": {"steps": [{"cmd": ["CrexeApp{{EXE_EXT}}", "--crexe-self-test"]}]},
            "run": {"cmd": ["CrexeApp{{EXE_EXT}}"]}
        }}
    }))?;
    let publish = if profile == "c-gtk" {
        vec!["make", "-f", MAKEFILE, "publish"]
    } else {
        build
            .iter()
            .map(|arg| match *arg {
                "CrexeApp.exe" => "CrexeApp-publish.exe",
                "CrexeApp" => "CrexeApp-publish",
                other => other,
            })
            .collect()
    };
    spec["targets"]["host"]["publish"] =
        serde_yaml::to_value(json!({"steps": [{"cmd": publish}]}))?;
    Ok(spec)
}

pub(super) fn preflight(profile: &str, workspace: &Path, settings: &Settings) -> Result<()> {
    let compiler = match profile {
        "cpp-win32" => "g++",
        "objc-cocoa" => "clang++",
        "c-gtk" => "gcc",
        _ => bail!("Unknown native profile"),
    };
    settings.execution.authorize(compiler)?;
    let target = crate::executor::output(
        &[compiler.into(), "-dumpmachine".into()],
        workspace,
        &settings.secret_names,
    )?;
    let arch = std::env::consts::ARCH;
    let matching_arch = target.trim().starts_with(arch)
        || (arch == "aarch64" && target.starts_with("arm64"))
        || (arch == "x86"
            && ["i386", "i686"]
                .iter()
                .any(|prefix| target.starts_with(prefix)));
    let matching_os = match profile {
        "cpp-win32" => target.contains("mingw"),
        "objc-cocoa" => target.contains("apple"),
        "c-gtk" => target.contains("linux"),
        _ => false,
    };
    if !matching_arch || !matching_os {
        bail!("Compiler target '{}' is incompatible with profile {profile} and engine architecture {arch}", target.trim());
    }
    if profile == "c-gtk" {
        for tool in ["make", "pkg-config"] {
            settings.execution.authorize(tool)?;
        }
        crate::executor::output(&["pkg-config".into(), "--exists".into(), "gtk+-3.0".into()], workspace, &settings.secret_names)
            .context("GTK3 development headers/libraries are required; configure pkg-config before generating")?;
    }
    Ok(())
}

pub(super) fn materialize(
    spec: &Value,
    workspace: &Path,
    files: &[GeneratedFile],
    ctx: &BTreeMap<String, String>,
) -> Result<Value> {
    let mut result = spec.clone();
    let Some(profile) = get_path(spec, &["engine_profile"]).and_then(Value::as_str) else {
        return Ok(result);
    };
    if !matches!(profile, "cpp-win32" | "objc-cocoa" | "c-gtk") {
        return Ok(result);
    }
    let extension = match profile {
        "cpp-win32" => "cpp",
        "objc-cocoa" => "mm",
        _ => "c",
    };
    // Use the actual written paths and never a wildcard or model-supplied compiler flags.
    let sources: Vec<_> = files
        .iter()
        .map(|file| crate::render_template(&file.path, spec, ctx))
        .filter(|path| {
            Path::new(path)
                .extension()
                .is_some_and(|ext| ext == extension)
        })
        .map(|path| {
            let relative = crate::paths::relative_file(&path)?;
            if !workspace.join(&relative).is_file() {
                bail!("Native source path was not materialized: {path}");
            }
            Ok(format!(
                "./{}",
                relative.to_string_lossy().replace('\\', "/")
            ))
        })
        .collect::<Result<_>>()?;
    if sources.is_empty() {
        bail!("Profile {profile} requires at least one .{extension} source");
    }
    if profile == "c-gtk" {
        if files
            .iter()
            .any(|file| file.path.eq_ignore_ascii_case(MAKEFILE))
        {
            bail!("{MAKEFILE} is owned by the engine");
        }
        let quoted = sources
            .iter()
            .map(|source| format!("'{}'", source.replace('\'', "'\"'\"'")).replace('$', "$$"))
            .collect::<Vec<_>>()
            .join(" ");
        // GNU Make expands pkg-config's escaped flags before the shell parses the command.
        // The source list is fixed, individually quoted, and contains no model-supplied flags.
        let makefile = format!(".PHONY: all publish\nall:\n\tgcc -std=c17 -O2 {quoted} $(shell pkg-config --cflags --libs gtk+-3.0) -lm -o CrexeApp\npublish:\n\tgcc -std=c17 -O2 {quoted} $(shell pkg-config --cflags --libs gtk+-3.0) -lm -o CrexeApp-publish\n");
        fs::write(workspace.join(MAKEFILE), makefile)?;
    } else {
        for operation in ["build", "publish"] {
            let arguments = result["targets"]["host"][operation]["steps"][0]["cmd"]
                .as_sequence_mut()
                .context("Invalid native build plan")?;
            let mut expanded = Vec::new();
            for arg in arguments.iter() {
                if arg.as_str() == Some(SOURCES) {
                    expanded.extend(sources.iter().cloned().map(Value::String));
                } else {
                    expanded.push(arg.clone());
                }
            }
            *arguments = expanded;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_profiles_for_other_hosts() {
        for (profile, os) in [
            ("cpp-win32", "windows"),
            ("objc-cocoa", "macos"),
            ("c-gtk", "linux"),
        ] {
            assert_eq!(
                resolve("app", "intent", profile).is_ok(),
                os == std::env::consts::OS
            );
        }
    }
}
