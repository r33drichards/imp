use anyhow::{Context, Result};
use caps::{CapSet, Capability};
use nix::mount::{mount, umount, MsFlags};
use nix::unistd::{chown, Gid, Uid};
use std::fs;
use std::os::unix::fs as unix_fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::config::Symlink;
use crate::generation::GenerationSymlink;

/// Manages symlink operations
pub struct SymlinkManager;

impl SymlinkManager {
    pub fn new() -> Self {
        Self
    }

    /// Check if the current process has CAP_SYS_ADMIN capability
    fn has_cap_sys_admin() -> bool {
        caps::has_cap(None, CapSet::Effective, Capability::CAP_SYS_ADMIN).unwrap_or(false)
    }

    /// Check if we're likely running in a restricted container environment
    fn is_container_environment() -> bool {
        // Check for common container indicators
        std::path::Path::new("/.dockerenv").exists()
            || std::fs::read_to_string("/proc/1/cgroup")
                .map(|s| s.contains("docker") || s.contains("lxc") || s.contains("containerd"))
                .unwrap_or(false)
    }

    /// Parse a mode string (e.g., "0755") into a numeric mode
    fn parse_mode(mode_str: &str) -> Result<u32> {
        // Remove "0o" or "0" prefix if present
        let mode_str = mode_str.trim_start_matches("0o").trim_start_matches("0");
        u32::from_str_radix(mode_str, 8).context(format!("Invalid mode string: {}", mode_str))
    }

    /// Get UID from username
    fn get_uid(username: &str) -> Result<Uid> {
        use nix::unistd::User;
        User::from_name(username)
            .context(format!("Failed to lookup user: {}", username))?
            .map(|user| user.uid)
            .context(format!("User not found: {}", username))
    }

    /// Get GID from group name
    fn get_gid(groupname: &str) -> Result<Gid> {
        use nix::unistd::Group;
        Group::from_name(groupname)
            .context(format!("Failed to lookup group: {}", groupname))?
            .map(|group| group.gid)
            .context(format!("Group not found: {}", groupname))
    }

    /// Apply ownership and permissions to a path
    fn apply_ownership_and_permissions(
        &self,
        path: &Path,
        user: Option<&str>,
        group: Option<&str>,
        mode: Option<&str>,
    ) -> Result<()> {
        // Apply ownership if specified
        if user.is_some() || group.is_some() {
            let uid = if let Some(u) = user {
                Some(Self::get_uid(u)?)
            } else {
                None
            };

            let gid = if let Some(g) = group {
                Some(Self::get_gid(g)?)
            } else {
                None
            };

            chown(path, uid, gid)
                .context(format!("Failed to change ownership of: {}", path.display()))?;
        }

        // Apply permissions if specified
        if let Some(mode_str) = mode {
            let mode = Self::parse_mode(mode_str)?;
            let permissions = fs::Permissions::from_mode(mode);
            fs::set_permissions(path, permissions)
                .context(format!("Failed to set permissions on: {}", path.display()))?;
        }

        Ok(())
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

    /// Create a single symlink or bind mount
    fn create_symlink(&self, symlink: &Symlink) -> Result<GenerationSymlink> {
        // Handle case where source doesn't exist but target does
        // In this case, create the source directory using target's permissions
        let source = if !symlink.source.exists() && symlink.target.exists() && symlink.is_directory
        {
            println!(
                "  ℹ Source {} doesn't exist but target {} does. Creating source from target.",
                symlink.source.display(),
                symlink.target.display()
            );

            // Get target metadata to copy to source
            let target_metadata = fs::metadata(&symlink.target).context(format!(
                "Failed to get metadata for target: {}",
                symlink.target.display()
            ))?;

            // Create source directory with target's permissions
            if let Some(parent) = symlink.source.parent() {
                fs::create_dir_all(parent).context(format!(
                    "Failed to create parent directories for source: {}",
                    symlink.source.display()
                ))?;
            }

            fs::create_dir_all(&symlink.source).context(format!(
                "Failed to create source directory: {}",
                symlink.source.display()
            ))?;

            // Set permissions to match target
            let target_mode = target_metadata.mode();
            let permissions = fs::Permissions::from_mode(target_mode);
            fs::set_permissions(&symlink.source, permissions).context(format!(
                "Failed to set permissions on source directory: {}",
                symlink.source.display()
            ))?;

            // Set ownership to match target
            let target_uid = Uid::from_raw(target_metadata.uid());
            let target_gid = Gid::from_raw(target_metadata.gid());

            chown(&symlink.source, Some(target_uid), Some(target_gid)).context(format!(
                "Failed to set ownership on source directory: {} (uid={}, gid={})",
                symlink.source.display(),
                target_uid,
                target_gid
            ))?;

            println!(
                "  ✓ Created source directory: {} (from target: {})",
                symlink.source.display(),
                symlink.target.display()
            );

            fs::canonicalize(&symlink.source).context(format!(
                "Failed to resolve source path: {}",
                symlink.source.display()
            ))?
        } else {
            fs::canonicalize(&symlink.source).context(format!(
                "Failed to resolve source path: {}",
                symlink.source.display()
            ))?
        };

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
                    // For directories, check if it's a mount point and unmount first
                    if self.is_mount_point(target)? {
                        umount(target).context(format!(
                            "Failed to unmount existing mount point: {}",
                            target.display()
                        ))?;
                    }
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

        // For directories, use bind mount; for files, use symlink
        if symlink.is_directory {
            // Get source metadata first to copy permissions and ownership
            let source_metadata = fs::metadata(&source).context(format!(
                "Failed to get metadata for source: {}",
                source.display()
            ))?;

            // Create the target directory if it doesn't exist
            if !target.exists() {
                fs::create_dir_all(target).context(format!(
                    "Failed to create target directory: {}",
                    target.display()
                ))?;

                // Set permissions on the newly created directory to match source
                let source_mode = source_metadata.mode();
                let permissions = fs::Permissions::from_mode(source_mode);
                fs::set_permissions(target, permissions).context(format!(
                    "Failed to set permissions on target directory: {}",
                    target.display()
                ))?;

                // Set ownership to match source
                let source_uid = Uid::from_raw(source_metadata.uid());
                let source_gid = Gid::from_raw(source_metadata.gid());

                chown(target, Some(source_uid), Some(source_gid)).context(format!(
                    "Failed to set ownership on target directory: {} (uid={}, gid={}). \
                     This usually means insufficient privileges. Try running as root or with CAP_CHOWN capability.",
                    target.display(),
                    source_uid,
                    source_gid
                ))?;
            }

            // Apply any explicitly specified ownership and permissions (overrides source defaults)
            let target_user = symlink.user.as_deref();
            let target_group = symlink.group.as_deref();
            let target_mode = symlink.mode.as_deref();

            if target_user.is_some() || target_group.is_some() || target_mode.is_some() {
                self.apply_ownership_and_permissions(
                    target,
                    target_user,
                    target_group,
                    target_mode,
                )
                .context(format!(
                    "Failed to apply explicit ownership/permissions on: {}",
                    target.display()
                ))?;
            }

            // Check for CAP_SYS_ADMIN before attempting bind mount
            if !Self::has_cap_sys_admin() {
                let mut error_msg = format!(
                    "Cannot create bind mount from {} to {}: Missing CAP_SYS_ADMIN capability.\n\n",
                    source.display(),
                    target.display()
                );

                if Self::is_container_environment() {
                    error_msg.push_str(
                        "You appear to be running in a container. To enable bind mounts, you need to:\n\
                         1. Run the container with --privileged flag, OR\n\
                         2. Add --cap-add SYS_ADMIN to your container run command, OR\n\
                         3. Add the capability in your docker-compose.yml:\n\
                            cap_add:\n\
                              - SYS_ADMIN\n\n\
                         For supervisord users: ensure your container is started with appropriate capabilities."
                    );
                } else {
                    error_msg.push_str(
                        "To enable bind mounts, you need to:\n\
                         1. Run as root (with full capabilities), OR\n\
                         2. Grant CAP_SYS_ADMIN capability to the imp binary:\n\
                            sudo setcap cap_sys_admin+ep /path/to/imp",
                    );
                }

                return Err(anyhow::anyhow!(error_msg));
            }

            // Verify target directory is empty before mounting
            if target.exists() {
                let entries = fs::read_dir(target)
                    .context(format!(
                        "Failed to read target directory: {}",
                        target.display()
                    ))?
                    .count();

                if entries > 0 {
                    return Err(anyhow::anyhow!(
                        "Target directory {} is not empty (contains {} entries). \
                         This should not happen - the directory should have been cleaned up. \
                         Please check if the directory is in use or has special permissions.",
                        target.display(),
                        entries
                    ));
                }
            }

            // Create bind mount
            mount(
                Some(&source),
                target,
                None::<&str>,
                MsFlags::MS_BIND,
                None::<&str>,
            )
            .context(format!(
                "Failed to create bind mount from {} to {}. \
                 This may indicate SELinux/AppArmor restrictions, or that one of the paths is inaccessible. \
                 Source exists: {}, Target exists: {}",
                source.display(),
                target.display(),
                source.exists(),
                target.exists()
            ))?;

            println!(
                "  ✓ Created bind mount: {} -> {}",
                target.display(),
                source.display()
            );
        } else {
            // Create the symlink for files
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
        }

        Ok(GenerationSymlink {
            source: source.clone(),
            target: target.clone(),
            backup_path,
        })
    }

    /// Check if a path is a mount point
    fn is_mount_point(&self, path: &Path) -> Result<bool> {
        // Read /proc/mounts to check if the path is a mount point
        let mounts = fs::read_to_string("/proc/mounts").context("Failed to read /proc/mounts")?;
        let canonical_path = match fs::canonicalize(path) {
            Ok(p) => p,
            Err(_) => return Ok(false), // If we can't canonicalize, it's probably not mounted
        };

        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let mount_point = parts[1];
                if Path::new(mount_point) == canonical_path {
                    return Ok(true);
                }
            }
        }
        Ok(false)
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

    /// Remove symlinks and unmount bind mounts from a generation
    pub fn remove(&self, generation_symlinks: &[GenerationSymlink]) -> Result<()> {
        for gen_symlink in generation_symlinks {
            // Check if it's a mount point (directory bind mount) or symlink (file)
            if self.is_mount_point(&gen_symlink.target)? {
                // Unmount the bind mount
                umount(&gen_symlink.target).context(format!(
                    "Failed to unmount: {}",
                    gen_symlink.target.display()
                ))?;

                println!("  ✓ Unmounted: {}", gen_symlink.target.display());

                // Optionally remove the now-empty directory
                if gen_symlink.target.is_dir() {
                    fs::remove_dir(&gen_symlink.target).ok(); // Ignore errors here
                }

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
            } else if gen_symlink.target.is_symlink() {
                // Remove symlink (for files)
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

    /// Verify that symlinks and bind mounts are correctly configured
    pub fn verify(&self, generation_symlinks: &[GenerationSymlink]) -> Result<Vec<String>> {
        let mut errors = Vec::new();

        for gen_symlink in generation_symlinks {
            // Check if target is a directory (should be a mount point) or file (should be a symlink)
            if gen_symlink.target.is_dir() {
                // For directories, verify it's a mount point
                if !self.is_mount_point(&gen_symlink.target)? {
                    errors.push(format!(
                        "Directory is not a mount point: {}",
                        gen_symlink.target.display()
                    ));
                    continue;
                }

                // Verify it's mounted from the correct source
                // We check this by reading /proc/mounts
                let mounts = fs::read_to_string("/proc/mounts")?;
                let canonical_target = fs::canonicalize(&gen_symlink.target)?;
                let canonical_source = fs::canonicalize(&gen_symlink.source)?;

                let mut found_correct_mount = false;
                for line in mounts.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let mount_source = parts[0];
                        let mount_point = parts[1];
                        if Path::new(mount_point) == canonical_target
                            && Path::new(mount_source) == canonical_source
                        {
                            found_correct_mount = true;
                            break;
                        }
                    }
                }

                if !found_correct_mount {
                    errors.push(format!(
                        "Directory is mounted but from wrong source: {} (expected source: {})",
                        gen_symlink.target.display(),
                        gen_symlink.source.display()
                    ));
                }
            } else {
                // For files, verify it's a symlink
                if !gen_symlink.target.is_symlink() {
                    errors.push(format!(
                        "File is not a symlink: {}",
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
        }

        Ok(errors)
    }
}
