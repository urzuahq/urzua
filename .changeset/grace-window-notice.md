---
default: minor
closes: []
---

`check` now tells you when `drift_grace_days` is holding drift back. One info notice,
`embodiment-drift-in-grace`, reports how many records `embodiment.consistency` will report once the window ends (records whose type declares no `Embodiment`, a sticky state such as `Deprecated`, or a stated `Drift detected` are not counted), the
date the earliest of them leaves the window and starts being reported, and their record paths (at most
20, with the full count). Dates follow `--as-of`. The notice never changes a finding or the exit code,
and a configuration without `drift_grace_days` reports exactly what it did before.
