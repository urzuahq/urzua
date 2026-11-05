---
default: patch
closes: []
---

The source back-pointer matcher now reads every id in `Implements: ADR-1 (note), ADR-2`, continuing
past a parenthetical or trailing note to the next comma-separated id, and reads a marker that
follows code on the same line (`let x = 1; // Implements: ADR-1`). `embodiment.back-pointer-decision-accepted`
judges the same widened set. Prose after an id still cannot name one: `ADR-1 and ADR-2` yields `ADR-1`.
