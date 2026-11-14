---
Stable-Id: 01M6R3NK5GHR70MNKQTXHQJGJB
Version: '0.25'
Date: 2026-09-08
Status: Shipped
Embodiment: Verified
Realized-by: code:.urzua/config.yaml@sha256:cc2f13533f7a9f3d16818203956c77d9dc7d918873079840ba4a0792a8d6cc43, test:rust/crates/urzua-cli/tests/spec_shipped_integration.rs@sha256:b03ddbc2eacaccee634b27b070f66f6a730197313c3b114a882becc342acb7a8
Author: beauwilliams
Subject: 'The `spec` record type -- schema, required fields (including this one, `Subject`), and lifecycle.'
Parent: —
Implements: ADR-10
---
# SPEC-18 — The `spec` record type

## Purpose

`spec` is one of the three founding record types — the type this very document is an instance of.
Distinct from `adr` (a frozen, point-in-time decision fact) and `rfc` (a proposal not yet decided):
a spec is a **living, current-truth document**, edited in place under ADR-14's model — a substantive
or structural change is a `Version` bump plus a Why-bearing revision-log entry on the same spec, its
own number permanent per subject (ADR-14's amendment). Backfilled alongside `SPEC-16`/`SPEC-17`,
closing the last of the three founding types' schema-spec gap (ADR-41, ADR-43).

## Schema

```yaml
record_types:
  spec:
    dir: "docs/specs"
    required_fields: ["Status", "Date", "Version", "Author", "Subject"]
    header_shape: "yaml-frontmatter"
    relation_fields: {date: "Date", author: "Author", title: "Subject"}
    known_fields: ["Stable-Id", "Embodiment", "Realized-by", "Derives-from", "Implements", "Parent", "Last-audited"]
    statuses:
      values: ["Draft", "Accepted", "Shipped"]
      success: ["Accepted", "Shipped"]
      implementable: ["Accepted", "Shipped"]
      initial: ["Draft"]
      transitions:
        Draft: ["Accepted"]
        Accepted: ["Shipped"]
      require:
        - {when: ["Shipped"], fields: ["Embodiment", "Realized-by"]}
    sections:
      from: headings
      required:
        - {name: Purpose}
    revision_log:
      marker: "**Revision log**"
      required: true
      version_bump: {field: "Version", classes: ["substantive", "structural"], initial: "0.1"}
    spec: "SPEC-18"
    pointer_fields: ["Derives-from", "Implements", "Parent"]
    narrative_fields: []
```

`Status` is mechanically validated against `statuses`: `status.undeclared-value` reports a value outside
the three, and `status.transition-allowed` reports a move the graph does not allow (on a `--base` run,
against the merge base's copy). The lifecycle is one-way: a spec is filed `Draft`, becomes `Accepted`, then
`Shipped`, and no move goes back or skips a step. `sections` requires a `Purpose` on every spec.

| Field | Values | Notes |
|---|---|---|
| `Status` | `Draft` \| `Accepted` \| `Shipped` | `Shipped` (`MILE-7`) says the work the spec describes is built, in the order `Draft`, `Accepted`, `Shipped`. A `Shipped` spec must carry `Embodiment` and `Realized-by` (`statuses.require`), the evidence for the claim; a `Draft` or `Accepted` spec may omit both. `Shipped` is a success and implementable status, like `Accepted`. Only a human moves a spec to `Shipped`. Unlike `adr`/`rfc`, a spec's `Status` doesn't track `Rejected`/`Superseded` states — a spec that's wrong gets corrected in place (that's the whole point of the living-document model), not superseded the way a frozen ADR would be. |
| `Date` | `YYYY-MM-DD` | The spec's original creation date — stays fixed across revisions (confirmed against `SPEC-1` through `SPEC-15`: none update `Date` on a version bump, only the revision log's own dated rows track when each change landed). |
| `Version` | `0.1`, `0.2`, ... | Bumped on every substantive or structural revision (ADR-14's amendment). |
| `Author` | a real identity, no `@` prefix | Resolved automatically by `urzua new`, same as `adr`/`rfc` (MILE-74's accountability decision, MILE-78's precedent). Required as of version `0.2` — backfilled onto every pre-existing spec that lacked it. Written without a decorative `@` prefix (BUG-18, SPEC-16); note MILE-78's backfill predates that and used the now-retired `'@handle'` form, corrected corpus-wide by BUG-18. |
| `Subject` | one line, free text | What feature, record type, schema, or practice the spec is actually about — readable at a glance without opening `Purpose`. No cross-reference or backlink semantics; not resolved by any rule. Required (`MILE-91`), backfilled onto every pre-existing spec. |
| `Stable-Id` | a ULID, optional | Assigned unconditionally by `urzua new spec` (ADR-21: every type gets one), first exercised by `SPEC-19` — no earlier spec was ever created through the tool. |
| `Embodiment` / `Realized-by` | optional; required once `Status` is `Shipped` | `SPEC-1`'s own field, kept as a project-wide convention rather than treated as its historical baggage (MILE-74). Declared together (`companion_fields`), `Embodiment` is computed from the `Realized-by` locators and checked by `embodiment.consistency`; `statuses.require` makes both mandatory at `Shipped`. |
| `Last-audited` | `YYYY-MM-DD`, optional | The day the spec's evidence was last audited. `embodiment.consistency` counts its drift grace window from this date instead of from the cited file's last change (`drift_audit_field`, `MILE-166`), since a spec's file changes for reasons unrelated to its evidence. With `embodiment.claim-expired` (`SPEC-3`, `ADR-97`) it is also the date the claim was last re-verified, counted from by the claim's expiry, and written only by `urzua reverify` (`SPEC-8`); one date, two readers, and a repository may name a different field for each. A date in the future is malformed. The `adr`, `milestone` and `bug` types declare it the same way when a repository opts in. No spec here carries it yet. |
| `Derives-from` / `Implements` | comma-separated, optional | A spec's real lineage (which ADR/RFC/bug it realizes or derives from) lives here. Declared as two of this type's `pointer_fields` entries (MILE-90/ADR-44) — clean, comma-separated references only, format-enforced by `header.pointer-field-clean`. |
| `Parent` | a single spec ID, or `—`, optional | **Not** a default root pointer — a genuine narrowing/split-off relationship to a broader spec whose own prose documents the split (e.g. `SPEC-2`–`5`/`8`/`11`–`15` really were split out of `SPEC-1`, per its own child-spec table). A spec whose real lineage is already expressed by `Implements`/`Derives-from` uses `—`, not `SPEC-1` by default (`BUG-10`, `MILE-91`) — declared as this type's third `pointer_fields` entry; `pointer.resolution` still validates a non-`—` value resolves, but resolving is not the same as being correct, and nothing mechanically checks that. |

MILE-74 decided this field set: `Author` required (matching `adr`/`rfc`'s accountability model, not
`SPEC-1`'s historical-artifact reading), `Embodiment`/`Derives-from` a project-wide convention rather
than `SPEC-1`-specific, `Implements`/`Parent` declared since every spec already carries them.
`MILE-91` added `Subject` and corrected `Parent`'s real meaning above, per `BUG-10`'s audit.

**The version bump is checked.** This repository declares `revision_log.version_bump` on the `spec`
type with the classes `substantive` and `structural` (`BUG-51`), so `urzua check --base` reports a
changed spec that adds a row of either class while its `Version` equals the merge base's. A row of
any other class, a new spec, or an unchanged log is not reported.

## What's deliberately not built

- **Mechanical enforcement of "replayable end-to-end"** — ADR-14's amendment requires a spec's body
  to stay a complete, self-sufficient specification of its subject at every revision; nothing
  currently checks this mechanically (and likely can't — it's a content-quality property, the same
  permanent ceiling SPEC-1 already names for structural-vs-content-scope checking generally).

## References

- ADR-10 — the decision this spec details: `adr`/`rfc`/`spec` as config-declared profiles of one
  unified core schema, not types the tool's source code hardcodes.
- RFC-1 — the proposal ADR-10 decided; the unified core-plus-profile schema `spec` is an instance of.
- RFC-6 — the original living-spec proposal.
- ADR-14 — the append-only-revision-log model (and its amendment: permanent numbers, real Why,
  replayable-end-to-end bodies) this type's entire lifecycle follows.
- ADR-33/42 — the `yaml-frontmatter` migration this spec's `header_shape` reflects.
- ADR-41 — the spec-organization principle this backfill follows.
- ADR-43 — the `type.no-declared-spec` rule whose live finding for `spec` this spec closes.
- MILE-74 — decided this spec's field set (`Author` required, `Embodiment`/`Derives-from` project-
  wide, `Implements`/`Parent` declared) and built `.urzua/templates/spec.md`.
- MILE-81 — the field-set-consistency rule that found `SPEC-1`'s own outlier shape live.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial spec. **Why:** `spec` — one of the three founding record types, and the type this very document is an instance of — never got a schema spec; found live via `type.no-declared-spec`'s own inventory check (ADR-43), alongside the same gap for `adr`/`rfc`. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-08 | Corrected: the previous bump changed `config.yaml` and this spec's own header but left the Schema section, field table, and "what's deliberately not built" describing the old, undecided state -- rewrote them to match the actual decided schema (`known_fields` declared, `known_fields`-is-undeclared section removed). Added `Implements: ADR-10` -- the actual decision record for `adr`/`rfc`/`spec` as configured profiles, missed in favor of a `Derives-from: RFC-1` pointer that skipped past it. | **substantive** |
> | 2026-09-08 | Added `Stable-Id` to `known_fields`. **Why:** found live creating the first-ever tool-generated spec (`SPEC-19`) -- `render_synthetic_yaml` assigns every type a `Stable-Id` unconditionally (ADR-21), but no `spec` had ever been created through `urzua new` before MILE-74 made that possible, so this gap sat unexercised until now. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`; also corrected `Parent` from `SPEC-1` to `—` (`BUG-10`): this spec's real lineage is already stated via its own `Implements`/`Derives-from`, not a narrowing of `SPEC-1`'s v0-CLI scope. | **structural** |
> | 2026-09-09 | Declared `pointer_fields = ["Derives-from", "Implements", "Parent"]`/`narrative_fields = []` in config (MILE-90/ADR-44), replacing the "resolved regardless of per-type declaration" text above -- that was accurate before this version but became false the moment `pointer.resolution` became config-driven. | **substantive** |
> | 2026-09-11 | Bumped to `0.7`. **Why:** `Author`'s row stated the resolution mechanism but not the written form, and cited MILE-78 as precedent -- MILE-78 being the record that established the `'@handle'` form BUG-18 has now retired, so the citation pointed at the superseded convention. Both corrected. Also fixed "required as of this version", which was written when this spec was at `0.2` and silently re-pointed at every later version. | **substantive** |
> | 2026-09-17 | Config moves from `.urzua/config.toml` to `.urzua/config.yaml` (`ADR-52`, shipped in the same change). The described mechanism changes, not just its rendering. | **substantive** |
> | 2026-11-04 | Bumped to `0.8`. Added `Shipped` to the `Status` values after `Accepted`, with `Embodiment` and `Realized-by` required of a `Shipped` spec. **Why:** `MILE-7`, maintainer decision of 2026-11-04: a spec whose status says the work shipped has to carry its evidence, and `required_fields` could only demand it of every spec or none. Declared in `.urzua/config.yaml` (`statuses` and `statuses.require`); no engine change. | **substantive** |
> | 2026-11-04 | Bumped to `0.9`. Stated that the `Version` bump with every substantive or structural entry is now checked on a `--base` run, by `revision-log.version-bump-required` with `revision_log.version_bump` declared on this type. **Why:** `BUG-51`, maintainer decision of 2026-11-04: only the entry half of the rule was enforced. | **substantive** |
> | 2026-11-05 | Bumped to `0.10`. Realized-by names the implementing code and tests, ready for `Shipped` (`MILE-7`). **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/spec_shipped_integration.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Bumped to `0.11`. Status changed from Accepted to Shipped. **Why:** Maintainer decision, 2026-11-05: its Realized-by evidence computes Verified and an independent review judged it sound. | **substantive** |
> | 2026-11-05 | Bumped to `0.12`. The schema excerpt now shows the type as configured: `Realized-by` and `Last-audited` among the known fields, the `statuses` declaration (`initial`, the one-way `Draft` to `Accepted` to `Shipped` `transitions`, and `require` of `Embodiment` and `Realized-by` at `Shipped`), the required `Purpose` section and the `version_bump` declaration. A `Last-audited` row is added and the `Embodiment` / `Realized-by` row no longer says none but `SPEC-1` use them. **Why:** the excerpt predated `Shipped` and the lifecycle, so it did not show that the order cannot be reversed or what a `Shipped` spec must carry. | **substantive** |
> | 2026-11-07 | Bumped to `0.13`. The schema excerpt declares `relation_fields.title: Subject` and `version_bump.initial: "0.1"`, so `urzua new spec` writes the title as the `Subject` and `0.1` as the `Version` and the scaffold passes `check` as written. **Why:** `MILE-74` -- a scaffolded spec failed `field.quality` on a blank `Version` and `Subject`. | **substantive** |
> | 2026-11-07 | Bumped to `0.14`. The `Last-audited` row says it is also the anchor of a claim's expiry (`embodiment.claim-expired`), written only by `urzua reverify`, and that the `adr`, `milestone` and `bug` types declare it the same way on opt-in. **Why:** `MILE-129`, the build of `ADR-97`; `ADR-97` section 2. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: .urzua/config.yaml@sha256:151830e5926d, rust/crates/urzua-cli/tests/spec_shipped_integration.rs@sha256:b03ddbc2eaca Reason: Pinned the files as they are now after checking each against the record it evidences: config.yaml and the own-config test still declare the adr, rfc, spec and condition types that SPEC-16, SPEC-17, SPEC-18 and SPEC-23 describe, with spec_shipped_integration.rs, rules/roadmap.rs, condition_type_integration.rs and work_types_integration.rs behind them, check.rs, rules.rs, check_integration.rs and fail_on_integration.rs still implement urzua check (SPEC-2), the workflows, the install-knope action and the release and gate test scripts still make up the pipeline SPEC-20 describes, the Makefile and checks.yml still dispatch the repository layout of SPEC-21, the harness modules, the bundled hook script and the harness integration tests still implement the request, verdict and adapter layout of SPEC-24, config.rs and the own-config and field-levels tests still implement SPEC-3, and acceptance.rs and acceptance_property.rs still implement SPEC-4. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 151830e5926d -> 0b59c6dfc8b9. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. config.yaml: config.yaml gained Kind among the milestone type's known fields and a kind declaration (build and decision) under statuses.delivery.items (ADR-96 amendment of 2026-11-07), and every other declaration the record relies on is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.15`. The pins on this spec's `Realized-by` locators were re-baselined against the files as they are after the sharded CI and the milestone `Kind` amendment landed on main (the `Re-baselined` row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97`: every stale pin is re-pinned with its real delta before expiry is enabled. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0b59c6dfc8b9 -> 0a00038a6388. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: this change declares status.done-items-shape (error), status.waived-item-stale (warn) and embodiment.claim-renewal-unattested (warn) and declares relation.cross-type-disallowed, section.shape and diff.source-and-records-mixed as off; every other declaration is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.16`. The pins this spec's `Realized-by` holds on files changed in this change were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. **Why:** a revision-log entry that adds a structural change needs a `Version` bump (`revision-log.version-bump-required`). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0a00038a6388 -> 3d51c2fac8b7. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: only the comment above the three rules declared off changed, to say they report `not-enabled` exactly as an undeclared rule does; no declaration changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 3d51c2fac8b7 -> 653bb0f54ca6. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: the only change is a new optional pending list (two placeholder patterns) under record_types::milestone statuses.delivery.items; nothing else in the file changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 653bb0f54ca6 -> 594d115541e3. Reason: config.yaml: read the diff since the previous pin; the only change is one added line, outside_writes: shared, under the harness key (this repository declares the new harness.outside_writes option, ADR-89 amendment of 2026-11-09). No record type, rule, status, relation or other declaration this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.18`. The pin this spec's `Realized-by` holds on `.urzua/config.yaml` was re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 594d115541e3 -> 2f0e023ed5a5. Reason: Read the diff of each pinned file or span since the old pin against what this record evidences. .urzua/config.yaml: embodiment.claim-expired gains `drift: {code: block, test: block, spec: acknowledge}` and its explanatory comment; nothing else in the file changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.19`. The pin this spec's `Realized-by` holds on `.urzua/config.yaml` was re-baselined (the `Re-baselined` row above) after the config gained the `drift` declaration under `embodiment.claim-expired`; no behaviour this spec describes changed. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 2f0e023ed5a5 -> cbb1c67c40a0. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: read the diff since the old pin; the only change is the added key authority.decider_table: base with its two-line comment (this repository declares the new decider_table option, ADR-89 amendment of 2026-11-09). No record type, rule, status, relation or other declaration this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.20`. The pins this spec's `Realized-by` holds on `.urzua/config.yaml` were re-baselined (the `Re-baselined` row above) after the config gained `authority.decider_table: base`; no behaviour this spec describes changed. **Why:** the version-bump rule requires a bump beside a structural revision row. | **structural** |
> | 2026-11-09 | Bumped to `0.21`. Re-pin rows in this revision log were reworded to name who re-verified, matching the tool's wording since 2026-11-09; hashes, reasons and dates are unchanged. **Why:** Maintainer decision, 2026-11-09. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` cbb1c67c40a0 -> 21e9ab57a37a. Reason: Read the diff of .urzua/config.yaml: the authority section gains `waivable: never` and a two-line comment; no other key changed. The pinned evidence still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.22`. Evidence pins on `.urzua/config.yaml` were re-pinned (row above) after the config's authority section gained `waivable: never`; the specification's content is unchanged. **Why:** `ADR-89` amendment of 2026-11-09 (gate waivers). | **substantive** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 21e9ab57a37a -> dd656d471f38. Reason: Read the diff of each pinned file since the previous pin. config.yaml gained section.duplicate, section.level-skipped and section.order at error under rules, and unique_headings, no_skipped_levels and order (from: template) under each type's sections, with depth: 2 added where a type had none; the change is additive; the claim holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Bumped to `0.23`. The pinned `.urzua/config.yaml` evidence was re-baselined for `MILE-104`; the text of this specification is unchanged. **Why:** the revision log gained a structural re-baseline row, which `revision-log.version-bump-required` asks a spec to answer with a version bump. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` dd656d471f38 -> 08ab457062ac. Reason: Read the diff of each re-pinned file against main and the claim still holds. config.yaml: the rules table gains one entry, code.dangling-bug-reference (level error, back_pointer_paths rust and scripts, extensions rs sh py, external_ids RFC-822 and RFC-4122, ignore_marker citation-fixture) with its comment; no other declaration changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Bumped to `0.24`. Evidence pins re-baselined for this change (the `Re-baselined` row above); the specification text is unchanged. **Why:** a revision-log entry that adds a structural change bumps `Version`. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 08ab457062ac -> ddff3b920ab0. Reason: Read the diff of each pinned file since the previous pin. .urzua/config.yaml: one line is added to the milestone type's waiver_categories, declaring the scope-changed-by-decision category (no cite, no expiry); no other key changed, so the record_types::milestone section differs only by that entry. AGENTS.md: one paragraph is added under 'A record's Status: who sets it' about quoting maintainer decisions and waiving a changed Done-when item; no other line changed. The claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Bumped to `0.25`. Evidence pins re-baselined for the `scope-changed-by-decision` waiver category added to `.urzua/config.yaml` (the `Re-baselined` row above); the specification text is unchanged. **Why:** a revision-log entry that adds a structural change asks for a version bump (`revision-log.version-bump-required`). | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` ddff3b920ab0 -> cc2f13533f7a. Reason: Pinned file(s) changed by the symlinked-record refusal: config.yaml: declares identity.record-symlink-escapes: error. The record's claim is unaffected by these changes. (Re-applied after merging main.) **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
