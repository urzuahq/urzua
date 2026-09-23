---
default: patch
---

The em-dash "no value" sentinel for pointer/relation fields was duplicated byte-for-byte in two rules
instead of living in one place alongside this project's other shared domain primitives. Extracted into
`values::is_no_value_sentinel`; no behavior change.
