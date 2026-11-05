---
default: patch
closes: []
---

`drift_grace_days` no longer contradicts a record that already states `Embodiment: Drift detected`.
Inside the window `embodiment.consistency` accepts either the tier value or `Drift detected`, and
`urzua fix` leaves a stated `Drift detected` alone; drift older than the window still computes
`Drift detected`.
