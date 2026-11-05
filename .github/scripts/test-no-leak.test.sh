#!/usr/bin/env bash
# Planted-violation coverage for test-no-leak.sh: each case is asserted in both
# directions, so a guard that always passes (or always fails) fails this.
set -uo pipefail

guard="$(dirname "$0")/test-no-leak.sh"
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

expect 0 "a run that leaves nothing passes" "$guard" true
expect 1 "a leaked directory fails" "$guard" bash -c 'mkdir "$TMPDIR/urzua-leaked-fixture"'
expect 1 "a leaked file fails" "$guard" bash -c 'touch "$TMPDIR/urzua-leaked-file"'
expect 0 "a non-urzua entry is not judged" "$guard" bash -c 'mkdir "$TMPDIR/other-tool-scratch"'
expect 3 "a failing test command keeps its own exit code" "$guard" bash -c 'exit 3'
expect 1 "a leak outranks a passing run even when the command fails" "$guard" bash -c 'mkdir "$TMPDIR/urzua-x"; exit 3'

exit "$fail"
