{
  description = "Imp - Generation-based symlink manager inspired by NixOS impermanence";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        imp = pkgs.rustPlatform.buildRustPackage {
          pname = "imp";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [ rustToolchain ];

          meta = with pkgs.lib; {
            description = "Generation-based symlink manager inspired by NixOS impermanence";
            homepage = "https://github.com/yourusername/imp";
            license = licenses.mit;
            maintainers = [ ];
            platforms = platforms.all;
          };
        };

      in
      {
        packages = {
          default = imp;
          imp = imp;
        };

        apps = {
          default = flake-utils.lib.mkApp {
            drv = imp;
          };
          imp = flake-utils.lib.mkApp {
            drv = imp;
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            cargo-watch
            cargo-edit
            rust-analyzer
            clippy
            rustfmt
          ];

          shellHook = ''
            echo "Imp development environment"
            echo "Rust version: $(rustc --version)"
            echo ""
            echo "Available commands:"
            echo "  cargo build       - Build the project"
            echo "  cargo run         - Run imp"
            echo "  cargo test        - Run tests"
            echo "  cargo clippy      - Run linter"
            echo "  cargo fmt         - Format code"
            echo ""
            echo "Run NixOS tests with: nix-build nixos-test.nix"
          '';
        };

        checks = {
          # Run the NixOS integration test
          nixos-test = import ./nixos-test.nix { inherit pkgs; };

          # Run cargo tests
          cargo-test = pkgs.runCommand "cargo-test" {
            buildInputs = [ rustToolchain imp ];
          } ''
            cd ${./.}
            cargo test
            touch $out
          '';

          # Run clippy
          cargo-clippy = pkgs.runCommand "cargo-clippy" {
            buildInputs = [ rustToolchain ];
          } ''
            cd ${./.}
            cargo clippy -- -D warnings
            touch $out
          '';

          # Run rustfmt check
          cargo-fmt = pkgs.runCommand "cargo-fmt" {
            buildInputs = [ rustToolchain ];
          } ''
            cd ${./.}
            cargo fmt -- --check
            touch $out
          '';
        };
      }
    ) // {
      # NixOS module for system-wide installation
      nixosModules.default = { config, lib, pkgs, ... }:
        with lib;
        let
          cfg = config.programs.imp;
        in {
          options.programs.imp = {
            enable = mkEnableOption "imp symlink manager";

            package = mkOption {
              type = types.package;
              default = self.packages.${pkgs.system}.default;
              description = "The imp package to use";
            };
          };

          config = mkIf cfg.enable {
            environment.systemPackages = [ cfg.package ];
          };
        };

      # Overlay for adding imp to nixpkgs
      overlays.default = final: prev: {
        imp = self.packages.${final.system}.default;
      };
    };
}
