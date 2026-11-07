---
Stable-Id: 01M6R3NK5BCRKD027XA81J0Q8Q
Version: '0.13'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/waiver.rs, code:rust/crates/urzua-core/src/rules/waiver.rs, test:rust/crates/urzua-cli/tests/waiver_stale_integration.rs
Author: beauwilliams
Subject: 'The `waiver` record type -- time-boxed, scoped exceptions to a check finding.'
Implements: ADR-11
Parent: —
---
# SPEC-10 — The `waiver` record type

## Purpose

A reviewed exception to a rule, as a first-class record — never a config-level ignore list. An
ignore list is unreviewable and grows forever; a waiver is a normal record with a reason and an
optional expiry, reviewable in the same diff as everything else. A waived finding stays listed in
`check`'s output — only excluded from the blocking exit code — so a waiver never makes a problem
invisible, only non-blocking.

## Schema

```yaml
record_types:
  waiver:
    dir: "docs/waiver"
    role: suppression
    required_fields: ["Rule", "Scope", "Reason"]
    header_shape: "yaml-frontmatter"
    known_fields: ["Stable-Id", "Expires", "Category", "Justified-by"]
    spec: "SPEC-10"
    pointer_fields: ["Justified-by"]
    narrative_fields: []
    # Optional (ADR-86): where a waiver without Expires may take effect.
    permanent:
      - rule: status.delivery-agreement
        reason_field: Category
        categories:
          - {name: docs-or-workflow-only}
          - {name: performance-only}
          - {name: test-removed-by-decision, cite: required}
          - {name: behaviour-preserving-refactor}
          - {name: unreachable-failure}
        cite_field: Justified-by
```

`role: suppression` (`SPEC-3`) is what makes this type the suppression type; the type's name is
free. When no type declares the role, the type named `waiver` is read as the suppression type, so
a config written before the declaration keeps working.

| Field | Values | Notes |
|---|---|---|
| `Rule` | a rule id, e.g. `field.quality` | Which rule this waiver covers. |
| `Scope` | a file path, or `*` | `*` covers every file the rule examines for this waiver's `Rule`. |
| `Reason` | free text, required | Why this exception is reviewed and accepted, not left to drift — the whole reviewability point of not using an ignore list. |
| `Expires` | `YYYY-MM-DD`, optional | No expiry means the exception is asserted **structural** (the same shape as SPEC-1's permanent content-scope ceiling). A stated expiry means the waiver **reverts to blocking automatically** once passed — no separate "expired" state anyone has to notice or configure. Where the type declares `permanent`, a waiver without `Expires` takes effect only under the conditions in "Permanent waivers" below. |
| `Category` | one of the declared vocabulary, required of a waiver without `Expires` once `permanent` lists its rule | Why the exception is permanent, in the repository's own vocabulary (this repository's is in the Schema block). The engine names no category; the field is whichever one the `permanent` entry's `reason_field` names. |
| `Justified-by` | a record id, a pointer field | The record that justifies a permanent waiver, required by a category declared `cite: required` (here `test-removed-by-decision`, citing the RFC that removed the test). Resolved by `pointer.resolution` like any pointer field, and a cited record that becomes `Superseded` is reported by `pointer.target-status` where that rule lists the field. |

One relationship field (MILE-90/ADR-44, amended by ADR-86): `Justified-by` is declared in
`pointer_fields`, so a waiver can name the record that justifies a permanent exception. Before
`ADR-86` the type declared both `pointer_fields` and `narrative_fields` as empty (explicit zero, not
omitted, satisfying `config.pointer-declaration-missing`); `narrative_fields` remains `[]`. A waiver
still names a rule and a scope first; the citation is optional and required only by a category that
declares it.

No `Status` field: a waiver's lifecycle is entirely computed from `Expires` (`is_active(today)`),
not separately authored — unlike every other record type, there is nothing for a human to declare
that isn't already implied by the date.

## Matching semantics

A waiver **covers** a finding when `Rule` matches the finding's rule id exactly, and either `Scope`
is `*` or matches the finding's file path exactly (`urzua-core::waiver::Waiver::covers`). A waiver
with a passed `Expires` is treated as though it doesn't exist (`is_active`) — `Expires` must be a real calendar date (`YYYY-MM-DD`; `2026-99-99` and `2026-02-30` are not) and is
compared as a date against wall-clock "today," which RFC-15 §1 notes as the first place this
schema reads real time rather than only corpus content. "Today" is the current date in UTC, whatever
the machine's time zone, so a waiver lapses at the same instant for every contributor and in CI; a
waiver whose `Expires` is today's UTC date is still in force.

`check` calls `load_suppressions` (filters discovered records to the type declaring `role: suppression`, else the type named `waiver`, skipping any
missing `Rule` or `Scope` rather than crashing the run — an unparseable waiver waives nothing, which
fails toward more findings, not fewer) then `apply_waivers`, which annotates matching findings with
`waived: <waiver's own path>` and never removes them from the report. `status`/`blocking` are
computed from non-waived findings only.

`Reason` is required by the table above but, by default, only `header.required-fields` enforces it,
and a waiver without one still suppresses. The type may instead declare `effective_requires: [Reason]`
(`SPEC-3`, `BUG-177`): a waiver then takes effect only once each listed field holds a real value,
that is present and neither blank, a placeholder nor pending. Otherwise it suppresses nothing, so its
target finding appears again (failing toward more findings, as for a missing `Rule`), and with
`waiver.matches-nothing` enabled a finding on the waiver names the missing field. The default is
unchanged.

## Permanent waivers

A waiver without `Expires` is permanent (`ADR-86`). Left ungoverned, any rule can be waived forever for
any reason. The suppression type can declare `permanent` (`SPEC-3`) to say, per rule, which reasons
allow it. Once declared, a waiver without `Expires` takes effect only when all of these hold:

1. its `Rule` has a `permanent` entry (declaring the key **closes permanence for every unlisted
   rule**; a repository that declares nothing is unchanged, as is a waiver with an `Expires`);
2. the entry's `reason_field` (`Category`) holds one of the entry's categories;
3. where that category is declared `cite: required`, the entry's `cite_field` (`Justified-by`) names a
   record that resolves.

Otherwise it is **ineffective**: it suppresses nothing, its finding is reported again, and
`waiver.matches-nothing` reports it as `waiver-ineffective` with a message naming whichever
requirement is unmet (the `effective_requires` field, the rule's lack of a `permanent` entry, the
category or the citation). A permanent waiver stays honest through the existing mechanisms: it is
still listed as `waived`, never omitted; it is reported by `waiver.matches-nothing` when its finding
disappears, whatever its expiry; and `check` adds an info notice (subject `permanent-waivers`) with
the number in effect by category, on every run while `permanent` is declared, counted over the whole
corpus. The scope stays one record per waiver; the permanent waiver is not re-examined when its
record is edited later.

## What's deliberately not built

- **Role enforcement on granting a waiver** (does it need the same Decider rigor as an ADR) — named
  as an open question in ADR-11, deferred with RFC-1 §1e's reciprocity work.
- **A `check` rule verifying a waiver's own `Rule` value actually names a real rule id** — a typo'd
  `Rule` value currently waives nothing silently (fails toward more findings, per the design above),
  but nothing flags the typo itself as likely a mistake.

## References

- ADR-11 — the decision this spec details: a waiver as a first-class record, not an ignore list.
- RFC-15 — the original proposal, including the waiver-as-record shape and `Expires`'s wall-clock
  read as this schema's first.
- MILE-59 — shipped the parsing/matching logic (`waiver.rs`) but never actually registered the type
  in `.urzua/config.yaml`, so `urzua new waiver` failed outright until that gap was found and fixed
  the same day this spec was written.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec. **Why:** `waiver` (ADR-11) was documented only as an ADR while `milestone`, the same shape of decision (a configured record type), got a matching spec (SPEC-6) — MILE-77 named this inconsistency and this spec resolves it for `waiver`. | **structural** |
> | 2026-09-08 | Header shape declared as `yaml-frontmatter` (ADR-33/42 amendment) -- `waiver` never declared `header_layout` (no records exist yet to have settled on a sub-format), so nothing to remove here, unlike the other five types. `spec = "SPEC-10"` declared in config, closing `type.no-declared-spec`'s live finding for this type (ADR-43). | **substantive** |
> | 2026-09-08 | Bumped to `0.3`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`; also corrected `Parent` from `SPEC-1` to `—` (`BUG-10`): this spec's real lineage is already stated via its own `Implements`/`Derives-from`, not a narrowing of `SPEC-1`'s v0-CLI scope. | **structural** |
> | 2026-09-09 | Declared `pointer_fields = []`/`narrative_fields = []` in config (MILE-90/ADR-44) — explicit zero, not omitted, since `waiver` is the one type with no relationship fields to declare. **Why:** MILE-90's new `config.pointer-declaration-missing` rule requires every type to make this choice explicit. | **structural** |
> | 2026-09-17 | Config moves from `.urzua/config.toml` to `.urzua/config.yaml` (`ADR-52`, shipped in the same change). The described mechanism changes, not just its rendering. | **substantive** |
> | 2026-11-04 | Bumped to `0.6`. Dropped the reference to the removed `header_layout` option. **Why:** `MILE-85`. | **substantive** |
> | 2026-11-05 | Bumped to `0.7`. Realized-by names the implementing code and tests, ready for `Shipped` (`MILE-7`). **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-core/src/waiver.rs, code:rust/crates/urzua-core/src/rules/waiver.rs, test:rust/crates/urzua-cli/tests/waiver_stale_integration.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Bumped to `0.8`. Status changed from Accepted to Shipped. **Why:** Maintainer decision, 2026-11-05: its Realized-by evidence computes Verified and an independent review judged it sound. | **substantive** |
> | 2026-11-05 | Bumped to `0.9`. The suppression type is declared with `role: suppression` rather than recognised by the name `waiver`, which remains the fallback only when no type declares the role. **Why:** `BUG-74`, maintainer decision of 2026-11-05. | **substantive** |
> | 2026-11-05 | Bumped to `0.10`. The type may declare `effective_requires` (`SPEC-3`): a waiver missing a listed field, or holding a blank, placeholder or pending value, does not suppress, its target finding is reported again, and `waiver.matches-nothing` reports the waiver (`waiver-ineffective`). Off unless declared; this repository requires `Reason`. **Why:** `BUG-177`, maintainer decision of 2026-11-05: the `Reason` is the whole reviewability point and was enforced only by the opt-in `header.required-fields`. | **substantive** |
> | 2026-11-05 | Bumped to `0.11`. The type may declare `permanent` (`SPEC-3`): a waiver without `Expires` takes effect only for a listed rule, under a declared category and, where the category requires it, a resolving `Justified-by`; declaring it closes permanence for unlisted rules, and `check` counts permanent waivers by category. Added the `Category` and `Justified-by` fields; the statement that the type has no relationship fields is amended, `Justified-by` being declared in `pointer_fields`. **Why:** `MILE-193` / `ADR-86`. | **substantive** |
> | 2026-11-06 | Bumped to `0.12`. States that `Expires` is compared against the current UTC date, not the machine's local date. **Why:** maintainer decision of 2026-11-05: every date comparison uses UTC; the engine had read local time here and UTC for the claim-age clock. | **substantive** |
> | 2026-11-07 | Bumped to `0.13`. `Expires` is validated as a real calendar date and compared as a date; a month 13-99 or day 32-99 no longer sorts above every real date, and a value that is not a calendar date expires immediately, the finding saying it is not a calendar date. **Why:** hardening review: `Expires: 2026-99-99` kept a waiver active forever. | **substantive** |
