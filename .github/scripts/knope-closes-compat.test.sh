#!/usr/bin/env bash
# The release tool tolerates the structured claim field (the one-line `closes`
# key of a changeset fragment). The pinned knope is run over a scratch
# repository carrying this repository's knope.toml, and a fragment with the key
# must give the same version bump and changelog as the same fragment without it.
# A block list and a list continued over two lines must stay rejected: the
# single-line form is the form claim.field-malformed demands because of it.
#
# The pin is read from the install-knope action, the one place it is written
# (knope-pin.test.sh fails when a workflow carries its own copy), so a knope
# upgrade re-runs this against the new version (the version installed here must
# equal the pin).
# Without knope on PATH it fails in CI and says SKIPPED anywhere else.
set -uo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
pin=$(sed -n 's|.*knope-dev/knope/releases/download/knope/v\([0-9][0-9.]*\)/.*|\1|p' \
  "$root/.github/actions/install-knope/action.yml" | head -1)
if [ -z "$pin" ]; then
  echo "knope-closes-compat: no pinned knope version found in the install-knope action" >&2
  exit 1
fi

if ! command -v knope >/dev/null 2>&1; then
  if [ -n "${CI:-}" ]; then
    echo "knope-closes-compat: knope $pin is not installed; CI must install the pinned release" >&2
    exit 1
  fi
  echo "knope-closes-compat: SKIPPED -- knope is not installed (CI installs the pinned $pin and runs this)"
  exit 0
fi
have=$(knope --version | awk '{print $2}')
if [ "$have" != "$pin" ]; then
  echo "knope-closes-compat: installed knope is $have but the install-knope action pins $pin" >&2
  exit 1
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp" || exit 1
git init -q
git config user.email "test@example.com"
git config user.name "test"
cp "$root/knope.toml" .
mkdir -p rust .changeset
printf '[workspace.package]\nversion = "0.3.0"\n' > rust/Cargo.toml
printf '# Changelog\n' > CHANGELOG.md
git add -A
git commit -qm "scratch"
git tag v0.3.0

fail=0

# Prints knope's dry run for a fragment whose frontmatter is $1 (lines after
# the bump key), and the exit code on the last line.
dry_run() {
  rm -f .changeset/*.md
  printf -- '---\ndefault: minor\n%s---\n\nAdd a thing.\n' "$1" > .changeset/a.md
  out=$(knope prepare-release --dry-run 2>&1)
  rc=$?
  printf '%s\n' "$out"
  return "$rc"
}

baseline=$(dry_run "")
if [ $? -ne 0 ]; then
  echo "  FAIL  the baseline fragment is not accepted:"
  printf '%s\n' "$baseline"
  exit 1
fi
if ! printf '%s' "$baseline" | grep -q 'Add a thing'; then
  echo "  FAIL  the baseline dry run does not show the fragment in the changelog"
  exit 1
fi

accepts() {
  local desc="$1" extra="$2" got
  got=$(dry_run "$extra")
  if [ $? -ne 0 ]; then
    echo "  FAIL  $desc: rejected"
    printf '%s\n' "$got" | sed 's/^/        /'
    fail=1
  elif [ "$got" != "$baseline" ]; then
    echo "  FAIL  $desc: the bump or changelog differs from the fragment without the key"
    fail=1
  else
    echo "  ok    $desc"
  fi
}

rejects() {
  local desc="$1" extra="$2" got
  got=$(dry_run "$extra")
  if [ $? -eq 0 ]; then
    echo "  FAIL  $desc: accepted, but the single-line form is required because it is rejected"
    fail=1
  elif ! printf '%s' "$got" | grep -q 'could_not_read_changeset'; then
    echo "  FAIL  $desc: rejected, but not as an unreadable changeset"
    printf '%s\n' "$got" | sed 's/^/        /'
    fail=1
  else
    echo "  ok    $desc"
  fi
}

accepts "closes: [BUG-60, BUG-61]" $'closes: [BUG-60, BUG-61]\n'
accepts "closes: BUG-62" $'closes: BUG-62\n'
accepts "closes: \"BUG-60, BUG-61\"" $'closes: "BUG-60, BUG-61"\n'
accepts "closes: []" $'closes: []\n'
rejects "a block list" $'closes:\n  - BUG-62\n'
rejects "a list continued over two lines" $'closes: [BUG-60,\n  BUG-61]\n'

[ "$fail" -eq 0 ] && echo "knope-closes-compat: all cases passed on knope $have"
exit "$fail"
