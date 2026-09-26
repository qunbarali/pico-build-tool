use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_board")]
    pub pico_board: String,
}

fn default_version() -> String { "0.1.0".into() }
fn default_board() -> String { "pico".into() }

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            name: "pico-project".into(),
            description: None,
            pico_board: default_board(),
        }
    }
}

pub fn load(project_dir: &Path) -> Result<ProjectConfig> {
    let path = project_dir.join("pico.toml");
    if !path.exists() {
        return Ok(ProjectConfig {
            name: project_dir.file_name().and_then(|s| s.to_str()).unwrap_or("pico-project").to_string(),
            ..Default::default()
        });
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let mut config: ProjectConfig = toml::from_str(&text)
        .with_context(|| format!("invalid {}", path.display()))?;
    if config.name.trim().is_empty() {
        config.name = project_dir.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("pico-project")
            .to_string();
    }
    validate_board(&config.pico_board)?;
    Ok(config)
}

pub fn validate_board(board: &str) -> Result<()> {
    let valid = board.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
    anyhow::ensure!(valid && !board.is_empty(), "invalid pico_board '{}'", board);
    Ok(())
}

pub fn project_path(input: &str) -> Result<PathBuf> {
    let path = fs::canonicalize(input)
        .with_context(|| format!("project path does not exist: {}", input))?;
    anyhow::ensure!(path.is_dir(), "project path is not a directory: {}", path.display());
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_validation_rejects_shell_metacharacters() {
        assert!(validate_board("pico").is_ok());
        assert!(validate_board("pico2").is_ok());
        assert!(validate_board("pico;rm").is_err());
    }
}
