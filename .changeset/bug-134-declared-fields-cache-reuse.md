---
default: patch
---

`declared_slots_for_roles` (used by three embodiment/relation rules) and `declared_cross_record_value`
(used by every cross-record `Status` read) each rebuilt a type's declared-field set from scratch on
every record/reference they examined, instead of using the per-type cache this codebase already
computes once elsewhere. Both now reuse that cache -- O(types) instead of O(records) or
O(records × references) per `check` run. No behavior change; findings and output are identical.
