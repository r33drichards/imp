# Testing Guide for Imp

This document describes how to run the NixOS integration tests for the Imp CLI tool.

## Prerequisites

1. **Cargo.lock file**: Before running NixOS tests, you need to generate a `Cargo.lock` file:
   ```bash
   cargo generate-lockfile
   ```

2. **Nix with flakes**: Ensure you have Nix installed with flakes enabled:
   ```bash
   # Add to ~/.config/nix/nix.conf or /etc/nix/nix.conf
   experimental-features = nix-command flakes
   ```

## Running Tests Locally

### Option 1: Using the standalone NixOS test

Run the NixOS integration test directly:

```bash
nix-build nixos-test.nix
```

This will:
- Build the imp binary from source
- Create a NixOS VM
- Run comprehensive integration tests covering:
  - Basic CLI functionality (apply, list, show, current, verify)
  - Symlink creation and management
  - Generation switching and rollback
  - Backup and restore functionality
  - Configuration validation
  - Parent directory creation
  - State directory structure

### Option 2: Using Nix flakes

Run all checks including the NixOS test:

```bash
nix flake check
```

Run only the NixOS integration test:

```bash
nix build .#checks.x86_64-linux.nixos-test
```

### Option 3: Using the flake dev shell

Enter the development environment:

```bash
nix develop
```

Then run any cargo commands or build the NixOS test:

```bash
cargo build
cargo test
nix-build nixos-test.nix
```

## GitHub Actions

The NixOS integration tests run automatically on:
- Push to `main` or `master` branch
- Pull requests targeting `main` or `master`
- Manual workflow dispatch

The workflow file is located at `.github/workflows/nixos-test.yml`.

### Workflow Features

- **Two test jobs**: One basic and one with cargo caching for faster builds
- **Automatic Nix installation**: Uses `cachix/install-nix-action`
- **Flakes support**: Enabled for modern Nix features
- **Test logs**: Uploaded as artifacts on failure for debugging

## Test Coverage

The NixOS integration test (`nixos-test.nix`) includes 24 comprehensive test scenarios:

1. **Setup & Installation**
   - Test directory and file creation
   - Configuration file creation
   - Help and version commands

2. **Core Functionality**
   - Apply configuration and create generation 0
   - Verify symlinks are created correctly
   - Check symlink targets point to correct sources
   - Validate symlink content is accessible

3. **Generation Management**
   - List all generations
   - Show current generation
   - Show generation details
   - Create multiple generations
   - Switch between generations
   - Delete generations

4. **Backup & Restore**
   - Backup existing files before creating symlinks
   - Verify backup files are created
   - Restore backups when switching generations

5. **Validation**
   - Configuration validation (missing sources)
   - Skip validation with `--skip-validation` flag
   - Verify command to check symlink integrity

6. **Edge Cases**
   - Parent directory creation
   - State directory structure verification
   - Default config location
   - Cannot delete active generation

## Troubleshooting

### Cargo.lock missing error

If you see an error about missing `Cargo.lock`:

```bash
cargo generate-lockfile
git add Cargo.lock
git commit -m "Add Cargo.lock for Nix builds"
```

### Nix flakes not enabled

Add to your `~/.config/nix/nix.conf`:

```
experimental-features = nix-command flakes
```

### Test VM doesn't start

Check system requirements:
- KVM support (Linux)
- Sufficient RAM (at least 2GB free)
- Disk space for NixOS VM image

### Permission denied errors

Ensure your user is in the `nix-users` group or run as root/sudo where appropriate.

## Continuous Integration

The GitHub Actions workflow automatically:
1. Installs Nix with flakes support
2. Sets up Cachix for faster builds (optional)
3. Runs `nix-build nixos-test.nix`
4. Reports test results
5. Uploads logs on failure

To view test results:
1. Go to the Actions tab in GitHub
2. Select the "NixOS Integration Tests" workflow
3. Click on a specific run to see details
4. Check the "Run NixOS integration tests" step for output

## Local Development

For faster iteration during development:

1. **Use the flake dev shell**:
   ```bash
   nix develop
   ```

2. **Build the binary manually**:
   ```bash
   cargo build --release
   ```

3. **Test specific functionality** without running the full NixOS test:
   ```bash
   # Create a test directory
   mkdir -p /tmp/imp-test
   cd /tmp/imp-test

   # Create test config
   cat > config.toml << EOF
   state_dir = "/tmp/imp-state"

   [[symlinks]]
   source = "/tmp/source"
   target = "/tmp/target"
   EOF

   # Run imp
   /path/to/imp apply --config config.toml
   ```

## Contributing

When adding new features:
1. Add corresponding test cases to `nixos-test.nix`
2. Ensure all tests pass locally before pushing
3. Check that GitHub Actions pass on your PR
4. Update this document if adding new test requirements

## Resources

- [NixOS Testing Documentation](https://nixos.org/manual/nixos/stable/#sec-nixos-tests)
- [Nix Flakes Documentation](https://nixos.wiki/wiki/Flakes)
- [Imp README](./README.md)
