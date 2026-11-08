#!/usr/bin/env bash
# The one definition of how CI runs the Rust tests, shared by the checks
# workflow and `make ci` so the two cannot drift. Every mode runs under
# test-no-leak.sh (a fresh TMPDIR that must be empty of urzua-* entries
# afterwards, BUG-193).
#
#   ci-tests.sh nextest [i/n]            the test suite except the acceptance runner,
#                                        under cargo-nextest; with i/n, shard i of n
#   ci-tests.sh doctest                  the doctests (nextest does not run them)
#   ci-tests.sh acceptance [i/n OUT]     the corpus acceptance runner; with i/n, shard i
#                                        of n, its proof written to OUT instead of the
#                                        coverage gate being judged
#   ci-tests.sh acceptance-merge DIR     the coverage gate over every shard's proof in DIR
#   ci-tests.sh all                      everything above, unsharded (what `make ci` runs)
#
# The acceptance runner is a custom harness (`harness = false`), one process, so
# nextest cannot list or partition it: it is excluded from the nextest run and
# sharded by its own i/n switch (rust/crates/urzua-cli/tests/acceptance/sharding.rs).
#
# Without cargo-nextest, `all` falls back to plain `cargo test` for a laptop, and says
# so; CI (CI=true) never falls back, so a missing tool cannot silently change what ran.
set -uo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
here="$root/.github/scripts"
cd "$root/rust" || exit 2

mode="${1:-all}"
shard="${2:-}"
out="${3:-}"

noleak() { "$here/test-no-leak.sh" "$@"; }

have_nextest() { cargo nextest --version >/dev/null 2>&1; }

run_nextest() {
  local args=(--workspace --locked --no-fail-fast -E 'not binary(acceptance)')
  [ -n "$shard" ] && args+=(--partition "count:$shard")
  noleak cargo nextest run "${args[@]}"
}

run_doctest() { noleak cargo test --workspace --doc --locked; }

run_acceptance() {
  if [ -n "$shard" ]; then
    [ -n "$out" ] || { echo "ci-tests: acceptance shard $shard needs an output file" >&2; return 2; }
    URZUA_ACCEPTANCE_SHARD="$shard" URZUA_ACCEPTANCE_PROVEN_OUT="$out" \
      noleak cargo test -p urzua-cli --locked --test acceptance
  else
    noleak cargo test -p urzua-cli --locked --test acceptance
  fi
}

case "$mode" in
  nextest)
    have_nextest || { echo "ci-tests: cargo-nextest is not installed" >&2; exit 2; }
    run_nextest ;;
  doctest) run_doctest ;;
  acceptance) run_acceptance ;;
  acceptance-merge)
    [ -n "$shard" ] || { echo "usage: ci-tests.sh acceptance-merge DIR" >&2; exit 2; }
    URZUA_ACCEPTANCE_MERGE="$shard" noleak cargo test -p urzua-cli --locked --test acceptance ;;
  all)
    if have_nextest; then
      shard="" ; run_nextest || exit $?
      run_doctest || exit $?
      run_acceptance
    elif [ -n "${CI:-}" ]; then
      echo "ci-tests: cargo-nextest is not installed and CI is set" >&2; exit 2
    else
      echo "ci-tests: cargo-nextest not installed; running plain cargo test (CI uses nextest: cargo install cargo-nextest --locked)" >&2
      noleak cargo test --workspace --locked --no-fail-fast
    fi ;;
  *) echo "usage: ci-tests.sh nextest|doctest|acceptance|acceptance-merge|all [args]" >&2; exit 2 ;;
esac
