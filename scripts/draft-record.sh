#!/usr/bin/env bash
# Shared steps of the drafting commands in .claude/commands/ (ADR-70).
#
#   draft-record.sh new <type> <title>   scaffold through `urzua new`, snapshot the scaffold
#   draft-record.sh finish <record>      guard, stage, regenerate indexes, run `urzua check`
#
# This script holds no schema knowledge: no record type, header field, state value or
# template text. Everything about a record's shape comes from `urzua new` and
# `.urzua/config.yaml`; the verdict comes from `urzua check`.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

bin="${URZUA:-}"
if [ -z "$bin" ]; then
  if [ -x "$root/rust/target/release/urzua" ]; then
    bin="$root/rust/target/release/urzua"
  elif command -v urzua >/dev/null 2>&1; then
    bin="$(command -v urzua)"
  else
    echo "draft-record: no urzua binary; run 'make rust-build' or set URZUA" >&2
    exit 2
  fi
fi

snapshots="$root/.urzua/cache/draft"

header() { awk 'NR==1 && $0!="---" {exit} NR>1 && $0=="---" {exit} NR>1 {print}' "$1"; }

case "${1:-}" in
new)
  [ $# -eq 3 ] || { echo "usage: draft-record.sh new <type> <title>" >&2; exit 2; }
  out="$("$bin" new "$2" "$3")"
  printf '%s\n' "$out"
  path="$(printf '%s\n' "$out" | sed -n 's/.*"path": *"\([^"]*\)".*/\1/p')"
  [ -n "$path" ] && [ -f "$path" ] || { echo "draft-record: urzua new reported no record path" >&2; exit 1; }
  mkdir -p "$snapshots"
  cp "$path" "$snapshots/$(basename "$path").scaffold"
  ;;
finish)
  [ $# -eq 2 ] || { echo "usage: draft-record.sh finish <record>" >&2; exit 2; }
  path="$2"
  scaffold="$snapshots/$(basename "$path").scaffold"
  [ -f "$path" ] || { echo "draft-record: $path does not exist" >&2; exit 2; }
  [ -f "$scaffold" ] || { echo "draft-record: $path was not scaffolded by 'draft-record.sh new'" >&2; exit 2; }

  # Never decide: only a field the scaffold left null may change, and no field may be
  # added, removed or reordered. A scaffolded state, date or id is a decision or an
  # identity, not content.
  if ! paste -d'\n' <(header "$scaffold") <(header "$path") | awk '
      NR % 2 == 1 { split($0, a, ": "); key = a[1]; sub(/^[^:]*: */, "", $0); old = $0; okey = key; next }
      { split($0, b, ": "); nkey = b[1]; val = $0; sub(/^[^:]*: */, "", val)
        if (nkey != okey) { printf "header field %s became %s\n", okey, nkey > "/dev/stderr"; bad = 1 }
        else if (val != old && old != "null") { printf "scaffolded value of %s was changed (%s -> %s)\n", okey, old, val > "/dev/stderr"; bad = 1 } }
      END { exit bad }'; then
    echo "draft-record: refusing; a drafting command fills null fields and the body only" >&2
    exit 1
  fi
  [ "$(header "$scaffold" | wc -l)" -eq "$(header "$path" | wc -l)" ] || {
    echo "draft-record: refusing; the header's field count differs from the scaffold" >&2; exit 1; }

  git add -- "$path"
  idx="$("$bin" index --apply)"
  printf '%s\n' "$idx" | sed -n 's/.*"path": *"\([^"]*\)".*/\1/p' | while read -r p; do
    [ -f "$p" ] && git add -- "$p"
  done
  report="$("$bin" check "$path")" && rc=0 || rc=$?
  printf '%s\n' "$report"
  examined="$(printf '%s\n' "$report" | sed -n 's/.*"files_examined": *\([0-9]*\).*/\1/p' | head -1)"
  if [ "${examined:-0}" -lt 1 ]; then
    echo "draft-record: urzua check examined no records; $path is NOT verified" >&2
    exit 1
  fi
  echo "draft-record: wrote $path; urzua check examined $examined record(s)" >&2
  exit "$rc"
  ;;
*)
  echo "usage: draft-record.sh new <type> <title> | finish <record>" >&2
  exit 2
  ;;
esac
