---
default: patch
closes: [BUG-231, BUG-232]
---

# `fix --pin --remove` edits the evidence field's copy of a pin, and the tier hint names the right tiers

`urzua fix --pin --remove OLD` now removes the pin from the `Realized-by` field when the same pin also sits in a Done-means item, and leaves the item's copy alone; before, it could edit the item and leave the field unchanged. When a bare `OLD` is cited in two tiers, the refusal names the spelling of each tier it is cited in (`spec:` and `record:` included), not always `code:` and `test:`.
