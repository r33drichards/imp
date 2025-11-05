use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    /// List of symlinks to manage
    pub symlinks: Vec<Symlink>,

    /// Optional: Where to store generation metadata (defaults to ~/.local/share/imp)
    #[serde(default = "default_state_dir")]
    pub state_dir: PathBuf,
}

fn default_state_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("imp")
}

/// Represents a single symlink configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Symlink {
    /// The source path (what to link to)
    pub source: PathBuf,

    /// The target path (where the symlink will be created)
    pub target: PathBuf,

    /// Optional: If true, create parent directories as needed
    #[serde(default)]
    pub create_parents: bool,

    /// Optional: If true, backup existing file/directory at target
    #[serde(default)]
    pub backup: bool,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        for symlink in &self.symlinks {
            if !symlink.source.exists() {
                anyhow::bail!(
                    "Source path does not exist: {}",
                    symlink.source.display()
                );
            }
        }
        Ok(())
    }
}
