# Imp - Generation-Based Symlink Manager

A simple, declarative symlink manager inspired by NixOS impermanence, but without the complexity. Imp manages your symlinks using a generation-based approach, allowing you to version and rollback your symlink configurations.

## Features

- **Declarative Configuration**: Define all your symlinks in a simple TOML file
- **Generation Tracking**: Every configuration application creates a new generation
- **Rollback Support**: Switch between different generations easily
- **Backup Management**: Optionally backup existing files before creating symlinks
- **Verification**: Check that your symlinks are correctly configured

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
[persistence."/nix/persist/system"]
hideMounts = true
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
[persistence."/nix/persist/home"]
directories = [
    "/home/user/.config/nvim",
    "/home/user/.mozilla",
]
files = [
    "/home/user/.zsh_history",
    "/home/user/.gitconfig",
]
```

2. Apply the configuration:

```bash
imp apply
```

3. List all generations:

```bash
imp list
```

## Commands

### Apply a Configuration

Create a new generation and apply the symlinks:

```bash
imp apply                       # Use default config: imp.toml
imp apply --config custom.toml  # Use custom config file
imp apply --skip-validation     # Skip source path validation
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

Roll back to a previous generation:

```bash
imp switch 2
```

### Delete a Generation

Remove a generation (cannot delete active generation):

```bash
imp delete 2
imp delete 2 --force  # Skip confirmation
```

### Verify Current Generation

Check that all symlinks in the current generation are correctly configured:

```bash
imp verify
```

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
[persistence."/nix/persist/system"]
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
- **Directories and files**: The paths you specify are the target locations where symlinks will be created
- **Source paths**: Automatically computed by combining the persistence directory with the target path
  - Example: `"/var/log"` becomes a symlink to `/nix/persist/system/var/log`
  - Example: `"/etc/machine-id"` becomes a symlink to `/nix/persist/system/etc/machine-id`

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

1. **Generation Creation**: When you run `imp apply`, it:
   - Validates your configuration
   - Removes symlinks from the previous active generation
   - Creates new symlinks according to your configuration
   - Saves the generation metadata to `~/.local/share/imp/generations.json`

2. **Generation Switching**: When you switch to a different generation:
   - Removes all symlinks from the current generation
   - Recreates all symlinks from the target generation
   - Updates the active generation marker

3. **Backup System**: If `backup = true`:
   - Existing files are renamed with a timestamp (e.g., `file.backup.20250106_123456`)
   - Backups are stored alongside the original location
   - When removing a generation's symlinks, backups can be restored

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

This creates:
- `/home/user/.vimrc` → `/home/user/dotfiles/home/user/.vimrc`
- `/home/user/.bashrc` → `/home/user/dotfiles/home/user/.bashrc`
- `/home/user/.gitconfig` → `/home/user/dotfiles/home/user/.gitconfig`

### XDG Config Directory Management

```toml
[persistence."/home/user/dotfiles"]
directories = [
    "/home/user/.config/nvim",
    "/home/user/.config/alacritty",
    "/home/user/.config/kitty",
]
```

### Impermanence-Style Setup (NixOS)

If you're using tmpfs for your root/home directory:

```toml
state_dir = "/persist/imp"

# System-wide persistence
[persistence."/nix/persist/system"]
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
[persistence."/nix/persist/home"]
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
[persistence."/nix/persist/system"]
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

## Tips

1. **Start Small**: Begin with a few symlinks and test thoroughly
2. **Use Backups**: Enable `backup = true` when testing to avoid data loss
3. **Verify Often**: Run `imp verify` to ensure symlinks are correct
4. **Keep Old Generations**: Don't delete generations immediately; keep them for rollback

## Troubleshooting

### "Source path does not exist"

Make sure the source path exists before running `imp apply`. You can skip validation with `--skip-validation`, but this is not recommended.

### "Failed to create symlink"

- Check that you have write permissions to the target directory
- Ensure parent directories exist or set `create_parents = true`
- Verify no file exists at the target or enable `backup = true`

### "Cannot delete active generation"

You cannot delete the currently active generation. Switch to a different generation first.

## License

MIT

## Contributing

Contributions welcome! Please open an issue or pull request.
