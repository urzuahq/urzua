# 0023 — Stdout is always JSON; no `--format` flag

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
> Date: 2026-09-06
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: ADR-0007
> Derives-from: RFC-0003 (Accepted)

## Context

RFC-0003's actual proposal was that stdout is *unconditionally* the JSON report — "every entry
point's final, primary output on stdout is exactly one JSON object, always," with human-readable
prose moved to stderr entirely, no flag deciding between them. ADR-0007 narrowed that at
implementation time to an opt-in `--format json` flag, human-readable by default — reasonable for a
CLI unsure yet who'd actually run it.

That premise no longer holds. Urzua is not a linter a human runs and reads; it's a structured record
engine whose primary consumer is an agent, and a human's access to that data goes through an agent
reading it, not through squinting at terminal output directly. A tool built entirely around
structured records should not make its own primary interface unstructured by default.

## Decision

In the context of a fully agent-driven tool, facing ADR-0007's default working against the actual
audience, we decided **stdout is always the JSON report, unconditionally — no `--format` flag, no
opt-in, no human default.** `check`, `fix`, and `migrate schema --report` print exactly one JSON
object to stdout on every invocation. The human-readable rendering — the same one `--format human`
used to produce — now prints to stderr, always, on every invocation too. A person running the tool
directly in a terminal sees both streams (they render to the same terminal by default); a consumer
that captures stdout alone sees JSON only, unconditionally, without having to know to ask for it.

`OutputFormat` and every `--format` argument on `check`/`fix`/`migrate schema` are removed, not
deprecated — there is no flag to reintroduce this ADR was written to eliminate.

## Reversibility

Same reversibility profile ADR-0007 already named: this is the output contract anything scripting
against `urzua` depends on. Cheap now — no known external consumer yet — expensive once one exists,
which motivates deciding this now rather than after v1.0.0.

## Consequences

- Every future command's default output changes shape: JSON on stdout is not something a caller
  opts into, it's what's already there.
- `urzua init` and `urzua doctor` do not yet follow this contract — they predate the JSON report
  shape entirely and print plain status text on stdout. Bringing them into line is real, separate
  follow-up work, not implied by this ADR.
- ADR-0007's "human-readable format is not contractual" clause still holds, now for the stderr
  rendering specifically.

## References

- ADR-0007 — the narrower decision this supersedes.
- RFC-0003 — the original proposal this restores.
- `rust/crates/urzua-cli/src/main.rs` — `print_report`, `print_fix_report`, and
  `run_migrate_schema_report`'s output paths.
