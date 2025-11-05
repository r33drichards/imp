{ pkgs ? import <nixpkgs> {} }:

let
  # Build the imp binary from source
  imp = pkgs.rustPlatform.buildRustPackage {
    pname = "imp";
    version = "0.1.0";

    src = ./.;

    cargoLock = {
      lockFile = ./Cargo.lock;
    };

    meta = {
      description = "Generation-based symlink manager inspired by NixOS impermanence";
      license = pkgs.lib.licenses.mit;
    };
  };

in pkgs.testers.nixosTest {
  name = "imp-integration-test";

  nodes.machine = { config, pkgs, ... }: {
    environment.systemPackages = [ imp ];

    # Create a test user
    users.users.testuser = {
      isNormalUser = true;
      home = "/home/testuser";
      createHome = true;
    };
  };

  testScript = ''
    start_all()
    machine.wait_for_unit("multi-user.target")

    # Test 1: Setup test directories and files
    machine.succeed("mkdir -p /tmp/dotfiles")
    machine.succeed("mkdir -p /tmp/persist")
    machine.succeed("echo 'test content' > /tmp/dotfiles/.testrc")
    machine.succeed("echo 'vim config' > /tmp/dotfiles/vimrc")
    machine.succeed("mkdir -p /tmp/dotfiles/nvim")
    machine.succeed("echo 'nvim init' > /tmp/dotfiles/nvim/init.vim")
    machine.succeed("echo 'persistent data' > /tmp/persist/history")

    # Test 2: Create a test configuration file
    machine.succeed("""
      cat > /tmp/test-config.toml << 'EOF'
    state_dir = "/tmp/imp-state"

    [[symlinks]]
    source = "/tmp/dotfiles/.testrc"
    target = "/tmp/target/.testrc"
    backup = true

    [[symlinks]]
    source = "/tmp/dotfiles/vimrc"
    target = "/tmp/target/.vimrc"
    create_parents = true
    backup = true

    [[symlinks]]
    source = "/tmp/dotfiles/nvim"
    target = "/tmp/target/.config/nvim"
    create_parents = true

    [[symlinks]]
    source = "/tmp/persist/history"
    target = "/tmp/target/.history"
    create_parents = true
    EOF
    """)

    # Test 3: Verify imp is installed and shows help
    machine.succeed("imp --help")
    machine.succeed("imp --version")

    # Test 4: Apply configuration (creates generation 0)
    print("Testing: imp apply")
    machine.succeed("imp apply --config /tmp/test-config.toml")

    # Test 5: Verify symlinks were created
    print("Testing: Symlink creation")
    machine.succeed("test -L /tmp/target/.testrc")
    machine.succeed("test -L /tmp/target/.vimrc")
    machine.succeed("test -L /tmp/target/.config/nvim")
    machine.succeed("test -L /tmp/target/.history")

    # Test 6: Verify symlink targets are correct
    print("Testing: Symlink targets")
    machine.succeed("readlink /tmp/target/.testrc | grep '/tmp/dotfiles/.testrc'")
    machine.succeed("readlink /tmp/target/.vimrc | grep '/tmp/dotfiles/vimrc'")
    machine.succeed("readlink /tmp/target/.config/nvim | grep '/tmp/dotfiles/nvim'")

    # Test 7: Verify symlink content is accessible
    print("Testing: Symlink content")
    machine.succeed("grep 'test content' /tmp/target/.testrc")
    machine.succeed("grep 'vim config' /tmp/target/.vimrc")
    machine.succeed("grep 'nvim init' /tmp/target/.config/nvim/init.vim")
    machine.succeed("grep 'persistent data' /tmp/target/.history")

    # Test 8: List generations
    print("Testing: imp list")
    output = machine.succeed("imp list")
    assert "Generation 0" in output, "Generation 0 should exist"

    # Test 9: Show current generation
    print("Testing: imp current")
    output = machine.succeed("imp current")
    assert "0" in output, "Current generation should be 0"

    # Test 10: Show generation details
    print("Testing: imp show")
    output = machine.succeed("imp show 0")
    assert "/tmp/dotfiles/.testrc" in output, "Generation details should include source paths"

    # Test 11: Verify symlinks
    print("Testing: imp verify")
    machine.succeed("imp verify")

    # Test 12: Test backup functionality - create a file that will be backed up
    machine.succeed("echo 'original file' > /tmp/target/.newfile")
    machine.succeed("""
      cat > /tmp/test-config2.toml << 'EOF'
    state_dir = "/tmp/imp-state"

    [[symlinks]]
    source = "/tmp/dotfiles/.testrc"
    target = "/tmp/target/.testrc"
    backup = true

    [[symlinks]]
    source = "/tmp/dotfiles/vimrc"
    target = "/tmp/target/.vimrc"
    create_parents = true
    backup = true

    [[symlinks]]
    source = "/tmp/dotfiles/nvim"
    target = "/tmp/target/.config/nvim"
    create_parents = true

    [[symlinks]]
    source = "/tmp/persist/history"
    target = "/tmp/target/.history"
    create_parents = true

    [[symlinks]]
    source = "/tmp/dotfiles/.testrc"
    target = "/tmp/target/.newfile"
    backup = true
    EOF
    """)

    # Test 13: Apply new configuration (creates generation 1)
    print("Testing: Generation creation with backup")
    machine.succeed("imp apply --config /tmp/test-config2.toml")

    # Test 14: Verify backup was created
    print("Testing: Backup functionality")
    machine.succeed("find /tmp/imp-state/backups -name '*.newfile' | grep -q .")

    # Test 15: Verify we now have 2 generations
    output = machine.succeed("imp list")
    assert "Generation 0" in output, "Generation 0 should still exist"
    assert "Generation 1" in output, "Generation 1 should exist"

    # Test 16: Switch to previous generation
    print("Testing: imp switch")
    machine.succeed("imp switch 0")

    # Test 17: Verify current generation changed
    output = machine.succeed("imp current")
    assert "0" in output, "Current generation should be 0 after switch"

    # Test 18: Verify symlinks match generation 0
    machine.succeed("test ! -L /tmp/target/.newfile || exit 1")

    # Test 19: Switch back to generation 1
    machine.succeed("imp switch 1")
    machine.succeed("test -L /tmp/target/.newfile")

    # Test 20: Test delete generation
    print("Testing: imp delete")
    # Cannot delete active generation
    machine.fail("imp delete 1")

    # Switch to generation 0 and delete generation 1
    machine.succeed("imp switch 0")
    machine.succeed("imp delete 1 --force")

    # Verify generation 1 is gone
    output = machine.succeed("imp list")
    assert "Generation 1" not in output, "Generation 1 should be deleted"

    # Test 21: Test validation
    print("Testing: Configuration validation")
    machine.succeed("""
      cat > /tmp/invalid-config.toml << 'EOF'
    state_dir = "/tmp/imp-state"

    [[symlinks]]
    source = "/tmp/nonexistent/source"
    target = "/tmp/target/.invalid"
    EOF
    """)

    # Should fail due to missing source
    machine.fail("imp apply --config /tmp/invalid-config.toml")

    # Should succeed with --skip-validation
    machine.succeed("imp apply --config /tmp/invalid-config.toml --skip-validation")

    # Test 22: Test parent directory creation
    print("Testing: Parent directory creation")
    machine.succeed("test -d /tmp/target/.config")

    # Test 23: Verify state directory structure
    print("Testing: State directory structure")
    machine.succeed("test -d /tmp/imp-state")
    machine.succeed("test -d /tmp/imp-state/generations")
    machine.succeed("test -d /tmp/imp-state/backups")
    machine.succeed("test -f /tmp/imp-state/current")

    # Test 24: Test with default config location (edge case)
    machine.succeed("mkdir -p /root/.config/imp")
    machine.succeed("cp /tmp/test-config.toml /root/.config/imp/config.toml")
    machine.succeed("imp apply")  # Should use default config

    print("All tests passed successfully!")
  '';
}
