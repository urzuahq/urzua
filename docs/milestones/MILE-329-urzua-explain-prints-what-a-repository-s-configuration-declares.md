---
Stable-Id: 01M7KN9RDHSDPVS80PTSTRR984
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 329 — urzua explain prints what a repository's configuration declares

## What

Add a read-only form of `urzua explain` that prints, from the configuration alone, what this repository declares for its record types, short enough for an agent to read at the start of a session.

## Why

On release 0.4.7 an agent new to a repository cannot learn its record model from the tool. `urzua explain` takes a path or a rule id; `urzua rules` reads no configuration; `urzua doctor` reports health; `urzua export` and the read-only server's record listing (`SPEC-26`) give each type's name, directory and record count and nothing more; the ordering command assumes the types are already understood. The only way to learn which fields a type carries, which values a field may hold, what a reference may point at, which states exist and how a record moves between them is to read the configuration file (336 lines in the reporting repository) and whatever notes the repository wrote for itself. The configuration is the source of truth and the tool already parses all of it; an agent that cannot ask the tool writes its own summary, which then drifts. The maintainer looked for this under `urzua explain` first.

## Done means

- `urzua explain --types` prints every declared record type and `urzua explain --type <name>` one of them, as JSON, read from the configuration and nothing else; the name was accepted by the maintainer on 2026-11-17: "types is more generic and more agent friendly and machine friendly" than a name that calls it a workflow. An undeclared name exits 2 with a `not-run` report.
- For each type the report gives: its identifier prefix and directories; required and optional fields; each field's declared values; each reference field with the types it may name; its states with the initial ones and the allowed moves; each declared requirement and prohibition with its conditions as one sentence, alongside the declared condition keys; any gate on its states or paths with who may act; the template in use; and the exact command that creates a record of the type, shown by a test against a configuration that uses each of these.
- The report lists the rules this configuration sets, with their levels, and separately the rules that are on because of a declaration, each naming the declaring key.
- The report contains no built-in default the configuration did not change, with exactly two exceptions, each marked as a default: a type's initial state, and a rule that is on by declaration. A test on a configuration declaring one type and nothing else shows no other entries.
- Every entry read from the configuration carries the key path and line it was read from, so a reader can go to the declaration; a marked default carries none.
- The report for a three-type fixture configuration committed with the test stays under a byte size the test states.
- The agent harness's session-start report names the command.
- Every value in the report is copied or derived from a configuration key; the only prose is the one-sentence form of a requirement or prohibition.

## Amendment (2026-11-17): four marked entries, a summary by default, the session-start line for the agent

The Done-means item "no built-in default ... with exactly two exceptions" is delivered with four marked entries, each `"default": true` with a `source` and no position: a type's initial state when `initial` is not declared; a rule on by declaration; the effective identifier prefix when `prefix` is not declared, because the prefix decides how a record's id reads; and the conventional template path when that file exists, because the template decides section order where a type takes its order from the template. `urzua explain --types` prints a summary, one compact entry per type and the rule ids with their levels, and points at `urzua explain --type <name>`; `urzua explain --types --full` prints every declaration. The harness's session-start line is passed to the agent alone on a healthy session start and shows the person nothing. These three decisions were made by the orchestrator on 2026-11-17 and accepted by the maintainer on 2026-11-17: "The three decisions you made make sense to me. I'm happy with your decisions".

## Amendment (2026-11-17): which wording the delivered command follows

The delivered command follows the previous amendment's wording, not the Done-means item's "exactly two exceptions": it prints the four marked entries named here, and also marks the engine's built-in pending and placeholder lists where a requirement relies on them; decided by the builder on 2026-11-17, pending maintainer review.

## Amendment (2026-11-17): what the delivered command does not meet

The delivered command does not meet the Done-means item "the rules that are on because of a declaration, each naming the declaring key": it lists only the rules set in the configuration, as written under `rules:`, and does not say which rules apply to a type, because which records a rule judges is decided in each rule's code and three reviews found every mapping kept outside the rules false. The marked defaults it prints are the undeclared initial state, the status `urzua new` writes, an undeclared prefix, the conventional template path and the built-in pending and placeholder lists; no rule is among them. Decided by the orchestrator on 2026-11-17, pending maintainer review.

## Amendment (2026-11-17): statements of earlier amendments the delivered command no longer makes

The delivered command no longer makes these statements of the earlier amendments: that "a rule on by declaration" is one of its marked entries; that the summary shows "the rule ids with their levels"; and that it says anything about which rules apply to a type or whether a declared condition is enforced. Path gates are listed once for the repository, as declared, and not attributed to types. Decided by the orchestrator on 2026-11-17, pending maintainer review.

## Amendment (2026-11-17): the template path is printed with whether its file exists

The fourth marked entry of the first amendment, "the conventional template path when that file exists", is printed whether or not the file exists, with `exists_when_run`, a file-system fact at the time of the run: an agent that reads a template path it cannot tell is there cannot tell a missing template from one the command does not know. The summary also carries, for a type that declares each, `gated`, `field_values`, the condition sentences in place of their count, `revision_log_classes` and a `checklist` marker, each copied from the same declaration `--type` shows (`MILE-340`). Decided by the builder on 2026-11-17, pending maintainer review.

## Amendment (2026-11-17): the template key is `readable_when_run`

The key the previous amendment names `exists_when_run` is `readable_when_run`: it reports whether `urzua new`'s own read of the conventional template file returned text when the command ran, so a file that exists but cannot be read as text, which `urzua new` ignores, reads false. The summary's `gated` entry is `gates`, one entry per gate naming the type, and the summary carries one note that these are declarations (`MILE-340`). Decided by the builder on 2026-11-17, pending maintainer review.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Reported on 2026-11-17 by an agent in an adopting repository at the maintainer's prompting; the maintainer had asked "if another agent comes along, can they figure out what's the workflow quickly through the urzua tool?", expected `urzua explain` to do it, and said a repository should not need its own working document for this. | **structural** |
> | 2026-11-17 | Amendment: the no-default item is delivered with four marked entries, `--types` prints a summary with `--full` for detail, and the session-start line goes to the agent alone. **Why:** decided by the orchestrator and accepted by the maintainer on 2026-11-17: "The three decisions you made make sense to me. I'm happy with your decisions". | **substantive** |
> | 2026-11-17 | Amendment: one sentence says the delivered command follows the amendment's four marked entries rather than the Done-means item's "exactly two exceptions", and that it also marks the built-in pending and placeholder lists. **Why:** review of 2026-11-17 noted the item and the amendment disagree; the item is left as written. | **structural** |
> | 2026-11-17 | The second amendment's sentence on which wording the command follows is marked as decided by the builder, pending maintainer review. **Why:** review of 2026-11-17: the statement is the builder's, not the maintainer's. | **structural** |
> | 2026-11-17 | Amendment: the delivered command does not list rules on by declaration or say which rules apply to a type; the Done-means items are left as written and the Status is unchanged. **Why:** decided by the orchestrator on 2026-11-17 after review found per-type rule ties false three times, pending maintainer review. | **substantive** |
> | 2026-11-17 | Amendment: lists the statements of earlier amendments the delivered command no longer makes (a rule on by declaration as a marked entry, rule ids in the summary, rule applicability, path gates per type). **Why:** decided by the orchestrator on 2026-11-17, pending maintainer review. | **structural** |
> | 2026-11-17 | Amendment: the conventional template path is printed with `exists_when_run` whether or not the file exists, and the summary carries gates by type, field values, condition sentences, revision-log classes and a checklist marker. **Why:** an adopting repository reported on 2026-11-17 that the summary omitted what `--type` shows and that `template` did not say whether its file exists; decided by the builder on 2026-11-17, pending maintainer review. | **substantive** |
> | 2026-11-17 | Amendment: the template key is `readable_when_run` (from `urzua new`'s own template read), the summary's gates are `gates`, one entry per gate, and the summary carries a note that these are declarations. **Why:** review of 2026-11-17 of the pull request that adds them; decided by the builder on 2026-11-17, pending maintainer review. | **substantive** |
