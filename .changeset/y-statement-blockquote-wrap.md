---
default: patch
---

`section.shape` with the `y-statement` shape no longer reports a phrase as missing when the block is
a blockquote and the phrase wraps across lines: the leading `>` on each line is ignored before
matching.
