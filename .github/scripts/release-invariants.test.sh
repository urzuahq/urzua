#!/usr/bin/env bash
# Planted-violation coverage for release-invariants.sh (AGENTS.md): each case
# asserted in both directions, so a guard that always passes fails this.
set -uo pipefail

guard="$(dirname "$0")/release-invariants.sh"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0

expect() {
  local want="$1" desc="$2"; shift 2
  "$@" >/dev/null 2>&1
  local got=$?
  if [ "$got" -ne "$want" ]; then
    echo "  FAIL  $desc (want exit $want, got $got)"
    fail=1
  else
    echo "  ok    $desc"
  fi
}

manifest() { printf 'version = "%s"\n' "$1" > "$2"; }

# --- 1. manifest version against the newest tag, while fragments pend

fresh=$tmp/matching; mkdir -p "$fresh/.changeset"
printf -- '---\ndefault: patch\n---\n\nA fix.\n' > "$fresh/.changeset/a.md"
manifest "0.3.0" "$fresh/Cargo.toml"
expect 0 "manifest matches the newest tag with fragments pending" \
  "$guard" "$fresh/.changeset" "$fresh/Cargo.toml" "v0.3.0"

leaked=$tmp/leaked; mkdir -p "$leaked/.changeset"
printf -- '---\ndefault: patch\n---\n\nA fix.\n' > "$leaked/.changeset/a.md"
manifest "0.4.0" "$leaked/Cargo.toml"
expect 1 "a release-prep bump ahead of its tag, with fragments pending (BUG-92)" \
  "$guard" "$leaked/.changeset" "$leaked/Cargo.toml" "v0.3.0"

inflight=$tmp/inflight; mkdir -p "$inflight/.changeset"
manifest "0.4.0" "$inflight/Cargo.toml"
expect 0 "the same bump is legitimate once the fragments are consumed" \
  "$guard" "$inflight/.changeset" "$inflight/Cargo.toml" "v0.3.0"

notag=$tmp/notag; mkdir -p "$notag/.changeset"
printf -- '---\ndefault: patch\n---\n\nA fix.\n' > "$notag/.changeset/a.md"
manifest "0.3.0" "$notag/Cargo.toml"
expect 1 "an undeterminable tag is not agreement" \
  "$guard" "$notag/.changeset" "$notag/Cargo.toml" ""

# --- 2. a fragment's declared level against its own prose

major=$tmp/major; mkdir -p "$major/.changeset"
printf -- '---\ndefault: major\n---\n\n**Breaking: the config is now YAML.**\n' > "$major/.changeset/a.md"
manifest "0.3.0" "$major/Cargo.toml"
expect 0 "a breaking change declaring major" \
  "$guard" "$major/.changeset" "$major/Cargo.toml" "v0.3.0"

mislevelled=$tmp/mislevelled; mkdir -p "$mislevelled/.changeset"
printf -- '---\ndefault: minor\n---\n\n**Breaking: the config is now YAML.**\n' > "$mislevelled/.changeset/a.md"
manifest "0.3.0" "$mislevelled/Cargo.toml"
expect 1 "a breaking change declaring minor ships as a patch pre-1.0" \
  "$guard" "$mislevelled/.changeset" "$mislevelled/Cargo.toml" "v0.3.0"

nonbreaking=$tmp/nonbreaking; mkdir -p "$nonbreaking/.changeset"
printf -- '---\ndefault: patch\n---\n\nA fix that mentions nothing alarming.\n' > "$nonbreaking/.changeset/a.md"
manifest "0.3.0" "$nonbreaking/Cargo.toml"
expect 0 "an ordinary patch fragment" \
  "$guard" "$nonbreaking/.changeset" "$nonbreaking/Cargo.toml" "v0.3.0"

# The guard fired on its own changeset, which described breaking changes
# without declaring one. A mention is not a declaration.
mention=$tmp/mention; mkdir -p "$mention/.changeset"
printf -- '---\ndefault: patch\n---\n\nChecks that a changeset describing a breaking change declares major.\n' > "$mention/.changeset/a.md"
manifest "0.3.0" "$mention/Cargo.toml"
expect 0 "prose about breaking changes is not a breaking change" \
  "$guard" "$mention/.changeset" "$mention/Cargo.toml" "v0.3.0"

if [ "$fail" -eq 0 ]; then
  echo "release-invariants: all cases passed"
else
  echo "release-invariants: FAILURES"
fi
exit "$fail"
