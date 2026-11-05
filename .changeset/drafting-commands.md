---
default: minor
closes: [MILE-177]
---

Add `/rfc`, `/adr` and `/milestone` drafting commands for AI agents working in this repository. Each scaffolds a record with `urzua new`, stages it, regenerates the declared indexes with `urzua index --apply`, runs `urzua check` on it and reports how many records were examined; none sets a decision status or changes a value `urzua new` wrote. `AGENTS.md` asks an agent to offer the matching command when it notices a proposal or a made decision.
