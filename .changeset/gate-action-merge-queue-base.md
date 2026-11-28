---
default: patch
closes: [MILE-374]
---

The `urzua-gate` action now runs the diff rules in a merge queue without a change to your workflow. With `base-ref` empty on a `merge_group` event, which is what `${{ github.base_ref }}` gives there, the action takes the queue's base branch from the event and runs `urzua check --base origin/<base>`. A `base-ref` you set always wins, and on any other event an empty `base-ref` stays empty. Before, a merge queue ran only the whole-tree check and skipped the diff rules, drift included, without saying so.
