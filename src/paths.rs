//! Validation of paths written by CREXE. This is not process isolation.
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn relative_file(value: &str) -> Result<PathBuf> {
    let normalized = value.replace('\\', "/");
    let mut result = PathBuf::new();
    for part in normalized.split('/') {
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with(['.', ' '])
            || part
                .chars()
                .any(|c| c.is_control() || "<>:\"|?*".contains(c))
        {
            bail!("Invalid project path: {value}");
        }
        let stem = part
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.as_bytes()[3].is_ascii_digit())
        {
            bail!("Reserved device name in project path: {value}");
        }
        if result.as_os_str().is_empty()
            && (part.eq_ignore_ascii_case("crexe.manifest.json")
                || part.eq_ignore_ascii_case(".build_ok")
                || part.to_ascii_lowercase().starts_with(".crexe"))
        {
            bail!("Reserved engine path: {value}");
        }
        result.push(part);
    }
    Ok(result)
}

pub(crate) fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0 // FILE_ATTRIBUTE_REPARSE_POINT
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

pub(crate) fn reject_links(path: &Path) -> Result<()> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if is_link(&metadata) => {
                // macOS's system-owned aliases are used by the OS temp directory.
                // Other symlinks, including everything below those roots, are rejected.
                #[cfg(target_os = "macos")]
                if matches!(ancestor.to_str(), Some("/var" | "/tmp"))
                    && fs::read_link(ancestor).ok().is_some_and(|target| {
                        let target = if target.is_absolute() {
                            target
                        } else {
                            ancestor.parent().unwrap().join(target)
                        };
                        target == Path::new("/private").join(ancestor.strip_prefix("/").unwrap())
                    })
                {
                    continue;
                }
                bail!("Links are not accepted here: {}", ancestor.display())
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("Inspecting {}", ancestor.display()))
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_escapes_and_portable_aliases() {
        for path in [
            "",
            "/tmp/out",
            "C:\\out",
            "\\\\host\\share",
            "../out",
            "src/../out",
            "src/./out",
            "src//out",
            "src/a:stream",
            "src/NUL.txt",
            "COM1",
            "LPT9.log",
            "file. ",
            "src/file.",
            ".build_ok",
            "crexe.manifest.json",
            ".crexe-state/x",
        ] {
            assert!(relative_file(path).is_err(), "accepted {path}");
        }
    }

    #[test]
    fn normalizes_separators_without_losing_spaces_and_unicode() {
        assert_eq!(
            relative_file("src\\ação final.cpp").unwrap(),
            Path::new("src").join("ação final.cpp")
        );
    }
}
