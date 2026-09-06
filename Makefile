# Urzua — root dispatch.
#
# Per ADR-0004 this is a polyglot monorepo organized by language at the root.
# The root Makefile is the contract: `make check`, `make test`, `make build`
# work without knowing which languages are involved. Native commands still work
# inside each language directory for anyone who prefers them.

SHELL := /bin/bash
.DEFAULT_GOAL := help

RUST_DIR := rust

.PHONY: help build check test fmt fmt-check lint clean records ci hooks-install \
	rust-build rust-check rust-test rust-fmt rust-fmt-check rust-lint rust-clean

help: ## Show available commands
	@echo "Urzua — available commands:"
	@echo
	@grep -hE '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

# ---------------------------------------------------------------- all languages

build: rust-build ## Build everything
check: rust-check ## Type-check / compile-check everything
test: rust-test ## Run all tests
fmt: rust-fmt ## Format all code
fmt-check: rust-fmt-check ## Verify formatting without writing
lint: rust-lint ## Lint everything
clean: rust-clean ## Remove build artifacts

ci: fmt-check lint rust-build test records ## Run exactly what CI runs, locally
	@echo "make ci: all checks passed"

hooks-install: ## Install the pre-push hook (fmt + clippy, not the full suite)
	@cp .git-hooks/pre-push .git/hooks/pre-push
	@chmod +x .git/hooks/pre-push
	@echo "pre-push hook installed"

# ---------------------------------------------------------------------- rust

rust-build: ## Build the urzua CLI (release)
	cd $(RUST_DIR) && cargo build --workspace --release --locked

rust-check: ## cargo check the workspace
	cd $(RUST_DIR) && cargo check --workspace --all-targets --locked

rust-test: ## cargo test the workspace
	cd $(RUST_DIR) && cargo test --workspace --locked

rust-fmt: ## cargo fmt
	cd $(RUST_DIR) && cargo fmt --all

rust-fmt-check: ## cargo fmt --check
	cd $(RUST_DIR) && cargo fmt --all -- --check

rust-lint: ## clippy, warnings denied
	cd $(RUST_DIR) && cargo clippy --workspace --all-targets --locked -- -D warnings

rust-clean: ## Remove the Rust target directory
	cd $(RUST_DIR) && cargo clean

# -------------------------------------------------------------------- records

records: rust-build ## Validate this repo's own governance records
	@$(RUST_DIR)/target/release/urzua check docs/
