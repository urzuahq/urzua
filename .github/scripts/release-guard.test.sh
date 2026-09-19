#!/usr/bin/env bash
# Planted-violation coverage for release-guard.sh (AGENTS.md): each case is
# asserted in both directions, so a guard that always passes fails this.
set -uo pipefail

guard="$(dirname "$0")/release-guard.sh"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0

expect() {
  local want="$1" desc="$2"; shift 2
  "$@" >/dev/null 2>&1
  local got=$?
  if [ "$got" -ne "$want" ]; then
    echo "  FAIL  $desc (want exit $want, got $got)"
    fail=1
  else
    echo "  ok    $desc"
  fi
}

no_release="Error: releases::no_release
  x No packages are ready to release"

mkdir -p "$tmp/empty"
expect 0 "no_release with no fragments is legitimate" \
  "$guard" "$tmp/empty" "$no_release"

# The planted violation: a fragment naming a package knope.toml does not
# declare. Inert, and previously reported as success.
mkdir -p "$tmp/unmatched"
printf -- '---\n"urzua": major\n---\n\nA change.\n' > "$tmp/unmatched/inert.md"
expect 1 "no_release with an unmatched-package fragment fails" \
  "$guard" "$tmp/unmatched" "$no_release"

# 2, not 0: a knope failure that is not a no-release must reach the caller as
# "no verdict", or the workflow reads silence as success and swallows it.
expect 2 "output with no no_release yields no verdict" \
  "$guard" "$tmp/unmatched" "Wrote changelog; version bumped to 0.4.0"

expect 2 "an unrelated knope failure yields no verdict" \
  "$guard" "$tmp/empty" "Error: some other failure entirely"

[ "$fail" -eq 0 ] && echo "release-guard: all cases passed"
exit "$fail"
