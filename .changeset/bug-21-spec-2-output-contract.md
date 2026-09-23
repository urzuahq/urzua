---
default: patch
---

`SPEC-2`'s documented `## Output contract` example JSON was RFC-3's original, pre-implementation
illustration -- `camelCase` fields, a 4-value `status` enum where the real one has 3, a claimed stderr
rendering removed years ago, `rules_executed` shown as a bare count instead of the real per-rule array,
and a `suggestedAction` field that never existed on `Finding` at all. Rewritten to match the real shipped
shape. Documentation only; no behavior change.
