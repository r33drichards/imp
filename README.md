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

# Define your symlinks
[[symlinks]]
source = "/nix/dotfiles/nvim"
target = "/home/user/.config/nvim"
create_parents = true
backup = true

[[symlinks]]
source = "/nix/dotfiles/zsh/.zshrc"
target = "/home/user/.zshrc"
backup = true

[[symlinks]]
source = "/nix/dotfiles/git/.gitconfig"
target = "/home/user/.gitconfig"
create_parents = false
backup = true
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

The configuration file is in TOML format:

```toml
# Optional: Override the default state directory
state_dir = "/path/to/state"

# Define symlinks
[[symlinks]]
source = "/path/to/source"          # Required: Path to the file/directory to link to
target = "/path/to/target"          # Required: Where the symlink will be created
create_parents = true               # Optional: Create parent directories (default: false)
backup = true                       # Optional: Backup existing target (default: false)
```

### Field Descriptions

- **source**: The actual file or directory you want to link to. Must exist.
- **target**: The location where the symlink will be created.
- **create_parents**: If true, creates parent directories for the target if they don't exist.
- **backup**: If true, backs up any existing file/directory at the target location with a timestamp.

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
[[symlinks]]
source = "/home/user/dotfiles/.vimrc"
target = "/home/user/.vimrc"
backup = true

[[symlinks]]
source = "/home/user/dotfiles/.bashrc"
target = "/home/user/.bashrc"
backup = true
```

### XDG Config Directory Management

```toml
[[symlinks]]
source = "/home/user/dotfiles/nvim"
target = "/home/user/.config/nvim"
create_parents = true
backup = true

[[symlinks]]
source = "/home/user/dotfiles/alacritty"
target = "/home/user/.config/alacritty"
create_parents = true
backup = true
```

### Impermanence-Style Setup

If you're using tmpfs for your home directory:

```toml
state_dir = "/persist/imp"

[[symlinks]]
source = "/persist/home/.zsh_history"
target = "/home/user/.zsh_history"
create_parents = true

[[symlinks]]
source = "/persist/home/.ssh"
target = "/home/user/.ssh"
create_parents = true

[[symlinks]]
source = "/persist/home/.gnupg"
target = "/home/user/.gnupg"
create_parents = true
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
