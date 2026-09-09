---
Stable-Id: 01M20SH9CXA18HFPABZJQY795D
Status: Accepted
Date: 2026-09-08
Version: '0.2'
Author: '@beauwilliams'
Implements: MILE-36
Parent: SPEC-1
---
# SPEC-19 — `AGENTS.md`

## Purpose

`AGENTS.md` is this repo's durable, discoverable home for practices an AI agent needs to work in it
correctly — written directly by [MILE-36](../milestones/MILE-36-write-agents-md-capturing-this-session-s-practices.md)
with no preceding RFC or ADR, the same "no decision point existed" situation as `doctor` (SPEC-15):
it was named as worth writing, not debated into existence. This spec exists to give that already-
built artifact the same governed home every other feature area has (ADR-41's "coherent feature area"
principle), going from the milestone straight to a spec — skipping the ADR stage is legitimate when,
as here, there was no real decision to record (RFC-4's own "common path, not the only path" already
established this).

Distinct from a `spec`-the-record-type instance in one respect: `AGENTS.md` itself is not a governed
record (`urzua check` doesn't validate it — it has no `.urzua/config.toml` entry, no header, no
`Status`). This spec is the governance layer *about* that file: what belongs in it, how it's
maintained, and a place for `Why` to accumulate as it changes, since `AGENTS.md`'s own prose doesn't
carry a revision log the way a record does.

## What belongs in `AGENTS.md`

Its own opening line draws the boundary already: `CONTRIBUTING.md` is the human-facing equivalent;
`AGENTS.md` states what that file doesn't need to. Concretely, as of this spec's `Version: 0.2`,
nine sections, grouped by what kind of guardrail they are:

- **What urzua is, and what it refuses to be** — the tool's own founding constraints, restated from
  `SPEC-1` so an agent has the project's actual engineering goals in view, not just process rules:
  config-driven over hardcoded, no silent no-op, three distinct field states, structural presence is
  not content-scope correctness, replacement (not coexistence) as the real success bar.
- **Git workflow** — never commit directly to `main`; branch and PR for every change.
- **Use the tool on itself** — track multi-step work as a `milestone`/`bug` via `urzua new`, not
  scratch notes; verify a cross-reference actually resolves via `urzua graph` before treating linked
  work as done; `urzua new`'s assigned number is provisional, not reserved (`MILE-89`); check
  declared config before hand-writing a header or field.
- **Verify before trusting** — run the actual command against real input rather than reading code
  and assuming it behaves as documented; a new rule needs an observed-failing planted-violation test
  before it needs anything else.
- **Never silently rewrite an Accepted decision** — a correction after `Accepted` gets a dated
  Amendment or a new narrowing/superseding record, never a silent edit to Decision/Consequences.
- **A record's Status is a human decision, not yours to set** — filing a record isn't deciding it;
  covers any field asserting a decision was reached on someone's behalf, not just the literal
  `Status` key.
- **Don't build speculative capability** — ship the smallest thing a real, evidenced case needs; the
  same discipline applies to *discovering* an existing shim whose justification no longer holds, not
  just to refraining from writing new ones.
- **Don't patch around a finding on your own new work** — a live finding (on code you wrote, code
  you're touching, or code you merely noticed) is signal to fix or file, never to quiet by widening a
  config list or ignore-list.
- **Before calling anything done** — `make ci` passes locally; a changeset exists for anything a
  person installing `urzua` would care about, per `ADR-29`.

## How it's maintained

Edited in place, like a spec — not append-only the way an ADR's Decision text is frozen. Unlike a
`spec`-type record, there's no mechanical enforcement of this (no `Version`/revision-log fields on
`AGENTS.md` itself to check); this spec's own revision log is where the "why this changed" history
actually lives, since the file being governed can't carry it.

**Every PR that changes `AGENTS.md` substantively adds a matching entry to this spec's own revision
log, in the same PR** — not as a follow-up. This is the mechanism for "track what's happening with
`AGENTS.md` over time" that the file itself can't provide on its own; skipping it here is exactly
the kind of gap this project's own history keeps finding elsewhere (a decision landing without the
spec describing its mechanism getting updated in the same pass — see `MILE-90`'s own `Why` for the
same pattern applied to a different feature area).

## What's deliberately not built

- **Making `AGENTS.md` itself a governed record type** — it would need a home directory, a header,
  and a `Status` that mean something (a single-instance type is a real pattern already used
  elsewhere? not currently, in this corpus) — not attempted here; this spec's own revision log is a
  sufficient history for a file this size, at this frequency of change, without inventing a new
  record-type instance for a single file.
- **A `check` rule verifying `AGENTS.md`'s content matches what this spec describes** — would require
  parsing prose for section presence, the same permanent content-scope ceiling SPEC-1 already names
  for structural-vs-content checking generally.

## References

- MILE-36 — wrote `AGENTS.md`; this spec's `Implements` pointer.
- ADR-41 — the "coherent feature area, decided editorially" principle this backfill follows, the
  same one that produced SPEC-6/9/10/16/17/18.
- RFC-4 — "RFC → ADR → Spec: the common path, not the only path" — the precedent for skipping
  the ADR stage when no real decision existed to record.
- SPEC-15 — `doctor`, the other feature area in this corpus with no ADR behind it, for the same
  reason.
- SPEC-1 — the source `AGENTS.md`'s "What urzua is" section restates; read that spec directly for
  the full success criteria and acceptance-suite bug classes this backfill only summarizes.
- MILE-89 — the display-number-race limitation `AGENTS.md` now names explicitly.
- BUG-9 — a real instance of the "flag a stale shim you find, don't just avoid writing new ones"
  guardrail `AGENTS.md` now states as a general rule.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial spec. **Why:** `AGENTS.md` was written directly by MILE-36 with no spec of its own — found live reviewing the dashboard, the same "implemented without any spec" gap ADR-41's review already found for `milestone`/`bug`/`waiver`/`adr`/`rfc`/`spec`, just for a file rather than a record type. | **structural** |
> | 2026-09-09 | `AGENTS.md` gained two sections: "A record's Status is a human decision, not yours to set" and "Don't patch around a finding on your own new work" (plus an extension to "Don't build speculative capability"). **Why:** two real, repeated agent mistakes this session — setting an RFC's `Status` without being asked, and widening `.urzua/config.toml`'s `known_fields` to silence a warning on a newly-created file instead of fixing the actual gap. | **substantive** |
> | 2026-09-09 | `AGENTS.md` gained "What urzua is, and what it refuses to be" (grounding agent behavior in `SPEC-1`'s own constraints, not just process rules) and "Git workflow" (never commit directly to `main`), and had three existing rules broadened after an adversarial review found real loopholes: Status-ownership now covers decision-proxy fields, not just the literal key; "don't patch around a finding" now covers findings on touched-not-authored code; "don't build speculative capability" now covers discovering an existing stale shim, not just writing new ones. **Why:** the git-workflow gap is exactly how an agent came to edit files directly on `main` earlier the same session; the loopholes were found by deliberately construing the scenario where each rule's prior wording didn't actually stop a slightly-different-shaped version of the mistake it was written for. | **substantive** |
> | 2026-09-09 | A second, independent adversarial review of the previous entry's own changes found a real, self-defeating defect: `AGENTS.md` told agents to check `pointer_fields`/`narrative_fields` in `.urzua/config.toml` as declared config, but those keys don't exist yet (`RFC-23`/`ADR-44` propose them; `MILE-90` hasn't built them), and `RecordTypeConfig` uses `deny_unknown_fields`, so following the instruction literally would break the build. Corrected to name only the two keys that exist today, with an explicit note on the proposed-not-built third axis. Also replaced an unfalsifiable "check for a PR number collision" instruction with two concrete manual commands, since no automated check exists yet (`MILE-89`). **Why:** this project's own "verify before trusting" discipline applies to `AGENTS.md`'s own claims about the codebase as much as to any other document — this backfill exists partly because that discipline had never been applied to `AGENTS.md` itself until an adversarial review did it after the fact, not before merge. | **substantive** |
> | 2026-09-09 | Backfilled this spec's "What belongs in `AGENTS.md`" section (stale since `Version: 0.1`, describing 3 sections against the file's actual 9) and added the standing rule that every substantive `AGENTS.md` change adds its own entry here, in the same PR — closing the exact gap that let the four rows above accumulate only after the fact, in one backfill, instead of one at a time as each change shipped. | **structural** |
