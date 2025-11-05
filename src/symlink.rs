use anyhow::{Context, Result};
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

use crate::config::Symlink;
use crate::generation::GenerationSymlink;

/// Manages symlink operations
pub struct SymlinkManager;

impl SymlinkManager {
    pub fn new() -> Self {
        Self
    }

    /// Apply a list of symlinks
    pub fn apply(&self, symlinks: &[Symlink]) -> Result<Vec<GenerationSymlink>> {
        let mut generation_symlinks = Vec::new();

        for symlink in symlinks {
            let gen_symlink = self.create_symlink(symlink)?;
            generation_symlinks.push(gen_symlink);
        }

        Ok(generation_symlinks)
    }

    /// Create a single symlink
    fn create_symlink(&self, symlink: &Symlink) -> Result<GenerationSymlink> {
        let source = fs::canonicalize(&symlink.source).context(format!(
            "Failed to resolve source path: {}",
            symlink.source.display()
        ))?;

        let target = &symlink.target;

        // Create parent directories if needed
        if symlink.create_parents {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).context(format!(
                    "Failed to create parent directories for: {}",
                    target.display()
                ))?;
            }
        }

        // Handle existing target
        let backup_path = if target.exists() || target.is_symlink() {
            if symlink.backup {
                Some(self.backup_target(target)?)
            } else {
                // Remove existing symlink or file
                if target.is_symlink() {
                    fs::remove_file(target).context(format!(
                        "Failed to remove existing symlink: {}",
                        target.display()
                    ))?;
                } else if target.is_dir() {
                    fs::remove_dir_all(target).context(format!(
                        "Failed to remove existing directory: {}",
                        target.display()
                    ))?;
                } else {
                    fs::remove_file(target).context(format!(
                        "Failed to remove existing file: {}",
                        target.display()
                    ))?;
                }
                None
            }
        } else {
            None
        };

        // Create the symlink
        unix_fs::symlink(&source, target).context(format!(
            "Failed to create symlink from {} to {}",
            source.display(),
            target.display()
        ))?;

        println!(
            "  ✓ Created symlink: {} -> {}",
            target.display(),
            source.display()
        );

        Ok(GenerationSymlink {
            source: source.clone(),
            target: target.clone(),
            backup_path,
        })
    }

    /// Backup an existing target
    fn backup_target(&self, target: &Path) -> Result<PathBuf> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = target.with_extension(format!("backup.{}", timestamp));

        if target.is_symlink() {
            // Read the symlink and create a new one
            let link_target = fs::read_link(target)?;
            fs::remove_file(target)?;
            unix_fs::symlink(link_target, &backup_path)?;
        } else {
            fs::rename(target, &backup_path)?;
        }

        println!("  ℹ Backed up to: {}", backup_path.display());

        Ok(backup_path)
    }

    /// Remove symlinks from a generation
    pub fn remove(&self, generation_symlinks: &[GenerationSymlink]) -> Result<()> {
        for gen_symlink in generation_symlinks {
            if gen_symlink.target.is_symlink() {
                fs::remove_file(&gen_symlink.target).context(format!(
                    "Failed to remove symlink: {}",
                    gen_symlink.target.display()
                ))?;

                println!("  ✓ Removed symlink: {}", gen_symlink.target.display());

                // Restore backup if it exists
                if let Some(backup_path) = &gen_symlink.backup_path {
                    if backup_path.exists() {
                        fs::rename(backup_path, &gen_symlink.target).context(format!(
                            "Failed to restore backup: {}",
                            backup_path.display()
                        ))?;
                        println!("  ℹ Restored backup: {}", gen_symlink.target.display());
                    }
                }
            }
        }

        Ok(())
    }

    /// Verify that symlinks are correctly configured
    pub fn verify(&self, generation_symlinks: &[GenerationSymlink]) -> Result<Vec<String>> {
        let mut errors = Vec::new();

        for gen_symlink in generation_symlinks {
            if !gen_symlink.target.is_symlink() {
                errors.push(format!(
                    "Target is not a symlink: {}",
                    gen_symlink.target.display()
                ));
                continue;
            }

            match fs::read_link(&gen_symlink.target) {
                Ok(link_target) => {
                    if link_target != gen_symlink.source {
                        errors.push(format!(
                            "Symlink points to wrong target: {} -> {} (expected: {})",
                            gen_symlink.target.display(),
                            link_target.display(),
                            gen_symlink.source.display()
                        ));
                    }
                }
                Err(e) => {
                    errors.push(format!(
                        "Failed to read symlink {}: {}",
                        gen_symlink.target.display(),
                        e
                    ));
                }
            }
        }

        Ok(errors)
    }
}
