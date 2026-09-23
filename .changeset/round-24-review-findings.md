---
default: minor
---

Fixes several real defects found by review, plus a standardization pass on a recurring nondeterminism
class:

- **`HashMap`/`HashSet` iteration-order nondeterminism, standardized.** A declared field set's
  iteration order (feeding `field.untrimmed-value`/`header.field-case-mismatch`'s finding order) is
  now a `BTreeSet`, not a `HashSet` — empirically confirmed flaky before the fix (13/20 separate
  process runs disagreed on finding order for the same unchanged corpus; 20/20 stable after).
  `Config.record_types`/`Config.rules` are now `BTreeMap`, closing the same class at its root — this
  also lets `Config::sorted_type_names()` (round 22's workaround) simplify to a plain `.keys()` call,
  since `BTreeMap` already iterates in sorted order. Every other `HashMap`/`HashSet` in the codebase
  was audited and confirmed genuinely order-independent (pure lookup/containment, never iterated into
  observable output) and left unchanged.
- **`urzua audit` silently used a lossy record index and never ran `identity.collision`**, unlike
  `check` — a real identifier collision was resolved against an arbitrary record with no finding at
  all. `audit` now builds its index the same way `check` does and runs `identity.collision` too. As a
  side effect, `audit` on a prefixless corpus (`BUG-99`) now correctly reports `ok` instead of
  spuriously `not-run`, since `identity.collision` doesn't need a declared prefix to judge a corpus —
  `BUG-99` stays `Open` (the root cause is unrelated and still blocked on `RFC-38`), but its own
  reproduction no longer reproduces.
- `check.rs`'s `claim.status-agreement` enablement check was duplicated across two independent call
  sites; both now share one `claim_status_agreement_setting` function.
- `RecordTypeConfig`'s `header_shape: none` gate (`config.header-none-has-no-required-fields`,
  `config.known-fields-declaration-missing`, and `header.required-fields`'s own check) was duplicated
  three times; all three now call one shared `has_no_header()` method.
- A stale `#[allow(dead_code)]` on `RevisionLogEntry`'s `date`/`change_class` fields removed — both
  are actually read.

No adopter-facing behavior change beyond correctness: verified with the full test suite (each fix has
a planted-violation test, individually confirmed genuinely observed-failing before and passing after),
clippy, `make ci`, and a real-corpus `check` run reporting the same 70 findings before and after.

Three more findings from the same review are filed rather than fixed here (each performance-only or
needing a larger refactor, not correctness-affecting): `BUG-133` (`is_record_reference`/
`parse_record_filename` grammar duplication), `BUG-134` (a per-type-constant field set rebuilt per
record instead of using the existing cache), `BUG-135` (`compute_drifted_records` spawning
uncached git subprocesses per locator per record).
