# Imp CLI Test Results

## Test Environment
- **Repository Used**: https://github.com/testcontainers/testcontainers-rs
- **Test Date**: 2025-11-05
- **Persistence Directories**: `/tmp/test-persist/home` and `/tmp/test-persist/config`
- **State Directory**: `/tmp/imp-state`

## Test Summary

All core CLI commands were tested successfully with the following results:

### ✓ Build Status
- Built successfully with Cargo in release mode
- Minor warning about unused fields in `Symlink` struct (user, group, mode) - these are for future use

### ✓ Apply Command
**Test 1: Initial Generation**
```bash
imp apply --config /tmp/testcontainers-rs/imp.toml
```
- ✓ Successfully created generation 1 with 3 symlinks
- ✓ Validated configuration before applying
- ✓ Created symlinks for:
  - `.cargo` directory
  - `.testcontainers.properties` file
  - `.config` directory

**Test 2: Second Generation**
```bash
imp apply --config /tmp/testcontainers-rs/imp-v2.toml
```
- ✓ Successfully created generation 2 with 4 symlinks (added `.env` file)
- ✓ Automatically removed symlinks from previous generation
- ✓ Applied new configuration with additional symlink

### ✓ List Command
```bash
imp list
```
- ✓ Successfully listed all generations
- ✓ Displayed generation number, creation date, symlink count
- ✓ Correctly marked active generation

### ✓ Current Command
```bash
imp current
```
- ✓ Displayed active generation information
- ✓ Showed creation date, config path, and symlink count

### ✓ Show Command
```bash
imp show 1
```
- ✓ Displayed detailed generation information
- ✓ Listed all symlinks with source and target paths

### ✓ Verify Command
```bash
imp verify
```
- ✓ Successfully verified all symlinks in active generation
- ✓ Confirmed all symlinks are correctly configured

### ✓ Switch Command
```bash
imp switch 1
```
- ✓ Successfully switched from generation 2 to generation 1
- ✓ Removed symlinks from generation 2 (4 symlinks including `.env`)
- ✓ Recreated symlinks from generation 1 (3 symlinks without `.env`)
- ✓ Updated active generation marker

### ✓ Delete Command
**Test 1: Delete Non-Active Generation**
```bash
imp delete 2 --force
```
- ✓ Successfully deleted generation 2
- ✓ Generation removed from list

**Test 2: Delete Active Generation (Expected Failure)**
```bash
imp delete 1 --force
```
- ✓ Correctly prevented deletion with error: "Cannot delete active generation"
- ✓ Expected behavior - protects active generation from deletion

## Configuration Files

### Test Configuration 1 (imp.toml)
```toml
state_dir = "/tmp/imp-state"

[persistence."/tmp/test-persist/home"]
directories = ["/tmp/testcontainers-rs/.cargo"]
files = ["/tmp/testcontainers-rs/.testcontainers.properties"]

[persistence."/tmp/test-persist/config"]
directories = ["/tmp/testcontainers-rs/.config"]
```

### Test Configuration 2 (imp-v2.toml)
Added `.env` file to test generation updates.

## Issues Identified and Resolved

### Issue 1: State Directory Inconsistency ✅ FIXED
- **Description**: The `apply` command uses `state_dir` from config file, but other commands (list, current, verify, switch, delete) were hardcoded to use `~/.local/share/imp`
- **Impact**: When using a custom state_dir, you needed to manually copy the generations.json file to the default location for other commands to work
- **Location**: src/main.rs:127-129, 155-157, 188-190, 228-230, 258-260, 286-288
- **Resolution**:
  - Added global `--config` flag to main CLI struct that applies to all commands
  - Created `get_state_dir()` helper function that reads state_dir from config file
  - Updated all commands to use config-based state directory with fallback to default
  - All commands now consistently respect the state_dir setting from configuration
- **Commit**: `50c76a1` - "Fix state_dir inconsistency across all CLI commands"
- **Status**: ✅ Verified and tested - all commands now work correctly with custom state_dir

## Symlink Verification

Created symlinks were verified manually:
```
.cargo -> /tmp/test-persist/home/tmp/testcontainers-rs/.cargo
.config -> /tmp/test-persist/config/tmp/testcontainers-rs/.config
.testcontainers.properties -> /tmp/test-persist/home/tmp/testcontainers-rs/.testcontainers.properties
.env -> /tmp/test-persist/home/tmp/testcontainers-rs/.env (generation 2 only)
```

All symlinks pointed to correct source paths and were properly removed/recreated during generation switches.

## Fix Verification Tests

After fixing the state directory inconsistency issue, the following additional tests were performed:

### Global --config Flag
```bash
imp --help
```
- ✓ Global `--config` flag now appears in help output
- ✓ Default value is `imp.toml`
- ✓ Flag applies to all commands

### Commands with Custom State Directory
All commands were retested with `--config /tmp/testcontainers-rs/imp.toml` (custom state_dir: `/tmp/imp-state`):

```bash
imp --config /tmp/testcontainers-rs/imp.toml apply
imp --config /tmp/testcontainers-rs/imp.toml list
imp --config /tmp/testcontainers-rs/imp.toml current
imp --config /tmp/testcontainers-rs/imp.toml verify
imp --config /tmp/testcontainers-rs/imp.toml show 1
imp --config /tmp/testcontainers-rs/imp.toml switch 1
imp --config /tmp/testcontainers-rs/imp.toml delete 2 --force
```

- ✓ All commands successfully read state_dir from config file
- ✓ No manual state file copying required
- ✓ State file correctly stored in `/tmp/imp-state/` as specified in config
- ✓ Generation switching works seamlessly across commands
- ✓ Generation deletion works with custom state_dir

### Fallback Behavior
```bash
imp --config /tmp/nonexistent.toml list
```
- ✓ When config file doesn't exist, falls back to default `~/.local/share/imp`
- ✓ No errors or crashes when config file is missing

## Conclusion

The `imp` CLI successfully passed all functional tests:
- ✓ Generation creation and management
- ✓ Symlink creation and removal
- ✓ Configuration validation
- ✓ Generation switching
- ✓ Generation deletion (with proper active generation protection)
- ✓ Symlink verification
- ✓ **[NEW]** Consistent state directory handling across all commands
- ✓ **[NEW]** Global --config flag functionality

The tool works as documented and provides reliable symlink management with generation-based versioning. The state directory inconsistency issue has been resolved, making the CLI fully functional with custom state directories.

## Test Artifacts

- Test configuration files: `/tmp/testcontainers-rs/imp.toml` and `/tmp/testcontainers-rs/imp-v2.toml`
- State directory: `/tmp/imp-state/generations.json`
- Test repository: `/tmp/testcontainers-rs/`
- Persistence directories: `/tmp/test-persist/home/` and `/tmp/test-persist/config/`
