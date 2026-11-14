#!/usr/bin/env bash
# The GitHub Release body for one version, read from CHANGELOG.md and kept
# under the limit the Releases API enforces (125000 characters; a longer body
# is a 422 and the release is not created -- the failure that left v0.4.0 with
# a tag and no release).
#
# The body is the version's changelog section when it fits. When it does not,
# it is the start of that section, cut at a paragraph boundary, followed by a
# link to the section on the tag. The cap is a safety net: a section meant for
# readers should stay short on its own.
#
# Usage: release-notes.sh VERSION CHANGELOG REPO TAG [LIMIT]
#   VERSION    0.4.0 (no leading v), the `## 0.4.0 (...)` heading to read
#   CHANGELOG  path to CHANGELOG.md
#   REPO       owner/name, for the link
#   TAG        the tag the link points at (v0.4.0)
#   LIMIT      largest body in bytes (default 60000, well under the API limit)
# Prints the body on stdout. Exits 1 when the section is missing or empty.
set -uo pipefail

version="${1:?usage: release-notes.sh VERSION CHANGELOG REPO TAG [LIMIT]}"
changelog="${2:?missing CHANGELOG}"
repo="${3:?missing REPO}"
tag="${4:?missing TAG}"
limit="${5:-60000}"
# How much of an oversized section is kept before the link.
summary_cap=$(( limit / 3 ))

[ -f "$changelog" ] || { echo "::error::$changelog not found" >&2; exit 1; }

heading=$(grep -m1 -E "^## ${version//./\\.}( |\$)" "$changelog" || true)
if [ -z "$heading" ]; then
  echo "::error::$changelog has no '## $version' section" >&2
  exit 1
fi

section=$(awk -v h="$heading" '
  found && /^## / { exit }
  found { print }
  $0 == h { found = 1 }
' "$changelog")
# Trim leading and trailing blank lines.
section=$(printf '%s\n' "$section" | sed -e '/./,$!d' | sed -e ':a' -e '/^\n*$/{$d;N;ba' -e '}')
if [ -z "$section" ]; then
  echo "::error::the '$version' section of $changelog is empty" >&2
  exit 1
fi

if [ "$(printf '%s\n' "$section" | wc -c)" -le "$limit" ]; then
  printf '%s\n' "$section"
  exit 0
fi

# GitHub's heading anchor: lowercase, drop everything but letters, digits,
# spaces, hyphens and underscores, then spaces become hyphens.
anchor=$(printf '%s' "${heading#\#\# }" | tr '[:upper:]' '[:lower:]' | sed -e 's/[^a-z0-9 _-]//g' -e 's/ /-/g')
url="https://github.com/$repo/blob/$tag/CHANGELOG.md#$anchor"

kept=""
size=0
pending=""
while IFS= read -r line || [ -n "$line" ]; do
  if [ -z "$line" ]; then
    # A blank line closes the paragraph gathered so far.
    if [ $(( size + ${#pending} + 1 )) -gt "$summary_cap" ]; then break; fi
    kept+="$pending"$'\n'
    size=$(( size + ${#pending} + 1 ))
    pending=""
  else
    pending+="$line"$'\n'
  fi
done <<< "$section"
# A section whose first paragraph alone is over the cap is cut by characters.
if [ -z "$kept" ]; then
  kept="${pending:0:$summary_cap}"$'\n'
fi

printf '%s\n---\n\nThe full notes for this release are too long for a GitHub Release. Read them in [CHANGELOG.md](%s).\n' "$kept" "$url"
