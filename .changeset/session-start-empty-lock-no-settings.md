---
default: patch
closes: [BUG-215]
---

`urzua harness run session-start` no longer reports the harness as installed and wired when `.urzua/harness.lock` lists no hook entries for the agent's settings file and that file is missing. It names the file as missing or unreadable, once, so the guardrail that is not running is reported.
