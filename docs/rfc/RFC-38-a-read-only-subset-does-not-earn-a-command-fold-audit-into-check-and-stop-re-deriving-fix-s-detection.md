---
Stable-Id: 01M30ZA1SK9EGJ4NWN49ZHW0CT
Status: Draft
Date: 2026-09-21
Author: beauwilliams
Supersedes / Superseded-by: —
Implements: SPEC-1
---
# 38 — A read-only subset does not earn a command: fold `audit` into `check`, and stop re-deriving `fix`'s detection

## Summary

`urzua audit` runs two rule functions that `urzua check` already runs, over the same corpus, producing
the same report shape and the same exit codes. `urzua fix`'s detect mode recomputes, in a second
module, the comparison `check`'s `embodiment.consistency` rule already performs. Three commands, one
engine, two overlaps.

This RFC proposes the discriminator `RFC-20` has the axes for but does not state: **writing earns a
command; a read-only subset does not.** `audit` becomes a rule filter on `check` and is deprecated;
`fix` keeps its command because it writes, but its detect mode consumes `check`'s rule result rather
than re-deriving it.

## Motivation

### The overlap is measurable, not stylistic

`check.rs` calls `rules::pointer_resolution` (line 278) and `rules::supersession_reciprocity`
(line 312). `audit.rs` calls the same two functions and nothing else. Both assemble a `CheckReport`
(`ADR-23`), apply waivers identically (`ADR-11`), and use the same exit codes. `audit.rs` is 99 lines,
of which `ADR-30` describes roughly 20 as glue.

Everything `audit` reports, `check` already reports.

### `RFC-20`'s own table shows it, and declares the question out of scope

`RFC-20` classifies every command on two axes. `check` and `audit` land in the **same cell on both**:

| Command | Operates on | Read-only / Writes |
|---|---|---|
| `check` | the corpus | Read-only |
| `audit` | the corpus | Read-only |

Every other pair in that table differs on at least one axis. `audit`'s row distinguishes itself only
in a parenthetical. `RFC-20` then states it *"does not reopen ADR-30"*, so the taxonomy that would
have surfaced the duplication ruled it out of scope.

`RFC-20` also asks, of a different command, exactly the question never asked of this one:

> Is a `--type`/`--format dot` filter on `graph` a new command or a flag? This table treats it as
> within `graph`'s existing row.

Same question, answered "flag" for `graph`.

### It has produced three bugs, and nothing depends on it

`BUG-42` (`audit` bypassed the rules table entirely, running both rules at hardcoded severity),
`BUG-84` (a guard made `audit` unsatisfiable on the configuration `init` generates) and `BUG-99`
(`audit` exits 2 on a prefixless corpus, because both its rules are ones `init` correctly declines)
all trace to one root: `audit` owns a rule set.

No milestone depends on `audit`. `MILE-62` built it and is closed. The only other roadmap references
are `MILE-101` and `MILE-106` citing bugs it caused. Its net contribution to the roadmap is negative.

### `ADR-53` moved the ground under it

`ADR-53` made every rule a declared, opt-in policy. A hardcoded rule subset inside the tool is the
tool overriding the adopter's declaration, which is the thing `ADR-53` exists to prevent. `BUG-42` is
that contradiction surfacing once already; `gated()` fixed the severity half and left the selection
half in place.

### `fix` has the copy `ADR-30` was written to prevent

`ADR-30`'s Context names the risk precisely: *"a naive implementation risks a second,
independently-drifting copy of the same two rules under a different name."* It avoided that for
`audit`. `fix` has it: `urzua_core::fix::detect_repairs` (`fix.rs:36`) re-derives the stated-versus-
computed `Embodiment` comparison that `rules::embodiment_consistency` performs.

The two have **already diverged**, and the code says so:

> Drift (ADR-0032) is deliberately not checked here: it needs the same git-history plumbing `check`
> gets from its caller.

So `check` reports drift and `fix` does not, for the same field, from two implementations.

### Every comparable tool made the other choice

| tool | validate | repair |
|---|---|---|
| ESLint | `eslint` | `eslint --fix` |
| RuboCop | `rubocop` | `rubocop -a` |
| Prettier | `--check` | `--write` |
| clippy | `cargo clippy` | `cargo clippy --fix` |
| ruff | `ruff check` | `ruff check --fix` |

None ships a second read-only command running a subset of the first. `ruff` does ship two commands,
`check` and `format` — genuinely different verbs with different output. That is the line this RFC
proposes drawing.

## Proposal

### 1. The discriminator

**A command is earned by a distinct verb or a distinct output shape, not by a narrower rule set.**
Concretely: a read-only operation over the corpus producing a `CheckReport` is `check`, however it is
filtered.

Applied to the current surface, this changes two rows and confirms the rest:

| command | verb | verdict |
|---|---|---|
| `check` | validate, read-only | keep |
| `audit` | validate, read-only — identical | **fold into `check`** |
| `fix` | repair, writes | keep; writing is the distinct verb |
| `new`, `migrate` | create/transform, writes | keep |
| `init` | writes config, not corpus | keep |
| `doctor` | subject is the tool | keep |
| `explain`, `graph` | different question, different shape | keep |
| `export`, `import` | format conversion | keep |

### 2. `check` gains rule selection

`check --rules <id>[,<id>…]` restricts the run to the named declared rules. A rule not in the
config's `rules` table stays `not-enabled` — selection narrows what runs, it never enables anything,
so `ADR-53` holds.

`audit`'s behaviour becomes `check --rules pointer.resolution,relation.supersession-reciprocity`.

**The filtered report's contract, which is not a detail.** `RuleExecution.status` has two values today,
`ran` and `not-enabled`, and both already mean something. A filtered run has a third population —
declared, enabled, and excluded by this invocation — and it must not borrow either name. Reporting an
excluded rule as `not-enabled` would say the repository declined a policy it declared, which is the
class of untruth this release spent itself removing.

So: an excluded rule is **omitted from `rules_executed` entirely**, and the report gains a top-level
`rules_selected` naming the filter that produced it. Omission plus the filter is recoverable;
`not-enabled` is not. `rules_executed` then holds exactly the two entries `audit` reports today, which
is the behaviour being replaced — and deprecated `audit` emits `rules_selected` as well, so the two
reports agree in shape and not merely in contents (see §3).

A requested rule the config does not declare is an **invocation error, exit 2** — asking for a policy
that does not exist is a mistake in the command, not a finding about the corpus. Selection narrows
what runs and never enables anything, so `ADR-53` holds.

`status` and the exit code are computed over the selected set exactly as they are today over the full
set, with no special case.

Whether a named group (`--rules cross-record`) should exist is an open question below; it requires
rules to declare a category, which this RFC does not decide.

### 3. `audit` is deprecated, not deleted in place

One release where `audit` runs as today and emits a `Notice` naming the replacement invocation
(`ADR-46`: a `Notice` never moves the exit code, so no adopter's build breaks on the warning). Removed
the release after.

**Deprecated `audit` emits `rules_selected` too**, naming its own two rule ids. `audit` has always
been a fixed selection; the field states which one rather than leaving a consumer to infer it from the
length of `rules_executed`. The point is that a consumer migrating from `audit` to
`check --rules …` sees **no schema change at all** — the deprecated command and its replacement emit
the same shape, so the migration is the invocation and nothing else.

Without this the equivalence claim below is ambiguous: `audit` would emit one shape and its stated
replacement another, and "reproduces `audit`'s two-entry report exactly" would be true of
`rules_executed` and false of the report. Adding the field to `audit` during deprecation is a strictly
additive change to a command already being retired, which is the cheapest place to absorb it.

This is pre-1.0 and `config.rs` records that there are *"no real external adopters yet whose behavior
a hardcoded fallback would need to preserve"* — the cheapest moment this decision will ever have.

`BUG-99` closes when `audit` is removed rather than being separately fixed. Its three candidate fixes
were all patches to a command this RFC proposes retiring.

### 4. `fix` detect consumes the rule, not a second copy

`fix`'s detect mode should stop re-deriving the comparison. **The direction is settled; the mechanism
is not, and three obstacles are named here rather than waved at.** They are why this half proposes a
goal and defers its design instead of claiming a change that has not been worked out.

1. **A `Finding` does not carry the computed value.** `build_fix_report` needs `Vec<Repair>`, and a
   repair is the *computed* `Embodiment` to write. Recovering it by parsing a finding's message would
   make that message a wire format. So the rule must return a structured computed value alongside its
   finding — a `rules.rs` signature change touching every rule's return type, or a new per-rule
   payload. Neither is free.

2. **`check` gates the rule; `fix` does not.** `check.rs:319` runs `embodiment_consistency` through
   `gated(&config, ...)`. If `fix` consumed the rule's result, a config declaring
   `embodiment.consistency: off` would silently produce no repairs, where today `fix` repairs
   regardless. That is a real behaviour change. It is arguably *correct* under `ADR-53` — but whether
   repair is governed by the same declaration as detection has never been decided.

3. **`check` computes `drifted`; `fix` does not.** The rule takes a `drifted: &HashSet<PathBuf>` built
   from git history by `check`'s caller. `fix` has no such plumbing, which is exactly why
   `detect_repairs` skips drift and says so in a comment. Sharing the rule means either giving `fix`
   that plumbing or passing an empty set and getting a different answer from the same function — a
   *new* divergence wearing the shape of a fix.

**So the divergence is named here, not closed.** An earlier draft of this RFC claimed sharing the rule
would give "one answer about drift". That was wrong: the difference is in the rule's *input*, not in
there being two implementations.

`fix`'s command status is unaffected either way. It writes.

## Open questions

- **Should rules declare a category** (`cross-record`, `per-record`, `config`) so `--rules
  cross-record` is expressible? It is the tidier end state and it is how `audit`'s intent would
  survive as data rather than as a hardcoded pair. It is also a schema change, and this RFC does not
  need it — an explicit rule list reproduces `audit` exactly today. Deliberately left open.
- **How does a rule return a structured computed value** alongside its findings, without making every
  rule's signature carry a payload most of them have nothing to put in? This gates the `fix` half
  entirely.
- **Is repair governed by the same declaration as detection?** If `fix` runs through `gated()`, then
  `embodiment.consistency: off` disables repair too. `ADR-53` suggests yes; nobody has decided it, and
  it is a behaviour change either way.
- **Does `fix` gain git-history plumbing, or accept a narrower answer than `check`** and say so in its
  report? Passing an empty `drifted` silently is the one option this RFC rules out.
- **Where does the command taxonomy live once settled** — `RFC-20` already asks this and does not
  answer it. If that RFC moves out of `Draft`, this one should be read against it.
- **Is one deprecation release enough**, given there is no telemetry on who invokes what? There are no
  known external adopters, but "known" is doing work in that sentence.

## Non-goals

- **Does not propose removing `fix`.** It writes; that is a distinct verb and it earns a command. Only
  its detect-mode duplication is in scope.
- **Does not decide `fix`'s capability model.** `RFC-19` owns that.
- **Does not change any rule's behaviour.** No finding appears or disappears from `check`; this is
  about which command surface reaches them. That holds for the `audit` half. The `fix` half *would*
  change `fix`'s behaviour, which is why it is proposed as a direction with its obstacles named rather
  than as a settled design.
- **Does not ship in `0.4.0`.** That release already carries a breaking report-contract change
  (`BUG-40`). A CLI-surface break wants its own release and its own migration note.

## References

- `RFC-20` — the command taxonomy whose axes this RFC applies to its own table.
- `ADR-30` — `audit` reusing `check`'s rule functions; its Context names the drift risk `fix` now has.
- `ADR-53` — every rule is a declared policy, which a hardcoded subset overrides.
- `ADR-23`, `ADR-46` — one report shape, and `Notice` never moving the exit code.
- `BUG-42`, `BUG-84`, `BUG-99` — the three bugs rooted in `audit` owning a rule set.
- `SPEC-1`, `SPEC-2`, `SPEC-8`, `SPEC-11` — the command surface as specified today.
- `MILE-62` — built `audit`; closed, and nothing depends on it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-21 | Deprecated `audit` emits `rules_selected` as well. **Why:** the equivalence claim was ambiguous for a consumer reading the report as a schema rather than as a list. `audit` has always been a fixed selection, so naming it costs nothing, and it means migrating from `audit` to `check --rules` changes the invocation and not the shape of what comes back. | **substantive** |
> | 2026-09-21 | Defined the filtered report's contract; downgraded the `fix` half from a settled design to a named direction. **Why:** review found three things the first draft asserted without working out. An excluded rule had no status that was not already taken, and reusing `not-enabled` would have reported a declared policy as declined. And sharing `embodiment_consistency` with `fix` was claimed to close the drift divergence and change no behaviour; it does neither, because `check` gates the rule and supplies a `drifted` set `fix` has no plumbing to build. The divergence is in the rule's input, not in there being two implementations. | **substantive** |
> | 2026-09-21 | Filed. **Why:** `BUG-99` was filed against `audit`'s empty-rule-set edge case, and examining it showed the edge case was a symptom: `audit` is a read-only subset of `check` that outlived `ADR-53`, and `fix` carries the duplicate-implementation risk `ADR-30` was written to avoid. Filed as an RFC rather than three bug fixes because the question is which commands the surface should have, not how to patch the ones it has. | **substantive** |
