---
default: minor
closes: []
---

Add `harness.outside_writes` (`allow`, `shared` or `deny`) and `harness.allow_paths`. With `shared`, an
agent's edit-tool write into a linked worktree of the repository, for example one at an older commit that
holds no gates, is also judged by the repository's own acceptance and protected-path gates, and a deny
wins. With `deny`, an agent's write outside the repository is refused unless it is under an `allow_paths`
directory. A key left undeclared behaves as `allow`, as before. `urzua init` proposes
`outside_writes: shared`, and `urzua doctor` warns when a guardrail is enabled and the key is undeclared.
