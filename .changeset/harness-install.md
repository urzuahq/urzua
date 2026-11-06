---
default: minor
closes: [MILE-198]
---

Add `urzua harness`, which installs an agent harness bundle into a repository and runs its guardrails. `harness install --adapter NAME` writes the adapter's hook entries, a translator for the agent and a lock file, merges the entries into the agent's project settings without touching anything else in them, and keeps one marked block of instructions in the file `harness.instructions_file` names; `--dry-run` lists what would change. `harness status` compares the lock with the files and exits 1 on drift, `harness update` rewrites what has not been changed by hand, `harness uninstall` removes only what it installed, and `harness adapters` lists what is embedded in the binary. The new `harness` configuration key turns each guardrail on by name, and `on_unjudged: deny` makes a guardrail that cannot judge refuse an agent's edit while a person's goes through. `harness run edit` judges a proposed edit to a record, with the verdicts the editor hook had, and says so loudly when it cannot judge; `harness run session-start` reports at the start of a session what would make a guardrail fail open. `urzua doctor` reports the harness when it is declared.
