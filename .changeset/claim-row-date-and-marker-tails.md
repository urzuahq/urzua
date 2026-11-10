---
default: patch
closes: []
---

A genuine `urzua reverify` row is no longer reported by `embodiment.claim-renewal-unattested` once
main has moved on: the row's date is compared with the date of the commit that added it, not with the
merge base's commit date, and a future date is still reported. A back-pointer `ignore_marker` that is
the tail of a record id head followed by a number (`DR-1`, `R-12`) is refused at load, `urzua new`
and `urzua init` numbering read record files with an upper-case `.MD` extension, and claim files
under `claim_paths` are found whatever the case of `.md`.
