use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    /// Persistence configurations, keyed by persistence directory path
    #[serde(default)]
    pub persistence: HashMap<String, PersistenceConfig>,

    /// Optional: Where to store generation metadata (defaults to ~/.local/share/imp)
    #[serde(default = "default_state_dir")]
    pub state_dir: PathBuf,
}

fn default_state_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("imp")
}

/// Configuration for a single persistence directory
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PersistenceConfig {
    /// Whether to hide mounts (optional, default false)
    #[serde(default)]
    pub hide_mounts: bool,

    /// Directories to persist
    #[serde(default)]
    pub directories: Vec<DirectoryEntry>,

    /// Files to persist
    #[serde(default)]
    pub files: Vec<FileEntry>,
}

/// Represents a directory entry - can be a simple string or a detailed object
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum DirectoryEntry {
    /// Simple string path
    Simple(String),
    /// Detailed configuration
    Detailed {
        directory: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        group: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mode: Option<String>,
    },
}

impl DirectoryEntry {
    /// Get the directory path
    pub fn path(&self) -> &str {
        match self {
            DirectoryEntry::Simple(path) => path,
            DirectoryEntry::Detailed { directory, .. } => directory,
        }
    }

    /// Get the user (if specified)
    pub fn user(&self) -> Option<&str> {
        match self {
            DirectoryEntry::Simple(_) => None,
            DirectoryEntry::Detailed { user, .. } => user.as_deref(),
        }
    }

    /// Get the group (if specified)
    pub fn group(&self) -> Option<&str> {
        match self {
            DirectoryEntry::Simple(_) => None,
            DirectoryEntry::Detailed { group, .. } => group.as_deref(),
        }
    }

    /// Get the mode (if specified)
    pub fn mode(&self) -> Option<&str> {
        match self {
            DirectoryEntry::Simple(_) => None,
            DirectoryEntry::Detailed { mode, .. } => mode.as_deref(),
        }
    }
}

/// Represents a file entry - can be a simple string or a detailed object
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum FileEntry {
    /// Simple string path
    Simple(String),
    /// Detailed configuration
    Detailed {
        file: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_directory: Option<ParentDirectoryConfig>,
    },
}

impl FileEntry {
    /// Get the file path
    pub fn path(&self) -> &str {
        match self {
            FileEntry::Simple(path) => path,
            FileEntry::Detailed { file, .. } => file,
        }
    }

    /// Get the parent directory config (if specified)
    pub fn parent_directory(&self) -> Option<&ParentDirectoryConfig> {
        match self {
            FileEntry::Simple(_) => None,
            FileEntry::Detailed {
                parent_directory, ..
            } => parent_directory.as_ref(),
        }
    }
}

/// Configuration for parent directory of a file
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ParentDirectoryConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

/// Internal representation of a symlink for compatibility with existing code
#[derive(Debug, Clone)]
pub struct Symlink {
    /// The source path (what to link to)
    pub source: PathBuf,

    /// The target path (where the symlink will be created)
    pub target: PathBuf,

    /// If true, create parent directories as needed
    pub create_parents: bool,

    /// If true, backup existing file/directory at target
    pub backup: bool,

    /// Optional: User ownership (reserved for future use)
    #[allow(dead_code)]
    pub user: Option<String>,

    /// Optional: Group ownership (reserved for future use)
    #[allow(dead_code)]
    pub group: Option<String>,

    /// Optional: Permissions mode (reserved for future use)
    #[allow(dead_code)]
    pub mode: Option<String>,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Convert the persistence config to a flat list of symlinks
    pub fn to_symlinks(&self) -> Vec<Symlink> {
        let mut symlinks = Vec::new();

        for (persist_dir, persist_config) in &self.persistence {
            // Process directories
            for dir_entry in &persist_config.directories {
                let target_path = PathBuf::from(dir_entry.path());
                let source_path = PathBuf::from(persist_dir)
                    .join(target_path.strip_prefix("/").unwrap_or(&target_path));

                symlinks.push(Symlink {
                    source: source_path,
                    target: target_path,
                    create_parents: true,
                    backup: false,
                    user: dir_entry.user().map(String::from),
                    group: dir_entry.group().map(String::from),
                    mode: dir_entry.mode().map(String::from),
                });
            }

            // Process files
            for file_entry in &persist_config.files {
                let target_path = PathBuf::from(file_entry.path());
                let source_path = PathBuf::from(persist_dir)
                    .join(target_path.strip_prefix("/").unwrap_or(&target_path));

                let create_parents = file_entry.parent_directory().is_some();

                symlinks.push(Symlink {
                    source: source_path,
                    target: target_path,
                    create_parents,
                    backup: false,
                    user: None,
                    group: None,
                    mode: file_entry.parent_directory().and_then(|p| p.mode.clone()),
                });
            }
        }

        symlinks
    }

    /// Validate the configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        let symlinks = self.to_symlinks();
        for symlink in &symlinks {
            if !symlink.source.exists() {
                anyhow::bail!("Source path does not exist: {}", symlink.source.display());
            }
        }
        Ok(())
    }
}
