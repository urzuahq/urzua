---
Stable-Id: 01M6JRM2B6F9Y24WBM7HSXJ5P4
Rule: config.scope-matches-nothing
Scope: .urzua/config.yaml
Reason: "citation.resolution counts a declared external id (RFC-822 and RFC-4122, public standards sharing the RFC prefix) as out of scope by design, and config.scope-matches-nothing reads any out-of-scope count as a configuration that fails to reach its candidates. Here that count is the declaration working as intended, not a misconfiguration. Expires so it is revisited when the two rules agree on external ids."
Expires: 2027-05-04
---
# WAIVER-34 — citation.resolution external ids counted out of scope

## What this covers

`config.scope-matches-nothing` reports `citation.resolution` as handed candidates the configuration does not reach. They are the citations of the two declared external ids, which `external_ids` deliberately counts out of scope so they are visible and never resolved. The finding is accurate and expected; it is not a defect in the configuration.

## References

- `.urzua/config.yaml` — the record whose text is covered.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed with the own-repo rollout of `citation.resolution` (`MILE-156`). | **substantive** |
