#!/usr/bin/env bash
# Installs a pinned urzua release and prints the binary path on stdout.
#
# A gate that installs whatever it was handed is not a gate, so this fails,
# with a stated reason, on: an unpinned version, a platform with no published
# build, a checksum mismatch, a failed build-provenance attestation, or a
# binary that does not report the pinned version.
#
# Inputs (environment): VERSION (exact tag, e.g. v0.3.0), REPO (default
# urzuahq/urzua), SHA256 (optional pin of the archive), DEST (install dir),
# TARGET_OS / TARGET_ARCH (default RUNNER_OS / RUNNER_ARCH), RELEASE_BASE_URL
# (default https://github.com; tests point it at a file:// tree), GH_TOKEN.
set -euo pipefail

fail() { echo "urzua-gate install: $1" >&2; exit 1; }

version="${VERSION:-}"
repo="${REPO:-urzuahq/urzua}"
base_url="${RELEASE_BASE_URL:-https://github.com}"
dest="${DEST:-${RUNNER_TEMP:-/tmp}/urzua-gate-bin}"
os="${TARGET_OS:-${RUNNER_OS:-$(uname -s)}}"
arch="${TARGET_ARCH:-${RUNNER_ARCH:-$(uname -m)}}"

[ -n "$version" ] || fail "no version given: pin an exact release tag (for example v0.3.0)"
if ! [[ "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]]; then
  fail "version '$version' is not an exact release tag like v0.3.0; 'latest' and branch names are not pins"
fi

case "$os/$arch" in
  Linux/X64 | Linux/x86_64) target="x86_64-unknown-linux-gnu" ;;
  macOS/ARM64 | Darwin/arm64) target="aarch64-apple-darwin" ;;
  macOS/X64 | Darwin/x86_64) target="x86_64-apple-darwin" ;;
  *) fail "no published urzua build for $os/$arch (published: x86_64 Linux, arm64 and x86_64 macOS)" ;;
esac

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

work="$(mktemp -d)"
archive="$work/urzua-$target.tar.gz"
url="$base_url/$repo/releases/download/$version/urzua-$target.tar.gz"
curl -fsSL -o "$archive" "$url" || fail "could not download $url"

actual="$(sha256_of "$archive")"
if [ -n "${SHA256:-}" ] && [ "$actual" != "$SHA256" ]; then
  fail "checksum mismatch for urzua-$target.tar.gz: expected $SHA256, got $actual"
fi

# The attestation binds this exact digest to the release workflow that built
# it; a swapped archive fails here even when no checksum was pinned.
if ! gh attestation verify "$archive" --repo "$repo" --signer-workflow "$repo/.github/workflows/publish-release.yml" >&2; then
  fail "build provenance attestation failed for urzua-$target.tar.gz ($actual)"
fi

mkdir -p "$dest"
tar -xzf "$archive" -C "$work"
bin="$work/urzua-$target/urzua"
[ -f "$bin" ] || fail "the archive does not contain urzua-$target/urzua"
install -m 0755 "$bin" "$dest/urzua"

reported="$("$dest/urzua" --version 2>&1 || true)"
case "$reported" in
  *"${version#v}"*) ;;
  *) fail "installed binary reports '$reported', not the pinned $version" ;;
esac

echo "urzua-gate install: $version ($target), sha256 $actual, attestation verified" >&2
echo "$dest/urzua"
