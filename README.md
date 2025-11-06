# Imp - Generation-Based Persistence Manager

A simple, declarative persistence manager inspired by NixOS impermanence, but without the complexity. Imp manages your directories and files using a generation-based approach, allowing you to version and rollback your configurations.

## Features

- **Declarative Configuration**: Define all your persisted directories and files in a simple TOML file
- **Bind Mounts for Directories**: Uses bind mounts instead of symlinks for directories, ensuring compatibility with databases (SQLite, PostgreSQL, etc.) and applications that require real directories
- **Symlinks for Files**: Individual files are still managed with symlinks
- **Generation Tracking**: Every configuration application creates a new generation
- **Rollback Support**: Switch between different generations easily
- **Backup Management**: Optionally backup existing files before creating mounts/symlinks
- **Verification**: Check that your mounts and symlinks are correctly configured
- **🤖 Automated CI Fixes**: Uses [mini-agent-action](https://github.com/r33drichards/mini-agent-action) to automatically detect and fix CI failures (cargo fmt, clippy, test)

## Important: Root Privileges Required

**Bind mounts require root/sudo privileges.** You must run `imp` commands with `sudo`:

```bash
sudo imp apply
sudo imp switch 2
sudo imp verify
```

This is necessary because creating and removing mount points requires elevated permissions.

## Installation

```bash
cargo install --path .
```

## Quick Start

1. Create a configuration file `imp.toml`:

```toml
# Optional: Specify where to store generation metadata
# Defaults to ~/.local/share/imp
state_dir = "/home/user/.local/share/imp"

# Define persistence directories using NixOS impermanence-style syntax
[persistence."/mnt/persist/system"]
directories = [
    "/var/log",
    "/var/lib/nixos",
    # You can also specify permissions, user, and group
    { directory = "/var/lib/colord", user = "colord", group = "colord", mode = "u=rwx,g=rx,o=" },
]
files = [
    "/etc/machine-id",
    # Files can specify parent directory permissions
    { file = "/etc/nix/id_rsa", parentDirectory = { mode = "u=rwx,g=,o=" } },
]

# You can have multiple persistence directories
[persistence."/mnt/persist/home"]
directories = [
    "/home/user/.config/nvim",
    "/home/user/.mozilla",
]
files = [
    "/home/user/.zsh_history",
    "/home/user/.gitconfig",
]
```

2. Apply the configuration (requires sudo):

```bash
sudo imp apply
```

3. List all generations:

```bash
imp list
```

## Commands

### Apply a Configuration

Create a new generation and apply the bind mounts and symlinks (requires sudo):

```bash
sudo imp apply                       # Use default config: imp.toml
sudo imp apply --config custom.toml  # Use custom config file
sudo imp apply --skip-validation     # Skip source path validation
```

### List Generations

Show all generations:

```bash
imp list
```

### Show Generation Details

Display detailed information about a specific generation:

```bash
imp show 3
```

### Switch Generations

Roll back to a previous generation (requires sudo):

```bash
sudo imp switch 2
```

### Delete a Generation

Remove a generation (cannot delete active generation):

```bash
imp delete 2
imp delete 2 --force  # Skip confirmation
```

### Verify Current Generation

Check that all bind mounts and symlinks in the current generation are correctly configured:

```bash
imp verify
```

Note: This command can be run without sudo for read-only verification.

### Show Current Generation

Display information about the currently active generation:

```bash
imp current
```

## Configuration Format

The configuration file uses a NixOS impermanence-style syntax in TOML format:

```toml
# Optional: Override the default state directory
state_dir = "/path/to/state"

# Define persistence directories
# The key is the persistence directory path (where files are actually stored)
# The value contains lists of directories and files to symlink
[persistence."/mnt/persist/system"]
hideMounts = true                    # Optional: Whether to hide mounts (default: false)

# Directories to symlink - can be simple strings or detailed objects
directories = [
    "/var/log",                      # Simple: just the target path
    "/var/lib/nixos",

    # Detailed: with permissions and ownership
    { directory = "/var/lib/colord", user = "colord", group = "colord", mode = "u=rwx,g=rx,o=" },
]

# Files to symlink - can be simple strings or detailed objects
files = [
    "/etc/machine-id",               # Simple: just the target path

    # Detailed: with parent directory permissions
    { file = "/etc/nix/id_rsa", parentDirectory = { mode = "u=rwx,g=,o=" } },
]
```

### How It Works

For each persistence directory (e.g., `/nix/persist/system`):
- **Directories**: The paths you specify are the target locations where **bind mounts** will be created
  - Bind mounts make the directory appear as if it's natively at that location
  - This ensures compatibility with databases (SQLite, PostgreSQL, etc.) and applications that require real directories
  - Example: `"/var/log"` becomes a bind mount to `/nix/persist/system/var/log`
- **Files**: The paths you specify are the target locations where **symlinks** will be created
  - Individual files use symlinks since bind mounts only work for directories
  - Example: `"/etc/machine-id"` becomes a symlink to `/nix/persist/system/etc/machine-id`
- **Source paths**: Automatically computed by combining the persistence directory with the target path

### Field Descriptions

- **persistence**: A map of persistence directory paths to their configurations
- **hideMounts**: Optional boolean flag (currently informational only)
- **directories**: Array of directory entries (simple strings or detailed objects)
  - **directory**: The target path where the symlink will be created
  - **user**: Optional user ownership (for future use)
  - **group**: Optional group ownership (for future use)
  - **mode**: Optional permissions mode (for future use)
- **files**: Array of file entries (simple strings or detailed objects)
  - **file**: The target path where the symlink will be created
  - **parentDirectory.mode**: Optional permissions mode for parent directory (for future use)

## How It Works

1. **Generation Creation**: When you run `sudo imp apply`, it:
   - Validates your configuration
   - Removes bind mounts and symlinks from the previous active generation
   - Creates new bind mounts for directories and symlinks for files according to your configuration
   - Saves the generation metadata to `~/.local/share/imp/generations.json`

2. **Generation Switching**: When you switch to a different generation:
   - Unmounts all bind mounts and removes all symlinks from the current generation
   - Recreates all bind mounts and symlinks from the target generation
   - Updates the active generation marker

3. **Backup System**: If `backup = true`:
   - Existing files/directories are renamed with a timestamp (e.g., `file.backup.20250106_123456`)
   - Backups are stored alongside the original location
   - When removing a generation's mounts/symlinks, backups can be restored

4. **Bind Mounts vs Symlinks**:
   - **Directories** use bind mounts to ensure compatibility with applications expecting real directories
   - **Files** use symlinks since bind mounts only work for directories
   - This hybrid approach solves the "readonly database" error common with SQLite on symlinked directories

## Examples

### Basic Dotfiles Management

```toml
[persistence."/home/user/dotfiles"]
files = [
    "/home/user/.vimrc",
    "/home/user/.bashrc",
    "/home/user/.gitconfig",
]
```

This creates symlinks for each file:
- `/home/user/.vimrc` → `/home/user/dotfiles/home/user/.vimrc` (symlink)
- `/home/user/.bashrc` → `/home/user/dotfiles/home/user/.bashrc` (symlink)
- `/home/user/.gitconfig` → `/home/user/dotfiles/home/user/.gitconfig` (symlink)

### XDG Config Directory Management

```toml
[persistence."/home/user/dotfiles"]
directories = [
    "/home/user/.config/nvim",
    "/home/user/.config/alacritty",
    "/home/user/.config/kitty",
]
```

This creates bind mounts for each directory:
- `/home/user/.config/nvim` ⟷ `/home/user/dotfiles/home/user/.config/nvim` (bind mount)
- `/home/user/.config/alacritty` ⟷ `/home/user/dotfiles/home/user/.config/alacritty` (bind mount)
- `/home/user/.config/kitty` ⟷ `/home/user/dotfiles/home/user/.config/kitty` (bind mount)

### Application Data with Databases

For applications that use databases (e.g., SQLite), bind mounts are essential:

```toml
[persistence."/mnt/persist/app-data"]
directories = [
    "/var/lib/app-with-database",  # Bind mount prevents "readonly database" errors
    "/home/user/.local/share/app",
]
```

### Impermanence-Style Setup (NixOS)

If you're using tmpfs for your root/home directory:

```toml
state_dir = "/persist/imp"

# System-wide persistence
[persistence."/mnt/persist/system"]
hideMounts = true
directories = [
    "/var/log",
    "/var/lib/bluetooth",
    "/var/lib/nixos",
    "/var/lib/systemd/coredump",
    "/etc/NetworkManager/system-connections",
]
files = [
    "/etc/machine-id",
]

# Home directory persistence
[persistence."/mnt/persist/home"]
directories = [
    "/home/user/.ssh",
    "/home/user/.gnupg",
    "/home/user/.config",
    "/home/user/.mozilla",
]
files = [
    "/home/user/.zsh_history",
    "/home/user/.bash_history",
]
```

### Advanced: Custom Permissions

```toml
[persistence."/mnt/persist/system"]
directories = [
    # Simple entries
    "/var/log",
    "/var/lib/nixos",

    # With custom ownership and permissions
    { directory = "/var/lib/colord", user = "colord", group = "colord", mode = "u=rwx,g=rx,o=" },
    { directory = "/var/lib/postgresql", user = "postgres", group = "postgres", mode = "u=rwx,g=,o=" },
]
files = [
    "/etc/machine-id",

    # With parent directory permissions
    { file = "/etc/ssh/ssh_host_rsa_key", parentDirectory = { mode = "u=rwx,g=,o=" } },
]
```

## Comparison with NixOS Impermanence

| Feature | Imp | NixOS Impermanence |
|---------|-----|-------------------|
| Language | Rust | Nix |
| Configuration | TOML | Nix expressions |
| OS Support | Linux/Unix | NixOS |
| Learning Curve | Low | High |
| Integration | Standalone | Requires NixOS |
| Generation Management | Built-in | Via NixOS |
| Directories | Bind mounts | Bind mounts |
| Files | Symlinks | Symlinks |
| Privileges Required | Root/sudo | Root (via systemd) |

## Automated CI Fixes

This repository uses [mini-agent-action](https://github.com/r33drichards/mini-agent-action) to automatically detect and fix CI failures. When CI checks fail on **main branch or pull requests**, the system:

1. **Detects** which checks failed (cargo fmt, clippy, or test)
2. **Launches** an AI agent to analyze and fix the issues
3. **Creates** a draft PR with the automated fixes

### Setup

To enable auto-fix in your fork:

1. Add `ANTHROPIC_API_KEY` secret to your repository
   - Get an API key at: https://console.anthropic.com/
   - Go to: Settings → Secrets and variables → Actions → New repository secret

2. The workflow (`.github/workflows/auto-fix.yml`) will automatically:
   - Monitor CI runs for failures
   - Run mini-agent-action to fix issues
   - Create draft PRs with fixes for review

For detailed documentation, see [.github/AUTO_FIX.md](.github/AUTO_FIX.md).

## Tips

1. **Use sudo**: Remember to run `imp` with `sudo` for commands that create/remove mounts (apply, switch)
2. **Start Small**: Begin with a few directories/files and test thoroughly
3. **Use Backups**: Enable `backup = true` when testing to avoid data loss
4. **Verify Often**: Run `imp verify` to ensure bind mounts and symlinks are correct
5. **Keep Old Generations**: Don't delete generations immediately; keep them for rollback
6. **Database Compatibility**: Bind mounts solve the "readonly database" error - no need for special SQLite configuration

## Troubleshooting

### "Operation not permitted" or "Permission denied"

You need to run `imp` with sudo for commands that create or remove bind mounts:

```bash
sudo imp apply
sudo imp switch 2
```

Commands like `imp list`, `imp show`, and `imp verify` can be run without sudo.

### "attempt to write a readonly database" (SQLite error)

This error occurs when using symlinked directories with SQLite databases. **This is exactly what `imp` now solves** by using bind mounts for directories instead of symlinks.

If you're still getting this error:
1. Make sure you're using the latest version of `imp`
2. Verify the directory is a bind mount (not a symlink): `mount | grep your-directory`
3. Run `imp verify` to check your configuration

### "Source path does not exist"

Make sure the source path exists before running `sudo imp apply`. You can skip validation with `--skip-validation`, but this is not recommended.

### "Failed to create mount" or "Failed to create symlink"

- Check that you're running with `sudo` for mount operations
- Check that you have write permissions to the target directory
- Ensure parent directories exist or set `create_parents = true`
- Verify no file exists at the target or enable `backup = true`

### "Cannot delete active generation"

You cannot delete the currently active generation. Switch to a different generation first.

## License

MIT

## Contributing

Contributions welcome! Please open an issue or pull request.
