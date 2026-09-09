---
default: minor
---

# Legacy pre-type-prefix filenames no longer resolve

`record_id`/`filename_number`/`next_display_number` now recognize only the `TYPE-NNNN-slug.md`
filename shape. The legacy `NNNN-slug.md` shape (accepted permanently by ADR-36's original
decision) is no longer parsed -- a reference to a filename in that shape is now correctly dangling,
the same as a reference to any other nonexistent record. BUG-9 found this repo's own corpus, and
every other known corpus, has zero files still using the legacy shape.
