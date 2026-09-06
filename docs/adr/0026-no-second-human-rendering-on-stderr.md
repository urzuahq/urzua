# 0026 — No second human-format rendering on stderr

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:rust/crates/urzua-cli/src/main.rs
> Date: 2026-09-06
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0003 (Accepted), ADR-0023 (Accepted)

## Context

ADR-0023 made stdout unconditionally the JSON report, but kept a second rendering on stderr — the
same content, reformatted as human-readable text, printed on every invocation regardless of outcome.
The reasoning at the time was RFC-0003's own framing: "a human running the tool directly in a
terminal still sees everything." That reasoning quietly reintroduced the thing ADR-0023 was written
to remove — a maintained second output shape someone has to keep in sync with the first, on a tool
whose whole premise is that there is one structured shape, not a machine one and a human one.

## Decision

In the context of a fully agent-driven tool where a human's access to Urzua's output goes through an
agent reading the JSON, not through reading a terminal directly, we decided **stdout is the only
rendering, full stop** — no second stderr copy, on any command. `print_report`, `print_fix_report`,
`run_explain`, and `run_graph` print exactly one JSON object and nothing else on success. Stderr is
reserved for what it already was for everywhere else in this codebase: a genuine error — "could not
run: config missing," "could not resolve an identity" — never a duplicate rendering of a result that
already printed.

## Reversibility

Strictly a reduction in surface: deleting output, not adding a new contract. Nothing depended on the
stderr rendering's format, since ADR-0023 itself already said the human-readable rendering carried no
compatibility guarantee.

## Consequences

- `print_report`/`print_fix_report`/`run_explain`/`run_graph` lose their `eprintln!` rendering blocks
  entirely — not reformatted, removed.
- A human running `urzua check docs/` directly in a terminal now sees raw JSON, same as an agent
  would. If a genuinely human-facing terminal rendering is ever wanted, it is a `--pretty`-style
  opt-in transform of the same JSON, not a second code path computed independently — avoiding the
  drift risk two independently-maintained renderings of the same data always carries.
- Error messages (`eprintln!("... could not run: {e}")`) are unaffected — they were never a
  duplicate rendering of a success result and remain exactly as they were.

## References

- ADR-0023 — the decision this narrows further.
- RFC-0003 — the original proposal; this ADR resolves its "human still sees everything on the same
  terminal" framing in favor of no second rendering at all, since this tool has no human-primary
  invocation path left to serve.
