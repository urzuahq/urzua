---
default: patch
closes: [BUG-203, BUG-204, BUG-205, BUG-206]
---

`urzua board` reads the configuration once per page, so a configuration edit made while it builds can no longer give a page whose records, declared types and check summary come from different configurations. `urzua board --prompt` now exits 2 when given `--port` or `--allowed-host`, which only apply with `--serve`. `status.children-agreement` and the export's `state_disagrees_with_children` no longer treat a parent whose `Status` its type does not declare as not started, so such a parent is not reported against its children (`status.undeclared-value` still reports the value). The editor hook computes the parent-state note after it has judged the edit rather than before, with the same verdict and output.
