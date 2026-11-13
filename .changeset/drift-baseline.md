---
default: minor
closes: [MILE-49]
---

Adds `drift_baseline` to `embodiment.consistency`, a git ref that separates drift that predates it from drift introduced since: `drift_baseline: {ref: v1.0.0}`. A record that already had the same drifted locators (same pin) at the ref, whose cited files were last changed at or before it, is pre-existing, so it draws no finding and does not block, and it is listed in an `embodiment-drift-baselined` info notice and a new `baseline` object in the `check` report. Drift introduced since the ref is reported and blocks as before. A ref that names no commit stops the run with exit 2 rather than reading as no pre-existing drift, and a whole-repository run whose baseline holds nothing out says so, so it can be removed. Without the option nothing changes.
