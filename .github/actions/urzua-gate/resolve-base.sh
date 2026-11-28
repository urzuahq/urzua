#!/usr/bin/env bash
# Prints the branch the diff rules compare against: the input when set, else, in
# a merge queue, the queue's base with its `refs/heads/` prefix removed, else empty.
# Expressions have no string replace, so the trim happens here.
# Reads BASE_REF (the input), EVENT_NAME and QUEUE_BASE.
set -euo pipefail
if [ -n "${BASE_REF:-}" ]; then
  printf '%s\n' "$BASE_REF"
elif [ "${EVENT_NAME:-}" = "merge_group" ]; then
  printf '%s\n' "${QUEUE_BASE#refs/heads/}"
else
  printf '\n'
fi
