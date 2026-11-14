#!/usr/bin/env bash
# Planted-violation coverage for release-notes.sh: a short section passes
# through, an oversized one is capped with a link, a missing one fails, and this
# repository's own newest section fits the Releases API limit whole.
set -uo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
script="$root/.github/scripts/release-notes.sh"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0

check() { # desc, then a command that must succeed
  local desc="$1"; shift
  if "$@"; then echo "  ok    $desc"; else echo "  FAIL  $desc"; fail=1; fi
}

cl="$tmp/CHANGELOG.md"
printf '# Changelog\n\n## 0.5.0 (2026-12-01)\n\nShort note.\n\n### Fixes\n\n- one\n\n## 0.4.0 (2026-11-14)\n\nOlder.\n' > "$cl"

short=$("$script" 0.5.0 "$cl" o/r v0.5.0 1000)
check "a short section is the body, to the next version heading" \
  test "$short" = "$(printf 'Short note.\n\n### Fixes\n\n- one')"

check "a version that is a prefix of another is not confused with it" \
  bash -c '"$0" 0.5 "$1" o/r v0.5 1000 >/dev/null 2>&1; test $? -eq 1' "$script" "$cl"

check "a missing section fails" \
  bash -c '"$0" 9.9.9 "$1" o/r v9.9.9 >/dev/null 2>&1; test $? -eq 1' "$script" "$cl"

{ printf '# Changelog\n\n## 1.0.0 (2027-01-02)\n\n'
  for i in $(seq 1 400); do printf 'Paragraph %s of an over-long release note that nobody wrote for a reader.\n\n' "$i"; done
  printf '## 0.9.0 (2026-12-31)\n\nOlder.\n'; } > "$tmp/long.md"
long=$("$script" 1.0.0 "$tmp/long.md" o/r v1.0.0 6000)
check "an oversized section is capped under the limit" \
  test "$(printf '%s' "$long" | wc -c)" -le 6000
check "the capped body keeps the start of the section" \
  bash -c 'printf "%s" "$1" | grep -q "^Paragraph 1 of"' _ "$long"
check "the capped body links to the heading on the tag" \
  bash -c 'printf "%s" "$1" | grep -qF "https://github.com/o/r/blob/v1.0.0/CHANGELOG.md#100-2027-01-02"' _ "$long"
check "the capped body does not run on into the next version" \
  bash -c '! printf "%s" "$1" | grep -q "Older"' _ "$long"

# One paragraph larger than the cap is still cut.
{ printf '## 2.0.0 (2027-02-01)\n\n'; head -c 20000 /dev/zero | tr '\0' 'x'; printf '\n'; } > "$tmp/one.md"
one=$("$script" 2.0.0 "$tmp/one.md" o/r v2.0.0 6000)
check "one oversized paragraph is cut under the limit" \
  test "$(printf '%s' "$one" | wc -c)" -le 6000

# The repository's own newest section must fit whole at the default limit.
newest=$(sed -n 's/^## \([0-9][0-9.]*\) .*/\1/p' "$root/CHANGELOG.md" | head -1)
body=$("$script" "$newest" "$root/CHANGELOG.md" urzuahq/urzua "v$newest")
check "this repository's $newest section fits whole under the API limit" \
  bash -c '! printf "%s" "$1" | grep -q "too long for a GitHub Release"' _ "$body"
check "and under 125000 bytes" test "$(printf '%s' "$body" | wc -c)" -lt 125000

if [ "$fail" -eq 0 ]; then echo "release-notes: all cases passed"; else echo "release-notes: FAILURES"; fi
exit "$fail"
