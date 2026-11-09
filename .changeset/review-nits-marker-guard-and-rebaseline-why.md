---
default: patch
closes: []
---

A back-pointer `ignore_marker` that would sit on every back-pointer line is now refused when the configuration loads:
one with fewer than 3 non-whitespace characters, one made only of comment leaders or punctuation (`//`, `#`), and one
that is part of a declared record id head such as `ADR-`. A specific marker such as `urzua:allow` loads as before.

The revision row `urzua fix --rebaseline --apply` writes now ends with a rationale that names who re-verified the
claim: an agent (to be confirmed by a reviewer in the PR), a person, or the attesting actor when none was declared.
