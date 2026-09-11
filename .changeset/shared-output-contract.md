---
default: major
---

# One shared output contract across every command: `Report`, `Notice`, `emit()`

`check`, `audit`, `fix`, `new`, `explain`, `graph`, and `doctor` previously each hand-rolled a
different, incompatible JSON shape. Every one of them now implements a shared `Report` contract and
prints through one function -- concretely:

- Every report gains an optional `notices` array: non-fatal observations (`severity: "info"|
  "warning"`, `subject`, `message`) that never affect the exit code, omitted entirely when empty.
  `--by`'s divergence from an authenticated `gh` login is the first real user of this.
- Every fatal "could not run" path now emits real JSON on stdout, not just an unstructured stderr
  message -- previously true only for `fix`; `new`, `explain`, `graph`, and `doctor`'s early guard
  printed nothing on stdout at all on failure.
- **`check`/`audit`'s failure-path JSON changes shape**: it used to include `scope`/`rules_executed`/
  `files_examined` as present-but-empty/zero keys even when the tool couldn't run at all; those keys
  are gone on that path now (`{"status": "not-run", "error": "..."}` instead). Every other command's
  change is additive (nothing on stdout before, real JSON now).
- Stderr is no longer used for anything this tool's own code controls, including genuine errors --
  the error message now lives in the JSON itself. `--help`/`--version` remain plain text on stdout
  (never JSON-wrapped), matching `cargo`'s own convention. An unexpected panic now prints a minimal
  JSON object to stdout instead of Rust's default raw text to stderr.
- A bad CLI flag now produces the same JSON-on-stdout, real-exit-code behavior as any other fatal
  error, instead of clap's own unstructured usage message.

`urzua init` and `urzua migrate ids` are not part of this change -- both remain plain-text prose end
to end, tracked separately (BUG-20).
