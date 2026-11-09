#!/usr/bin/env bash
# Posts the summary as ONE pull request comment, updated in place on later
# runs (found by its marker), never stacked. A failed post (a fork's read-only
# token, a rate limit, a missing permission) is a warning and never turns the
# gate's verdict red or green: this script always exits 0.
#
#   post-comment.sh BODY_FILE
#
# Only a comment written by a bot account (`.user.type == "Bot"`, which is what
# the workflow's GITHUB_TOKEN and a GitHub App post as) is taken for the gate's
# own, so a comment that merely contains the marker is never updated or relied
# on. If the token is a person's personal access token, the gate's comment is
# not found and a new one is posted each run: use the workflow token or an App.
#
# Environment: REPO (owner/name), PR_NUMBER, GH_TOKEN. A clean result with no
# earlier comment to update posts nothing.
set -uo pipefail

body="$1"
marker='<!-- urzua-gate -->'
warn() { echo "::warning title=urzua gate comment::$1"; }

[ -n "${PR_NUMBER:-}" ] && [ -n "${REPO:-}" ] || { warn "no pull request to comment on"; exit 0; }

existing="$(gh api --paginate "repos/$REPO/issues/$PR_NUMBER/comments" \
  --jq ".[] | select(.user.type == \"Bot\" and (.body | contains(\"$marker\"))) | .id" 2>/dev/null | head -1)" \
  || { warn "could not list comments; the findings are in the annotations and the job log"; exit 0; }

if [ -n "$existing" ]; then
  gh api -X PATCH "repos/$REPO/issues/comments/$existing" -F "body=@$body" >/dev/null \
    || warn "could not update the existing comment"
elif grep -q '^Clean: no findings\.' "$body"; then
  echo "urzua-gate: clean and no earlier comment; nothing to post" >&2
else
  gh api -X POST "repos/$REPO/issues/$PR_NUMBER/comments" -F "body=@$body" >/dev/null \
    || warn "could not post the comment"
fi
exit 0
