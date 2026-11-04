#!/usr/bin/env bash
# Harness-neutral core of the record-edit hook (ADR-69, MILE-178).
#
#   record-edit-hook.sh [--override NAME] <record-path> < proposed-content
#
# Judges the content a harness is about to write to <record-path>, before it is
# written, by relaying `urzua fix --stdin-path` (SPEC-8). This script holds no
# derivation: it never computes Embodiment, it reads urzua's JSON. Only the
# computed fields urzua reports in `repairs` are judged (today Embodiment);
# header, field and status rules are `urzua check`'s, at commit and CI.
#
# Contract
#   stdin    the proposed full content of the record
#   argv     the record path, absolute or relative to the repo root
#   env      URZUA_BIN             urzua binary (default: rust/target/release/urzua, then PATH)
#            URZUA_HOOK_OVERRIDE   a name; same as --override
#   stdout   exactly one JSON object:
#              verdict  allow | block | allow-unjudged | allow-override | not-governed
#              path, message, repairs (the repairs that were blocked or overridden),
#              override (the name, when one was given)
#   exit     0 the edit may proceed (every verdict but block), 1 block
#
# Verdicts
#   block           an edit changed a computed field to a value urzua does not compute
#   allow-unjudged  urzua could not judge (missing, not-run, history unreadable,
#                   unparsable output). The edit proceeds; the message says so.
#   allow-override  a block was overridden by name; the name is in the output
#   not-governed    urzua says no record type governs the path: not a record
#   allow           judged, nothing to block
# A repair whose stated value is already what the file on disk says (a
# disagreement this edit did not introduce) does not block the edit; it is
# listed in `message` as pre-existing.
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

emit() { # verdict message [repairs-json]
  jq -cn --arg v "$1" --arg m "$2" --arg p "$rel" --arg o "$override" --argjson g "$override_given" \
    --argjson r "${3:-[]}" \
    '{verdict:$v, path:$p, message:$m, repairs:$r} + (if $g==1 and $o!="" then {override:$o} else {} end)'
}
unjudged() { emit allow-unjudged "record-edit-hook: urzua did NOT judge this edit to $rel ($1). The edit is allowed; run 'urzua fix' and 'urzua check' before committing." ; exit 0; }

command -v jq >/dev/null 2>&1 || { echo "{\"verdict\":\"allow-unjudged\",\"path\":\"$rel\",\"message\":\"record-edit-hook: urzua did NOT judge this edit to $rel (jq is not installed). The edit is allowed.\",\"repairs\":[]}"; exit 0; }

bin="${URZUA_BIN:-}"
[ -n "$bin" ] || { [ -x "$root/rust/target/release/urzua" ] && bin="$root/rust/target/release/urzua" || bin="$(command -v urzua || true)"; }
{ [ -n "$bin" ] && [ -x "$bin" ]; } || { cat >/dev/null; unjudged "no urzua binary found; run 'make rust-build' or set URZUA_BIN"; }

report="$("$bin" fix --stdin-path "$rel" 2>/dev/null)"; code=$?
echo "$report" | jq -e . >/dev/null 2>&1 || unjudged "urzua exited $code with output that is not JSON"
status="$(echo "$report" | jq -r '.status // ""')"

if [ "$code" -eq 2 ] || [ "$status" = "not-run" ]; then
  err="$(echo "$report" | jq -r '.error // empty')"
  case "$err" in
    "no record type governs "*) emit not-governed "$rel is not a governed record; nothing to judge"; exit 0 ;;
  esac
  if [ -n "$err" ]; then unjudged "urzua reported not-run: $err"; fi
  unjudged "urzua reported not-run: the record states no Embodiment to judge"
fi
[ "$code" -eq 0 ] || unjudged "urzua exited $code with status '$status'"

hist="$(echo "$report" | jq -r '[.notices[]? | select(.subject=="embodiment-drift-git") | .message] | join("; ")')"
[ -z "$hist" ] || unjudged "git history unreadable: $hist"

# Split the repairs into those this edit introduced and those already on disk.
introduced='[]'; existing='[]'
while IFS= read -r r; do
  field="$(echo "$r" | jq -r .field)"; cur="$(echo "$r" | jq -r .current_value)"
  disk=""
  if [ -f "$rel" ]; then
    disk="$(awk -v k="$field:" 'index($0,k)==1 {sub(/^[^:]*:[ \t]*/,""); sub(/[ \t]+$/,""); print; exit}' "$rel")"
  fi
  if [ -f "$rel" ] && [ "$disk" = "$cur" ]; then existing="$(echo "$existing" | jq -c ". + [$r]")"
  else introduced="$(echo "$introduced" | jq -c ". + [$r]")"; fi
done < <(echo "$report" | jq -c '.repairs[]?')

describe() { echo "$1" | jq -r '.[] | "  \(.record):\(.line): \(.field) is \"\(.current_value)\", urzua computes \"\(.computed_value)\" from \(.evidence)"'; }
note=""
[ "$existing" = "[]" ] || note=$'\nAlready disagreeing on disk before this edit (not blocked):\n'"$(describe "$existing")"

if [ "$introduced" = "[]" ]; then
  emit allow "record-edit-hook: urzua judged $rel; no computed field disagrees.$note" "$existing"; exit 0
fi

if [ "$override_given" -eq 1 ]; then
  if [ -z "${override//[[:space:]]/}" ]; then
    emit block "record-edit-hook: blocked. An override needs a name (--override NAME or URZUA_HOOK_OVERRIDE=NAME); an empty one is not accepted."$'\n'"$(describe "$introduced")" "$introduced"; exit 1
  fi
  # Extension point (ADR-69 leaves persistence and routing undecided, RFC-7 / MILE-12):
  # persist_override is where a recorded or routed override would go. Today the
  # override exists only in this output.
  persist_override() { :; }
  persist_override "$override" "$rel" "$introduced"
  emit allow-override "record-edit-hook: OVERRIDE '$override' let this edit to $rel through despite:"$'\n'"$(describe "$introduced")" "$introduced"; exit 0
fi

emit block "record-edit-hook: blocked. This edit sets a field urzua computes to a value that disagrees with the evidence:"$'\n'"$(describe "$introduced")"$'\n'"Set the computed value (or run 'urzua fix --apply'). The rest of the edit is fine. To proceed anyway, a named override is required: URZUA_HOOK_OVERRIDE=<name>.$note" "$introduced"
exit 1
