---
Stable-Id: 01M6R3NK5BCRKD027XA81J0Q8Q
Version: '0.22'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/waiver.rs@sha256:2ac7aae607817d5c346a0e6a9d9c38254a71ccd3dd4a083324ba4819f285c53e, code:rust/crates/urzua-core/src/rules/waiver.rs@sha256:6bd392b9a1d962cdfdd923b5a3135e116aae7f0ef7824833bcf0307a293591aa, test:rust/crates/urzua-cli/tests/waiver_stale_integration.rs@sha256:34de7eb7e0b26307bd3136ca9aaada2395f10aac03ea2f68885f8e0251cbe774
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
    # A waiver with no Reason suppresses nothing.
    effective_requires: [Reason]
    identity: {from: filename, pattern: '^(?P<prefix>WAIVER)-(?P<number>\d{4})-(?P<slug>.+)$'}
    required_fields: ["Rule", "Scope", "Reason", "Stable-Id"]
    header_shape: "yaml-frontmatter"
    known_fields: ["Expires", "Category", "Justified-by", "Variant", "Key-path"]
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
| `Variant` | a finding's action variant, optional | Narrows the waiver to findings of that variant (`rule-table` and `SPEC-2`'s suggested-action table list each rule's variants; `waive --from-report` drafts write the finding's own). A finding of another variant of the same rule in the same file is not covered. Absent or blank covers every variant, as a waiver written before the field did. Required, with `Key-path` as the alternative, of a waiver scoped to the configuration file (see "Narrowing to one finding"). |
| `Key-path` | the keys of a finding's `suggested_action.target.key_path`, optional | Narrows the waiver to the finding at that configuration key, for example `[record_types, milestone, sections, exempt_statuses]`. A YAML list; a header reads it as its items joined by commas, and a comma-separated line is read the same, so a key containing a comma cannot be named. A finding with no key path is not covered by a waiver that names one. |
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
is `*` or matches the finding's file path exactly, and, when the waiver names them, `Variant` equals the finding's action variant and `Key-path` equals its key path (`urzua-core::waiver::Suppression::covers`). A waiver
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
that is present and neither blank, a placeholder nor pending. A value is a placeholder or pending only
as a whole (trimmed) value: it equals a declared `pending` value, the text the type's template
(`.urzua/templates/<type>.md`) gives that field, or, compared case-insensitively, one of the built-in
placeholder words (`TBD`, `TODO`, `name` and the like) or `pending` alone. "Pending the upstream fix" is a real reason; "Pending"
alone, where `pending` is declared, is not, and the finding on the waiver says it is a placeholder, not
that the field is missing. Otherwise it suppresses nothing, so its
target finding appears again (failing toward more findings, as for a missing `Rule`), and with
`waiver.matches-nothing` enabled a finding on the waiver names the missing field. The default is
unchanged.

## Narrowing to one finding

Maintainer decision, 2026-11-14 (the first-run review, finding 1.6): a waiver scoped to `.urzua/config.yaml` matched on rule and file alone, so one written for a single finding (this repository's `WAIVER-0052`, for the milestone type's `WontDo` exemption) also silenced every later `type.dir-matches-nothing` finding in the configuration, a misdeclared directory included, and `waiver.matches-nothing` could not see it because the waiver still matched something. A waiver may therefore name the finding it excuses, by `Variant` and, for a configuration finding, `Key-path`, both already on every finding (`variant` is the rule table's action variant, `key_path` the one `suggested_action.target.key_path` carries). Both are optional on a waiver and a waiver naming neither matches as before, so existing waivers keep working.

For a waiver whose `Scope` is the configuration file, naming one of them is required. A configuration-scoped waiver that names neither stays in effect, so no suppression is withdrawn silently, and `waiver.matches-nothing` reports it (variant `waiver-config-scope-too-broad`, on the waiver's `Variant` field) with the two ways to narrow it. The requirement is for the configuration file only: a waiver scoped to a record, or `*`, is not asked to narrow. The fields are reserved names read by the engine, like `Rule` and `Scope`; a type with a closed field set (`known_fields`) lists them to carry them, and one that does not list them can still write the waivers it wrote before.

## Drafts

`urzua waive --from-report` (`SPEC-25`, `ADR-91`) drafts waivers for the current findings: one per rule and
file, and per finding variant and key path where the type lists `Variant` and `Key-path`, with `Rule`, `Scope`, and those two copied from the findings, a `Reason` holding the pending value, and
no `Expires` key and `Category` and `Justified-by` left blank (`--apply` writes the drafts). The pending value is the first whole-value entry of the
type's `statuses.pending` (a waiver has no `Status`, so the type may declare `pending` alone, without
`values`, `SPEC-3`), else `Pending`. A draft is not in effect: its `Reason` is pending, so its finding is reported
again and `waiver.matches-nothing` reports the draft. The command refuses to run where a draft would take
effect: when the type does not require `Reason`, and when it neither declares `permanent` nor requires
`Expires`, since a `Reason` alone would then make a permanent waiver.

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
> | 2026-11-07 | Bumped to `0.14`. A required field (`effective_requires`) is a placeholder only when its whole trimmed value equals a declared `pending` value or the template's text for it; the word "pending" inside real prose is a real reason, and the built-in placeholder words count only when they are the whole value (`TBD`, `TODO` alone are placeholders; "TBD until the upstream fix ships" is a real reason). `waiver.matches-nothing` says "is a placeholder" or "is a declared pending value" for such a field, and "lacks" only for an absent or blank one. **Why:** maintainer decision of 2026-11-07, from the core-model review. | **substantive** |
> | 2026-11-07 | Bumped to `0.15`. The Schema block matches the declaration this repository runs: `Stable-Id` is a required field rather than a known one, and `identity` and `effective_requires: [Reason]` are shown. **Why:** the block had fallen behind the configuration; `effective_requires` was described in prose only. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/waiver.rs@sha256:b8d83e370c42, rust/crates/urzua-core/src/rules/waiver.rs@sha256:5139d2d975e5, rust/crates/urzua-cli/tests/waiver_stale_integration.rs@sha256:34de7eb7e0b2 Reason: Pinned the files as they are now after checking each against the record it evidences: declaration.rs and config.yaml still declare pointer_fields and narrative_fields per type (MILE-90), config.yaml still requires Subject on specs with checks.yml gating it (MILE-91), document_model.rs, both section.rs files, schema_version.rs and config.yaml still read the declared identity, fields and sections (MILE-98), .git-hooks/pre-push, the Makefile and checks.yml still run urzua check (MILE-99), waiver.rs, rules/waiver.rs and waiver_stale_integration.rs still implement the waiver type SPEC-10 describes, audit.rs and the own-config and check integration tests still implement SPEC-11, new.rs, new_record.rs and new_integration.rs still implement SPEC-12, explain.rs, both graph.rs files and the own-config test still implement SPEC-13, both migrate.rs files and check_integration.rs still implement SPEC-14, and doctor.rs and its tests still implement SPEC-15. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Bumped to `0.16`. The `code:` and `test:` locators of `Realized-by` are now hash-pinned by `urzua reverify --pin-only` (the `Pinned` row above) and, where the record stated `Drift detected`, its `Embodiment` was recomputed (the tool-authored row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97` section 10: a claim is renewed only over pinned code and test evidence, so the corpus is pinned once before expiry is enabled. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/waiver.rs` 5139d2d975e5 -> 040169d8cf3a. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: waiver.rs: split waiver_matches_nothing into the WaiverRun::judge method and the unmet_requirements and waiver_report helpers. Behaviour unchanged, outputs byte-identical (check, check --base and fix --rebaseline JSON compared before and after). **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/waiver.rs` 040169d8cf3a -> 121dd8d67a90. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: waiver.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.18`. The waiver rules' findings and executions are built by the shared `Finding` and `RuleExecution` constructors in `report.rs` instead of repeated struct literals (the re-baselined row above); no behaviour this spec describes changes. **Why:** adding a field to a finding had meant editing every rule module; the constructors make it one edit. | **structural** |
> | 2026-11-09 | Bumped to `0.19`. Re-pin rows in this revision log were reworded to name who re-verified, matching the tool's wording since 2026-11-09; hashes, reasons and dates are unchanged. **Why:** Maintainer decision, 2026-11-09. | **structural** |
> | 2026-11-13 | Bumped to `0.20`. Adds the `Drafts` section: the waiver `urzua waive --from-report` drafts and the refusals that keep a draft ineffective (`SPEC-25`). **Why:** `MILE-205`, `ADR-91` point 5. | **substantive** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/waiver.rs` b8d83e370c42 -> 748f534e72aa. Reason: Read the diff of each re-pinned file against main. waiver.rs: added SuppressionType::field_state, and load_suppressions now calls it; the classification it holds is the code that was inline in load_suppressions, moved unchanged, so which waivers take effect is unchanged. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Bumped to `0.21`. The `Drafts` section says a draft has no `Expires` key, and that `--apply` writes the drafts. **Why:** `MILE-205`, the `--apply` change. | **structural** |
> | 2026-11-14 | Bumped to `0.22`. A waiver may name the finding variant (`Variant`) and configuration key path (`Key-path`) it excuses, matched against the finding's action variant and key path; a waiver scoped to the configuration file must name one, or `waiver.matches-nothing` reports it as `waiver-config-scope-too-broad` while it stays in effect; existing waivers are unchanged. Drafts carry both fields where the type lists them. Maintainer decision, 2026-11-14. **Why:** the first-run review (finding 1.6): `WAIVER-0052`, written for one finding, silenced every `type.dir-matches-nothing` finding in the configuration, and nothing could report it. | **substantive** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/waiver.rs` 748f534e72aa -> 2ac7aae60781, `rust/crates/urzua-core/src/rules/waiver.rs` 121dd8d67a90 -> 6bd392b9a1d9. Reason: Read the diff of each pinned file against main and the claim still holds. rules/waiver.rs: waiver_matches_nothing takes the config file and reports a waiver scoped to it that names neither Variant nor Key-path as waiver-config-scope-too-broad; the expired, ineffective, unknown-rule and unmatched cases are unchanged; waiver.rs: Suppression gained variant and key_path (read from the Variant and Key-path header fields), names and is_narrowed, and covers is now names plus waivability; matching on rule and scope is unchanged for a waiver naming neither field. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
