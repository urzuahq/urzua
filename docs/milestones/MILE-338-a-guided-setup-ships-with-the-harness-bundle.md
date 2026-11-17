---
Stable-Id: 01M7KSH0YBASNSJPQGWPJW2RCN
Status: Planned
Phase: '2'
Track: integrations
Blocked-on: null
---
# 338 — A guided setup ships with the harness bundle

## What

Ship, in the harness bundle and installable by one verified command, an agent skill that interviews the person adopting urzua and writes the configuration from tested blocks, so no agent writes keys from guesses.

## Why

`urzua init` proposes a configuration from what a repository already has; it does not ask what the person wants. The questions that turned out to matter in one adopting repository were never asked: which record types exist and how they nest; who may accept a decision; whether agents work in the repository and which guardrails to install; whether a finished record must cite evidence per check, and where evidence that cannot be committed goes; which fields have fixed values; what CI should block. Each answer maps to a block of configuration that exists and is tested. The generated drafting commands show the bundle can already carry agent-facing instructions per repository, and `MILE-280` plans to ship the harness as an agent plugin; the maintainer distributes skills for another of his repositories with a one-line installer and asked for the same here.

## Done means

- The harness bundle carries a setup skill that asks the person the questions above one at a time, in plain words with a recommendation, and writes only configuration blocks that a test in this repository loads and exercises, shown by a test that every block the skill can write is such a block.
- The skill reads `urzua doctor`'s list of undeclared capabilities as its agenda and finishes by running `doctor` and `check`, so the skill and the tool cannot disagree about what is left.
- The skill is installable by one command whose download is verified by checksum or attestation the way the binary's is, and through the plugin distribution `MILE-280` describes; the command is documented in the adoption guide.
- A decision the person declines is written down as declined with their reason, not left unset.
- The skill holds no record type, status or field name of its own beyond the starter profiles `urzua init` already offers.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Proposed on 2026-11-17 by the maintainer, relayed by an agent in an adopting repository: "I feel like we've missed a bunch of things during setup that we probably should have turned on. Is this a documentation issue?", with the proposal that the tool supply skills that "tell the agent all the things that you're missing ... like a guided setup with the user". | **structural** |
