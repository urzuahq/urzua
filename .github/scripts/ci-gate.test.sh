#!/usr/bin/env bash
# Planted-violation cases for ci-gate.sh: the gate must fail closed on a
# failure, a cancellation, a skip nobody allowed, a missing result and an empty
# or malformed input, and pass only when everything it needs succeeded.
set -uo pipefail

gate="$(cd "$(dirname "$0")" && pwd)/ci-gate.sh"
fail=0

expect() { # want-exit description needs-json [gate args...]
  local want="$1" desc="$2" needs="$3" got
  shift 3
  NEEDS="$needs" "$gate" "$@" >/dev/null 2>&1
  got=$?
  if [ "$got" -ne "$want" ]; then
    echo "  FAIL  $desc (want exit $want, got $got)"
    fail=1
  else
    echo "  ok    $desc"
  fi
}

expect 0 "every job succeeded" '{"a":{"result":"success"},"b":{"result":"success"}}'
expect 1 "a failed job" '{"a":{"result":"success"},"b":{"result":"failure"}}'
expect 1 "a cancelled job" '{"a":{"result":"cancelled"},"b":{"result":"success"}}'
expect 1 "a skipped job nobody allowed" '{"a":{"result":"success"},"b":{"result":"skipped"}}'
expect 0 "a skipped job the caller allowed" '{"a":{"result":"success"},"b":{"result":"skipped"}}' --allow-skipped b
expect 1 "an allowed name does not excuse a different skipped job" '{"a":{"result":"skipped"},"b":{"result":"success"}}' --allow-skipped b
expect 1 "an allowed job that failed" '{"a":{"result":"success"},"b":{"result":"failure"}}' --allow-skipped b
expect 1 "a job with no result" '{"a":{"result":"success"},"b":{}}'
expect 2 "an empty needs object" '{}'
expect 2 "malformed needs" 'not json'
expect 2 "needs that is not an object" '["a"]'
expect 2 "an unknown argument" '{"a":{"result":"success"}}' --bogus

unset NEEDS
"$gate" >/dev/null 2>&1; got=$?
if [ "$got" -ne 2 ]; then echo "  FAIL  NEEDS unset (want exit 2, got $got)"; fail=1; else echo "  ok    NEEDS unset"; fi

if [ "$fail" -eq 0 ]; then echo "ci-gate: all cases passed"; else echo "ci-gate: FAILURES"; fi
exit "$fail"
