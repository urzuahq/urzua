---
Stable-Id: 01M6J44PX8CWX5PG5JZK0Z1D70
Status: Accepted
Accepted-on: 2026-11-04
Date: 2026-11-04
Author: beauwilliams
Deciders: beauwilliams
Derives-from: RFC-2
---
# 67 — Evidence pins, a drift grace window and a claim age are opt-in refinements of git-history drift

## Context

`ADR-32` made drift unconditional: a `Realized-by` locator whose last commit follows the commit that
last touched the `Realized-by` line is drifted, and the only way to clear it is to touch the line
again. `RFC-2` proposed the opposite on each point (recorded and current hashes, a grace window, an
attributed refresh) and says its content-hash detection is not built. `MILE-129` (decision recorded
2026-11-04) settles the tension: content-hash pins on evidence, a configurable grace window in days,
and a warning when a claim exceeds a configured age, each opt-in and off by default.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Keep drift unconditional | Nothing to build | A large corpus blocks constantly; no way to say "re-verified, still holds" |
| B. Three independent opt-ins, no new field (chosen) | Behaviour is unchanged until written; reuses the `Realized-by` line and the rule's options; core stays pure | A pin is re-set by hand; no attributed re-baseline command |
| C. Pins as a separate header field | Room for per-locator metadata | A schema change to `Realized-by`, which `ADR-18` kept flat |

## Decision

In the context of `ADR-32`'s unconditional git-history drift, facing a corpus too large to re-verify on
every unrelated commit, we decided: **evidence pins, a grace window and a claim age are three
opt-in refinements, and none changes behaviour until a repository writes it.**

- **Pin syntax.** A suffix on a locator, `code:path@sha256:<64 lowercase hex>`, the narrowest form:
  no new field and no change to the locator grammar for an unpinned locator. Any other text after
  `@sha256:` is not a pin and stays part of the path, so a malformed pin fails `embodiment.locator-exists`
  instead of reading as unpinned.
- **Pin semantics.** A pinned locator drifts when its current content hash differs, and git history is
  not consulted for it. An unpinned locator in the same record keeps the heuristic.
- **Grace window.** `drift_grace_days` on `embodiment.consistency`: drift is not reported while the
  cited file's last commit is newer than the window. It applies to both heuristic and pin drift. A
  commit git cannot date is never held back.
- **Claim age.** `claim_max_age_days` on `embodiment.consistency`: a warning when the commit that last
  touched the `Realized-by` line is older than the limit. It is an additional finding of the same rule,
  not a new rule.
- **Purity.** The CLI computes hashes, commit dates and ages and hands plain data to the pure core,
  as `ADR-32` does for drift.
- **Not decided here.** An attributed re-baseline (`urzua fix` re-pinning with a revision row) is
  not in the `MILE-129` decision and is not built; a pin is re-set by hand.

## Reversibility

Additive and off by default: two optional rule options, one optional locator suffix, one new
parameter on a pure function. Removing the options or the suffix returns the `ADR-32` behaviour.

## Consequences

- A pinned locator's drift is exact, and an unrelated commit never reports.
- A pin is checked against the working tree, so an uncommitted edit to a pinned file reports at once and
  the grace window cannot hold it back (its last commit is old).
- Without a re-baseline command, a changed pinned file is cleared by editing the hash, which has no
  attribution beyond the commit that does it.
- The grace window delays a true positive by design; it trades latency for fewer blocks.

## Amendment (2026-11-04): re-baselining is built, and three behaviours shipped since

*Status of this amendment: accepted 2026-11-04 by the maintainer (drafted 2026-11-04 from the drift review the maintainer approved). The Decision text above is unchanged.*

Statements in the Context-to-Consequences text are no longer true, and three behaviours that change
when a record reports are not recorded. The three opt-ins as decided (pin syntax, grace window,
claim age) are built as written.

- **Attributed re-baselining is built.** "Not decided here" says an attributed re-baseline "is not
  built; a pin is re-set by hand". `urzua fix --rebaseline` (`#238`, `MILE-181`) now lists each stale
  pin with its recorded and current hash and, with `--apply`, rewrites a record's pins and appends an
  attributed revision row. It is gated: it needs `--ids` (a record path) or `--force`, an identity,
  and `--reason`. Option B's con "no attributed re-baseline command" and the Consequences bullet
  "Without a re-baseline command, a changed pinned file is cleared by editing the hash" no longer
  apply. A record with no pins to re-set (an unpinned record) is still pinned by hand, as `SPEC-8`
  describes.
- **A stated `Drift detected` is accepted inside the window (`#182`).** Within `drift_grace_days` a
  record that states `Drift detected` agrees with what is computed, and `urzua fix` leaves it alone.
- **A declared audit date moves the window's anchor (`MILE-166`).** `drift_audit_field` (here
  `Last-audited`) makes the window count from the date a record declares its evidence was audited,
  not from the cited file's last commit.
- **Git failures and shallow clones are reported (`#214`).** A commit git cannot find is an error
  the check reports, not a read of "no drift". `ADR-32`'s amendment records the same change.

## Amendment (2026-11-04): a locator may carry a scope

The pin syntax of the Decision, `code:path@sha256:<64 hex>`, gains an optional scope between the path
and the pin: `code:path#<symbol>@sha256:<hex>` or `code:path#L<a>-L<b>@sha256:<hex>`, so a pin hashes
one item and not the whole file (`ADR-74`, built by `MILE-185`). The Decision text above is
unchanged: a file-level pin keeps its meaning and still hashes the raw bytes of the file. For a scoped
pin the grace window is not counted from the cited file's last commit, since that commit says nothing
about the scope; a scoped pin that no longer matches is drift immediately unless the record declares
an audit date (`drift_audit_field`), which anchors the window as for a file-level pin.

## Amendment (2026-11-16): `urzua fix --conflicts` resolves re-pin conflicts after a merge, and re-pin rows get their own class

*Accepted by the maintainer on 2026-11-16. The Decision text above is unchanged.*

When two branches each re-pin the same record, the second to merge conflicts in the pin, in `Version`
and in the appended revision-log row, and had to be redone by hand (`MILE-284`).

The maintainer was asked: "Should urzua get a command that resolves those conflicts itself: keep both
pull requests' log rows, recompute the pin from the merged code, and add a row saying the pin was
recomputed on merge? It would refuse any record that conflicts in its real content." The answer: "Yes,
add the command".

The maintainer was then asked: "Should fixing conflicts after a merge be one command under `fix`? It
would do both jobs in one run: give a record whose number is now taken the next free number (renaming
the file and rewriting every reference to it), and resolve records that conflict only in their evidence
pins. Anything it can't safely fix, it lists and leaves alone." The answer: "`urzua fix --conflicts`".
The maintainer's words before that: "Maybe there's one go-to command that fixes this whole class of
problems of conflicts... Maybe this is just what the fix command should be used for."

The maintainer was also asked: "Re-pins and spec versions: ... Should a re-pin stop bumping the spec's
version?" The answer: "Yes, stop bumping".

- **What it does.** `urzua fix --conflicts` looks at the records git lists as unmerged during a merge or
  rebase and lists each by kind of conflict. Kind `pin`: when the two sides differ only in the evidence
  field's pin hashes, the version field and rows appended to the revision log, `--apply` keeps both
  sides' appended rows (de-duplicated, in date order), takes the higher version (bumped once more when
  `version_bump` would otherwise fail against either side), recomputes each pin both sides moved to
  different hashes, and appends one row saying the pin was recomputed on merge, naming both sides'
  hashes, the new hash and the identity and actor as every attested row does. An edit only one side made
  elsewhere in a conflicted record, such as another header field, is kept as git merged it.
- **When a pin is recomputed.** Only when the merged file is git's own merge of two attested states: the
  pinned file at our commit hashes, under the pin's scope, to our side's pin; at their commit to their
  side's pin; and the index holds exactly git's automatic merge of the two, with the working tree equal
  to it and the file not unmerged. A file changed on either side without a re-pin, or edited by hand
  during the merge (staged or not), fails one of these, and the record is refused.
- **What it refuses.** A record that conflicts anywhere else (body text, another header field, an
  existing log row), a pin that fails the conditions above, a record with no merge-base, ours or theirs
  version, and one resolved by hand are listed with the reason and left untouched; a refused pin needs an
  ordinary re-pin with a person's reason. Identity is resolved exactly as for `--apply`. A stale pin that
  did not conflict is not touched.
- **The honest-reason rule.** A merge-recomputed pin carries a machine-written row that says no person
  wrote a reason for the merged state. Each side's re-pin row, with its reason, is kept. `check --base`
  lists every pin a pull request recomputed on merge as a notice, so its reviewer can confirm each one;
  nothing enforces that a reviewer does.
- **Re-pin rows get their own class.** A type may declare `revision_log.evidence_class`; the rows of
  `fix --rebaseline --apply`, `reverify --apply` and `fix --conflicts --apply` carry it. Left out of
  `version_bump.classes`, a re-pin no longer bumps a versioned record's version; the row is still
  written. A type that declares none writes `tool_class` as before. Rows already in records keep their
  class.
- **Renumbering** a record whose number the merge made a duplicate will be the command's second kind
  of conflict (`MILE-229`); it is not built yet.

**Addition (2026-11-16), decided by the builder on 2026-11-16, pending maintainer review.** The text
above is unchanged; this adds to it.

- **The residual trust, said plainly.** Each side's pin is trusted as committed, even when no reason
  row accompanies it: the conditions above prove that the file at a commit hashes to that commit's pin,
  not that anyone reviewed the change behind it. And a clean textual merge can combine two attested
  changes into a state neither side reviewed. That is why `check --base` lists every pin recomputed on
  merge.
- **Done-means pins.** A pin in a Done-means item's evidence clause (`ADR-96`) is resolved like one in
  the evidence field, under the same conditions, provided the items of both sides are otherwise
  identical (text, waiver, locators and their order, the order of the items).
- **Index stages.** The record's index stages 1, 2 and 3 must equal the record at a merge base and at
  the two commits; stages written by hand are refused.

## Amendment (2026-11-20): a locator one side changed is taken, and its pin recomputed

The Decision above is unchanged. The addition of 2026-11-16 required the Done-means items of the two
sides to be identical, locators included, so a record where one side narrowed a locator and the other
only re-pinned the old one was refused, and `MILE-284`'s purpose (re-pins should not make a pull
request conflict) went unmet for exactly the records a narrowing touches. The maintainer decided on
2026-11-20, option "Auto-resolve (Recommended)": "Amend SPEC-8/ADR-67: pointer change wins, hash
recomputed from the merged file and recorded in a merge revision row." So, per locator, in the
evidence field and in an item's clause alike: a side that narrowed a whole-file locator to a scope of the same file wins it, the pin is recomputed from the merged file
under the winning locator (each side attested by its own locator and the pinned file merged cleanly),
and the merge row names the locator it replaced. A range or symbol replaced by another scope, a move to another file, and a locator
both sides replaced with different ones are refused with the reason, because the other side's re-pin
does not attest what the new pin would cover. `SPEC-8` ("A locator one side changed")
states it.

## References

- `ADR-32` -- the git-history drift this refines.
- `RFC-2` -- the hash, grace window and attributed refresh proposal.
- `MILE-129` -- the decision this builds.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Amendment added: attributed re-baselining is built (`#238`, `MILE-181`), and `#182`, `MILE-166` and `#214` are recorded. **Why:** drift review of 2026-11-04, approved by beauwilliams; proposed, pending maintainer acceptance. `Status` stays `Proposed` and the Decision text is unchanged. | **substantive** |
> | 2026-11-04 | Status changed from Proposed to Accepted, including the amendment of 2026-11-04. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | `Implements: ADR-67` back-pointers added in `rust/crates/urzua-cli/src/discovery.rs` (the pin branch, the grace window, the audit anchor), `rust/crates/urzua-core/src/fix.rs` (`detect_rebaselines`, `apply_rebaseline`) and `rust/crates/urzua-cli/src/commands/fix.rs` (`run_rebaseline`). Comments only; no behaviour changes. **Why:** the drift review of 2026-11-04 found no back-pointer from the code to this record. | **structural** |
> | 2026-11-04 | Amendment added: a locator may carry a scope between its path and its pin, and a scoped pin has no file-based grace window. **Why:** `ADR-74` (Accepted 2026-11-04) decides the scoped-locator grammar; `MILE-185` builds it. The Decision text is unchanged. | **substantive** |
> | 2026-11-16 | Amendment added: `urzua fix --conflicts` resolves a record whose two sides conflict only in re-pin lines, recomputing a pin only from git's merge of two attested states, with a machine-written row, and refuses any other conflict; re-pin rows may carry a declared evidence class that does not bump the version. **Why:** maintainer decisions of 2026-11-16, quoted in the amendment; built by `MILE-284`. The Decision text is unchanged. | **substantive** |
> | 2026-11-16 | Dated addition to the amendment of 2026-11-16, decided by the builder on 2026-11-16, pending maintainer review: the residual trust stated plainly (each side's pin trusted as committed, a clean merge can combine two attested changes into an unreviewed state), Done-means item pins resolved under the same conditions, and index stages checked against the two commits. The amendment's existing text and the Decision text are unchanged. **Why:** a documentation and security re-check of `fix --conflicts`; real use refused most records for a pin inside a Done-means item. | **substantive** |
> | 2026-11-20 | Amendment added: a locator one side changed is taken and its pin recomputed from the merged file, with the maintainer's chosen option quoted. The Decision text is unchanged. **Why:** `BUG-210`; `SPEC-8` carries the rule. | **substantive** |
> | 2026-11-20 | The amendment of 2026-11-20 is limited to a narrowing within one file; a widening and a move to another file are refused. The Decision text is unchanged. **Why:** review of the pull request showed the other side's re-pin of the old locator does not cover a wider span or another file. | **substantive** |
> | 2026-11-20 | The amendment of 2026-11-20 is limited further to a whole file narrowed to a scope of itself; an old range or symbol is refused. The Decision text is unchanged. **Why:** a second review showed the other side's re-pin of a range or symbol covers that scope in its own file, not the changer's. | **substantive** |
