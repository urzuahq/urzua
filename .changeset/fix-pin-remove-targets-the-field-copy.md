---
default: patch
closes: [BUG-231, BUG-232, BUG-233]
---

# `fix --pin --remove` edits the evidence field only, reads a folded field, and names the items that still cite the pin

`urzua fix --pin --remove OLD` removes the pin from the `Realized-by` field only. A Done-means item that cites the same pin keeps it, and the report lists each such item under `still_cited_by_items` as `{item, line, locator}`; a pin cited only in items is removed from the first item that cites it, and the others are listed, as before. It also removes an entry from a folded (`>-`) `Realized-by` field, which it used to refuse. When a bare `OLD` is cited in two tiers, the refusal names the spelling of each tier it is cited in (`spec:` and `record:` included), not always `code:` and `test:`.
