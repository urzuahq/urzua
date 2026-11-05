#!/usr/bin/env bash
# Fails, with a stated reason, when the checkout cannot answer what the gate
# asks of it. Drift reads git history, and a shallow checkout makes it unable to
# detect anything while reading as clean, so history depth is
# a precondition of the result. With BASE_REF set, also makes sure the base
# branch is present locally as origin/<BASE_REF>.
set -euo pipefail

fail() { echo "urzua-gate: $1" >&2; exit 1; }

git rev-parse --git-dir >/dev/null 2>&1 || fail "not a git checkout"
if [ "$(git rev-parse --is-shallow-repository)" = "true" ]; then
  fail "the checkout is shallow, so drift cannot see history and would read as clean; use actions/checkout with fetch-depth: 0"
fi

base="${BASE_REF:-}"
if [ -n "$base" ]; then
  git fetch --no-tags --quiet origin "+refs/heads/$base:refs/remotes/origin/$base" \
    || fail "could not fetch the base branch '$base' from origin"
  git rev-parse --verify --quiet "refs/remotes/origin/$base" >/dev/null \
    || fail "base branch origin/$base is not present after fetching"
  if [ "$(git rev-parse --is-shallow-repository)" = "true" ]; then
    fail "fetching the base branch left the checkout shallow; use fetch-depth: 0"
  fi
fi
echo "urzua-gate: full history present${base:+, base origin/$base available}" >&2
