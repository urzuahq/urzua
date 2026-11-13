---
Stable-Id: 01M6R3NK5F9GRPBMYF7DMG289H
Version: '0.21'
Date: 2026-09-08
Status: Shipped
Embodiment: Verified
Realized-by: code:.urzua/config.yaml@sha256:dd656d471f380787e18d7a2d2267975552c953858cc33a165c33b0afde087d7e, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs@sha256:dc29b51bde5a2ba79d95c5e0a09466a54e816e8e2cc4a86a80e4459be477a447
Author: beauwilliams
Subject: 'The `adr` record type -- schema, required fields, and lifecycle for decision records.'
Parent: —
Implements: ADR-10
---
# SPEC-16 — The `adr` record type

## Purpose

`adr` is one of the three founding record types this project is built around, alongside `rfc` and
`spec` — a point-in-time decision fact, never edited in place once `Accepted`, amended via visible,
dated `## Amendment` sections instead (ADR-14). Backfilled the same way `milestone` (SPEC-6), `bug`
(SPEC-9), and `waiver` (SPEC-10) already were, once ADR-41's review found the three founding types
had never gotten this treatment despite being the oldest and most-used types in the schema.

## Schema

```yaml
record_types:
  adr:
    dir: "docs/adr"
    required_fields: ["Status", "Date", "Author", "Deciders"]
    header_shape: "yaml-frontmatter"
    known_fields: ["Embodiment", "Realized-by", "Stable-Id", "Accepted-on", "Derives-from", "Supersedes / Superseded-by", "Non-reference-ids"]
    statuses:
      values: ["Proposed", "Accepted", "Rejected", "Superseded", "Retired"]
      success: ["Accepted"]
      failure: ["Rejected", "Superseded", "Retired"]
      implementable: ["Accepted"]
      claim_retained: ["Superseded", "Retired"]
      initial: ["Proposed"]
      transitions:
        Proposed: ["Accepted", "Rejected"]
        Accepted: ["Superseded", "Retired"]
      pending:
        - "pending"
        - "unknown"
        - "none recorded"
        - {pattern: '(?i)^(tbd|to be (decided|determined))\b'}
      require:
        - {when: ["Accepted"], fields: ["Deciders"]}
        - {when: ["Accepted", "Superseded", "Retired"], fields: ["Accepted-on"]}
      forbid:
        - {when: ["Proposed", "Rejected"], fields: ["Accepted-on"]}
    sections:
      from: headings
      required:
        - {name: Context}
        - {name: Decision}
        - {name: Consequences}
      exempt_statuses: ["Rejected"]
    spec: "SPEC-16"
    pointer_fields: ["Derives-from"]
    narrative_fields: []
```

`Status` is mechanically validated against `statuses`: `status.undeclared-value` reports a value outside the
five, and `status.transition-allowed` reports a move the graph does not allow (on a `--base` run, against
the merge base's copy). A decision is filed `Proposed` (`initial`), is then `Accepted` or `Rejected`, and
an `Accepted` one is later `Superseded` or `Retired`; no move leaves `Rejected`, `Superseded` or `Retired`.
`require` makes an `Accepted` decision name its `Deciders` (a placeholder in `pending` counts as missing)
and makes a decision that has been accepted, whatever became of it after, carry `Accepted-on`; `forbid`
keeps `Accepted-on` off a decision that was never accepted. `sections` requires `Context`, `Decision` and
`Consequences` on every decision except a `Rejected` one, which made no decision and is set aside on
purpose (`exempt_statuses`).

| Field | Values | Notes |
|---|---|---|
| `Status` | `Proposed` \| `Accepted` \| `Rejected` \| `Superseded` \| `Retired` | `Retired`: an accepted decision whose mechanism was removed with no successor (declared a `failure` and `claim_retained` status, like `Superseded`, so its `Realized-by` evidence stays as history). Terminal once `Accepted`/`Rejected`/`Superseded`/`Retired` — the decision text itself is then frozen; further evolution is a dated `## Amendment` section, never a silent edit (ADR-14). |
| `Date` | `YYYY-MM-DD` | When decided, not when last touched. |
| `Author` | a real identity, no `@` prefix | Resolved automatically by `urzua new` (`resolve_identity()`), same tiering `fix --apply` uses (RFC-2 §2). Written without a decorative `@` prefix: nothing in any crate reads, strips, or validates one, and `@` is a reserved YAML indicator, so a prefixed value is invalid as a plain scalar and has to be quoted to parse at all (BUG-18). The value itself is whatever tier resolved — a `gh` login, a `git config user.name`, or an explicit `--by` — and is free text, so a value needing quoting for its own reasons (a colon, a comma) is still quoted; `render_synthetic_yaml` handles that. |
| `Deciders` | one or more real identities, no `@` prefix | Free text today — not yet tool-resolved (MILE-18, rescoped to note `Author` already is). Same no-`@` form as `Author`, for the same reason (BUG-18). Required, and not a `pending` placeholder, once `Accepted` (`status.required-field-missing`). |
| `Accepted-on` | `YYYY-MM-DD`, optional | The day the decision was accepted, not the day it was written; never earlier than `Date` (`date.field-order`) and well-formed (`date.field-valid`). Required at `Accepted`, `Superseded` and `Retired`, forbidden at `Proposed` and `Rejected`, through `statuses.require` and `statuses.forbid` (`SPEC-3`). |
| `Embodiment` | `Not started` \| `Specified` \| `Implemented` \| `Verified` \| `Drift detected` \| `Inactive` | Optional; computed-vs-stated consistency checked by `embodiment.consistency` (ADR-18) when `Realized-by` is also present. |
| `Realized-by` | categorized locators (`spec:`/`code:`/`test:`), optional | The evidence `Embodiment` is computed from. |
| `Stable-Id` | a ULID, optional | Backfilled by `migrate ids` (ADR-3/21) for records predating it. |
| `Derives-from` | comma-separated, optional | Points at the RFC(s) this ADR decides. Declared as this type's one `pointer_fields` entry (MILE-90/ADR-44) — clean, comma-separated references only, format-enforced by `header.pointer-field-clean`. |
| `Supersedes / Superseded-by` | comma-separated or `—`, optional | Reciprocity checked by `relation.supersession-reciprocity`, status-aware (only binds once the claiming record is itself terminal-accepted). Stays outside the `pointer_fields`/`narrative_fields` axis (own mechanism, own reciprocity check) even though `urzua graph` reports its edges `kind: pointer` (MILE-90/ADR-44) — its values are always clean references, never prose. |
| `Non-reference-ids` | comma-separated exact ids, optional | Ids this record names without citing as a reference (an example, a retracted number): `citation.resolution` does not resolve or report a citation of them in this record only, and reports a listed id the record never cites. Named by the rule's `non_reference_field` option (SPEC-3). Not a pointer field. |

## Why amendments, not revisions

Distinct from `spec`'s living-document model (ADR-14/SPEC-18): an ADR is a record of what was
decided and why, at the moment it was decided. Silently editing that text would erase the ability to
answer "what did we think originally, and when did reality diverge" — the exact property a
decision-record system exists to guarantee. A visible, dated `## Amendment` section is added instead,
the original Decision text never touched. This session amended `ADR-33`, `ADR-34`, `ADR-36`, and
`ADR-38` this way, each preserving the original Decision alongside dated corrections.

## What's deliberately not built

- **Role rules on `Deciders`** (self-acknowledgement, real identity resolution) — named in SPEC-1's
  original design, not yet built (MILE-18, MILE-55).

## References

- ADR-10 — the decision this spec details: `adr`/`rfc`/`spec` as config-declared profiles of one
  unified core schema, not types the tool's source code hardcodes.
- RFC-1 — the proposal ADR-10 decided; the unified core-plus-profile schema `adr` is an instance of.
- RFC-10 — the closed-header model.
- ADR-3 — stable identifiers, `Stable-Id`'s origin.
- ADR-14 — the amendment-not-edit-in-place model this spec's own "why amendments" section explains.
- ADR-18/32 — Embodiment consistency and drift detection.
- ADR-33/42 — the `yaml-frontmatter` migration this spec's own `header_shape` declaration reflects.
- ADR-41 — the "coherent feature area, decided editorially" principle under which this spec was
  written, backfilled for the founding types last.
- ADR-43 — the `type.no-declared-spec` rule whose live finding for `adr` this spec closes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial spec. **Why:** `adr` — one of the three founding record types — never got a schema spec, unlike `milestone`/`bug`/`waiver`, despite being older and more heavily used than any of them; found live via `type.no-declared-spec`'s own inventory check (ADR-43). | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-08 | Added `Derives-from: RFC-1 (Accepted)`. **Why:** `adr` is a founding type decided by RFC-1, not by any single ADR (unlike `milestone`/`bug`/`waiver`, each pointing at the ADR that decided them) -- this spec cited RFC-1 in prose and References but never backlinked it in the header, the same pointer every other type declares. | **substantive** |
> | 2026-09-08 | Corrected: replaced `Derives-from: RFC-1` with `Implements: ADR-10`. **Why:** the previous entry's premise was wrong -- `ADR-10` is exactly the single decision record `milestone`/`bug`/`waiver`'s own specs each point at (`Implements: ADR-N`), just missed when this spec was first written; `adr` is not an exception to that pattern after all. RFC-1 stays cited in References as the proposal ADR-10 decided. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`; also corrected `Parent` from `SPEC-1` to `—` (`BUG-10`): this spec's real lineage is already stated via its own `Implements`/`Derives-from`, not a narrowing of `SPEC-1`'s v0-CLI scope. | **structural** |
> | 2026-09-09 | Declared `pointer_fields = ["Derives-from"]`/`narrative_fields = []` in config (MILE-90/ADR-44), replacing `pointer.resolution`'s prior hardcoded field list. Noted `Supersedes / Superseded-by`'s `urzua graph` `kind: pointer` labeling stays a separate, unchanged mechanism outside this axis. | **substantive** |
> | 2026-09-11 | Bumped to `0.7`. **Why:** `Author`/`Deciders` now state the no-`@` form explicitly (BUG-18). The `@` prefix nearly every hand-typed value in this corpus carried was decorative -- nothing in any crate read, stripped, or validated it -- while being a reserved YAML indicator, so it forced the quoting its own presence necessitated, and disagreed with what `resolve_identity()` emits at every tier. 132 of the corpus's 134 `Author`/`Deciders` values were backfilled to match in the same change; the other two (ADR-37's) were already unprefixed. | **substantive** |
> | 2026-09-17 | This type's `[record_types.*]` declaration re-rendered in YAML (`ADR-52`, shipped in the same change). The same declaration, same keys, same values -- only the syntax differs. | **structural** |
> | 2026-11-04 | Bumped to `0.8`. Added `Retired` to the `Status` vocabulary, declared in `statuses.values`, `failure` and `claim_retained`, and to the `adr` template. **Why:** maintainer decision of 2026-11-04 for `ADR-38`, whose mechanism was deleted with no successor; `Superseded` needs a successor record and does not fit. `Retired` is `claim_retained` because a retired decision's `Realized-by` evidence stays true history. Setting `Retired` on a record is the maintainer's act. The allowed move `Accepted` to `Retired` is not yet declared: no type declares `transitions` in this repository's config. | **substantive** |
> | 2026-11-04 | Bumped to `0.9`. Added the optional `Non-reference-ids` field to `known_fields`: the exact ids a record names without citing as a reference (`BUG-190`, `ADR-75`). **Why:** `ADR-18` writes an example id from another corpus that a real record could otherwise capture. | **substantive** |
> | 2026-11-05 | Bumped to `0.10`. Added `Realized-by` naming the implementing code and the tests that exercise this spec: `an_accepted_adr_must_name_its_deciders_and_enters_proposed`, `adr_supersession_must_be_reciprocated_and_a_retired_decision_is_a_declared_status`. **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`; the tests run this repository's own `.urzua/config.yaml`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Status changed from Accepted to Shipped and bumped to `0.11`. **Why:** maintainer decision of 2026-11-05 to ship the specs whose implementation is verified; `Embodiment: Verified` with `Realized-by` naming the implementing code and tests. | **structural** |
> | 2026-11-05 | Bumped to `0.12`. The schema excerpt now shows the type as configured: `Accepted-on` among the known fields, the `statuses` declaration (`initial`, the `Proposed` to `Accepted`/`Rejected` and `Accepted` to `Superseded`/`Retired` `transitions`, `pending`, and `require`/`forbid` for `Deciders` and `Accepted-on`) and the required `Context`, `Decision` and `Consequences` sections. An `Accepted-on` row is added, and "Mechanical `Status` enum validation: not built" is removed, since `status.undeclared-value` and `status.transition-allowed` run. The `0.8` row's note that no type declares `transitions` is history and stands. **Why:** the spec described an engine and a config that had since moved on, and `Accepted-on` is newly required and forbidden by status in this repository's configuration. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: .urzua/config.yaml@sha256:151830e5926d, rust/crates/urzua-cli/tests/record_types_own_config_integration.rs@sha256:7caa23975490 Reason: Pinned the files as they are now after checking each against the record it evidences: config.yaml and the own-config test still declare the adr, rfc, spec and condition types that SPEC-16, SPEC-17, SPEC-18 and SPEC-23 describe, with spec_shipped_integration.rs, rules/roadmap.rs, condition_type_integration.rs and work_types_integration.rs behind them, check.rs, rules.rs, check_integration.rs and fail_on_integration.rs still implement urzua check (SPEC-2), the workflows, the install-knope action and the release and gate test scripts still make up the pipeline SPEC-20 describes, the Makefile and checks.yml still dispatch the repository layout of SPEC-21, the harness modules, the bundled hook script and the harness integration tests still implement the request, verdict and adapter layout of SPEC-24, config.rs and the own-config and field-levels tests still implement SPEC-3, and acceptance.rs and acceptance_property.rs still implement SPEC-4. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Bumped to `0.13`. The `code:` and `test:` locators of `Realized-by` are now hash-pinned by `urzua reverify --pin-only` (the `Pinned` row above) and, where the record stated `Drift detected`, its `Embodiment` was recomputed (the tool-authored row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97` section 10: a claim is renewed only over pinned code and test evidence, so the corpus is pinned once before expiry is enabled. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 151830e5926d -> 0b59c6dfc8b9. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. config.yaml: config.yaml gained Kind among the milestone type's known fields and a kind declaration (build and decision) under statuses.delivery.items (ADR-96 amendment of 2026-11-07), and every other declaration the record relies on is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0b59c6dfc8b9 -> 0a00038a6388. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: this change declares status.done-items-shape (error), status.waived-item-stale (warn) and embodiment.claim-renewal-unattested (warn) and declares relation.cross-type-disallowed, section.shape and diff.source-and-records-mixed as off; every other declaration is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.14`. The pins this spec's `Realized-by` holds on files changed in this change were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. **Why:** a revision-log entry that adds a structural change needs a `Version` bump (`revision-log.version-bump-required`). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` 7caa23975490 -> 44fea1626a54. Reason: Read the diff of each pinned file since the old pin and the claim still holds. record_types_own_config_integration.rs: the milestone fixture's Done means is now a list item instead of a prose sentence (the repository config checks Done means as items), and the helper that swaps in items replaces that item; no assertion changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0a00038a6388 -> 3d51c2fac8b7. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: only the comment above the three rules declared off changed, to say they report `not-enabled` exactly as an undeclared rule does; no declaration changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 3d51c2fac8b7 -> 653bb0f54ca6. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: the only change is a new optional pending list (two placeholder patterns) under record_types::milestone statuses.delivery.items; nothing else in the file changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 653bb0f54ca6 -> 594d115541e3. Reason: config.yaml: read the diff since the previous pin; the only change is one added line, outside_writes: shared, under the harness key (this repository declares the new harness.outside_writes option, ADR-89 amendment of 2026-11-09). No record type, rule, status, relation or other declaration this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.16`. The pin this spec's `Realized-by` holds on `.urzua/config.yaml` was re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 594d115541e3 -> 2f0e023ed5a5. Reason: Read the diff of each pinned file or span since the old pin against what this record evidences. .urzua/config.yaml: embodiment.claim-expired gains `drift: {code: block, test: block, spec: acknowledge}` and its explanatory comment; nothing else in the file changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.17`. The pin this spec's `Realized-by` holds on `.urzua/config.yaml` was re-baselined (the `Re-baselined` row above) after the config gained the `drift` declaration under `embodiment.claim-expired`; no behaviour this spec describes changed. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 2f0e023ed5a5 -> cbb1c67c40a0. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: read the diff since the old pin; the only change is the added key authority.decider_table: base with its two-line comment (this repository declares the new decider_table option, ADR-89 amendment of 2026-11-09). No record type, rule, status, relation or other declaration this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.18`. The pins this spec's `Realized-by` holds on `.urzua/config.yaml` were re-baselined (the `Re-baselined` row above) after the config gained `authority.decider_table: base`; no behaviour this spec describes changed. **Why:** the version-bump rule requires a bump beside a structural revision row. | **structural** |
> | 2026-11-09 | Bumped to `0.19`. Re-pin rows in this revision log were reworded to name who re-verified, matching the tool's wording since 2026-11-09; hashes, reasons and dates are unchanged. **Why:** Maintainer decision, 2026-11-09. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` cbb1c67c40a0 -> 21e9ab57a37a. Reason: Read the diff of .urzua/config.yaml: the authority section gains `waivable: never` and a two-line comment; no other key changed. The pinned evidence still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.20`. Evidence pins on `.urzua/config.yaml` were re-pinned (row above) after the config's authority section gained `waivable: never`; the specification's content is unchanged. **Why:** `ADR-89` amendment of 2026-11-09 (gate waivers). | **substantive** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 21e9ab57a37a -> dd656d471f38. Reason: Read the diff of each pinned file since the previous pin. config.yaml gained section.duplicate, section.level-skipped and section.order at error under rules, and unique_headings, no_skipped_levels and order (from: template) under each type's sections, with depth: 2 added where a type had none; the change is additive; the claim holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Bumped to `0.21`. The pinned `.urzua/config.yaml` evidence was re-baselined for `MILE-104`; the text of this specification is unchanged. **Why:** the revision log gained a structural re-baseline row, which `revision-log.version-bump-required` asks a spec to answer with a version bump. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` 44fea1626a54 -> dc29b51bde5a. Reason: Read the diff of each pinned test file since the previous pin. The repo() fixture helper now also copies the repository's .urzua/templates into the fixture, because the own config declares sections.order from templates; no assertion or test body changed; the claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
