---
default: minor
---

# `urzua audit`: supersession reciprocity and dangling references

`urzua audit` reports cross-record issues `check` doesn't scope to: a record claiming to supersede
another that doesn't point back, and cross-references that don't resolve. Read-only -- never writes,
since a bulk cross-reference rewrite is a real data-loss risk without a review step.
