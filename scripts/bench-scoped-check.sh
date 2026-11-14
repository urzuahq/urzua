#!/bin/bash
# Times `urzua check <path>` (a path-scoped run) with one or more urzua binaries, so a
# change to what a scoped run reads can be measured before and after (MILE-207).
#
#   scripts/bench-scoped-check.sh <path> <urzua-binary>...
#
# Run it from the root of the repository to measure. Each binary runs five times and the
# median is printed, with the whole-corpus `check` beside it as the cost a scoped run
# should stay well under on a large repository.
set -euo pipefail

if [ "$#" -lt 2 ]; then
  echo "usage: $0 <path> <urzua-binary>..." >&2
  exit 2
fi
path=$1
shift

median() {
  local bin=$1
  shift
  local times=() i start end
  for i in 1 2 3 4 5; do
    start=$(date +%s.%N)
    "$bin" check "$@" >/dev/null 2>&1 || true
    end=$(date +%s.%N)
    times+=("$(echo "$end - $start" | bc)")
  done
  printf '%s\n' "${times[@]}" | sort -n | sed -n 3p
}

for bin in "$@"; do
  printf '%-40s scoped %6.2fs   whole-corpus %6.2fs\n' \
    "$bin" "$(median "$bin" "$path")" "$(median "$bin")"
done
