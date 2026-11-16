#!/bin/sh
# Installs a urzua release binary after verifying it. POSIX sh.
#
# Nothing is installed, and the downloaded binary is not run, until the
# archive has passed verification: its build provenance attestation (needs an
# authenticated `gh`) and/or a sha256 the caller supplied. With neither
# available this fails rather than install unverified; --no-verify is the
# explicit, loud opt-out.
#
# The final line on stdout is the only stdout output:
#   installed urzua=<tag> target=<target> sha256=<hex> verified=<how> path=<file>
# Everything else goes to stderr. Exit codes: 0 installed, 1 refused or failed,
# 2 bad usage.
set -eu

repo="urzuahq/urzua"
base_url="${URZUA_RELEASE_BASE_URL:-https://github.com}"
install_doc="https://github.com/$repo/blob/main/docs/reference/install.md"

version="${URZUA_VERSION:-}"
dir="${URZUA_INSTALL_DIR:-}"
expected_sha="${URZUA_SHA256:-}"
no_attestation=0
no_verify=0

say() { printf 'urzua-install: %s\n' "$1" >&2; }
fail() { printf 'urzua-install: error: %s\n' "$1" >&2; exit 1; }
usage_error() { printf 'urzua-install: error: %s (see --help)\n' "$1" >&2; exit 2; }

usage() {
  cat <<'EOF'
Usage: install.sh [--version vX.Y.Z] [--dir PATH] [--sha256 HEX] [--no-attestation] [--no-verify] [--help]

Installs the urzua binary for this platform (x86_64 Linux or Apple Silicon macOS).

Flags (a flag overrides its environment variable):
  --version vX.Y.Z   exact release tag (env URZUA_VERSION); default: the latest release
  --dir PATH         install directory, created if missing (env URZUA_INSTALL_DIR); default: $HOME/.local/bin
  --sha256 HEX       expected sha256 of the archive urzua-<target>.tar.gz (env URZUA_SHA256)
  --no-attestation   skip the build provenance check; --sha256 is then required
  --no-verify        install with no verification at all; use only if you accept that
  --help             print this text

Verification, before anything is installed:
  gh installed and logged in   the build provenance attestation must verify
  otherwise                    --sha256 must match, or pass --no-verify
  --sha256 given               it must match, in every mode

Environment: URZUA_VERSION, URZUA_INSTALL_DIR, URZUA_SHA256, and URZUA_RELEASE_BASE_URL
(default https://github.com; must be https, for a mirror).

Last stdout line: installed urzua=<tag> target=<target> sha256=<hex> verified=<how> path=<file>
Exit codes: 0 installed, 1 refused or failed, 2 bad usage.
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --help | -h) usage; exit 0 ;;
    --version)
      [ $# -ge 2 ] && [ -n "$2" ] || usage_error "--version needs a value like v0.4.4 (an empty one would silently mean the latest release)"
      version="$2"; shift 2 ;;
    --dir)
      [ $# -ge 2 ] || usage_error "--dir needs a path"
      dir="$2"; shift 2 ;;
    --sha256)
      [ $# -ge 2 ] || usage_error "--sha256 needs a 64-character hex digest"
      expected_sha="$2"; shift 2 ;;
    --no-attestation) no_attestation=1; shift ;;
    --no-verify) no_verify=1; shift ;;
    *) usage_error "unknown argument '$1'" ;;
  esac
done

if [ -z "$dir" ]; then
  [ -n "${HOME:-}" ] || usage_error "no install directory: HOME is unset; pass --dir PATH"
  dir="$HOME/.local/bin"
fi

case "$base_url" in
  https://*) ;;
  *) fail "URZUA_RELEASE_BASE_URL must start with https:// (got '$base_url')" ;;
esac

if [ -n "$expected_sha" ]; then
  expected_sha="$(printf '%s' "$expected_sha" | tr 'A-F' 'a-f')"
  printf '%s' "$expected_sha" | grep -Eq '^[0-9a-f]{64}$' ||
    usage_error "--sha256 must be 64 hex characters"
fi

is_tag() { printf '%s' "$1" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$'; }

if [ -n "$version" ] && ! is_tag "$version"; then
  usage_error "version '$version' is not an exact release tag like v0.4.4; omit it for the latest release"
fi

os="$(uname -s)"
arch="$(uname -m)"
case "$os/$arch" in
  Linux/x86_64 | Linux/amd64) target="x86_64-unknown-linux-gnu" ;;
  Darwin/arm64 | Darwin/aarch64) target="aarch64-apple-darwin" ;;
  *) fail "no published urzua build for $os/$arch (published: x86_64-unknown-linux-gnu, aarch64-apple-darwin); build from source: $install_doc#build-from-source" ;;
esac

if command -v curl >/dev/null 2>&1; then
  downloader=curl
elif command -v wget >/dev/null 2>&1; then
  downloader=wget
else
  fail "neither curl nor wget is installed; install one, or download the archive by hand: $install_doc"
fi

# curl is held to https for the first request and every redirect. wget has no
# such switch, so with wget a downgrade is caught only by the verification
# below, which is why curl is preferred when both exist.
fetch() { # url, destination file
  if [ "$downloader" = curl ]; then
    curl --fail --silent --show-error --location --retry 2 \
      --proto '=https' --proto-redir '=https' --tlsv1.2 --output "$2" "$1"
  else
    wget --quiet --max-redirect=5 --secure-protocol=TLSv1_2 -O "$2" "$1"
  fi
}

# Reads the Location of the releases/latest redirect, which needs no API call
# or token and is not rate limited like the REST API.
latest_tag() {
  url="$base_url/$repo/releases/latest"
  if [ "$downloader" = curl ]; then
    location="$(curl --silent --show-error --proto '=https' --proto-redir '=https' --tlsv1.2 \
      --output /dev/null --write-out '%{redirect_url}' "$url" || true)"
  else
    location="$(wget --quiet --server-response --max-redirect=0 --secure-protocol=TLSv1_2 \
      -O /dev/null "$url" 2>&1 | sed -n 's/^ *[Ll]ocation: *//p' | tr -d '\r' | head -n 1 || true)"
  fi
  printf '%s' "${location##*/}"
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d' ' -f1
  else
    fail "neither sha256sum nor shasum is installed, so the archive cannot be checked; install coreutils or perl's shasum"
  fi
}

work=""
staged=""
cleanup() {
  [ -z "$staged" ] || rm -f "$staged"
  [ -z "$work" ] || rm -rf "$work"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

work="$(mktemp -d "${TMPDIR:-/tmp}/urzua-install.XXXXXX")" || fail "could not create a temporary directory under ${TMPDIR:-/tmp}"

if [ -z "$version" ]; then
  version="$(latest_tag)"
  is_tag "$version" ||
    fail "could not resolve the latest release from $base_url/$repo/releases/latest; pass --version vX.Y.Z"
fi

archive_name="urzua-$target.tar.gz"
archive="$work/$archive_name"
url="$base_url/$repo/releases/download/$version/$archive_name"
say "downloading $version for $target"
fetch "$url" "$archive" ||
  fail "could not download $url; check that the tag $version exists and that this machine can reach $base_url"

actual_sha="$(sha256_of "$archive")"
verified=""

if [ -n "$expected_sha" ] && [ "$actual_sha" != "$expected_sha" ]; then
  fail "checksum mismatch for $archive_name: expected $expected_sha, got $actual_sha; nothing installed"
fi
[ -z "$expected_sha" ] || verified="sha256"

gh_ready=0
gh_note=""
if [ "$no_attestation" -eq 1 ]; then
  gh_note="--no-attestation was passed"
elif ! command -v gh >/dev/null 2>&1; then
  gh_note="gh is not installed"
elif ! gh auth status >/dev/null 2>&1; then
  gh_note="gh is not logged in (run gh auth login, or set GH_TOKEN)"
else
  gh_ready=1
fi

if [ "$no_verify" -eq 1 ]; then
  say "WARNING: --no-verify: the build provenance of $archive_name is NOT checked${expected_sha:+ (the sha256 you gave matched)}"
  [ -n "$verified" ] || verified="none"
elif [ "$gh_ready" -eq 1 ]; then
  # The attestation binds this exact digest to the release workflow that built
  # it, so a swapped archive fails here even when no checksum was supplied.
  gh attestation verify "$archive" --repo "$repo" \
    --signer-workflow "$repo/.github/workflows/publish-release.yml" >&2 ||
    fail "build provenance attestation failed for $archive_name ($actual_sha); nothing installed"
  if [ -n "$verified" ]; then verified="attestation+sha256"; else verified="attestation"; fi
elif [ -n "$expected_sha" ]; then
  if [ "$no_attestation" -eq 1 ]; then
    say "WARNING: --no-attestation: build provenance is NOT checked; only the sha256 you gave"
  fi
else
  fail "cannot verify $archive_name: $gh_note. Install gh and log in (gh auth login), or pass the expected checksum with --sha256 <hex> (or URZUA_SHA256), or pass --no-verify to install without verification; nothing installed"
fi

mkdir -p "$dir" || fail "could not create the install directory $dir"
dir="$(cd "$dir" && pwd)"

mkdir "$work/x"
tar -xzf "$archive" -C "$work/x" "urzua-$target/urzua" 2>/dev/null ||
  fail "the archive does not contain urzua-$target/urzua; nothing installed"
bin="$work/x/urzua-$target/urzua"
{ [ -f "$bin" ] && [ ! -L "$bin" ]; } || fail "urzua-$target/urzua in the archive is not a regular file; nothing installed"

# Staged beside the destination so the final rename is atomic, and checked
# before it replaces anything: a binary that reports another version never
# lands as `urzua`.
staged="$dir/.urzua.$$.new"
cp "$bin" "$staged" && chmod 0755 "$staged" || fail "could not write to $dir; pass --dir with a writable directory"

want="urzua ${version#v}"
reported="$("$staged" --version 2>&1 || true)"
if [ "$reported" != "$want" ]; then
  fail "the binary reports '$reported', expected exactly '$want'; nothing installed (the published Linux build needs glibc, so a binary that does not run may mean a musl system)"
fi

mv -f "$staged" "$dir/urzua"
staged=""

case ":${PATH:-}:" in
  *":$dir:"*) ;;
  *) say "note: $dir is not on PATH; add it with: export PATH=\"$dir:\$PATH\"" ;;
esac

printf 'installed urzua=%s target=%s sha256=%s verified=%s path=%s\n' \
  "$version" "$target" "$actual_sha" "$verified" "$dir/urzua"
