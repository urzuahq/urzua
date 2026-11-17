---
default: patch
closes: []
---

The `chrono` dependency is removed, because `cargo deny` reports it as unmaintained with no safe upgrade. `urzua-io` reads today's UTC date, parses `--as-of` and formats a Unix time through a small private calendar module in the standard library. Behaviour does not change: `--as-of` accepts and rejects the same inputs, and a Unix time out of range still has no date.
