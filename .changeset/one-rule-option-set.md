---
default: patch
---

A rule option declared in the configuration is now defined in one place inside the tool, so a newly added option can no longer be accepted by the loader yet ignored by the rule. Configuration load behaves as before: an option on a rule that does not take it is still refused, with the same message.
