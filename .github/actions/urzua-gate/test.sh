#!/usr/bin/env bash
# Recorded-fixture tests for the urzua-gate action's scripts: rendering,
# update-in-place commenting, the history precondition and the installer's
# refusals. No network: the installer reads a file:// release tree and a fake
# `gh` stands in for the GitHub API and attestation verifier.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
pass() { echo "ok   $1"; }
flunk() { echo "FAIL $1" >&2; fails=$((fails + 1)); }
expect() { # name, condition exit status
  if [ "$2" -eq 0 ]; then pass "$1"; else flunk "$1"; fi
}

# A fake gh: logs every call; `api` answers from FAKE_COMMENTS and can fail;
# `attestation verify` fails when FAKE_ATTEST=fail.
mkdir -p "$tmp/fakebin"
cat >"$tmp/fakebin/gh" <<'GH'
#!/usr/bin/env bash
echo "$*" >>"$FAKE_GH_LOG"
case "$1" in
  attestation) [ "${FAKE_ATTEST:-ok}" = "ok" ]; exit $? ;;
  api)
    [ "${FAKE_API:-ok}" = "fail" ] && { echo "HTTP 403" >&2; exit 1; }
    case "$*" in
      *"/pulls/"*"/reviews"*) cat "$FAKE_REVIEWS"; exit 0 ;;
      *"/pulls/"*) cat "$FAKE_PULL"; exit 0 ;;
      *"-X PATCH"* | *"-X POST"*) cat "$(echo "$*" | sed -n 's/.*body=@\([^ ]*\).*/\1/p')" >>"$FAKE_GH_LOG"; exit 0 ;;
      *) printf '%s' "${FAKE_COMMENT_ID:-}"; [ -n "${FAKE_COMMENT_ID:-}" ] && echo; exit 0 ;;
    esac ;;
esac
GH
chmod +x "$tmp/fakebin/gh"
export PATH="$tmp/fakebin:$PATH"
export FAKE_GH_LOG="$tmp/gh.log"

# ---- render ----
"$here/render.sh" "$tmp/c.md" check "$here/fixtures/whole-findings.json" 1 \
  base "$here/fixtures/base-findings.json" 1 >"$tmp/ann.txt"
diff -u "$here/fixtures/expected-annotations.txt" "$tmp/ann.txt"
expect "render: annotations match the recorded fixture (deduped, encoded)" $?
diff -u "$here/fixtures/expected-comment.md" "$tmp/c.md"
expect "render: summary comment matches the recorded fixture" $?

"$here/render.sh" "$tmp/c.md" check "$here/fixtures/clean.json" 0 >"$tmp/ann.txt"
{ [ ! -s "$tmp/ann.txt" ] && grep -q '^Clean: no findings\.' "$tmp/c.md"; }
expect "render: a clean report renders clean with no annotations" $?

"$here/render.sh" "$tmp/c.md" check "$here/fixtures/not-run.json" 2 >"$tmp/ann.txt"
{ grep -q 'NOT RUN' "$tmp/c.md" && grep -q 'config.yaml' "$tmp/c.md" && ! grep -q '^Clean' "$tmp/c.md"; }
expect "render: a run that did not run is NOT RUN with its reason, never clean" $?

"$here/render.sh" "$tmp/c.md" check "$tmp/missing.json" 1 >"$tmp/ann.txt"
grep -q 'NOT RUN' "$tmp/c.md"
expect "render: a missing report is NOT RUN, not clean" $?

# A report over the 128 KB single-argument limit: generated here, mostly waived
# findings (which a report lists in full), plus two live ones.
jq -c '.findings = ([range(0; 700) | {rule: "example.rule", severity: "warning", file: "docs/example/\(.).md", line: 1, message: ("a waived finding with a long message " + ("x" * 150)), waived: "docs/waiver/W-1.md"}] + [.findings[0:2][]])' \
  "$here/fixtures/whole-findings.json" >"$tmp/large.json"
[ "$(wc -c <"$tmp/large.json")" -gt 200000 ]
expect "render: the large-report fixture is over 200 KB" $?
"$here/render.sh" "$tmp/c.md" check "$tmp/large.json" 1 >"$tmp/ann.txt"
{ ! grep -q 'NOT RUN' "$tmp/c.md" && grep -q '701 waived' "$tmp/c.md" && grep -q '^::' "$tmp/ann.txt"; }
expect "render: a report over 200 KB renders its findings, not NOT RUN" $?

# ---- post-comment ----
export REPO=o/r PR_NUMBER=7 GH_TOKEN=x
"$here/render.sh" "$tmp/findings.md" check "$here/fixtures/whole-findings.json" 1 >/dev/null
"$here/render.sh" "$tmp/clean.md" check "$here/fixtures/clean.json" 0 >/dev/null

: >"$FAKE_GH_LOG"; FAKE_COMMENT_ID= "$here/post-comment.sh" "$tmp/findings.md" >/dev/null
grep -q -- '-X POST repos/o/r/issues/7/comments' "$FAKE_GH_LOG"
expect "comment: first run posts a new comment" $?

: >"$FAKE_GH_LOG"; FAKE_COMMENT_ID=4242 "$here/post-comment.sh" "$tmp/findings.md" >/dev/null
{ grep -q -- '-X PATCH repos/o/r/issues/comments/4242' "$FAKE_GH_LOG" && ! grep -q -- '-X POST' "$FAKE_GH_LOG"; }
expect "comment: a later run updates the existing comment in place, not stacking" $?

: >"$FAKE_GH_LOG"; FAKE_COMMENT_ID=4242 "$here/post-comment.sh" "$tmp/clean.md" >/dev/null
grep -q 'Clean: no findings' "$FAKE_GH_LOG" && grep -q -- '-X PATCH' "$FAKE_GH_LOG"
expect "comment: a result that became clean overwrites the stale findings" $?

: >"$FAKE_GH_LOG"; FAKE_COMMENT_ID= "$here/post-comment.sh" "$tmp/clean.md" >/dev/null
! grep -q -- '-X ' "$FAKE_GH_LOG"
expect "comment: clean with nothing to update posts nothing" $?

FAKE_API=fail "$here/post-comment.sh" "$tmp/findings.md" >"$tmp/out.txt"
rc=$?
{ [ "$rc" -eq 0 ] && grep -q '::warning' "$tmp/out.txt"; }
expect "comment: a failed post warns and exits 0 (never changes the verdict)" $?

# ---- actors ----
actors() { # pull fixture, reviews fixture -> the file's JSON on stdout
  FAKE_PULL="$here/fixtures/$1" FAKE_REVIEWS="$here/fixtures/$2" \
    REPO=o/r PR_NUMBER=7 "$here/actors.sh" "$tmp/actors.json" && cat "$tmp/actors.json"
}

actors pull.json reviews-approved.json >"$tmp/a.json"
{ [ "$(jq -c . "$tmp/a.json")" = '{"schema":1,"pr_author":"alice","head_sha":"bbbb222","approvers":["bob"]}' ]; }
expect "actors: an approved pull request lists its author and the approver on the head commit" $?

actors pull.json reviews-none.json >"$tmp/a.json"
{ [ "$(jq -c '.approvers' "$tmp/a.json")" = '[]' ] && [ "$(jq -r '.pr_author' "$tmp/a.json")" = alice ]; }
expect "actors: an unapproved, self-authored pull request lists its author and no approver" $?

actors pull-agent-authored.json reviews-none.json >"$tmp/a.json"
{ [ "$(jq -r '.pr_author' "$tmp/a.json")" = mallory ] && [ "$(jq -c '.approvers' "$tmp/a.json")" = '[]' ]; }
expect "actors: a pull request authored by someone outside the group is just that author, with no approver" $?

actors pull.json reviews-stale.json >"$tmp/a.json"
{ [ "$(jq -c '.approvers' "$tmp/a.json")" = '[]' ]; }
expect "actors: an approval given before a later push, and one later withdrawn, are not current" $?

FAKE_API=fail actors pull.json reviews-approved.json >"$tmp/out.txt" 2>&1; rc=$?
{ [ "$rc" -ne 0 ] && grep -q 'could not read pull request' "$tmp/out.txt"; }
expect "actors: an API failure fails with a stated reason and writes no pass" $?

REPO= PR_NUMBER= "$here/actors.sh" "$tmp/x.json" >"$tmp/out.txt" 2>&1; rc=$?
{ [ "$rc" -ne 0 ] && grep -q 'REPO and PR_NUMBER are required' "$tmp/out.txt"; }
expect "actors: a missing repository or pull request number fails" $?

# ---- history ----
git init -q "$tmp/origin" && (
  cd "$tmp/origin" && git config user.email a@b && git config user.name a
  echo 1 >f && git add f && git commit -qm one && git branch -M main
  echo 2 >f && git commit -qam two
)
git clone -q --depth 1 "file://$tmp/origin" "$tmp/shallow" 2>/dev/null
(cd "$tmp/shallow" && "$here/check-history.sh") >"$tmp/out.txt" 2>&1
rc=$?
{ [ "$rc" -ne 0 ] && grep -q 'shallow' "$tmp/out.txt"; }
expect "history: a shallow checkout fails with a stated reason" $?

git clone -q "file://$tmp/origin" "$tmp/full" 2>/dev/null
(cd "$tmp/full" && BASE_REF=main "$here/check-history.sh") >"$tmp/out.txt" 2>&1
expect "history: a full checkout with the base branch passes" $?

(cd "$tmp/full" && BASE_REF=nope "$here/check-history.sh") >"$tmp/out.txt" 2>&1
rc=$?
{ [ "$rc" -ne 0 ] && grep -q "base branch 'nope'" "$tmp/out.txt"; }
expect "history: a base branch that cannot be fetched fails with a stated reason" $?

# ---- install ----
mk_release() { # version reported
  local tree="$tmp/rel/o/r/releases/download/v1.2.3/pkg"
  mkdir -p "$tree/urzua-x86_64-unknown-linux-gnu"
  printf '#!/bin/sh\necho "urzua %s"\n' "$1" >"$tree/urzua-x86_64-unknown-linux-gnu/urzua"
  chmod +x "$tree/urzua-x86_64-unknown-linux-gnu/urzua"
  tar -czf "$tmp/rel/o/r/releases/download/v1.2.3/urzua-x86_64-unknown-linux-gnu.tar.gz" -C "$tree" urzua-x86_64-unknown-linux-gnu
}
mk_release 1.2.3
archive="$tmp/rel/o/r/releases/download/v1.2.3/urzua-x86_64-unknown-linux-gnu.tar.gz"
sum="$(sha256sum "$archive" | cut -d' ' -f1)"
mkdir -p "$tmp/inst-tmp"
inst() { # extra env assignments via "$@"
  env VERSION=v1.2.3 REPO=o/r RELEASE_BASE_URL="file://$tmp/rel" DEST="$tmp/dest" TARGET_OS=Linux TARGET_ARCH=X64 TMPDIR="$tmp/inst-tmp" "$@" \
    "$here/install.sh"
}

bin="$(inst SHA256="$sum" 2>"$tmp/err.txt")"
{ [ "$bin" = "$tmp/dest/urzua" ] && [ -x "$bin" ]; }
expect "install: a pinned release with a matching checksum and attestation installs" $?

inst SHA256=deadbeef >/dev/null 2>"$tmp/err.txt"; rc=$?
{ [ "$rc" -ne 0 ] && grep -q 'checksum mismatch' "$tmp/err.txt"; }
expect "install: a checksum mismatch fails" $?

grep -q -- "--signer-workflow o/r/.github/workflows/publish-release.yml" "$tmp/gh.log"
expect "install: the attestation is pinned to the release workflow" $?

FAKE_ATTEST=fail inst >/dev/null 2>"$tmp/err.txt"; rc=$?
{ [ "$rc" -ne 0 ] && grep -q 'attestation failed' "$tmp/err.txt"; }
expect "install: a failed attestation fails" $?

inst VERSION=latest >/dev/null 2>"$tmp/err.txt"; rc=$?
{ [ "$rc" -ne 0 ] && grep -q 'not an exact release tag' "$tmp/err.txt"; }
expect "install: an unpinned version fails" $?

inst TARGET_ARCH=ARM64 >/dev/null 2>"$tmp/err.txt"; rc=$?
{ [ "$rc" -ne 0 ] && grep -q 'no published urzua build for Linux/ARM64' "$tmp/err.txt"; }
expect "install: a platform with no published build fails with that reason" $?

mk_release 9.9.9
inst >/dev/null 2>"$tmp/err.txt"; rc=$?
{ [ "$rc" -ne 0 ] && grep -q 'not the pinned' "$tmp/err.txt"; }
expect "install: a binary that reports another version fails" $?

# A reported version that merely contains the pinned one as a substring is another version.
mk_release 11.2.35
inst >/dev/null 2>"$tmp/err.txt"; rc=$?
{ [ "$rc" -ne 0 ] && grep -q 'not the pinned' "$tmp/err.txt"; }
expect "install: a reported version that only contains the pinned one fails" $?

# Every install above, successful or refused, downloaded into a work directory
# under TMPDIR; none may be left behind on the runner.
[ -z "$(ls -A "$tmp/inst-tmp")" ]
expect "install: no work directory is left behind, on success or on a refusal" $?

[ "$fails" -eq 0 ] && echo "urzua-gate: all tests passed" || { echo "urzua-gate: $fails test(s) failed" >&2; exit 1; }
