---
default: minor
closes: []
---

Add `authority.decider_table` (`base`, `head` or `both`), which says whose table `diff.decider-required`
reads. With `base`, who may decide (the groups, their logins and the gates) comes from the configuration at
the pull request's base revision, so a pull request can no longer add its author to a decider group, or
delete a gate, in the same change that moves a record to a gated status. `head`, which an undeclared key
means, keeps reading the checkout's table. `urzua init` now proposes `decider_table: base`, and `urzua
doctor` warns when gates are declared without the key.
Under `base` and `both`, a base with no `authority` is judged against the head table instead of passing
unjudged, and the rule's level and `fail_on` are the stricter of the base's and the head's, so a pull request
cannot switch the rule off or lower it. `urzua doctor` says an `authority` with no gates gates nothing.
