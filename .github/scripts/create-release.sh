#!/usr/bin/env bash
# Creates the GitHub Release for an existing tag, with a body that fits the
# Releases API limit, unless the release already exists. Shared by the normal
# publish path and the recovery path in publish-release.yml so both write the
# same release.
#
# It never creates, moves or deletes a tag: the tag must already exist on the
# remote, and `gh release create --verify-tag` refuses otherwise.
#
# Usage: create-release.sh VERSION TAG CHANGELOG REPO LINK_REF
#   CHANGELOG  the file whose `## VERSION` section is the release notes
#   LINK_REF   the ref the "full notes" link points at when the section is cut
# Needs GH_TOKEN. Exits 1 when the release cannot be created.
set -euo pipefail

version="${1:?usage: create-release.sh VERSION TAG CHANGELOG REPO LINK_REF}"
tag="${2:?missing TAG}"
changelog="${3:?missing CHANGELOG}"
repo="${4:?missing REPO}"
link_ref="${5:?missing LINK_REF}"
here="$(cd "$(dirname "$0")" && pwd)"

if gh release view "$tag" --repo "$repo" >/dev/null 2>&1; then
  echo "release $tag already exists; leaving it as it is"
  exit 0
fi

notes=$(mktemp)
trap 'rm -f "$notes"' EXIT
"$here/release-notes.sh" "$version" "$changelog" "$repo" "$link_ref" > "$notes"
title=$(grep -m1 -E "^## ${version//./\\.}( |\$)" "$changelog" | sed 's/^## //')

echo "creating release $tag ($(wc -c < "$notes") byte body)"
gh release create "$tag" --repo "$repo" --verify-tag --title "$title" --notes-file "$notes"
