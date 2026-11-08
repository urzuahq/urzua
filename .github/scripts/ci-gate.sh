#!/usr/bin/env bash
# The aggregate gate's verdict: every job it needs must have succeeded.
#
# A job that failed, was cancelled or was skipped is a red gate. Skipped is not
# harmless: a shard that never started (an upstream failure, a bad `if:`) would
# otherwise read as a pass, so the only skip allowed is one the caller names in
# --allow-skipped because the job is legitimately conditional (it says why).
# Every job is reported, so one run names every failure.
#
# usage: NEEDS='<toJSON(needs)>' ci-gate.sh [--allow-skipped job[,job...]]
# exit:  0 every needed job succeeded; 1 otherwise; 2 bad invocation or input.
set -uo pipefail

allow=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --allow-skipped) allow="${2:-}"; shift 2 ;;
    *) echo "ci-gate: unknown argument $1" >&2; exit 2 ;;
  esac
done

command -v jq >/dev/null || { echo "ci-gate: jq is required" >&2; exit 2; }
[ -n "${NEEDS:-}" ] || { echo "ci-gate: NEEDS (the toJSON(needs) of the gate job) is required" >&2; exit 2; }
if ! printf '%s' "$NEEDS" | jq -e 'type == "object" and length > 0' >/dev/null 2>&1; then
  echo "ci-gate: NEEDS is not a non-empty JSON object; an empty gate would pass on nothing" >&2
  exit 2
fi

bad=0
while IFS=$'\t' read -r job result; do
  if [ "$result" = "success" ]; then
    echo "  ok       $job"
  elif [ "$result" = "skipped" ] && [[ ",$allow," == *",$job,"* ]]; then
    echo "  skipped  $job (allowed)"
  else
    echo "  FAIL     $job: ${result:-no result}"
    bad=1
  fi
done < <(printf '%s' "$NEEDS" | jq -r 'to_entries[] | [.key, (.value.result // "")] | @tsv')

if [ "$bad" -ne 0 ]; then
  echo "ci-gate: one or more required jobs did not succeed" >&2
  exit 1
fi
echo "ci-gate: every required job succeeded"
