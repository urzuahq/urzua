---
default: minor
closes: []
---

A record can declare its kind, and the kind decides which evidence tiers count as delivered. Under `statuses.delivery.items`, a new `kind` key names a header field and a vocabulary of kinds; each kind may list its own `delivered_tiers` and may set `cited_records: success`. A `decision` kind that counts `Specified` computes delivered when every Done means item resolves at `Specified` or higher, so a milestone that only records a decision can now compute `Done`. With `cited_records: success`, a locator that resolves to a record counts only while that record's status is one its own type declares `success`: a Proposed ADR or a Draft spec evidences nothing. A record with no kind, or a kind that names no tiers, is judged by the type's `delivered_tiers` as before, and a configuration with no `kind` key is unchanged.

`status.done-items-shape` reports a `Kind` value outside the declared vocabulary, naming the allowed kinds, and the record is then judged as one with no kind. A `kind` declaration is refused at load when its field is blank or not one the type declares, when it lists no kinds or one twice, or when a kind's `delivered_tiers` names a tier that is not declared, not in `resolving_tiers` or in `unevidenced_tiers`. See `statuses.delivery.items.kind` in the configuration reference.
