---
default: minor
closes: []
---

Adds `roadmap.work_types`, an optional list of the record types that are work. A record of any other type, such as a fact that is met or dropped and never built, still blocks the records that depend on it but no longer appears in `urzua roadmap`'s waves, ready set, critical path, records or groups. Waves and the critical path count work only and follow dependencies through those records, so work behind one is ordered after the work it waits on. A new top-level `waiting_on` list names each unfinished non-work record with its type, status and the work it blocks. Without the setting the output is unchanged. An unknown, repeated or blank type name, or an empty list, is refused when the configuration loads.
