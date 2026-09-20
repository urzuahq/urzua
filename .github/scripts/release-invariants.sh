#!/usr/bin/env bash
# Two facts about a release that nothing else checks.
#
# Both defects this guards against shipped in one evening (BUG-92, and the
# changeset level below), and both had the same single symptom: the release
# PR's title changing quietly. A title changing on an already-open PR is not
# something anyone re-reads, so it went unnoticed twice.
#
# Extracted from the workflow so it can be exercised, matching release-guard.sh.
set -uo pipefail

changeset_dir="${1:-.changeset}"
manifest="${2:-rust/Cargo.toml}"
latest_tag="${3:-}"

fail=0

# 1. A release-prep version bump must not reach main ahead of its tag.
#
# With fragments pending, nothing has been released since the last tag, so the
# manifest must still carry that tag's version. BUG-92 reached main because a
# bugfix branch was cut from `release` and carried its prep commit: the
# manifest said 0.4.0 while the newest tag was v0.3.0, and knope then computed
# every subsequent release from a version that was never published.
#
# Only checked while fragments are pending. `publish-release` merges to main
# before `knope release` tags, so a legitimate window exists where the manifest
# leads the tag -- and that window always has an empty changeset directory.
pending=$(find "$changeset_dir" -maxdepth 1 -name '*.md' 2>/dev/null | wc -l | tr -d ' ')
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$manifest" | head -1)

if [ "$pending" -gt 0 ]; then
  if [ -z "$latest_tag" ]; then
    # "I could not determine the newest tag" is not "the versions agree" --
    # the distinction doctor's ci-wired check got right this same release.
    echo "::error::could not determine the newest tag (shallow checkout, or no tags) -- cannot compare $manifest's version against it"
    fail=1
  elif [ "v$version" != "$latest_tag" ]; then
    echo "::error::$manifest says $version but the newest tag is $latest_tag, with $pending changeset fragment(s) still pending -- a release-prep bump has reached this branch ahead of its release (BUG-92)"
    fail=1
  fi
fi

# 2. A fragment describing a breaking change must declare `major`.
#
# Pre-1.0, knope maps major to a minor bump and minor to a patch bump. So
# `minor` on a breaking change ships it as a patch, and the generated CHANGELOG
# carries no Breaking Changes section -- an adopter upgrading a patch gets a
# config that does not load, with nothing in the release notes saying so.
for fragment in "$changeset_dir"/*.md; do
  [ -e "$fragment" ] || continue
  level=$(sed -n '2s/^default: \(.*\)$/\1/p' "$fragment")
  [ -n "$level" ] || continue
  [ "$level" = "major" ] && continue
  # A *declaration*, not a mention: `**Breaking: ...` or `Breaking: ...` at the
  # start of a line, which is how a breaking fragment is actually written. A
  # bare keyword match fires on prose *about* breaking changes -- it did, on
  # this guard's own changeset, which is the same wrong-in-both-directions
  # failure `claim.status-agreement` documents for its closing verbs.
  if grep -qE '^\*{0,2}Breaking[:*]' "$fragment"; then
    echo "::error::$fragment declares '$level' but its text describes a breaking change -- pre-1.0 that ships as a $( [ "$level" = "minor" ] && echo patch || echo "$level" ) release with no Breaking Changes section"
    fail=1
  fi
done

[ "$fail" -eq 0 ] && echo "::notice::release invariants hold: manifest $version, newest tag ${latest_tag:-none}, $pending fragment(s) pending"
exit "$fail"
