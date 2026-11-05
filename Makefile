# Urzua — root dispatch.
#
# Per ADR-0004 this is a polyglot monorepo organized by language at the root.
# The root Makefile is the contract: `make check`, `make test`, `make build`
# work without knowing which languages are involved. Native commands still work
# inside each language directory for anyone who prefers them.

SHELL := /bin/bash
.DEFAULT_GOAL := help

RUST_DIR := rust

.PHONY: help build check test fmt fmt-check lint clean records rule-table rule-table-check ci release-guard release-invariants knope-closes-compat gate-action-test hooks-install hooks-auto \
	rust-build rust-check rust-test rust-fmt rust-fmt-check rust-lint deny rust-clean

help: ## Show available commands
	@echo "Urzua — available commands:"
	@echo
	@grep -hE '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

# ---------------------------------------------------------------- all languages

build: rust-build hooks-auto ## Build everything and wire the git hooks
check: rust-check ## Type-check / compile-check everything
test: rust-test ## Run all tests
fmt: rust-fmt ## Format all code
fmt-check: rust-fmt-check ## Verify formatting without writing
lint: rust-lint ## Lint everything
clean: rust-clean ## Remove build artifacts

ci: fmt-check lint deny rust-build test records rule-table-check release-guard release-invariants knope-closes-compat gate-action-test ## Run exactly what CI runs, locally
	@echo "make ci: all checks passed"

gate-action-test: ## Test the reusable CI gate action's scripts (recorded fixtures, no network)
	.github/actions/urzua-gate/test.sh

hooks-install: ## Wire the versioned git hooks (core.hooksPath=.git-hooks): pre-commit (urzua check --base HEAD), pre-push (fmt + clippy + urzua check)
	@current="$$(git config --get core.hooksPath || true)"; \
	if [ -n "$$current" ] && [ "$$current" != ".git-hooks" ]; then \
	  echo "hooks-install: core.hooksPath is already '$$current'; not overriding it." >&2; \
	  echo "hooks-install: point it at .git-hooks, or call .git-hooks/pre-commit from your hook. 'urzua doctor' reports the wiring." >&2; \
	  exit 1; \
	fi; \
	chmod +x .git-hooks/pre-commit .git-hooks/pre-push; \
	git config core.hooksPath .git-hooks; \
	echo "git hooks wired: core.hooksPath=.git-hooks (pre-commit, pre-push)"

# Run by `make build`, so the first build of a clone wires the hooks with no
# opt-in step (MILE-179). Never fails the build: outside a git work tree there
# is nothing to wire, and a hooksPath the contributor chose is theirs. Both are
# said on stderr, and `urzua doctor` reports the state either way.
hooks-auto:
	@if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then \
	  $(MAKE) --no-print-directory hooks-install || true; \
	else \
	  echo "hooks-auto: not a git work tree; git hooks not wired" >&2; \
	fi

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

deny: ## Unused dependencies (cargo machete) and supply chain (cargo deny); needs both installed, versions pinned in checks.yml
	cd $(RUST_DIR) && cargo machete && cargo deny check

rust-clean: ## Remove the Rust target directory
	cd $(RUST_DIR) && cargo clean

# -------------------------------------------------------------------- records

records: rust-build ## Validate this repo's own governance records
	@$(RUST_DIR)/target/release/urzua check

rule-table: rust-build ## Regenerate SPEC-2's rule table from the shipped rule set
	@python3 scripts/generate-rule-table.py

rule-table-check: rust-build ## Fail if SPEC-2's committed rule table is stale
	@python3 scripts/generate-rule-table.py --check

release-guard: ## Exercise the release guard's planted-violation cases
	@.github/scripts/release-guard.test.sh

knope-closes-compat: ## Run the pinned knope over a fragment carrying the closes key (skipped, loudly, where knope is not installed; CI installs it)
	@.github/scripts/knope-closes-compat.test.sh

release-invariants: ## Check the release invariants, and exercise their planted-violation cases
	@.github/scripts/release-invariants.test.sh
	@.github/scripts/release-invariants.sh .changeset rust/Cargo.toml "$$(git tag --sort=-v:refname | head -1)"
