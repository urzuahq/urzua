#!/usr/bin/env bash
# Harness-neutral core of the record-edit hook (ADR-69, ADR-78, MILE-178, MILE-191).
#
#   record-edit-hook.sh [--override NAME] <record-path> < proposed-content
#
# Judges the content a harness is about to write to <record-path>, before it is
# written, by relaying `urzua check --stdin-path` (SPEC-2): every enabled rule that
# can judge a single record. The rules it could not judge are named in the message. This script holds no rule knowledge: it never computes
# Embodiment or checks a header, it reads urzua's JSON.
#
# Contract
#   stdin    the proposed full content of the record
#   argv     the record path, absolute or relative to the repo root
#   env      URZUA_BIN             urzua binary (default: rust/target/release/urzua, then PATH)
#            URZUA_HOOK_OVERRIDE   a name; same as --override
#   stdout   exactly one JSON object:
#              verdict  allow | allow-with-warnings | block | allow-unjudged |
#                       allow-override | not-governed
#              path, message, findings (the findings that were blocked, overridden or
#              shown), override (the name, when one was given)
#   exit     0 the edit may proceed (every verdict but block), 1 block
#
# Verdicts
#   block                an unwaived error finding this edit introduced
#   allow-with-warnings  judged; warnings are shown in `message`, nothing blocks
#   allow-unjudged       urzua could not judge (missing, not-run, history unreadable,
#                        unparsable output). The edit proceeds; the message says so.
#   allow-override       a block was overridden by name; the name is in the output
#   not-governed         urzua says no record type governs the path: not a record
#   allow                judged, nothing to report; the message counts the rules that ran
#                        and names the enabled rules a single record cannot be judged by
# An error finding that is also reported for the record as it is on disk (a
# problem this edit did not introduce) does not block the edit; it is listed in
# `message` as pre-existing.
set -u

override="${URZUA_HOOK_OVERRIDE:-}"
override_given=0
[ -n "$override" ] && override_given=1
if [ "${1:-}" = "--override" ]; then
  override="${2-}"; override_given=1; shift 2 || true
fi
path="${1:-}"
[ -n "$path" ] || { echo '{"verdict":"allow-unjudged","message":"record-edit-hook: no record path given; the edit was not judged"}'; exit 0; }

root="$(git rev-parse --show-toplevel 2>/dev/null || (cd "$(dirname "$0")/../.." && pwd))"
cd "$root" || exit 0
case "$path" in "$root"/*) rel="${path#"$root"/}" ;; *) rel="$path" ;; esac

emit() { # verdict message [findings-json]
  jq -cn --arg v "$1" --arg m "$2" --arg p "$rel" --arg o "$override" --argjson g "$override_given" \
    --argjson f "${3:-[]}" \
    '{verdict:$v, path:$p, message:$m, findings:$f} + (if $g==1 and $o!="" then {override:$o} else {} end)'
}
unjudged() { emit allow-unjudged "record-edit-hook: urzua did NOT judge this edit to $rel ($1). The edit is allowed; run 'urzua check' before committing." ; exit 0; }

command -v jq >/dev/null 2>&1 || { echo "{\"verdict\":\"allow-unjudged\",\"path\":\"$rel\",\"message\":\"record-edit-hook: urzua did NOT judge this edit to $rel (jq is not installed). The edit is allowed.\",\"findings\":[]}"; exit 0; }

bin="${URZUA_BIN:-}"
[ -n "$bin" ] || { [ -x "$root/rust/target/release/urzua" ] && bin="$root/rust/target/release/urzua" || bin="$(command -v urzua || true)"; }
{ [ -n "$bin" ] && [ -x "$bin" ]; } || { cat >/dev/null; unjudged "no urzua binary found; run 'make rust-build' or set URZUA_BIN"; }

report="$("$bin" check --stdin-path "$rel" 2>/dev/null)"; code=$?
echo "$report" | jq -e . >/dev/null 2>&1 || unjudged "urzua exited $code with output that is not JSON"
status="$(echo "$report" | jq -r '.status // ""')"

if [ "$code" -eq 2 ] || [ "$status" = "not-run" ]; then
  err="$(echo "$report" | jq -r '.error // empty')"
  case "$err" in
    "no record type governs "*) emit not-governed "$rel is not a governed record; nothing to judge"; exit 0 ;;
  esac
  if [ -n "$err" ]; then unjudged "urzua reported not-run: $err"; fi
  unjudged "urzua reported not-run: no enabled rule examined the record"
fi
[ "$code" -le 1 ] || unjudged "urzua exited $code with status '$status'"

hist="$(echo "$report" | jq -r '[.notices[]? | select(.subject=="embodiment-drift-git") | .message] | join("; ")')"
[ -z "$hist" ] || unjudged "git history unreadable: $hist"

# Unwaived findings only: a waived finding is excluded from the blocking exit code.
live='[.findings[]? | select((.waived // null) == null)]'
errors="$(echo "$report" | jq -c "$live | map(select(.severity==\"error\"))")"
others="$(echo "$report" | jq -c "$live | map(select(.severity!=\"error\"))")"

# Split the errors into those this edit introduced and those the record on disk
# already has. A baseline run that cannot judge leaves every error introduced.
introduced="$errors"; existing='[]'
if [ "$errors" != "[]" ] && [ -f "$rel" ]; then
  # Only an edit with errors pays for the second run (about the first's time again).
  base="$("$bin" check --stdin-path "$rel" < "$rel" 2>/dev/null)"
  if echo "$base" | jq -e '.status == "ok" or .status == "findings-present"' >/dev/null 2>&1; then
    keys="$(echo "$base" | jq -c "$live | map(select(.severity==\"error\") | .rule + \"|\" + .message)")"
    introduced="$(echo "$errors" | jq -c --argjson k "$keys" 'map(select((.rule + "|" + .message) as $x | $k | index($x) | not))')"
    existing="$(echo "$errors" | jq -c --argjson k "$keys" 'map(select((.rule + "|" + .message) as $x | $k | index($x)))')"
  fi
fi

describe() { echo "$1" | jq -r '.[] | "  \(.file):\(.line // "-"): [\(.rule)] \(.message)" + (if .suggested_action.kind == "run-command" then " (run: " + (.suggested_action.target.argv | join(" ")) + ")" else "" end)'; }
# What actually ran, from the report: the rules that judged this record, and the
# enabled rules a single-record edit cannot judge (they run at commit and in CI).
ran_summary() {
  echo "$report" | jq -r '
    [.rules_executed[]? | select(.status=="ran") | .rule] as $ran
    | [.rules_executed[]? | select(.status=="not-applicable") | .rule] as $na
    | "\($ran | length) rules ran on this record"
      + (if ($na | length) > 0
         then "; \($na | length) enabled rules cannot judge a single-record edit and were NOT run here (they run at commit and in CI): \($na | join(", "))"
         else "" end)
      + "."'
}
note=""
[ "$existing" = "[]" ] || note+=$'\nAlready failing on disk before this edit (not blocked):\n'"$(describe "$existing")"
[ "$others" = "[]" ] || note+=$'\nWarnings (shown, not blocking):\n'"$(describe "$others")"

if [ "$introduced" = "[]" ]; then
  shown="$(jq -cn --argjson a "$existing" --argjson b "$others" '$a + $b')"
  if [ "$shown" = "[]" ]; then
    emit allow "record-edit-hook: urzua judged $rel; no findings. $(ran_summary)"; exit 0
  fi
  emit allow-with-warnings "record-edit-hook: urzua judged $rel; nothing this edit introduced blocks it. $(ran_summary)$note" "$shown"; exit 0
fi

if [ "$override_given" -eq 1 ]; then
  if [ -z "${override//[[:space:]]/}" ]; then
    emit block "record-edit-hook: blocked. An override needs a name (--override NAME or URZUA_HOOK_OVERRIDE=NAME); an empty one is not accepted."$'\n'"$(describe "$introduced")" "$introduced"; exit 1
  fi
  # Extension point (ADR-78 item 1, MILE-188): persist_override is where the
  # committed override log is written. Today the override exists only in this output.
  persist_override() { :; }
  persist_override "$override" "$rel" "$introduced"
  emit allow-override "record-edit-hook: OVERRIDE '$override' let this edit to $rel through despite:"$'\n'"$(describe "$introduced")$note" "$introduced"; exit 0
fi

emit block "record-edit-hook: blocked. urzua reports errors this edit introduced:"$'\n'"$(describe "$introduced")"$'\n'"Fix the record as the findings say. The rest of the edit is fine. To proceed anyway, a named override is required: URZUA_HOOK_OVERRIDE=<name>.$note" "$introduced"
exit 1
