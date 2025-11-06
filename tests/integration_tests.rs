use std::path::PathBuf;
use testcontainers::{
    core::{Mount, WaitFor},
    runners::SyncRunner,
    GenericImage, ImageExt,
};

/// Helper to get the path to the imp binary
fn get_imp_binary_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push(if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    });
    path.push("imp");
    path
}

/// Build the imp binary before running tests
fn ensure_binary_built() {
    let binary_path = get_imp_binary_path();
    if !binary_path.exists() {
        panic!(
            "Binary not found at {:?}. Run 'cargo build --release' first.",
            binary_path
        );
    }
}

#[test]
fn test_bind_mounts_with_privileged_container() {
    ensure_binary_built();
    let binary_path = get_imp_binary_path();
    let binary_dir = binary_path.parent().unwrap().to_str().unwrap();

    // Create an Ubuntu container with privileged mode for mount operations
    let image = GenericImage::new("ubuntu", "22.04")
        .with_wait_for(WaitFor::Nothing)
        .with_cmd(vec!["sleep", "infinity"])
        .with_privileged(true)
        .with_mount(Mount::bind_mount(binary_dir, "/imp-bin"));

    let container = image.start().expect("Failed to start container");

    // The test script that will run inside the container
    let test_script = r#"
#!/bin/bash
set -e

echo "=== Setting up test environment ==="

# Create test directories
mkdir -p /tmp/test-persist/home/tmp/test-repo
mkdir -p /tmp/test-persist/config/tmp/test-repo/.config

# Create source files in persistence directories
mkdir -p /tmp/test-persist/home/tmp/test-repo/.cargo
echo '[build]' > /tmp/test-persist/home/tmp/test-repo/.cargo/config.toml
echo 'RUST_LOG=debug' > /tmp/test-persist/home/tmp/test-repo/.env
touch /tmp/test-persist/home/tmp/test-repo/.testfile
echo 'log_level = "debug"' > /tmp/test-persist/config/tmp/test-repo/.config/settings.toml

# Create test directory
mkdir -p /tmp/test-repo

# Create test configuration file
cat > /tmp/test-repo/imp.toml <<'EOF'
state_dir = "/tmp/imp-state"

[persistence."/tmp/test-persist/home"]
directories = [
    "/tmp/test-repo/.cargo",
]
files = [
    "/tmp/test-repo/.testfile",
]

[persistence."/tmp/test-persist/config"]
directories = [
    "/tmp/test-repo/.config",
]
EOF

# Create second test configuration
cat > /tmp/test-repo/imp-v2.toml <<'EOF'
state_dir = "/tmp/imp-state"

[persistence."/tmp/test-persist/home"]
directories = [
    "/tmp/test-repo/.cargo",
]
files = [
    "/tmp/test-repo/.testfile",
    "/tmp/test-repo/.env",
]

[persistence."/tmp/test-persist/config"]
directories = [
    "/tmp/test-repo/.config",
]
EOF

IMP="/imp-bin/imp"

echo "=== Test 1: Apply first configuration ==="
$IMP --config /tmp/test-repo/imp.toml apply

echo "=== Test 2: List generations ==="
$IMP --config /tmp/test-repo/imp.toml list | grep "1 - .* - 3"

echo "=== Test 3: Show current generation ==="
$IMP --config /tmp/test-repo/imp.toml current | grep "Current generation: 1"

echo "=== Test 4: Show generation details ==="
$IMP --config /tmp/test-repo/imp.toml show 1 | grep "Generation 1:"

echo "=== Test 5: Verify mounts and symlinks ==="
$IMP --config /tmp/test-repo/imp.toml verify

echo "=== Test 6: Verify bind mount and symlink exist ==="
# Check if .cargo is a mount point
mount | grep "/tmp/test-repo/.cargo" || { echo "ERROR: .cargo bind mount missing"; exit 1; }
# Check if .testfile is a symlink
test -L /tmp/test-repo/.testfile || { echo "ERROR: .testfile symlink missing"; exit 1; }
# Check if .config is a mount point
mount | grep "/tmp/test-repo/.config" || { echo "ERROR: .config bind mount missing"; exit 1; }

echo "=== Test 7: Verify directory is writable (not readonly) ==="
# This is the key test - write to the mounted directory to ensure it's not readonly
touch /tmp/test-repo/.cargo/test-write || { echo "ERROR: Cannot write to .cargo directory"; exit 1; }
rm /tmp/test-repo/.cargo/test-write

echo "=== Test 8: Apply second configuration (generation 2) ==="
$IMP --config /tmp/test-repo/imp-v2.toml apply

echo "=== Test 9: Verify generation 2 is active ==="
$IMP --config /tmp/test-repo/imp-v2.toml list | grep "2 - .* - 4"

echo "=== Test 10: Verify .env symlink exists in generation 2 ==="
test -L /tmp/test-repo/.env || { echo "ERROR: .env symlink missing in gen 2"; exit 1; }

echo "=== Test 11: Switch back to generation 1 ==="
$IMP --config /tmp/test-repo/imp-v2.toml switch 1

echo "=== Test 12: Verify generation 1 is active after switch ==="
$IMP --config /tmp/test-repo/imp.toml current | grep "Current generation: 1"

echo "=== Test 13: Verify .env symlink removed after switch ==="
test ! -e /tmp/test-repo/.env || { echo "ERROR: .env symlink should not exist in gen 1"; exit 1; }

echo "=== Test 14: Delete generation 2 ==="
$IMP --config /tmp/test-repo/imp.toml delete 2 --force

echo "=== Test 15: Verify generation 2 is deleted ==="
! $IMP --config /tmp/test-repo/imp.toml list | grep "2 - " || { echo "ERROR: Generation 2 still exists"; exit 1; }

echo "=== Test 16: Verify state directory is custom location ==="
test -f /tmp/imp-state/generations.json || { echo "ERROR: State file not in custom location"; exit 1; }

echo "=== Test 17: Verify cannot delete active generation ==="
if $IMP --config /tmp/test-repo/imp.toml delete 1 --force 2>&1 | grep -q "Cannot delete active generation"; then
    echo "✓ Correctly prevented deletion of active generation"
else
    echo "ERROR: Should not allow deleting active generation"
    exit 1
fi

echo ""
echo "✅ All integration tests passed!"
"#;

    // Write and execute the test script
    let mut exec_result = container
        .exec(testcontainers::core::ExecCommand::new(vec![
            "bash",
            "-c",
            &format!("cat > /tmp/test.sh << 'EOFSCRIPT'\n{}\nEOFSCRIPT\nchmod +x /tmp/test.sh && /tmp/test.sh", test_script),
        ]))
        .expect("Failed to create and run test script");

    // Check for success
    let output =
        String::from_utf8_lossy(&exec_result.stdout_to_vec().expect("Failed to get stdout"))
            .to_string();
    let errors =
        String::from_utf8_lossy(&exec_result.stderr_to_vec().expect("Failed to get stderr"))
            .to_string();

    let exit_code = exec_result.exit_code().expect("Failed to get exit code");
    if exit_code != Some(0) {
        panic!(
            "Integration tests failed with exit code: {:?}\nStdout: {}\nStderr: {}",
            exit_code, output, errors
        );
    }

    println!("STDOUT:\n{}", output);
    if !errors.is_empty() {
        println!("STDERR:\n{}", errors);
    }

    assert!(
        output.contains("✅ All integration tests passed!"),
        "Integration tests did not pass. See output above."
    );
}

#[test]
fn test_sqlite_database_compatibility() {
    ensure_binary_built();
    let binary_path = get_imp_binary_path();
    let binary_dir = binary_path.parent().unwrap().to_str().unwrap();

    // Create an Ubuntu container with privileged mode for mount operations
    let image = GenericImage::new("ubuntu", "22.04")
        .with_wait_for(WaitFor::Nothing)
        .with_cmd(vec!["sleep", "infinity"])
        .with_privileged(true)
        .with_mount(Mount::bind_mount(binary_dir, "/imp-bin"));

    let container = image.start().expect("Failed to start container");

    // Install sqlite3 and run tests
    let test_script = r#"
#!/bin/bash
set -e

echo "=== Installing SQLite ==="
apt-get update -qq
apt-get install -y sqlite3 > /dev/null 2>&1

echo "=== Testing SQLite compatibility with bind mounts ==="

# Create persistence directory in the location that matches the bind mount
mkdir -p /persist/var/lib/myapp

# Create initial database in persistence directory
sqlite3 /persist/var/lib/myapp/test.db "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);"
sqlite3 /persist/var/lib/myapp/test.db "INSERT INTO users (name) VALUES ('Alice');"

# Create imp configuration
cat > /tmp/imp.toml <<'EOF'
state_dir = "/tmp/imp-state"

[persistence."/persist"]
directories = [
    "/var/lib/myapp",
]
EOF

# Apply configuration (create bind mount)
/imp-bin/imp --config /tmp/imp.toml apply

# Verify bind mount exists
mount | grep "/var/lib/myapp" || { echo "ERROR: Bind mount not created"; exit 1; }

# Test SQLite operations on bind-mounted directory
echo "=== Testing SQLite INSERT on bind mount ==="
sqlite3 /var/lib/myapp/test.db "INSERT INTO users (name) VALUES ('Bob');"

echo "=== Testing SQLite SELECT on bind mount ==="
sqlite3 /var/lib/myapp/test.db "SELECT * FROM users;" | grep "Bob" || { echo "ERROR: SQLite read failed"; exit 1; }

echo "=== Testing SQLite UPDATE on bind mount ==="
sqlite3 /var/lib/myapp/test.db "UPDATE users SET name = 'Charlie' WHERE name = 'Bob';"

echo "=== Testing SQLite DELETE on bind mount ==="
sqlite3 /var/lib/myapp/test.db "DELETE FROM users WHERE name = 'Alice';"

# Verify the database is still accessible
sqlite3 /var/lib/myapp/test.db "SELECT COUNT(*) FROM users;" | grep "1" || { echo "ERROR: Database state incorrect"; exit 1; }

# Test that lock files can be created (this is what fails with symlinks)
echo "=== Testing concurrent SQLite access (transaction with locking) ==="
sqlite3 /var/lib/myapp/test.db "BEGIN EXCLUSIVE TRANSACTION; INSERT INTO users (name) VALUES ('David'); COMMIT;"

# Verify the write succeeded
sqlite3 /var/lib/myapp/test.db "SELECT name FROM users WHERE name = 'David';" | grep "David" || { echo "ERROR: Transaction failed"; exit 1; }

echo ""
echo "✅ SQLite compatibility tests passed! No 'readonly database' errors!"
"#;

    // Write and execute the test script
    let mut exec_result = container
        .exec(testcontainers::core::ExecCommand::new(vec![
            "bash",
            "-c",
            &format!("cat > /tmp/sqlite_test.sh << 'EOFSCRIPT'\n{}\nEOFSCRIPT\nchmod +x /tmp/sqlite_test.sh && /tmp/sqlite_test.sh", test_script),
        ]))
        .expect("Failed to create and run SQLite test script");

    // Check for success
    let output =
        String::from_utf8_lossy(&exec_result.stdout_to_vec().expect("Failed to get stdout"))
            .to_string();
    let errors =
        String::from_utf8_lossy(&exec_result.stderr_to_vec().expect("Failed to get stderr"))
            .to_string();

    let exit_code = exec_result.exit_code().expect("Failed to get exit code");
    if exit_code != Some(0) {
        panic!(
            "SQLite tests failed with exit code: {:?}\nStdout: {}\nStderr: {}",
            exit_code, output, errors
        );
    }

    println!("STDOUT:\n{}", output);
    if !errors.is_empty() {
        println!("STDERR:\n{}", errors);
    }

    assert!(
        output.contains("✅ SQLite compatibility tests passed!"),
        "SQLite compatibility tests did not pass. See output above."
    );
}
