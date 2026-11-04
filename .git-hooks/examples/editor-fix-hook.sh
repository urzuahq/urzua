#!/usr/bin/env bash
# Example editor hook, not installed by anything (`make hooks-install` does not
# touch it). Judges the content an editor is about to save, before it is saved:
#
#   editor-fix-hook.sh docs/adr/ADR-9-new.md < proposed-content.md
#
# `urzua fix --stdin-path <path>` reads the proposed content for that one
# record from stdin, computes drift for that record only, and writes nothing.
# The path need not exist yet. A run takes a fraction of a whole-corpus
# `urzua fix` (SPEC-8, "Judging proposed content").
#
# Prints one line per pending repair, `path:line: field: stated -> computed`
# (an editor can parse that as a location), and exits 1 when there is one.
set -eu

path="${1:?usage: editor-fix-hook.sh <record-path> < proposed-content}"
root="$(git rev-parse --show-toplevel)"
bin="$root/rust/target/release/urzua"
[ -x "$bin" ] || { echo "editor-fix-hook: $bin not built -- run 'make rust-build'" >&2; exit 0; }

cd "$root"
# Exit 1 is "a repair is pending"; 2 means urzua could not run and said why.
set +e
report="$("$bin" fix --stdin-path "$path")"
code=$?
set -e
if [ "$code" -eq 2 ]; then
  echo "$report" | jq -r '"editor-fix-hook: " + (.error // .message // tostring)' >&2
  exit 0
fi
echo "$report" | jq -r '.repairs[] | "\(.record):\(.line): \(.field): \(.current_value) -> \(.computed_value)"'
[ "$(echo "$report" | jq '.repairs | length')" -eq 0 ]
