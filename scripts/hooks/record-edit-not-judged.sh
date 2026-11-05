#!/usr/bin/env bash
# Shared "this edit was not judged" notice of the record-edit hook (ADR-69, ADR-78).
#
#   record-edit-not-judged.sh <why>
#
# The hook fails open when it cannot run (python3 missing, CLAUDE_PROJECT_DIR unset, an
# unreadable payload). Failing open stays, silently does not: this prints a notice on
# stderr and, when the config declares `override_log`, appends one row to that log
# (name "(unjudged)", Reviewed-by blank) so `override.unreviewed` shows it in review.
# Always exits 0. Prints the notice on stdout too, for a caller that relays it.
why="${1:-unknown reason}"
msg="record-edit-hook: urzua did NOT judge this edit ($why). It is allowed; run 'urzua check' before committing."
printf '%s\n' "$msg" >&2
root="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || (cd "$(dirname "$0")/../.." && pwd))}"
log="$(awk '/^override_log:[ \t]/ { sub(/^override_log:[ \t]*/, ""); sub(/[ \t]+#.*$/, ""); sub(/[ \t]+$/, ""); gsub(/^["\047]|["\047]$/, ""); print; exit }' "$root/.urzua/config.yaml" 2>/dev/null)"
if [ -n "$log" ] && [ -s "$root/$log" ]; then
  cell="$(printf '%s' "$why" | tr '\n|' '  ')"
  printf '| %s | (unjudged) | - | - | - | %s | |\n' "$(date -u +%F)" "$cell" >> "$root/$log" 2>/dev/null || true
fi
printf '%s\n' "$msg"
exit 0
