---
default: patch
closes: []
---

The SPEC-22 function-size bound (a function is at most 100 lines) now applies to every non-test
function in the workspace, not only `check.rs`. The `fn_length_ratchet` test measures each crate's
source against a committed ceiling list, `rust/fn-length-ceilings.tsv`, which records the lengths
of the functions that were already over the bound. A function outside the list fails at 101 lines,
a listed one fails when it grows, and a listed one that has shrunk must have its entry lowered or
deleted, so the list can only shrink.
