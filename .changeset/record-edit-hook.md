---
default: minor
closes: [MILE-178]
---

A record-edit hook for coding-agent harnesses judges an edit to a computed field such as `Embodiment`
before it is written, by relaying `urzua fix --stdin-path`. It blocks a value that disagrees with the
evidence, says plainly when it could not judge, and accepts a named override. A Claude Code adapter
and `.claude/settings.json` wiring ship in this repository.
