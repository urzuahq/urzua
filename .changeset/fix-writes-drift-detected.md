---
default: minor
---

`urzua fix` now detects and, with `--apply`, writes `Embodiment: Drift detected` for a record whose
`Realized-by` locator changed after the `Realized-by` line was last touched -- the same condition
`urzua check` already reports. Previously `fix` ignored drift, so a drifted record that `check`
flagged could not be repaired by the tool. A stated `Drift detected` that is no longer drifted is
repaired back to the tier-computed value.
