---
Stable-Id: 01M7KSXQTJK9CSPTZ48GZ6S02Z
Status: Planned
Phase: '1'
Track: integrations
Blocked-on: null
---
# 339 — The harness adapter runs from the binary, not from a script in the repository

## What

Move the translation between an agent harness's hook events and urzua's verdicts out of the installed script and into the binary, so the hook command calls urzua directly, no executable file is installed in the repository, and nothing needs a hash pinned.

## Why

`urzua harness install` writes a script into the repository that translates an agent harness's hook events, and the hook command in the agent's settings carries that script's sha256 and checks it before running it, so that an altered enforcer does not run. Measured on release 0.4.7 on 2026-11-17: re-indenting the settings file changes nothing; reformatting a generated command file leaves it working but `urzua harness status` reports drift; changing the script by two blank lines makes the hook refuse to run, and with the gate set to refuse what it cannot judge every agent edit in the repository is refused with "the editor hook could NOT run ... does not match the sha256". The check cannot tell tampering from a formatter. Asking every adopter to exclude the harness files from every formatter puts the cost of the design on them and locks their agents out when they miss it; an adopting repository added the exclusion by hand before installing because it saw the risk. The binary is already verified at install by attestation, so code that lives in it needs no second check.

## Done means

- The hook command an install writes calls the urzua binary directly with the harness's event, and the binary does the translation an installed script did, shown by tests that feed the same events and compare verdicts with the script's for every event the script handled.
- `urzua harness install` writes no executable file into the repository, and no settings entry carries a file hash; an existing installation is moved by `urzua harness update`, which removes the script and its pin and says so.
- Reformatting every file an install writes, with a common formatter for each file type, leaves the harness working and `urzua harness status` at ok, shown by a test; a generated instruction file is compared ignoring formatting and reported as drift only when its words change.
- The property the pin gave is kept and stated: an agent cannot weaken the enforcer by editing a file in the repository. The settings file and the configuration remain protected paths, and the decision record for the harness is amended to say where the enforcer now lives and what verifies it.
- A settings entry that calls a urzua binary that is missing or older than the configuration needs is reported at session start with the command that fixes it, and the choice between refusing and allowing while it cannot judge stays the configuration's.
- The file that records what was installed can be rewritten by a formatter without `urzua harness status` reporting drift, shown by a test.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** The maintainer said on 2026-11-17, on hearing that a formatter touching the installed hook script locks an agent out: "a formatter shouldn't break things", and of moving the script's work into the binary: "it should [be] in urzua. This is much better. If we just own that code, then we don't also have to install it, and then they have to be in their repo and everything. So much better." | **structural** |
