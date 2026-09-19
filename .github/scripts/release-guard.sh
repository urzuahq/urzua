#!/usr/bin/env bash
# Decides whether knope's `releases::no_release` exit is legitimate.
#
# It is, with no fragments waiting -- the ordinary case on a push that follows
# a release merge. It is not when fragments exist and none of them applied: a
# changeset naming a package `knope.toml` does not declare is silently inert,
# and seven accumulated behind this notice while `prepare release` reported
# success on three consecutive pushes (BUG-48).
#
# Extracted from the workflow so it can be exercised. A guard that has never
# been observed failing is indistinguishable from one that cannot fire.
set -uo pipefail

changeset_dir="${1:-.changeset}"
knope_output="${2:-}"

# 0 = a legitimate no-release, swallow it. 1 = fragments present and inert,
# fail. 2 = not a no-release at all, so this guard has no opinion and the
# caller must not read the absence of a verdict as success.
if ! printf '%s' "$knope_output" | grep -q 'releases::no_release'; then
  exit 2
fi

pending=$(find "$changeset_dir" -maxdepth 1 -name '*.md' 2>/dev/null | wc -l | tr -d ' ')
if [ "$pending" -gt 0 ]; then
  echo "::error::$pending changeset fragment(s) are present but none applied -- check the package key against knope.toml's [package]"
  exit 1
fi

echo "::notice::nothing to release -- no changeset fragment and no releasable commit since the last tag"
exit 0
