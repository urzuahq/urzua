---
default: minor
closes: []
---

The post-command report weighs a gated change by whether it holds exactly the content on a base ref. When every gated path a command changed matches the base tree (as after a fast-forward pull of the default branch), the report is one sentence naming the count and the branch, at level `info`, saying nothing needs doing. A change that differs keeps level `error`, grouped under its gate with each gate named once. New keys `harness.guardrails.post-command.base` (default `origin/HEAD`) and `base_match` (`info`, `warning` or `error`; default `info`). A path matches only when its mode and blob both match (a symlink compared as its link text), the one sentence names the paths and calls a non-remote ref local, a command that changes the configuration is judged by the comparison rule from before it, and a missing base is stated. The comparison uses local refs, so it is advisory; every gated change is still printed. The "arrived by a git move" wording and the stash tracking behind it are removed.
