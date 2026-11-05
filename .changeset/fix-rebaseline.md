---
default: minor
closes: [MILE-181]
---

Add `urzua fix --rebaseline`, an attributed re-baseline for `@sha256:` evidence pins. Without
`--apply` it lists each record whose pinned locator changed, with the recorded and current hash.
With `--apply`, `--ids` (or `--force`), `--by` and a required `--reason` it rewrites those records'
pins to the current hash and appends a revision-log row naming who attested and why; nothing else
on the `Realized-by` line moves.
