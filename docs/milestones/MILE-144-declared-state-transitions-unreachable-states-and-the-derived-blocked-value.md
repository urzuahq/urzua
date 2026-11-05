---
Stable-Id: 01M6HZ0VVE907G809JG5R61PH8
Status: Done
Phase: '0'
Track: schema-governance
Implements: RFC-48
Realized-by: code:rust/crates/urzua-core/src/rules/sequence.rs, test:rust/crates/urzua-cli/tests/status_agreement_integration.rs
Depends-on: MILE-27
---
# 144 — Declared state transitions, unreachable states and the derived blocked value

## What

The second half of declared state machines, split from `MILE-132`: a type's declaration also carries
the allowed transitions, and the engine reports:

- a declared state that nothing can reach, or that no record uses (informational);
- a transition not allowed by the declaration, between the record's committed status and its status
  in the change under check;
- a hand-written status that a derived value contradicts (for example a written `Blocked` while the
  derived `blocked` value from `RFC-48`'s sequencing relation is false). This is an error. Under
  `ADR-83` a repository that follows `RFC-48` writes no `Blocked` at all, so the contradiction that
  matters there is a blocked record that states a status claiming progress.

The same primitive covers computed lifecycles such as the embodiment tier, whose order is a
hard-coded `if` chain in `rules.rs` today.

## Why

`MILE-132` declares the vocabulary and what each state requires. Transitions and derived values need a
committed-versus-changed comparison and the sequencing relation, which is more machinery than the
adopter's linter uses: it checks a status enum and per-status fields, and states its lifecycle only in
prose. Splitting keeps the Phase 0 exit on the smaller piece.

## Done means

An undeclared transition between a committed and a changed status is reported; an unreachable declared
state is reported as informational; a record that states a status its type declares as claiming progress
while the derived value says it is blocked is an error, and so is a written blocked status the derived
value contradicts for a type that still declares one (`ADR-83`);
the embodiment tier order is read from a declaration instead of a hard-coded chain.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed at Phase 1, split from `MILE-132`. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter needs only the status vocabulary and status-dependent fields, which stay in `MILE-132` at Phase 0, so transitions, unreachable states and derived blocked move out of the exit. | **substantive** |
> | 2026-11-04 | Noted that a type's `statuses.claim_retained` list (`MILE-138`) can later be derived from the declared transitions (the statuses reachable only from an accepted one) rather than declared by hand. **Why:** maintainer decision of 2026-11-04 introduced the hand-declared list first. | **substantive** |
> | 2026-11-04 | Re-phased from Phase 1 to Phase 0 (pulled into Phase 0 as foundation work); `Status` unchanged. **Why:** Maintainer decision of 2026-11-04 defining Phase 0 as parity on a solid codebase (`SPEC-1`'s Phase 0 exit): it completes `MILE-132` (Phase 0) and replaces the hardcoded embodiment and status vocabulary with a declaration, per `ADR-64`. | **substantive** |
> | 2026-11-04 | Scope note: the hardcoded embodiment state vocabulary is part of this milestone. The five state names are bare literals in `urzua-core/src/rules/embodiment.rs` (`compute_embodiment` and the `Implemented`/`Verified` checks in the consistency rules) and `"Drift detected"` is compared again in `urzua-core/src/fix.rs`. Done means additionally: no state-name literal outside the declaration and its default, and `fix` and the consistency rules read the same declared vocabulary. A typed `EmbodimentState` in core is an acceptable first step before the declaration exists. This is also the vocabulary part of the `MILE-161` note. No separate record is filed. **Why:** the foundation review (finding 5) found the same type knowledge repeated in three places, which `ADR-64` and `SPEC-22`'s extract-on-second-occurrence rule forbid; the field names are already role-resolved, so only the vocabulary is left, and `MILE-144` already owns reading the tier order from a declaration. | **substantive** |
> | 2026-11-04 | Vocabulary half built: the `Embodiment` states and tier order are a top-level `embodiment` declaration in config (`SPEC-3` 0.52), read by `compute`, `fix`, the consistency rules, `sticky_states` and `statuses.delivery`; this repository declares it, and `check` output is byte-identical to before. `Status` stays `Planned`: declared transitions, unreachable states and the derived `blocked` value are not built. **Why:** the scope note above. Declared transitions need a decision no accepted record makes (`RFC-1` 1a proposes them, `ADR-10` deferred them) and the derived `blocked` value needs the `Depends-on` relation (`MILE-27`), so they wait for a Proposed ADR and that milestone. | **substantive** |
> | 2026-11-04 | Transitions built per `ADR-72`: a type's `statuses` declares `initial` and `transitions`; `status.transition-allowed` reports an edge the graph does not list, comparing the merge base's `Status` with the change under check (needs `--base`), and a new record outside `initial`; `status.unreachable-state` reports a declared status no path reaches. This repository declares the lifecycle of its five types. `Status` Planned -> InProgress. **What is left:** the derived `blocked` value, and a written `Blocked` contradicted by it, which need the `Depends-on` relation (`MILE-27`, `RFC-48`); this record stays open until that exists or its Done means is amended to exclude the value. **Why:** `ADR-72` is Accepted, and the Done means are met except the derived value. | **substantive** |
> | 2026-11-05 | `Depends-on: MILE-27` added. **Why:** the 2026-11-04 row states the derived `blocked` value needs the `Depends-on` relation `MILE-27` built (Done). Proposed for maintainer review; `Status` unchanged. | **structural** |
> | 2026-11-05 | Derived `blocked` value built: a type's `statuses` declares `blocked` (the written statuses that mean waiting), `success` is the one list of done statuses, and the opt-in rule `sequence.status-agreement` reports a written blocked status while nothing blocks the record and a blocked record that states an `implementable` status, both errors; documented in `SPEC-2` and `SPEC-3`, and this repository enables it for milestones. `Realized-by` added and `Status` InProgress -> Done. **Why:** `ADR-64` clause 4 and `ADR-76` (the edges are the ones `sequence.cycle` checks); checked against this record's Done means: the undeclared transition, the unreachable state and the embodiment tier declaration were already built, and the contradicted written `Blocked` is now an error. | **substantive** |
> | 2026-11-05 | Rebuilt to `ADR-83` (maintainer decision of 2026-11-05: `Blocked` is derived only, per `RFC-48`). A type's `statuses` declares `progress` (statuses that claim work advanced, `Done` included explicitly) and optionally `blocked` (written waiting statuses, checked both ways); a top-level `roadmap.pairs` names the dependency pairs, so a containment pair declared `acyclic` blocks nothing; `sequence.status-agreement` reports a blocked record that states a `progress` status, and a value that cannot be computed (a dangling or unreadable dependency, an unreadable inverse writer) is counted unreadable, not guessed. This repository declares `progress` and no `blocked`, and its milestone vocabulary loses `Blocked`. Done means amended to say so; `Status` stays `Done`. **Why:** the review of #313 found no decision record for the readings and a departure from `RFC-48`; `ADR-83` is Proposed, and the maintainer accepts it. | **substantive** |
> | 2026-11-05 | The guard for a value that cannot be computed now also covers a record in a `progress` status when its type declares no `blocked` status: such a record with an unresolved or unreadable dependency, or an unreadable inverse writer, is counted unreadable, not examined and clean (`ADR-83`; tests added for a repository declaring only `progress`). `ADR-83` is Accepted. The maintainer accepted the rewritten Done means (a written blocked status is checked only for a type that still declares one) on 2026-11-05; `Status` stays `Done`. **Why:** the review of #313 found the guard ran only for a written blocked status, and the rewritten Done means is a scope change the maintainer consciously accepted. | **substantive** |
