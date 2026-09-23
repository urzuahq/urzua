---
default: patch
---

`relation.supersession-reciprocity` read a resolved target's own field via a plain `.get()` with no
check for whether the target's header was unreadable -- which reads identically to a genuinely absent
field. A record correctly citing a target whose header failed to parse got a spurious "does not
reciprocally name back" finding, blaming it for the target's own unrelated parse failure. The rule now
skips a target with an unreadable header rather than judging it.
