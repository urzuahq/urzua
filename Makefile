# Urzua — root dispatch.
#
# Per ADR-0004 this is a polyglot monorepo organized by language at the root.
# The root Makefile is the contract: `make check`, `make test`, `make build`
# work without knowing which languages are involved. Native commands still work
# inside each language directory for anyone who prefers them.

SHELL := /bin/bash
.DEFAULT_GOAL := help

RUST_DIR := rust

.PHONY: help build check test fmt fmt-check lint clean records records-base roadmap rule-table rule-table-check ci test-no-leak release-guard release-invariants knope-pin nextest-pin ci-gate-test knope-closes-compat gate-action-test hooks-install hooks-auto \
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

# Not run here, because each needs something a laptop does not have: the
# changeset gate reads the pull request's labels from the GitHub API; the
# pinned-release gate downloads a release and verifies its attestation; the
# gate's --actors file comes from the pull request's reviews. CI runs all three.
ci: fmt-check lint deny rust-build test-no-leak records records-base rule-table-check release-guard release-invariants knope-pin nextest-pin knope-closes-compat ci-gate-test gate-action-test ## Run the CI checks that can run locally (not the changeset label gate, the pinned-release gate or --actors)
	@echo "make ci: the local checks passed; CI also runs the changeset label gate, the pinned-release gate and the --actors file"

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

rust-test: ## Run the tests as CI does: nextest (all shards at once), doctests and the acceptance runner, under the leak guard
	@.github/scripts/ci-tests.sh all

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

records: rust-build ## Validate this repo's own governance records and the installed harness bundle
	@$(RUST_DIR)/target/release/urzua check
	@$(RUST_DIR)/target/release/urzua harness status

# CI's gate also runs `check --base` on a pull request, which is where the diff
# rules (diff.source-requires-record, diff.decider-required,
# identity.open-pr-collision) judge a change. A missing base is an error, never
# a skip: a check that did not run is not a pass. identity.open-pr-collision
# reads pull-request head refs that `check` never fetches, so they are fetched
# first, as the gate's workflow does; offline, the refs already present are used.
BASE ?= origin/main

records-base: rust-build ## urzua check --base $(BASE) (default origin/main): the diff rules CI runs on a pull request
	@git rev-parse --verify --quiet "$(BASE)" >/dev/null || { echo "records-base: $(BASE) not found; run 'git fetch origin main' or pass BASE=<ref>" >&2; exit 1; }
	@git fetch --no-tags --quiet origin '+refs/pull/*/head:refs/urzua/pr/origin/*' \
	  || echo "records-base: could not fetch pull request refs; identity.open-pr-collision judges the refs already present" >&2
	@$(RUST_DIR)/target/release/urzua check --base "$(BASE)"

roadmap: rust-build ## Print this repo's computed roadmap (waves, ready set, critical path) as JSON
	@$(RUST_DIR)/target/release/urzua roadmap

rule-table: rust-build ## Regenerate SPEC-2's rule table from the shipped rule set
	@python3 scripts/generate-rule-table.py

rule-table-check: rust-build ## Fail if SPEC-2's committed rule table is stale
	@python3 scripts/generate-rule-table.py --check

test-no-leak: ## Run the tests under a fresh TMPDIR and fail if they leave any urzua-* entry behind (BUG-193)
	@.github/scripts/test-no-leak.test.sh
	@.github/scripts/ci-tests.sh all

ci-gate-test: ## Exercise the aggregate gate's planted-violation cases (it must fail closed)
	@.github/scripts/ci-gate.test.sh

nextest-pin: ## Fail when a workflow carries its own nextest pin instead of the install-nextest action
	@.github/scripts/nextest-pin.test.sh

release-guard: ## Exercise the release guard's planted-violation cases
	@.github/scripts/release-guard.test.sh

knope-pin: ## Fail when a workflow carries its own knope pin instead of the install-knope action (and exercise the planted-violation cases)
	@.github/scripts/knope-pin.test.sh

knope-closes-compat: ## Run the pinned knope over a fragment carrying the closes key (skipped, loudly, where knope is not installed; CI installs it)
	@.github/scripts/knope-closes-compat.test.sh

release-invariants: ## Check the release invariants, and exercise their planted-violation cases
	@.github/scripts/release-invariants.test.sh
	@.github/scripts/release-invariants.sh .changeset rust/Cargo.toml "$$(git tag --sort=-v:refname | head -1)"
