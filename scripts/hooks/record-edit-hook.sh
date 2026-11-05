#!/usr/bin/env bash
# Harness-neutral core of the record-edit hook (ADR-69, ADR-78, MILE-178, MILE-191).
#
#   record-edit-hook.sh [--override NAME [--reason TEXT]] <record-path> < proposed-content
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
#            URZUA_HOOK_REASON     why; same as --reason
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
#   allow-override       a block was overridden by name; the name is in the output and,
#                        when the config declares `override_log`, one row per overridden
#                        finding is appended to that committed log (ADR-78, MILE-188):
#                        date, name, record, field, rule, reason, and a blank Reviewed-by
#                        that a person fills in later. A reason is then required.
#                        With no `override_log` declared the override is not recorded and
#                        the message says so.
#   not-governed         urzua says no record type governs the path: not a record
#   allow                judged, nothing to report; the message counts the rules that ran
#                        and names the enabled rules a single record cannot be judged by
# By default (`block_on: introduced`, also what an absent key means) an error
# finding that is also reported for the record as it is on disk (a problem this
# edit did not introduce) does not block the edit; it is listed in `message` as
# pre-existing. With the config's top-level `block_on: all` (SPEC-3, MILE-195) any
# unwaived error in the proposed content blocks, the on-disk baseline run is
# skipped, and nothing is listed as pre-existing. Any other value is not judged:
# the verdict is allow-unjudged and the message names the value.
set -u

override="${URZUA_HOOK_OVERRIDE:-}"
reason="${URZUA_HOOK_REASON:-}"
override_given=0
[ -n "$override" ] && override_given=1
while [ "${1:-}" = "--override" ] || [ "${1:-}" = "--reason" ]; do
  if [ "$1" = "--override" ]; then override="${2-}"; override_given=1
  else reason="${2-}"; fi
  shift 2 || break
done
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

# The config's top-level `block_on` scalar (SPEC-3), read the way `override_log` is.
declared_block_on() {
  awk '/^block_on:[ \t]/ { sub(/^block_on:[ \t]*/, ""); sub(/[ \t]+#.*$/, ""); sub(/[ \t]+$/, ""); gsub(/^["\047]|["\047]$/, ""); print; exit }' .urzua/config.yaml 2>/dev/null
}
block_on="$(declared_block_on)"
case "$block_on" in
  ""|introduced) block_on=introduced ;;
  all) ;;
  *) cat >/dev/null; unjudged "block_on '$block_on' in .urzua/config.yaml is not one of introduced, all" ;;
esac

bin="${URZUA_BIN:-}"
[ -n "$bin" ] || { [ -x "$root/rust/target/release/urzua" ] && bin="$root/rust/target/release/urzua" || bin="$(command -v urzua || true)"; }
{ [ -n "$bin" ] && [ -x "$bin" ]; } || { cat >/dev/null; unjudged "no urzua binary found; install urzua on PATH or set URZUA_BIN"; }

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
# Missing history costs only the rules that read it (embodiment drift). Every other
# finding is still judged and still blocks; the message says what was skipped.
histnote=""
[ -z "$hist" ] || histnote=$'\nNOT judged here: embodiment drift (the age of the evidence), because git history is unreadable: '"$hist"

# Unwaived findings only: a waived finding is excluded from the blocking exit code.
live='[.findings[]? | select((.waived // null) == null)]'
errors="$(echo "$report" | jq -c "$live | map(select(.severity==\"error\"))")"
others="$(echo "$report" | jq -c "$live | map(select(.severity!=\"error\"))")"

# Split the errors into those this edit introduced and those the record on disk
# already has. A baseline run that cannot judge leaves every error introduced.
introduced="$errors"; existing='[]'
if [ "$block_on" = introduced ] && [ "$errors" != "[]" ] && [ -f "$rel" ]; then
  # Only an edit with errors pays for the second run (about the first's time again).
  base="$("$bin" check --stdin-path "$rel" < "$rel" 2>/dev/null)"
  if echo "$base" | jq -e '.status == "ok" or .status == "findings-present"' >/dev/null 2>&1; then
    keys="$(echo "$base" | jq -c "$live | map(select(.severity==\"error\") | .rule + \"|\" + .message)")"
    introduced="$(echo "$errors" | jq -c --argjson k "$keys" 'map(select((.rule + "|" + .message) as $x | $k | index($x) | not))')"
    existing="$(echo "$errors" | jq -c --argjson k "$keys" 'map(select((.rule + "|" + .message) as $x | $k | index($x)))')"
  fi
fi

# A "run:" hint repairs the record on disk, so it is shown only for a finding the
# disk already has. A finding this edit introduced is absent from disk, where the
# command would answer "matched no pending repair": those are listed without it.
describe() { echo "$1" | jq -r --arg hint "${2:-hint}" '.[] | "  \(.file):\(.line // "-"): [\(.rule)] \(.message)" + (if $hint == "hint" and .suggested_action.kind == "run-command" then " (run: " + (.suggested_action.target.argv | join(" ")) + ")" else "" end)'; }
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
note="$histnote"
[ "$existing" = "[]" ] || note+=$'\nAlready failing on disk before this edit (not blocked):\n'"$(describe "$existing")"
[ "$others" = "[]" ] || note+=$'\nWarnings (shown, not blocking):\n'"$(describe "$others")"

if [ "$introduced" = "[]" ]; then
  shown="$(jq -cn --argjson a "$existing" --argjson b "$others" '$a + $b')"
  if [ "$shown" = "[]" ]; then
    if [ -n "$histnote" ]; then
      emit allow-with-warnings "record-edit-hook: urzua judged $rel with what it can see; no findings. $(ran_summary)$histnote"; exit 0
    fi
    emit allow "record-edit-hook: urzua judged $rel; no findings. $(ran_summary)"; exit 0
  fi
  emit allow-with-warnings "record-edit-hook: urzua judged $rel; nothing this edit introduced blocks it. $(ran_summary)$note" "$shown"; exit 0
fi

# The committed override log is the config's top-level `override_log` scalar
# (SPEC-3); this is the only config line the hook reads.
declared_log() {
  awk '/^override_log:[ \t]/ { sub(/^override_log:[ \t]*/, ""); sub(/[ \t]+#.*$/, ""); sub(/[ \t]+$/, ""); gsub(/^["\047]|["\047]$/, ""); print; exit }' .urzua/config.yaml 2>/dev/null
}
# One cell of a Markdown table row: no pipe, no newline.
cell() { printf '%s' "$1" | tr '\n|' '  ' | sed -e 's/^ *//' -e 's/ *$//'; }

# Appends one row per overridden finding. Returns non-zero when the log cannot be
# written, which blocks the edit: an override that is not recorded is not allowed.
persist_override() { # log-path name rel findings-json reason
  local log="$1" name="$2" record="$3" found="$4" why="$5" today
  today="$(date -u +%F)"
  mkdir -p "$(dirname "$log")" 2>/dev/null || return 1
  if [ ! -s "$log" ]; then
    printf '# Override log\n\nEach row is an editor-hook override that let a blocked record edit through\n(written by scripts/hooks/record-edit-hook.sh). A person sets Reviewed-by once they\nhave looked at it; `override.unreviewed` reports every row where it is blank.\n\n| Date | Name | Record | Field | Rule | Reason | Reviewed-by |\n|---|---|---|---|---|---|---|\n' > "$log" || return 1
  fi
  while IFS=$'\t' read -r rule field; do
    printf '| %s | %s | %s | %s | %s | %s | |\n' "$today" "$(cell "$name")" "$(cell "$record")" "$(cell "$field")" "$(cell "$rule")" "$(cell "$why")" >> "$log" || return 1
  done < <(echo "$found" | jq -r '.[] | [.rule, (.suggested_action.target.field // "-")] | @tsv')
}

if [ "$override_given" -eq 1 ]; then
  if [ -z "${override//[[:space:]]/}" ]; then
    emit block "record-edit-hook: blocked. An override needs a name (--override NAME or URZUA_HOOK_OVERRIDE=NAME); an empty one is not accepted."$'\n'"$(describe "$introduced" nohint)" "$introduced"; exit 1
  fi
  logpath="$(declared_log)"
  recorded="record-edit-hook: this override is NOT recorded: no override_log is declared in .urzua/config.yaml."
  if [ -n "$logpath" ]; then
    if [ -z "${reason//[[:space:]]/}" ]; then
      emit block "record-edit-hook: blocked. An override is recorded in $logpath and needs a reason (--reason TEXT or URZUA_HOOK_REASON=TEXT)."$'\n'"$(describe "$introduced" nohint)" "$introduced"; exit 1
    fi
    persist_override "$logpath" "$override" "$rel" "$introduced" "$reason" || {
      emit block "record-edit-hook: blocked. The override could not be recorded in $logpath, and an override that is not recorded is not allowed."$'\n'"$(describe "$introduced" nohint)" "$introduced"; exit 1; }
    recorded="Recorded in $logpath; a person reviews it by setting Reviewed-by on its row."
  fi
  emit allow-override "record-edit-hook: OVERRIDE '$override' let this edit to $rel through despite:"$'\n'"$(describe "$introduced" nohint)"$'\n'"$recorded$note" "$introduced"; exit 0
fi

emit block "record-edit-hook: blocked. urzua reports errors this edit introduced:"$'\n'"$(describe "$introduced" nohint)"$'\n'"Fix the record as the findings say. The rest of the edit is fine. To proceed anyway, a named override is required: URZUA_HOOK_OVERRIDE=<name>.$note" "$introduced"
exit 1
