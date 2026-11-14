#!/usr/bin/env bash
# Shared steps of the commands in .claude/commands/, installed by `urzua harness install`.
# Do not edit it: the install keeps its hash in .urzua/harness.lock.
#
#   urzua-record.sh new <type> <title>       scaffold through `urzua new`, snapshot the scaffold
#   urzua-record.sh finish <record>          guard, stage, regenerate indexes, run `urzua check`
#   urzua-record.sh amend-finish <record>    an amendment only adds; stage, run `urzua check`
#   urzua-record.sh repin <record> <reason>  `urzua fix --rebaseline`, only with a reason
#   urzua-record.sh reverify <record> <reason>  `urzua reverify`, only with a reason
#   urzua-record.sh done <record>            `urzua fix --apply`: the engine's computed value
#
# This script holds no schema knowledge: no record type, header field, state value or
# template text. Everything about a record's shape comes from `urzua new` and
# `.urzua/config.yaml`; the verdict comes from `urzua check`.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

# URZUA_BIN is the name the translator and the docs use; URZUA is still accepted.
bin="${URZUA_BIN:-${URZUA:-}}"
if [ -z "$bin" ]; then
  if [ -x "$root/.urzua/bin/urzua" ]; then
    bin="$root/.urzua/bin/urzua"
  elif [ -x "$root/rust/target/release/urzua" ]; then
    bin="$root/rust/target/release/urzua"
  elif command -v urzua >/dev/null 2>&1; then
    bin="$(command -v urzua)"
  else
    echo "urzua-record: no urzua binary at .urzua/bin/urzua or on PATH; install urzua or set URZUA_BIN" >&2
    exit 2
  fi
fi

snapshots="$root/.urzua/cache/draft"

header() { awk 'NR==1 && $0!="---" {exit} NR>1 && $0=="---" {exit} NR>1 {print}' "$1"; }

case "${1:-}" in
new)
  [ $# -eq 3 ] || { echo "usage: urzua-record.sh new <type> <title>" >&2; exit 2; }
  out="$("$bin" new "$2" "$3")"
  printf '%s\n' "$out"
  path="$(printf '%s\n' "$out" | sed -n 's/.*"path": *"\([^"]*\)".*/\1/p')"
  [ -n "$path" ] && [ -f "$path" ] || { echo "urzua-record: urzua new reported no record path" >&2; exit 1; }
  mkdir -p "$snapshots"
  cp "$path" "$snapshots/$(basename "$path").scaffold"
  ;;
finish)
  [ $# -eq 2 ] || { echo "usage: urzua-record.sh finish <record>" >&2; exit 2; }
  path="$2"
  scaffold="$snapshots/$(basename "$path").scaffold"
  [ -f "$path" ] || { echo "urzua-record: $path does not exist" >&2; exit 2; }
  [ -f "$scaffold" ] || { echo "urzua-record: $path was not scaffolded by 'urzua-record.sh new'" >&2; exit 2; }

  # Never decide: only a field the scaffold left null may change, and no field may be
  # added, removed or reordered. A scaffolded state, date or id is a decision or an
  # identity, not content.
  if ! paste -d'\n' <(header "$scaffold") <(header "$path") | awk '
      NR % 2 == 1 { split($0, a, ": "); key = a[1]; sub(/^[^:]*: */, "", $0); old = $0; okey = key; next }
      { split($0, b, ": "); nkey = b[1]; val = $0; sub(/^[^:]*: */, "", val)
        if (nkey != okey) { printf "header field %s became %s\n", okey, nkey > "/dev/stderr"; bad = 1 }
        else if (val != old && old != "null") { printf "scaffolded value of %s was changed (%s -> %s)\n", okey, old, val > "/dev/stderr"; bad = 1 } }
      END { exit bad }'; then
    echo "urzua-record: refusing; a drafting command fills null fields and the body only" >&2
    exit 1
  fi
  [ "$(header "$scaffold" | wc -l)" -eq "$(header "$path" | wc -l)" ] || {
    echo "urzua-record: refusing; the header's field count differs from the scaffold" >&2; exit 1; }

  git add -- "$path"
  idx="$("$bin" index --apply)"
  printf '%s\n' "$idx" | sed -n 's/.*"path": *"\([^"]*\)".*/\1/p' | while read -r p; do
    [ -f "$p" ] && git add -- "$p"
  done
  report="$("$bin" check "$path")" && rc=0 || rc=$?
  printf '%s\n' "$report"
  examined="$(printf '%s\n' "$report" | sed -n 's/.*"files_examined": *\([0-9]*\).*/\1/p' | head -1)"
  if [ "${examined:-0}" -lt 1 ]; then
    echo "urzua-record: urzua check examined no records; $path is NOT verified" >&2
    exit 1
  fi
  rm -f -- "$scaffold"
  echo "urzua-record: wrote $path; urzua check examined $examined record(s)" >&2
  exit "$rc"
  ;;
amend-finish)
  [ $# -eq 2 ] || { echo "usage: urzua-record.sh amend-finish <record>" >&2; exit 2; }
  path="$2"
  [ -f "$path" ] || { echo "urzua-record: $path does not exist" >&2; exit 2; }
  diff="$(git diff -U0 HEAD -- "$path")"
  # Every removed line is a change to existing text: a removed list item (`-- x`), blank
  # line (`-`) or rule (`----`) included. Only lines after the first hunk header count,
  # so the `--- a/path` file header is never mistaken for one.
  removed="$(printf '%s\n' "$diff" | awk '/^@@/ {h = 1} h && /^-/ {n++} END {print n + 0}')"
  if [ "$removed" -ne 0 ]; then
    echo "urzua-record: refusing; an amendment only adds lines, and $removed existing line(s) of $path were changed or removed" >&2
    exit 1
  fi
  grep -q '^+## Amendment (' <<<"$diff" || {
    echo "urzua-record: refusing; no added '## Amendment (YYYY-MM-DD): ...' section in $path" >&2; exit 1; }
  grep -q '^+> |' <<<"$diff" || {
    echo "urzua-record: refusing; no added revision-log row in $path" >&2; exit 1; }
  git add -- "$path"
  report="$("$bin" check "$path")" && rc=0 || rc=$?
  printf '%s\n' "$report"
  examined="$(printf '%s\n' "$report" | sed -n 's/.*"files_examined": *\([0-9]*\).*/\1/p' | head -1)"
  if [ "${examined:-0}" -lt 1 ]; then
    echo "urzua-record: urzua check examined no records; $path is NOT verified" >&2
    exit 1
  fi
  echo "urzua-record: amended $path; urzua check examined $examined record(s)" >&2
  exit "$rc"
  ;;
repin)
  [ $# -ge 2 ] || { echo "usage: urzua-record.sh repin <record> <reason>" >&2; exit 2; }
  path="$2"
  reason="${3:-}"
  if [ -z "$(printf '%s' "$reason" | tr -d '[:space:]')" ]; then
    echo "urzua-record: refusing; a re-pin needs a non-empty reason that says what actually changed in the pinned code, not that the pin was stale" >&2
    exit 1
  fi
  by="${URZUA_BY:-$(git config user.name || true)}"
  [ -n "$by" ] || { echo "urzua-record: no identity; set URZUA_BY to the person who authorised the re-pin" >&2; exit 2; }
  echo "urzua-record: the stale pins of $path (look at the diff of each pinned file before going on):" >&2
  "$bin" fix --rebaseline --ids "$path" >&2 || true
  out="$("$bin" fix --rebaseline --apply --ids "$path" --by "$by" --actor agent --reason "$reason")" && rc=0 || rc=$?
  printf '%s\n' "$out"
  grep -q '"status": "applied"' <<<"$out" || {
    echo "urzua-record: nothing was re-pinned for $path" >&2; exit 1; }
  [ "$rc" -eq 0 ] || exit "$rc"
  git add -- "$path"
  echo "Re-pin: $path -- $reason" >&2
  echo "urzua-record: list the line above in the pull request body for the reviewer" >&2
  ;;
reverify)
  [ $# -ge 2 ] || { echo "usage: urzua-record.sh reverify <record> <reason>" >&2; exit 2; }
  path="$2"
  reason="${3:-}"
  if [ -z "$(printf '%s' "$reason" | tr -d '[:space:]')" ]; then
    echo "urzua-record: refusing; a renewal needs a non-empty reason that says what you checked, not that the claim was old" >&2
    exit 1
  fi
  by="${URZUA_BY:-$(git config user.name || true)}"
  [ -n "$by" ] || { echo "urzua-record: no identity; set URZUA_BY to the person who authorised the renewal" >&2; exit 2; }
  echo "urzua-record: the review of $path (read every locator and span it lists before going on):" >&2
  review="$("$bin" reverify --ids "$path")" && rc=0 || rc=$?
  printf '%s\n' "$review" >&2
  out="$("$bin" reverify --apply --ids "$path" --by "$by" --actor agent --reason "$reason")" && rc=0 || rc=$?
  printf '%s\n' "$out"
  grep -q '"status": "applied"' <<<"$out" || {
    echo "urzua-record: nothing was renewed for $path" >&2; exit 1; }
  [ "$rc" -eq 0 ] || exit "$rc"
  git add -- "$path"
  if grep -q '"check": "will-pin"' <<<"$review"; then
    echo "Pinned: $path -- $reason" >&2
  fi
  echo "Re-verified: $path -- $reason" >&2
  echo "urzua-record: list every Pinned and Re-verified line above in the pull request body for the reviewer" >&2
  ;;
done)
  [ $# -eq 2 ] || { echo "usage: urzua-record.sh done <record>" >&2; exit 2; }
  path="$2"
  by="${URZUA_BY:-$(git config user.name || true)}"
  [ -n "$by" ] || { echo "urzua-record: no identity; set URZUA_BY to the person who authorised this" >&2; exit 2; }
  out="$("$bin" fix --apply --ids "$path" --by "$by" --actor agent)" && rc=0 || rc=$?
  printf '%s\n' "$out"
  if ! grep -q '"computed_value"' <<<"$out"; then
    echo "urzua-record: the engine computed no value for $path from its evidence, so nothing was set (add the evidence first, on each item of the record's completion criteria that the change delivered; the notices above name any item still unresolved; this command never writes a value itself)" >&2
    exit 1
  fi
  [ "$rc" -eq 0 ] || exit "$rc"
  git add -- "$path"
  report="$("$bin" check "$path")" && rc=0 || rc=$?
  printf '%s\n' "$report"
  examined="$(printf '%s\n' "$report" | sed -n 's/.*"files_examined": *\([0-9]*\).*/\1/p' | head -1)"
  if [ "${examined:-0}" -lt 1 ]; then
    echo "urzua-record: urzua check examined no records; $path is NOT verified" >&2
    exit 1
  fi
  echo "urzua-record: $path updated by the engine; urzua check examined $examined record(s)" >&2
  exit "$rc"
  ;;
*)
  echo "usage: urzua-record.sh new|finish|amend-finish|repin|reverify|done ..." >&2
  exit 2
  ;;
esac
