#!/usr/bin/env bash
# The knope pin is written once. Three workflows install knope (prepare-release,
# publish-release, checks), and a pin copied into each was checked in only one,
# so a bump in one file left the publish path on a version the compat test never
# blessed. The URL and checksum now live in .github/actions/install-knope, every
# workflow that needs knope uses that action, and this fails when a workflow
# carries a pin of its own or stops using the action.
#
# It checks the repository it sits in, then the same check over scratch copies
# with a planted violation, so a check that always passes fails here.
set -uo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
action=".github/actions/install-knope/action.yml"
users=(.github/workflows/prepare-release.yml .github/workflows/publish-release.yml .github/workflows/checks.yml)

# Prints one line per problem found under $1 and returns 1 when there is one.
verify() {
  local dir="$1" bad=0 f
  [ -f "$dir/$action" ] || { echo "missing $action"; return 1; }
  [ "$(grep -c 'knope-dev/knope/releases/download' "$dir/$action")" -eq 1 ] \
    || { echo "$action must name the knope download exactly once"; bad=1; }
  [ "$(grep -c 'sha256sum -c' "$dir/$action")" -eq 1 ] \
    || { echo "$action must check one sha256"; bad=1; }
  # No other workflow or action file installs knope itself.
  while IFS= read -r f; do
    [ "$f" = "$dir/$action" ] && continue
    echo "${f#"$dir"/} carries its own knope download; use the install-knope action"
    bad=1
  done < <(grep -rl 'knope-dev/knope/releases/download' "$dir/.github/workflows" "$dir/.github/actions" 2>/dev/null)
  for f in "${users[@]}"; do
    grep -q 'uses: ./.github/actions/install-knope' "$dir/$f" 2>/dev/null \
      || { echo "$f does not use the install-knope action"; bad=1; }
  done
  return "$bad"
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0

expect() {
  local want="$1" desc="$2" dir="$3" out got
  out=$(verify "$dir"); got=$?
  if [ "$got" -ne "$want" ]; then
    echo "  FAIL  $desc (want exit $want, got $got)"
    [ -n "$out" ] && printf '          %s\n' "$out"
    fail=1
  else
    echo "  ok    $desc"
  fi
}

plant() { # a scratch copy of the files the check reads
  local dir="$1"
  mkdir -p "$dir/.github/workflows" "$dir/.github/actions/install-knope"
  cp "$root"/.github/workflows/*.yml "$dir/.github/workflows/"
  cp "$root/$action" "$dir/$action"
}

expect 0 "the repository's own workflows" "$root"

copy=$tmp/copy; plant "$copy"
expect 0 "an unmodified copy" "$copy"

dup=$tmp/dup; plant "$dup"
printf '      - run: curl -o k.tgz https://github.com/knope-dev/knope/releases/download/knope/v0.0.1/knope.tgz\n' >> "$dup/.github/workflows/publish-release.yml"
expect 1 "a workflow carrying its own pin" "$dup"

unused=$tmp/unused; plant "$unused"
sed -i 's|uses: ./.github/actions/install-knope|run: true|' "$unused/.github/workflows/checks.yml"
expect 1 "a workflow that stops using the action" "$unused"

twice=$tmp/twice; plant "$twice"
cat >> "$twice/$action" <<'YAML'
        curl -fsSL -o other.tgz "https://github.com/knope-dev/knope/releases/download/knope/v0.0.1/knope.tgz"
YAML
expect 1 "an action that names two downloads" "$twice"

gone=$tmp/gone; plant "$gone"; rm "$gone/$action"
expect 1 "the action file missing" "$gone"

if [ "$fail" -eq 0 ]; then
  echo "knope-pin: all cases passed"
else
  echo "knope-pin: FAILURES"
fi
exit "$fail"
