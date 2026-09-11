---
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/main.rs
Date: 2026-09-06
Author: '@beauwilliams'
Deciders: '@beauwilliams'
Supersedes / Superseded-by: —
Derives-from: RFC-3, ADR-23
---
# 26 — No second human-format rendering on stderr

## Context

ADR-23 made stdout unconditionally the JSON report, but kept a second rendering on stderr — the
same content, reformatted as human-readable text, printed on every invocation regardless of outcome.
The reasoning at the time was RFC-3's own framing: "a human running the tool directly in a
terminal still sees everything." That reasoning quietly reintroduced the thing ADR-23 was written
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
stderr rendering's format, since ADR-23 itself already said the human-readable rendering carried no
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

## Amendment (2026-09-11): stderr eliminated even for the genuine-error case

This ADR's own Consequences said: *"Error messages (`eprintln!("... could not run: {e}")`) are
unaffected — they were never a duplicate rendering of a success result and remain exactly as they
were."* That was correct at the time, because the JSON on a fatal path didn't contain the actual
error message — only a bare `{"status": "not-run"}`, with the real text existing solely on stderr.
ADR-46's `CouldNotRun` type changes that premise: the error message now lives inside the JSON itself.
Once it does, the `eprintln!` of the same text **is** the duplicate rendering this ADR exists to
eliminate — the exemption's own justification no longer holds.

Decided: stderr is eliminated for every path this codebase's own code controls, fatal or not. The
same reasoning that killed the success-path stderr rendering applies without change to the error
path once the message is structured — this tool's consumer, even reading a CI log after a failure,
is an agent, not a human at a terminal; most CI log viewers (GitHub Actions included) merge stdout
and stderr into one interleaved stream regardless, so a stream split doesn't reliably survive to
whoever reads it either way.

Two things this does **not** touch, both already decided:

- `--help`/`--version` stay plain text on stdout, never JSON-wrapped — clap's own output, not this
  codebase's, and the same industry-standard exemption `cargo`/`rustc`/every clap-based tool already
  makes (help/version generation is never gated behind a machine-format flag). See ADR-46.
- Rust panics still hit stderr by default — the language's panic hook, not this ADR's stderr rule,
  and outside what any `eprintln!` removal touches. ADR-46's `std::panic::set_hook` is the deliberate,
  separate answer to that gap.

## References

- ADR-23 — the decision this narrows further.
- ADR-46 — `Report`/`Notice`/`emit()`; `CouldNotRun` is what made the amendment above possible by
  putting the error message inside the JSON in the first place.
- RFC-3 — the original proposal; this ADR resolves its "human still sees everything on the same
  terminal" framing in favor of no second rendering at all, since this tool has no human-primary
  invocation path left to serve.
