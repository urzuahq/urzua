---
default: minor
closes: [BUG-99]
---

`urzua check --rules <id>[,<id>...]` runs only the named rules of your configuration, and every report gains a top-level `rules_selected` (the ids, or `null` when not restricted). A rule left out is omitted from `rules_executed` and is not run; naming a rule your configuration does not declare, or one that does not exist, exits 2.

`urzua audit` is deprecated: it is now an alias of `urzua check` over your configured rules with no rule set of its own, so it reports exactly what `check` reports (`urzua --help` and `urzua audit --help` now say so) and adds one deprecation notice that never changes the exit code. Because it now runs every rule your configuration declares, a CI step running `urzua audit` fails on a blocking finding from any of them, not only supersession reciprocity and dangling references, and takes about as long as `check`. It no longer exits 2 on a configuration `check` accepts, such as the one `urzua init` writes for a corpus with no type prefix. Use `urzua check` (or `urzua check --rules pointer.resolution,relation.supersession-reciprocity` for the narrower report); `audit` is removed in the next release. `urzua init` no longer warns that `audit` has nothing declared.
