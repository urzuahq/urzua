#!/usr/bin/env bash
# Runs a test command under a fresh, empty TMPDIR and fails when it leaves any
# urzua-* entry behind (BUG-193). Every fixture directory a test builds must be
# removed by its owner, including when the test panics; a leak is invisible in a
# single run and fills the disk across thousands of them.
#
# usage: test-no-leak.sh [command...]   (default: cargo test --workspace --locked --no-fail-fast, from rust/)
# exit:  1 when anything leaked (even if the test command also failed); otherwise the
#        test command's own exit code.
set -uo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

if [ "$#" -eq 0 ]; then
  set -- cargo test --workspace --locked --no-fail-fast
  cd "$root/rust" || exit 2
fi

TMPDIR="$tmp" "$@"
status=$?

# Entries other than urzua-* (a compiler's own scratch files) are not ours to judge.
leaked=$(find "$tmp" -mindepth 1 -maxdepth 1 -name 'urzua-*' | sort)
if [ -n "$leaked" ]; then
  echo "test-no-leak: the run left $(printf '%s\n' "$leaked" | wc -l | tr -d ' ') urzua-* entries in TMPDIR:" >&2
  printf '%s\n' "$leaked" | head -20 | sed 's|.*/|  |' >&2
  exit 1
fi
echo "test-no-leak: no urzua-* entries left in TMPDIR"
exit "$status"
