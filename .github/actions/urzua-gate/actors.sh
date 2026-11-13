#!/usr/bin/env bash
# Writes the actors file `urzua check --actors FILE` reads:
# who opened the pull request and whose approval is current for its head
# commit, from the GitHub API. urzua never fetches these itself.
#
#   REPO=owner/name PR_NUMBER=7 GH_TOKEN=... actors.sh OUT.json
#
# {"schema": 1, "pr_author": "alice", "head_sha": "<sha>", "approvers": ["bob"]}
#
# A reviewer's latest approving, changes-requesting or dismissing review is the
# one that counts, and it counts only when it was given on the head commit: an
# approval given before a later push is stale and is not listed. A review that
# only comments changes nothing. This proves who made a pull request and who
# approved it, not who decided: with one maintainer and agents under that
# maintainer's token, the author is always the maintainer.
set -euo pipefail

out="${1:?usage: actors.sh OUT.json}"
fail() { echo "urzua-gate: actors: $1" >&2; exit 1; }
[ -n "${REPO:-}" ] && [ -n "${PR_NUMBER:-}" ] || fail "REPO and PR_NUMBER are required"

pull="$(gh api "repos/$REPO/pulls/$PR_NUMBER")" \
  || fail "could not read pull request $PR_NUMBER from the API"
reviews="$(gh api --paginate "repos/$REPO/pulls/$PR_NUMBER/reviews" | jq -s 'add // []')" \
  || fail "could not read the reviews of pull request $PR_NUMBER from the API"

jq -n --argjson pull "$pull" --argjson reviews "$reviews" '
  ($pull.head.sha) as $head
  | {
      schema: 1,
      pr_author: ($pull.user.login // null),
      head_sha: $head,
      approvers: (
        $reviews
        | map(select(.user != null and (.state | IN("APPROVED", "CHANGES_REQUESTED", "DISMISSED"))))
        | sort_by(.submitted_at)
        | group_by(.user.login)
        | map(last)
        | map(select(.state == "APPROVED" and .commit_id == $head))
        | map(.user.login)
        | sort
      )
    }' >"$out" || fail "could not write $out"
if [ "$(jq -r '.pr_author // empty' "$out")" = "" ]; then
  fail "the API gave no author for pull request $PR_NUMBER"
fi
