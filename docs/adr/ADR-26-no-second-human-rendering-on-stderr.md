---
Stable-Id: 01M6R3NK4EH33J8N5EFNC3FYJY
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/main.rs#emit@sha256:29d071bbb8232c8cd5678d8c5d104830ec9b083471d1575ab768050e59704a71, test:rust/crates/urzua-cli/tests/check_integration.rs#fix_could_not_run_emits_json@sha256:3c19eb5997529c40ff9d2f64f5f34bdf0f2db19dcc43ff5b97f670bd2c566b83, test:rust/crates/urzua-cli/tests/check_integration.rs#explain_could_not_run_emits_json@sha256:1d06a1d80aa3059a75170cf25f732a714eafef07169d382b1943031675546a62, test:rust/crates/urzua-cli/tests/check_integration.rs#graph_could_not_run_emits_json@sha256:7557a40f07ae8c953456434234ba8fc3a88b71e61c161b1e4c7a19c126885aa3
Date: 2026-09-06
Accepted-on: 2026-09-06
Author: beauwilliams
Deciders: beauwilliams
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

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | `Realized-by` locators pinned by content hash (`@sha256:`) after re-verifying each cited file against the Decision. **Why:** Re-verified against current code; maintainer decision 2026-11-04. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 314829bf1. | **structural** |
> | 2026-11-04 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/src/main.rs` 1b6a076de8bd -> e085b319c363, `rust/crates/urzua-cli/tests/check_integration.rs` 288a08b371a8 -> b668d62f2f4a. Reason: Re-verified: results still print one JSON object to stdout; the only eprintln! calls are the two pre-existing urzua new numbering notices. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Narrowed to the item the decision concerns; re-pin approved by the maintainer's 2026-11-04 drift decision (`ADR-74`, `MILE-185`): `main.rs` to `emit` (lines 39-56); `check_integration.rs` to `fix_could_not_run_emits_json` (lines 577-593), `explain_could_not_run_emits_json` (lines 623-636), `graph_could_not_run_emits_json` (lines 638-651). **Why:** an unrelated edit to a volatile file must not stale this pin again; the spans were chosen by the author of this change and not reviewed line by line by the maintainer. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#fix_could_not_run_emits_json` (lines 550-564) 61ac67a58509 -> 3c19eb599752, `rust/crates/urzua-cli/tests/check_integration.rs#explain_could_not_run_emits_json` (lines 592-603) ca1589422ef2 -> 1d06a1d80aa3, `rust/crates/urzua-cli/tests/check_integration.rs#graph_could_not_run_emits_json` (lines 605-616) 4cade01073a0 -> 7557a40f07ae. Reason: BUG-193: the cited tests only changed how their fixture directory is owned (a TempDir guard removes it on drop, and the trailing remove_dir_all call is gone); every command and assertion in them is unchanged, so the evidence still holds. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
