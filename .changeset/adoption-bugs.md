---
default: minor
---

`urzua init` can adopt a corpus that does not live under `docs/` (`BUG-36`), and
both halves of adoption now recognise the same filenames (`BUG-37`).

`detect_record_types` groups by each record's own parent directory from the
tracked set. A corpus at `doc/adr/` adopts as `dir: doc/adr`; previously `init`
refused, and proposing `docs/<dir>` regardless would have been worse -- `check`
would then examine zero files and report success.

`parse_record_filename` is one recogniser for both conventions, `0001-slug.md`
and `ADR-1-slug.md`, used by the adopt scan and by `urzua new`'s numbering.
They carried one each and accepted disjoint sets, so adopt mode could not read
the records this tool itself writes, and in an adopted Nygard corpus `new`
returned 1 beside an existing `0001-`.

Two hazards are handled explicitly: a proposed directory that contains another
is dropped, since discovery matches by path prefix and the outer one would
claim the inner one's records; and two directories ending in the same component
are qualified rather than emitted as a duplicate type name.

`urzua new` still writes `ADR-4-slug.md` into a corpus whose own convention is
`0004-slug.md`. Recognising both shapes is not the same as writing the one a
corpus uses; that is `identity.pattern`, in `RFC-33`'s document model.
