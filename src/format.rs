//! Deterministic syntax recognition; plaintext is the user's entire prompt.
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_yaml::Value;
use std::path::Path;

pub(crate) enum Document {
    Yaml(Value),
    Intent {
        name: String,
        prompt: String,
        profile: Option<String>,
        format: &'static str,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrontMatter {
    crexe: u32,
    name: Option<String>,
    profile: Option<String>,
}

pub(crate) fn parse(raw: &str, path: &Path) -> Result<Document> {
    if raw.len() > 1024 * 1024 {
        bail!("Recipe exceeds 1 MiB");
    }
    let text = raw.trim_start_matches('\u{feff}').trim();
    if text.is_empty() {
        bail!("Empty CREXE document");
    }
    let name = path
        .file_stem()
        .and_then(|v| v.to_str())
        .context("Recipe filename must be UTF-8")?
        .to_owned();
    if text.lines().next().map(str::trim) == Some("---") {
        let mut header = Vec::new();
        let mut body = Vec::new();
        let mut closed = false;
        for line in text.lines().skip(1) {
            if !closed && line.trim() == "---" {
                closed = true;
            } else if closed {
                body.push(line);
            } else {
                header.push(line);
            }
        }
        if !closed {
            bail!("Unclosed Markdown front matter");
        }
        let metadata: FrontMatter = serde_yaml::from_str(&header.join("\n"))
            .context("Invalid Markdown front matter; expected crexe: 1")?;
        if metadata.crexe != 1 {
            bail!("Unsupported Markdown version");
        }
        let prompt = body.join("\n");
        if prompt.trim().is_empty() {
            bail!("Markdown body must contain the application's intent");
        }
        return Ok(Document::Intent {
            name: metadata.name.unwrap_or(name),
            prompt,
            profile: metadata.profile,
            format: "markdown",
        });
    }
    // Only reserved schema keys identify YAML; ordinary prompts containing ':' stay plaintext.
    let yaml_marker = text.starts_with('{')
        || text.lines().any(|line| {
            !line.starts_with(char::is_whitespace)
                && ["version:", "targets:", "selectors:", "prompt_core:"]
                    .iter()
                    .any(|key| line.starts_with(key))
        });
    if yaml_marker {
        let spec: Value = serde_yaml::from_str(text).context("Invalid YAML recipe")?;
        super::validate_spec(&spec)?;
        return Ok(Document::Yaml(spec));
    }
    Ok(Document::Intent {
        name,
        prompt: text.to_owned(),
        profile: None,
        format: "plaintext",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn free_intent_needs_no_sections_or_metadata() {
        for text in [
            "gerar uma calculadora simples",
            "Objetivo: calcular regra de 3",
            "# Minha calculadora\nCom botões",
            "\u{feff}gerar uma calculadora\r\n",
        ] {
            let Document::Intent {
                name,
                prompt,
                format,
                ..
            } = parse(text, Path::new("ação.crexe")).unwrap()
            else {
                panic!()
            };
            assert_eq!(name, "ação");
            assert_eq!(format, "plaintext");
            assert!(!prompt.is_empty());
        }
    }
    #[test]
    fn structured_documents_do_not_silently_fall_back() {
        for text in [
            "version: 2",
            "version: [",
            "---\ncrexe: 1",
            "---\ncrexe: 2\n---\nhello",
            "---\ncrexe: 1\nunknown: true\n---\nhello",
            " ",
        ] {
            assert!(parse(text, Path::new("a.crexe")).is_err());
        }
        assert!(matches!(
            parse(
                "---\r\ncrexe: 1\r\n---\r\n# App\r\nhi",
                Path::new("a.crexe")
            )
            .unwrap(),
            Document::Intent {
                format: "markdown",
                ..
            }
        ));
    }
}
