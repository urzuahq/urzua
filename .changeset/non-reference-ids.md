---
default: minor
closes: [BUG-190]
---

`citation.resolution` accepts an optional `non_reference_field` naming a header field in which a
record lists, as comma-separated exact ids, the ids it names without citing as a reference (an
example, or a number minted and retracted). A citation of a listed id is neither resolved nor
reported in that record only, so it no longer starts resolving to an unrelated record when one is
later created with that number; the same id cited in another record still resolves. A listed id the
record never cites and a malformed entry are reported, a field that is unknown or is a pointer,
narrative or relation-role field is a config error, and `urzua check` prints one notice naming the
listed ids that now match a real record.
