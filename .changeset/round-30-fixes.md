---
default: patch
closes: [BUG-173, BUG-174, BUG-175]
---

An invalid record-type setting is now refused when the config loads, instead of producing unrecognizable or colliding records. A `prefix` that is empty, contains a path separator, `..`, whitespace, lower-case letters or a trailing hyphen, or that two record types share, is an error, as is an empty or `.` `dir`. `urzua doctor` reports the same errors, and now also reports an error when no rule is enabled, a config on which `check` exits 2 and examines nothing. `embodiment.locator-exists` no longer reports a `./`- or `..`-spelled path to a tracked file as missing.
