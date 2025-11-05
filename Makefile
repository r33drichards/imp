.PHONY: help test nixos-test build clean dev fmt lint check-all

help:
	@echo "Imp - Available Make Targets"
	@echo ""
	@echo "  make build        - Build the imp binary using cargo"
	@echo "  make test         - Run cargo tests"
	@echo "  make nixos-test   - Run NixOS integration tests"
	@echo "  make dev          - Enter Nix development shell"
	@echo "  make fmt          - Format code with rustfmt"
	@echo "  make lint         - Run clippy linter"
	@echo "  make check-all    - Run all checks (flake check)"
	@echo "  make clean        - Clean build artifacts"
	@echo "  make lock         - Generate Cargo.lock file"
	@echo ""

build:
	cargo build --release

test:
	cargo test

nixos-test:
	nix-build nixos-test.nix --show-trace

dev:
	nix develop

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings

check-all:
	nix flake check

clean:
	cargo clean
	rm -f result result-*

lock:
	cargo generate-lockfile
