//! Local command policy is an execution gate, not process isolation.
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};

pub(crate) fn requires_shell(command: &str) -> bool {
    let path = Path::new(command);
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(command)
        .to_ascii_lowercase();
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    ["cmd", "powershell", "pwsh", "sh", "bash", "zsh", "fish"].contains(&name.as_str())
        || ["bat", "cmd", "ps1", "sh"].contains(&extension.as_str())
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Policy {
    pub allowed_tools: Vec<String>,
    pub allow_shells: bool,
    pub build_timeout_seconds: u64,
    pub test_timeout_seconds: u64,
    pub generation_timeout_seconds: u64,
    pub max_provider_requests: u8,
    pub max_output_tokens_total: u32,
    pub max_prompt_bytes: usize,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            allowed_tools: [
                "dotnet",
                "rustc",
                "g++",
                "gcc",
                "clang++",
                "clang",
                "windres",
                "make",
                "pkg-config",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            allow_shells: false,
            build_timeout_seconds: 300,
            test_timeout_seconds: 60,
            generation_timeout_seconds: 1800,
            max_provider_requests: 3,
            max_output_tokens_total: 18000,
            max_prompt_bytes: 131072,
        }
    }
}

impl Policy {
    pub fn validate(&self) -> Result<()> {
        if self.allowed_tools.len() > 64
            || self.allowed_tools.iter().any(|tool| tool.trim().is_empty())
        {
            bail!("Invalid local allowed_tools list");
        }
        if !(1..=3600).contains(&self.build_timeout_seconds)
            || !(1..=600).contains(&self.test_timeout_seconds)
            || !(1..=7200).contains(&self.generation_timeout_seconds)
            || !(1..=3).contains(&self.max_provider_requests)
            || !(128..=196608).contains(&self.max_output_tokens_total)
            || !(1024..=1048576).contains(&self.max_prompt_bytes)
        {
            bail!("Invalid local execution budget");
        }
        Ok(())
    }

    pub fn authorize(&self, command: &str) -> Result<()> {
        let requested = super::executor::resolve_tool(command)?;
        if (requires_shell(command) || requires_shell(&requested.to_string_lossy()))
            && !self.allow_shells
        {
            bail!("Shell commands require execution.allow_shells = true in trusted local configuration; the recipe cannot authorize them");
        }
        let requested = requested
            .canonicalize()
            .context("Cannot resolve requested tool")?;
        for tool in &self.allowed_tools {
            if super::executor::resolve_tool(tool)
                .ok()
                .and_then(|p| p.canonicalize().ok())
                .is_some_and(|p| p == requested)
            {
                return Ok(());
            }
        }
        bail!("Tool '{command}' is not approved by local execution.allowed_tools; recipe allowlists cannot expand local permissions")
    }

    pub fn timeout(&self, phase: &str, recipe: Option<u64>) -> Option<Duration> {
        match phase {
            "run" => recipe.map(Duration::from_secs),
            "test" => Some(Duration::from_secs(
                recipe
                    .unwrap_or(self.test_timeout_seconds)
                    .min(self.test_timeout_seconds),
            )),
            _ => Some(Duration::from_secs(
                recipe
                    .unwrap_or(self.build_timeout_seconds)
                    .min(self.build_timeout_seconds),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_policy_rejects_unapproved_absolute_executables_and_clamps_recipe_limits() {
        let policy = Policy {
            allowed_tools: Vec::new(),
            ..Policy::default()
        };
        assert!(policy
            .authorize(std::env::current_exe().unwrap().to_str().unwrap())
            .is_err());
        assert_eq!(
            policy.timeout("build", Some(9999)),
            Some(Duration::from_secs(300))
        );
        assert_eq!(
            policy.timeout("test", Some(1)),
            Some(Duration::from_secs(1))
        );
        assert_eq!(policy.timeout("run", None), None);
    }
}
