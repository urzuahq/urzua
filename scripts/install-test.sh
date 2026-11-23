#!/bin/sh
# Tests for scripts/install.sh with no network: `curl`, `wget`, `uname` and
# `gh` are stubs on PATH, and the "release" is a directory of archives built
# here. Every scenario runs once under each of bash and dash that exists.
set -u

here="$(cd "$(dirname "$0")" && pwd)"
installer="$here/install.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
pass() { echo "ok   $1"; }
flunk() { echo "FAIL $1" >&2; fails=$((fails + 1)); }
expect() { # name, condition exit status
  if [ "$2" -eq 0 ]; then pass "$1"; else flunk "$1"; fi
}
has() { grep -q -- "$2" "$1"; }

# ---- a PATH with only what the installer needs, plus stubs ----
base="$tmp/bin-base" with_curl="$tmp/bin-curl" with_gh="$tmp/bin-gh" with_wget="$tmp/bin-wget"
mkdir -p "$base" "$with_curl" "$with_gh" "$with_wget"
for tool in mktemp tar gzip sed grep tr cut head rm mv cp chmod mkdir cat sha256sum dirname; do
  src="$(command -v "$tool")" || { echo "install-test: needs $tool on PATH" >&2; exit 1; }
  ln -s "$src" "$base/$tool"
done

cat >"$base/uname" <<'STUB'
#!/bin/sh
case "$1" in -s) echo "$FAKE_OS" ;; -m) echo "$FAKE_ARCH" ;; esac
STUB

# Serves $FAKE_REL/<tag>/<file>; refuses anything that is not https or that
# was not asked to keep every redirect on https.
cat >"$with_curl/curl" <<'STUB'
#!/bin/sh
echo "curl $*" >>"$FAKE_LOG"
args="$*"
case "$args" in *"--proto =https"*"--proto-redir =https"*) ;; *) echo "curl stub: https was not enforced" >&2; exit 99 ;; esac
out="" write="" url=""
while [ $# -gt 0 ]; do
  case "$1" in
    --output) out="$2"; shift ;;
    --write-out) write="$2"; shift ;;
    --proto | --proto-redir | --tlsv1.2 | --retry) [ "$1" = --tlsv1.2 ] || shift ;;
    -*) ;;
    *) url="$1" ;;
  esac
  shift
done
case "$url" in https://*) ;; *) echo "curl stub: not https: $url" >&2; exit 99 ;; esac
case "$url" in
  */releases/latest) printf '%s' "${url%/releases/latest}/releases/tag/$FAKE_LATEST" ;;
  */releases/download/*)
    rest="${url##*/releases/download/}"
    [ -f "$FAKE_REL/$rest" ] || exit 22
    cp "$FAKE_REL/$rest" "$out" ;;
  *) exit 22 ;;
esac
STUB

cat >"$with_wget/wget" <<'STUB'
#!/bin/sh
echo "wget $*" >>"$FAKE_LOG"
out="" url=""
while [ $# -gt 0 ]; do
  case "$1" in -O) out="$2"; shift ;; -*) ;; *) url="$1" ;; esac
  shift
done
rest="${url##*/releases/download/}"
[ -f "$FAKE_REL/$rest" ] || exit 8
cp "$FAKE_REL/$rest" "$out"
STUB

# `attestation verify` logs whether the downloaded binary had been run yet.
cat >"$with_gh/gh" <<'STUB'
#!/bin/sh
echo "gh $*" >>"$FAKE_LOG"
case "$1" in
  auth) [ "${FAKE_GH_AUTH:-ok}" = ok ]; exit $? ;;
  attestation) [ -e "$FAKE_RAN" ] && echo "binary-ran-before-verify" >>"$FAKE_LOG"; [ "${FAKE_ATTEST:-ok}" = ok ]; exit $? ;;
esac
STUB
chmod +x "$base"/uname "$with_curl"/curl "$with_wget"/wget "$with_gh"/gh

# ---- a fake release ----
export FAKE_REL="$tmp/rel" FAKE_LOG="$tmp/calls.log" FAKE_RAN="$tmp/binary-ran"
export FAKE_LATEST=v1.2.3 FAKE_OS=Linux FAKE_ARCH=x86_64
mk_release() { # tag, target, version the binary reports
  d="$tmp/build/$2"
  rm -rf "$d"
  mkdir -p "$d/urzua-$2" "$FAKE_REL/$1"
  printf '#!/bin/sh\necho ran >>"%s"\necho "urzua %s"\n' "$FAKE_RAN" "$3" >"$d/urzua-$2/urzua"
  chmod +x "$d/urzua-$2/urzua"
  tar -czf "$FAKE_REL/$1/urzua-$2.tar.gz" -C "$d" "urzua-$2"
  sum="$(release_sha "$1" $LINUX 2>/dev/null)"
  mac_sum="$(release_sha "$1" $MAC 2>/dev/null)"
}
LINUX=x86_64-unknown-linux-gnu
MAC=aarch64-apple-darwin
release_sha() { sha256sum "$FAKE_REL/$1/urzua-$2.tar.gz" | cut -d' ' -f1; }

mk_release v1.2.3 $MAC 1.2.3
mk_release v1.2.3 $LINUX 1.2.3
wrong="0000000000000000000000000000000000000000000000000000000000000000"

# ---- helpers ----
mkdir -p "$tmp/tmpd" "$tmp/home"
run() { # args for install.sh; uses $sh_bin and $PATH_MODE; env passes through
  : >"$FAKE_LOG"; rm -f "$FAKE_RAN"
  case "$PATH_MODE" in
    gh) p="$base:$with_curl:$with_gh" ;;
    nogh) p="$base:$with_curl" ;;
    wget) p="$base:$with_wget:$with_gh" ;;
  esac
  PATH="$p${EXTRA_PATH:+:$EXTRA_PATH}" HOME="${RUN_HOME-$tmp/home}" TMPDIR="$tmp/tmpd" "$sh_bin" "$installer" "$@" >"$tmp/out" 2>"$tmp/err"
  rc=$?
}
clean_tmp() { [ -z "$(ls -A "$tmp/tmpd")" ]; }
dest="$tmp/dest"
fresh() { rm -rf "$dest"; }
installed() { [ -x "$dest/urzua" ]; }
nothing_installed() { [ ! -e "$dest/urzua" ] && { [ ! -d "$dest" ] || [ -z "$(ls -A "$dest")" ]; }; }

lists_all() {
  for w in --version --dir --sha256 --no-attestation --no-verify URZUA_VERSION URZUA_INSTALL_DIR URZUA_SHA256; do
    has "$tmp/out" "$w" || return 1
  done
}

suite() {
  PATH_MODE=gh

  fresh
  run --version v1.2.3 --dir "$dest/nested/bin"
  [ "$rc" -eq 0 ] && [ -x "$dest/nested/bin/urzua" ]
  expect "$label install: attested release installs and the missing directory is created" $?
  [ "$(wc -l <"$tmp/out")" -eq 1 ] &&
    [ "$(cat "$tmp/out")" = "installed urzua=v1.2.3 target=$LINUX sha256=$sum verified=attestation path=$dest/nested/bin/urzua" ]
  expect "$label install: stdout is exactly one parseable final line" $?
  [ "$(ls -ld "$dest/nested/bin/urzua" | cut -c1-10)" = "-rwxr-xr-x" ]
  expect "$label install: the binary is mode 0755" $?
  has "$FAKE_LOG" "--signer-workflow urzuahq/urzua/.github/workflows/publish-release.yml" &&
    has "$FAKE_LOG" "attestation verify .*urzua-$LINUX.tar.gz --repo urzuahq/urzua"
  expect "$label install: the attestation is pinned to the repository and release workflow" $?
  ! has "$FAKE_LOG" binary-ran-before-verify && [ -e "$FAKE_RAN" ]
  expect "$label install: the binary runs only after verification" $?
  has "$FAKE_LOG" "--proto =https --proto-redir =https"
  expect "$label install: curl is held to https, redirects included" $?
  clean_tmp
  expect "$label install: no temporary directory is left behind on success" $?
  [ -z "$(ls -A "$dest/nested/bin" | grep -v '^urzua$')" ]
  expect "$label install: no staging file is left in the install directory" $?

  fresh; FAKE_OS=Darwin FAKE_ARCH=arm64 run --version v1.2.3 --dir "$dest"
  [ "$rc" -eq 0 ] && has "$tmp/out" "target=$MAC sha256=$mac_sum"
  expect "$label install: Apple Silicon maps to aarch64-apple-darwin" $?

  fresh; run --dir "$dest"
  [ "$rc" -eq 0 ] && has "$tmp/out" "urzua=v1.2.3 " && has "$FAKE_LOG" "releases/latest"
  expect "$label install: no version resolves the latest release through the redirect, with no gh API call" $?
  ! has "$FAKE_LOG" "gh api"
  expect "$label install: resolving the latest release does not need gh" $?

  fresh; URZUA_VERSION=v1.2.3 URZUA_INSTALL_DIR="$dest" run
  [ "$rc" -eq 0 ] && installed
  expect "$label install: URZUA_VERSION and URZUA_INSTALL_DIR are honoured" $?

  fresh; run --version v1.2.3
  [ "$rc" -eq 0 ] && [ -x "$tmp/home/.local/bin/urzua" ]
  expect "$label install: the default directory is \$HOME/.local/bin" $?
  rm -rf "$tmp/home/.local"

  fresh; RUN_HOME= run --version v1.2.3
  [ "$rc" -eq 2 ] && has "$tmp/err" "HOME is unset" && clean_tmp
  expect "$label install: no HOME and no --dir is a usage error naming --dir" $?

  # ---- platforms ----
  for combo in "Linux aarch64" "Darwin x86_64" "FreeBSD amd64"; do
    fresh; FAKE_OS="${combo% *}" FAKE_ARCH="${combo#* }" run --version v1.2.3 --dir "$dest"
    [ "$rc" -ne 0 ] && has "$tmp/err" "no published urzua build for ${combo% *}/${combo#* }" &&
      has "$tmp/err" "x86_64-unknown-linux-gnu, aarch64-apple-darwin" && has "$tmp/err" "build from source" &&
      [ ! -d "$dest" ] && clean_tmp
    expect "$label platform: $combo is refused naming the published targets and building from source" $?
  done

  # ---- verification ----
  fresh; mkdir -p "$dest"; printf 'old' >"$dest/urzua"
  mk_release v1.2.3 $LINUX 1.2.3; good="$FAKE_REL/v1.2.3/urzua-$LINUX.tar.gz"
  run --version v1.2.3 --dir "$dest" --sha256 "$wrong"
  [ "$rc" -eq 1 ] && has "$tmp/err" "checksum mismatch" && [ "$(cat "$dest/urzua")" = old ] && [ ! -e "$FAKE_RAN" ] && clean_tmp
  expect "$label verify: a checksum mismatch is refused, the existing binary is untouched and nothing ran" $?
  ! has "$FAKE_LOG" "attestation verify"
  expect "$label verify: a checksum mismatch stops before the attestation call" $?

  # A tampered archive: the pinned checksum is of the genuine one.
  fresh
  tampered="$tmp/build/tampered"; rm -rf "$tampered"; mkdir -p "$tampered/urzua-$LINUX"
  printf '#!/bin/sh\necho ran >>"%s"\necho "urzua 1.2.3"\n# tampered\n' "$FAKE_RAN" >"$tampered/urzua-$LINUX/urzua"
  chmod +x "$tampered/urzua-$LINUX/urzua"
  tar -czf "$tmp/tampered.tar.gz" -C "$tampered" "urzua-$LINUX"
  cp "$good" "$tmp/genuine.tar.gz"; cp "$tmp/tampered.tar.gz" "$good"
  run --version v1.2.3 --dir "$dest" --sha256 "$sum"
  [ "$rc" -eq 1 ] && has "$tmp/err" "checksum mismatch" && nothing_installed && [ ! -e "$FAKE_RAN" ] && clean_tmp
  expect "$label verify: a tampered archive is refused and nothing is installed" $?
  FAKE_ATTEST=fail run --version v1.2.3 --dir "$dest"
  [ "$rc" -eq 1 ] && has "$tmp/err" "attestation failed" && nothing_installed && [ ! -e "$FAKE_RAN" ] && clean_tmp
  expect "$label verify: a tampered archive that fails its attestation is refused and nothing is installed" $?
  cp "$tmp/genuine.tar.gz" "$good"

  fresh; FAKE_ATTEST=fail run --version v1.2.3 --dir "$dest"
  [ "$rc" -eq 1 ] && has "$tmp/err" "build provenance attestation failed" && nothing_installed && [ ! -e "$FAKE_RAN" ] && clean_tmp
  expect "$label verify: a failed attestation is refused, nothing installed, nothing ran, no temp left" $?

  fresh; run --version v1.2.3 --dir "$dest" --sha256 "$(echo "$sum" | tr a-f A-F)"
  [ "$rc" -eq 0 ] && has "$tmp/out" "verified=attestation+sha256"
  expect "$label verify: attestation and a matching checksum are both reported (checksum case-insensitive)" $?

  fresh; FAKE_GH_AUTH=fail run --version v1.2.3 --dir "$dest"
  [ "$rc" -eq 1 ] && has "$tmp/err" "gh is not logged in" && has "$tmp/err" "--sha256" && nothing_installed && clean_tmp
  expect "$label verify: gh installed but not logged in is the same refusal as no gh" $?

  fresh; run --version v1.2.3 --dir "$dest" --no-attestation
  [ "$rc" -eq 1 ] && has "$tmp/err" "--no-attestation was passed" && nothing_installed
  expect "$label verify: --no-attestation without a checksum is refused" $?
  run --version v1.2.3 --dir "$dest" --no-attestation --sha256 "$sum"
  [ "$rc" -eq 0 ] && has "$tmp/err" "WARNING: --no-attestation" && has "$tmp/out" "verified=sha256" && ! has "$FAKE_LOG" "attestation verify"
  expect "$label verify: --no-attestation with a checksum installs, says so loudly and skips the attestation" $?

  # ---- no gh ----
  PATH_MODE=nogh
  fresh; run --version v1.2.3 --dir "$dest"
  [ "$rc" -eq 1 ] && has "$tmp/err" "gh is not installed" && has "$tmp/err" "--sha256" && has "$tmp/err" "--no-verify" &&
    nothing_installed && [ ! -e "$FAKE_RAN" ] && clean_tmp
  expect "$label no-gh: without gh and without a checksum it fails and says what to do" $?
  run --version v1.2.3 --dir "$dest" --sha256 "$wrong"
  [ "$rc" -eq 1 ] && has "$tmp/err" "checksum mismatch" && nothing_installed && clean_tmp
  expect "$label no-gh: a wrong checksum is refused" $?
  run --version v1.2.3 --dir "$dest" --sha256 "$sum"
  [ "$rc" -eq 0 ] && has "$tmp/out" "verified=sha256 " && installed && clean_tmp
  expect "$label no-gh: an explicit checksum installs" $?
  fresh; URZUA_SHA256="$sum" run --version v1.2.3 --dir "$dest"
  [ "$rc" -eq 0 ] && installed
  expect "$label no-gh: URZUA_SHA256 is the same as --sha256" $?
  fresh; run --version v1.2.3 --dir "$dest" --no-verify
  [ "$rc" -eq 0 ] && has "$tmp/err" "WARNING: --no-verify" && has "$tmp/out" "verified=none" && installed && clean_tmp
  expect "$label no-gh: --no-verify installs, warning loudly, and reports verified=none" $?

  # ---- the reported version ----
  PATH_MODE=gh
  for reported in 11.2.35 1.2.3-rc.1 1.2.30 9.9.9; do
    fresh; mk_release v1.2.3 $LINUX "$reported"
    run --version v1.2.3 --dir "$dest"
    [ "$rc" -eq 1 ] && has "$tmp/err" "expected exactly 'urzua 1.2.3'" && nothing_installed && clean_tmp
    expect "$label version: a binary reporting $reported instead of 1.2.3 is refused and nothing is installed" $?
  done
  fresh; mkdir -p "$dest"; printf 'old' >"$dest/urzua"
  mk_release v1.2.3 $LINUX 11.2.35
  run --version v1.2.3 --dir "$dest"
  [ "$rc" -eq 1 ] && [ "$(cat "$dest/urzua")" = old ] && [ "$(ls -A "$dest")" = urzua ]
  expect "$label version: a refused version leaves an existing install and no staging file" $?
  mk_release v1.2.3 $LINUX 1.2.3

  # ---- usage ----
  fresh
  for bad in latest 1.2.3 v1.2 "v1.2.3;rm" ""; do
    run --version "$bad" --dir "$dest"
    [ "$rc" -ne 0 ] && nothing_installed
    expect "$label usage: --version '$bad' is not an exact tag and is refused" $?
  done
  run --bogus
  [ "$rc" -eq 2 ] && has "$tmp/err" "unknown argument"
  expect "$label usage: an unknown flag exits 2" $?
  run --sha256 abc --dir "$dest"
  [ "$rc" -eq 2 ] && has "$tmp/err" "64 hex"
  expect "$label usage: a malformed checksum exits 2" $?
  URZUA_RELEASE_BASE_URL=http://example.test run --version v1.2.3 --dir "$dest"
  [ "$rc" -eq 1 ] && has "$tmp/err" "must start with https://" && [ ! -s "$FAKE_LOG" ]
  expect "$label usage: a non-https release base URL is refused before any request" $?
  run --version v9.9.9 --dir "$dest"
  [ "$rc" -eq 1 ] && has "$tmp/err" "could not download" && has "$tmp/err" "v9.9.9" && clean_tmp
  expect "$label usage: a tag with no release fails naming the cause, no temp left" $?
  run --help
  [ "$rc" -eq 0 ] && lists_all
  expect "$label usage: --help lists every flag and environment variable" $?

  # ---- PATH note ----
  fresh; run --version v1.2.3 --dir "$dest"
  has "$tmp/err" "$dest is not on PATH"
  expect "$label path: a directory off PATH is noted on stderr" $?
  fresh; EXTRA_PATH="$dest" run --version v1.2.3 --dir "$dest"
  ! has "$tmp/err" "not on PATH"
  expect "$label path: no note when the directory is on PATH" $?

  # ---- wget ----
  PATH_MODE=wget
  fresh; run --version v1.2.3 --dir "$dest"
  [ "$rc" -eq 0 ] && has "$FAKE_LOG" "^wget " && ! has "$FAKE_LOG" "^curl "
  expect "$label download: wget is used when curl is absent" $?
  PATH_MODE=gh
}

ran=0
for sh in bash dash; do
  sh_bin="$(command -v "$sh")" || continue
  ran=$((ran + 1))
  label="[$(basename "$sh_bin")]"
  suite
done
[ "$ran" -gt 0 ] || flunk "no bash or dash to run the installer under"

# A piped install exits 0 with nothing installed when the download fails (sh reads empty input).
repo="$(cd "$here/.." && pwd)"
piped="$(grep -rnE '\|[[:space:]]*(sudo([[:space:]]+-[^[:space:]]+)*[[:space:]]+)?([^[:space:]|]*/)?(ba|da|z)?sh([[:space:]]|$)' "$repo/README.md" "$repo/docs/reference" "$repo/scripts/install.sh" "$repo/.github/actions" 2>/dev/null)"
[ -z "$piped" ]
expect "docs: no documented command pipes a download into a shell" $?
[ -z "$piped" ] || printf '%s\n' "$piped" >&2

command -v dash >/dev/null 2>&1 || echo "note dash not found; ran under bash only"

[ "$fails" -eq 0 ] && echo "install.sh: all tests passed" || { echo "install.sh: $fails test(s) failed" >&2; exit 1; }
