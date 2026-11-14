#!/usr/bin/env bash
# create-release.sh against a fake `gh`: it creates a missing release with a
# capped body, leaves an existing release alone, never touches tags, and fails
# when the API refuses.
set -uo pipefail

script="$(cd "$(dirname "$0")" && pwd)/create-release.sh"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0
mkdir -p "$tmp/bin"

# The fake records every call and its notes file. GH_FAKE_EXISTS=1 makes
# `release view` succeed; GH_FAKE_CREATE_FAIL=1 makes `release create` fail.
cat > "$tmp/bin/gh" <<'SH'
#!/usr/bin/env bash
echo "$*" >> "$GH_FAKE_LOG"
if [ "$1 $2" = "release view" ]; then [ "${GH_FAKE_EXISTS:-0}" = 1 ]; exit; fi
if [ "$1 $2" = "release create" ]; then
  [ "${GH_FAKE_CREATE_FAIL:-0}" = 1 ] && { echo "HTTP 422" >&2; exit 1; }
  while [ $# -gt 0 ]; do [ "$1" = "--notes-file" ] && cp "$2" "$GH_FAKE_NOTES"; shift; done
  exit 0
fi
exit 99
SH
chmod +x "$tmp/bin/gh"

{ printf '# Changelog\n\n## 1.0.0 (2027-01-02)\n\n'
  for i in $(seq 1 3000); do printf 'Paragraph %s of an over-long release note that nobody wrote for a reader.\n\n' "$i"; done; } > "$tmp/cl.md"
printf '# Changelog\n\n## 1.1.0 (2027-02-02)\n\nShort.\n' > "$tmp/short.md"

run() { # exists create_fail changelog version
  export GH_FAKE_LOG="$tmp/log" GH_FAKE_NOTES="$tmp/notes" GH_FAKE_EXISTS="$1" GH_FAKE_CREATE_FAIL="$2"
  : > "$tmp/log"; rm -f "$tmp/notes"
  PATH="$tmp/bin:$PATH" "$script" "$4" "v$4" "$3" o/r abc123 >/dev/null 2>&1
}
ok() { if [ "$2" = ok ]; then echo "  ok    $1"; else echo "  FAIL  $1"; fail=1; fi; }

run 0 0 "$tmp/cl.md" 1.0.0; st=$?
[ "$st" -eq 0 ] && grep -q '^release create v1.0.0 .*--verify-tag' "$tmp/log" && ok "a missing release is created with --verify-tag" ok || ok "a missing release is created with --verify-tag" no
[ "$(wc -c < "$tmp/notes")" -lt 125000 ] && grep -qF 'blob/abc123/CHANGELOG.md#100-2027-01-02' "$tmp/notes" && ok "an oversized changelog yields a capped body linking to the ref" ok || ok "an oversized changelog yields a capped body linking to the ref" no
grep -q -- '--title 1.0.0 (2027-01-02)' "$tmp/log" && ok "the title is the changelog heading" ok || ok "the title is the changelog heading" no

run 0 0 "$tmp/short.md" 1.1.0
[ "$(cat "$tmp/notes")" = "Short." ] && ok "a short section is the whole body" ok || ok "a short section is the whole body" no

run 1 0 "$tmp/cl.md" 1.0.0; st=$?
[ "$st" -eq 0 ] && ! grep -q 'release create' "$tmp/log" && ok "an existing release is left alone" ok || ok "an existing release is left alone" no

run 0 1 "$tmp/cl.md" 1.0.0; st=$?
[ "$st" -ne 0 ] && ok "a refused create fails the script" ok || ok "a refused create fails the script" no

run 0 0 "$tmp/cl.md" 9.9.9; st=$?
[ "$st" -ne 0 ] && ! grep -q 'release create' "$tmp/log" && ok "a version with no changelog section creates nothing" ok || ok "a version with no changelog section creates nothing" no

! grep -rqE 'git tag|git push|tag delete|knope' "$script" && ok "the script never creates or moves a tag" ok || ok "the script never creates or moves a tag" no

if [ "$fail" -eq 0 ]; then echo "create-release: all cases passed"; else echo "create-release: FAILURES"; fi
exit "$fail"
