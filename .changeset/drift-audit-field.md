---
default: minor
closes: []
---

`embodiment.consistency` accepts `drift_audit_field`, the name of a record field holding the date its evidence was last audited. With `drift_grace_days` set, a record carrying a `YYYY-MM-DD` date in that field has its grace window counted from that date instead of from the cited file's last change; a record without it counts from the file as before, and a malformed date is reported as a warning. The option needs `drift_grace_days` and a field that a record type declares, else the config is refused at load.
