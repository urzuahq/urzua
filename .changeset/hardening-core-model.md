---
default: patch
closes: []
---

A YAML frontmatter header is now read as written: a version such as `1.10` or a number such as `0x1F` is no longer
rewritten to `1.1` or `31`, a leading byte-order mark no longer hides the header, and a non-string key keeps its
text instead of vanishing. `urzua migrate schema --report` judges a field the way `field.quality` does for types
whose placeholders come from their template. A refs snapshot with unsorted numbers is refused, `urzua new` stops
with an error when no record number is left to issue, and `urzua fix` reports the real reason when it cannot
create its lock file.
