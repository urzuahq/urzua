---
Stable-Id: 01M6R3NK5RYJF3MHRJT1RSA1K9
Version: '0.19'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:.urzua/config.yaml@sha256:dd656d471f380787e18d7a2d2267975552c953858cc33a165c33b0afde087d7e, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs@sha256:dc29b51bde5a2ba79d95c5e0a09466a54e816e8e2cc4a86a80e4459be477a447, test:rust/crates/urzua-cli/tests/own_config_integration.rs@sha256:e60116ae0a7034a14619b4949265b88470479f15e854b37803f4d63a2d17646a
Author: beauwilliams
Subject: 'The `bug` record type -- real, already-found defects and a Status that follows the test evidence in Realized-by.'
Implements: ADR-35
Parent: —
---
# SPEC-9 — The `bug` record type

## Purpose

Tracks a real defect this project's own tooling had — retrospective, not prospective: what was
actually wrong, how it was found, and a named, checkable pointer to the test proving it's fixed, in `Realized-by`.
`bug` is a sibling of `milestone` (SPEC-6) and `waiver` (SPEC-10), all configured record types with
zero `urzua-core` changes, but answers a genuinely different question than either: a milestone is
*planned* work moving toward done, a bug is *what already broke*.

## Why not fold this into `milestone`

Checked directly before building a new type: forcing a bug into milestone's schema (`Status`,
`Phase`, `Track`) either drops the one real governance gain a bug record needs — a named pointer to
the regression test proving the fix — or bolts it on as an optional field nothing requires. A bug
record's whole value is making "nobody verified this is actually fixed" impossible to state by
accident.

## Schema

```yaml
record_types:
  bug:
    dir: "docs/bugs"
    required_fields: ["Status", "Found-in"]
    header_shape: "yaml-frontmatter"
    known_fields: ["Realized-by", "Stable-Id", "Implements", "Blocked-on"]
    statuses:
      values: ["Open", "Fixed", "WontFix", "Not a bug", "Rejected"]
      success: ["Fixed"]
      failure: ["WontFix", "Not a bug", "Rejected"]
      implementable: ["Open"]
      initial: ["Open", "Fixed"]
      transitions:
        Open: ["Fixed", "WontFix", "Not a bug", "Rejected"]
        Fixed: ["Open"]
        WontFix: ["Open"]
        "Not a bug": ["Open"]
        Rejected: ["Open"]
      pending:
        - "pending"
        - "not yet written"
        - {pattern: '(?i)^(not yet written|planned:)'}
      delivery:
        delivered_tiers: ["Verified", "Implemented"]
        unevidenced_tiers: ["Not started"]
        not_done: ["Open"]
        set_status: "Fixed"
        back_pointer_evidence: false
    revision_log: {marker: "**Revision log**", required: true}
    sections:
      from: headings
      required:
        - {name: "What was wrong", aliases: ["What is wrong", "What was suspected", "What"]}
    spec: "SPEC-9"
    pointer_fields: ["Implements"]
    narrative_fields: []
```

| Field | Values | Notes |
|---|---|---|
| `Status` | `Open` \| `Fixed` \| `WontFix` \| `Not a bug` \| `Rejected` | The bug's own lifecycle, independent of any milestone or ADR. `Fixed` is the one success; `WontFix`, `Not a bug` (the report described correct behaviour) and `Rejected` (the report was declined) are the failures. A bug is filed `Open`, or `Fixed` when the PR that files it fixes it (`initial`). `Open` moves to any of the four others; each of them can be reopened to `Open` and no other move is allowed (`status.transition-allowed`). |
| `Found-in` | free text | How/where it was actually discovered — a narrative, not a resolvable reference (e.g. `"check docs/adr/ vs check docs/ compared by hand"`). |
| `Realized-by` | optional, `known_fields` | The evidence field, the same one every other type uses (`ADR-84`). The test proving the fix is a `test:path#name@sha256:<pin>` entry (`urzua explain <path>#<name>` prints the pin); several are comma-separated before each `test:` prefix. A `test:` entry computes to the test tier, a `code:` entry to the code tier. The bug's `Status` follows it, see below. |
| `Stable-Id` | optional, `known_fields` | Assigned by `urzua new bug` or backfilled by `migrate ids` (SPEC-14), same as any other type. |
| `Implements` | comma-separated, optional | Already-generic field (RFC-1) — points at whichever RFC(s)/ADR(s) this bug's fix realizes, when the bug arose from following one. Not yet used by any bug record in this corpus, but declared as this type's one `pointer_fields` entry (MILE-90/ADR-44) — no `narrative_fields` for `bug`. |

### Status follows the evidence (`ADR-84`)

`statuses.delivery` holds a bug's `Status` to its `Realized-by`, as `SPEC-6` does for a milestone. A
bug that is `Open` while its `Realized-by` computes to `Verified` or `Implemented` is a
`status.delivery-agreement` finding, and `urzua fix --apply --ids <bug>` sets `Fixed` with a
structural revision row. A bug that is deliberately open with evidence (a partial fix) is not given a
new status: it is waived with the existing `waiver` record, scoped to the bug's path, with the rule
set to `status.delivery-agreement` and a `Reason`. This type declares `Not started` as an unevidenced
tier, so a `Fixed` bug with no evidence is a `status.delivery-agreement` finding (`ADR-84`: a done
record with no evidence is reported). A bug that legitimately has no test (a documentation, records,
workflow or performance change) is waived with the same `waiver` record, not given invented evidence.
`back_pointer_evidence: false` makes a source back-pointer that cites the bug not count.

The retired `Regression-test` field is no longer known to the type; its values were migrated into
`Realized-by` as locators. A value that named no test (prose, `none`, a test that no longer exists)
is not evidence: the original text of such a record is kept under a `## Regression test` heading in
its body, and its `Realized-by` stays empty.

The first section is `What was wrong`; `What is wrong`, `What was suspected` and `What` are accepted aliases for older records, and `check` discloses which records use one. Placeholder `Realized-by` and similar values (`pending`, `not yet written`, `planned: ...`) are the type's `pending` vocabulary, not evidence.

No `Phase`/`Track` — a bug isn't planned or grouped the way a milestone is; it either exists because
something broke, or it doesn't exist yet.

## What's deliberately not built

- **Severity or priority fields.** Not named as a real gap yet — `Status` alone has been sufficient
  for every bug this corpus has recorded. Add only once a real case needs to distinguish urgency.
- **A generated bug-index view**, for the same reason SPEC-6 declines one for milestones: build once
  `urzua check docs/bugs/` stops being sufficient to read the backlog, not before.

## References

- ADR-35 — the decision this spec details: `bug` as a configured type, zero `urzua-core` changes.
- ADR-34/SPEC-6 — the `milestone` precedent this type is a sibling of, and the reasoning it follows.
- ADR-11/SPEC-10 — the `waiver` precedent, same footprint.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec. **Why:** `bug` (ADR-35) was documented only as an ADR while `milestone`, the same shape of decision (a configured record type), got a matching spec (SPEC-6) — MILE-77 named this inconsistency and this spec resolves it for `bug`. | **structural** |
> | 2026-09-08 | Header shape changed from `blockquote` to `yaml-frontmatter`, `header_layout` removed (ADR-33/38 amendments); `spec = "SPEC-9"` declared in config, closing `type.no-declared-spec`'s live finding for this type (ADR-43). | **substantive** |
> | 2026-09-08 | Bumped to `0.3`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`; also corrected `Parent` from `SPEC-1` to `—` (`BUG-10`): this spec's real lineage is already stated via its own `Implements`/`Derives-from`, not a narrowing of `SPEC-1`'s v0-CLI scope. | **structural** |
> | 2026-09-09 | Added `Implements` to `known_fields`, declared `pointer_fields = ["Implements"]`/`narrative_fields = []` in config (MILE-90/ADR-44), realizing this spec's own field-table text above (a bug can point at the decision its fix realizes) as a checked config declaration, not just documented prose. **Why:** MILE-90 closes `BUG-8`'s claim that relationship fields are config-declared generically -- `bug` was one of the types that hadn't actually declared this axis yet. | **substantive** |
> | 2026-09-17 | This type's `[record_types.*]` declaration re-rendered in YAML (`ADR-52`, shipped in the same change). The same declaration, same keys, same values -- only the syntax differs. | **structural** |
> | 2026-11-05 | Bumped to `0.6`. Added `Realized-by` naming the implementing code and the tests that exercise this spec: `a_bug_needs_found_in_and_regression_test_and_a_what_was_wrong_section`. **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`; the tests run this repository's own `.urzua/config.yaml`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs, test:rust/crates/urzua-cli/tests/own_config_integration.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Status changed from Accepted to Shipped and bumped to `0.7`. **Why:** maintainer decision of 2026-11-05 to ship the specs whose implementation is verified; `Embodiment: Verified` with `Realized-by` naming the implementing code and tests. | **structural** |
> | 2026-11-05 | Bumped to `0.8`. The `Regression-test` field is retired: its values moved into `Realized-by` as `test:` locators, `required_fields` is `Status` and `Found-in`, and the type declares `statuses.delivery` (`ADR-84`), so a bug's `Status` follows its `Realized-by`. A deliberately open bug with evidence, and a `Fixed` bug with no evidence, are each a waiver. **Why:** `ADR-84` makes `Realized-by` the one evidence field for every type, accepted by the maintainer on 2026-11-05. | **substantive** |
> | 2026-11-05 | Bumped to `0.9`. The schema excerpt and the `Status` row now show all five statuses (`Not a bug` and `Rejected` beside `Open`, `Fixed`, `WontFix`), the `initial` set, the reopen `transitions`, the `pending` vocabulary, the `sections` requirement and the revision-log declaration, as configured. The `0.6` row above names a test that has since been renamed to `a_bug_needs_found_in_and_a_what_was_wrong_section` (`record_types_own_config_integration.rs`); that row is history and stands. **Why:** the spec listed three of the five statuses and none of the lifecycle the engine enforces, so a reader could not tell that `Not a bug` and `Rejected` exist or that a fixed bug can be reopened. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: .urzua/config.yaml@sha256:151830e5926d, rust/crates/urzua-cli/tests/record_types_own_config_integration.rs@sha256:7caa23975490, rust/crates/urzua-cli/tests/own_config_integration.rs@sha256:cba8b084a518 Reason: Pinned the files as they are now after checking each against the record it evidences: init.rs and check_integration.rs still implement and test urzua init adopt mode (SPEC-5), status_model.rs, the own-config test and status_delivery_integration.rs still implement the milestone type and its delivery status (SPEC-6), both fix.rs files and the paths, rebaseline and drift integration tests still implement urzua fix (SPEC-8), and config.yaml and the own-config tests still declare the bug type (SPEC-9). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Bumped to `0.10`. The `code:` and `test:` locators of `Realized-by` are now hash-pinned by `urzua reverify --pin-only` (the `Pinned` row above) and, where the record stated `Drift detected`, its `Embodiment` was recomputed (the tool-authored row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97` section 10: a claim is renewed only over pinned code and test evidence, so the corpus is pinned once before expiry is enabled. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 151830e5926d -> 0b59c6dfc8b9. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. config.yaml: config.yaml gained Kind among the milestone type's known fields and a kind declaration (build and decision) under statuses.delivery.items (ADR-96 amendment of 2026-11-07), and every other declaration the record relies on is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0b59c6dfc8b9 -> 0a00038a6388, `rust/crates/urzua-cli/tests/own_config_integration.rs` cba8b084a518 -> 06fa48e432cb. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: this change declares status.done-items-shape (error), status.waived-item-stale (warn) and embodiment.claim-renewal-unattested (warn) and declares relation.cross-type-disallowed, section.shape and diff.source-and-records-mixed as off; every other declaration is unchanged; own_config_integration.rs: one test added, own_config_declares_a_level_for_every_shipped_rule, which fails when a shipped rule has no declared level in the repository config; the existing tests are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.11`. The pins this spec's `Realized-by` holds on files changed in this change were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. **Why:** a revision-log entry that adds a structural change needs a `Version` bump (`revision-log.version-bump-required`). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` 7caa23975490 -> 44fea1626a54. Reason: Read the diff of each pinned file since the old pin and the claim still holds. record_types_own_config_integration.rs: the milestone fixture's Done means is now a list item instead of a prose sentence (the repository config checks Done means as items), and the helper that swaps in items replaces that item; no assertion changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0a00038a6388 -> 3d51c2fac8b7, `rust/crates/urzua-cli/tests/own_config_integration.rs` 06fa48e432cb -> 92a2b9ceca89. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: only the comment above the three rules declared off changed, to say they report `not-enabled` exactly as an undeclared rule does; no declaration changed; own_config_integration.rs: own_config_declares_a_level_for_every_shipped_rule now checks the parsed config's rule levels instead of text-matching the file; no other test changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 3d51c2fac8b7 -> 653bb0f54ca6. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: the only change is a new optional pending list (two placeholder patterns) under record_types::milestone statuses.delivery.items; nothing else in the file changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 653bb0f54ca6 -> 594d115541e3. Reason: config.yaml: read the diff since the previous pin; the only change is one added line, outside_writes: shared, under the harness key (this repository declares the new harness.outside_writes option, ADR-89 amendment of 2026-11-09). No record type, rule, status, relation or other declaration this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.13`. The pin this spec's `Realized-by` holds on `.urzua/config.yaml` was re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 594d115541e3 -> 2f0e023ed5a5. Reason: Read the diff of each pinned file or span since the old pin against what this record evidences. .urzua/config.yaml: embodiment.claim-expired gains `drift: {code: block, test: block, spec: acknowledge}` and its explanatory comment; nothing else in the file changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.14`. The pin this spec's `Realized-by` holds on `.urzua/config.yaml` was re-baselined (the `Re-baselined` row above) after the config gained the `drift` declaration under `embodiment.claim-expired`; no behaviour this spec describes changed. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 2f0e023ed5a5 -> cbb1c67c40a0. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: read the diff since the old pin; the only change is the added key authority.decider_table: base with its two-line comment (this repository declares the new decider_table option, ADR-89 amendment of 2026-11-09). No record type, rule, status, relation or other declaration this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.15`. The pins this spec's `Realized-by` holds on `.urzua/config.yaml` were re-baselined (the `Re-baselined` row above) after the config gained `authority.decider_table: base`; no behaviour this spec describes changed. **Why:** the version-bump rule requires a bump beside a structural revision row. | **structural** |
> | 2026-11-09 | Bumped to `0.16`. Re-pin rows in this revision log were reworded to name who re-verified, matching the tool's wording since 2026-11-09; hashes, reasons and dates are unchanged. **Why:** Maintainer decision, 2026-11-09. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` cbb1c67c40a0 -> 21e9ab57a37a. Reason: Read the diff of .urzua/config.yaml: the authority section gains `waivable: never` and a two-line comment; no other key changed. The pinned evidence still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.17`. Evidence pins on `.urzua/config.yaml` were re-pinned (row above) after the config's authority section gained `waivable: never`; the specification's content is unchanged. **Why:** `ADR-89` amendment of 2026-11-09 (gate waivers). | **substantive** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 21e9ab57a37a -> dd656d471f38. Reason: Read the diff of each pinned file since the previous pin. config.yaml gained section.duplicate, section.level-skipped and section.order at error under rules, and unique_headings, no_skipped_levels and order (from: template) under each type's sections, with depth: 2 added where a type had none; the change is additive; the claim holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Bumped to `0.18`. The pinned `.urzua/config.yaml` evidence was re-baselined for `MILE-104`; the text of this specification is unchanged. **Why:** the revision log gained a structural re-baseline row, which `revision-log.version-bump-required` asks a spec to answer with a version bump. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` 44fea1626a54 -> dc29b51bde5a, `rust/crates/urzua-cli/tests/own_config_integration.rs` 92a2b9ceca89 -> 5aafdaf17779. Reason: Read the diff of each pinned test file since the previous pin. The repo() fixture helper now also copies the repository's .urzua/templates into the fixture, because the own config declares sections.order from templates; no assertion or test body changed; the claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/own_config_integration.rs` 5aafdaf17779 -> 5ca7a48405d4. Reason: own_config_integration.rs gained the test that the own config's shape accepts a `drift_baseline` on `embodiment.consistency` without declaring one; other tests unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Bumped to `0.19`. Evidence pins re-baselined for this change (the `Re-baselined` row above); the specification text is unchanged. **Why:** a revision-log entry that adds a structural change bumps `Version`. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/own_config_integration.rs` 5ca7a48405d4 -> e60116ae0a70. Reason: own_config_integration.rs: rustfmt wrapped one assert in the test added by this change; formatting only, no behaviour change. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
