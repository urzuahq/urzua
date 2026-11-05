---
default: minor
closes: []
---

The `Embodiment` states are now declared in config. A top-level `embodiment` key names the drift
state, the tiers from highest to lowest with the kind of evidence (`test`, `code` or `spec`) that
lifts a record to each, and the state computed from no evidence; `claims_built` marks the states
that say the work was built. `urzua check` and `urzua fix` compute and repair against those names.
Omitted, the built-in `Drift detected`, `Verified`, `Implemented`, `Specified` and `Not started`
apply, so existing output is unchanged.
