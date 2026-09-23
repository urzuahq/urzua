---
Stable-Id: 01M36PD9E50TMPKS818MB7DWRX
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 23"
Regression-test: a_post_disambiguation_name_collision_is_refused_not_silently_dropped_observed_failing (rust/crates/urzua-cli/tests/check_integration.rs)
---
# 132 — init's directory-name disambiguation doesn't check the final joined name for a second collision

## What was wrong

`detect_record_types` (`init.rs`) disambiguates a proposed type name when two directories share the
same last path component (`docs/adr` and `legacy/adr` would both propose `adr`): the colliding ones
are rewritten to their full, hyphen-joined path (`docs-adr`, `legacy-adr`). The collision check itself
only compares each directory's *base* name (`names.entry(proposed_name(dir))`), never the disambiguated
name it produces.

If an unrelated, single-component directory already happens to be named exactly what a disambiguated
nested directory would join to -- a flat `x-y` alongside a nested `x/y` that collided with some other
`.../y` and got disambiguated to `x-y` -- both end up with `rt.name == "x-y"`. `render_config_yaml`
then does a plain `types.insert(Value::from(rt.name.as_str()), ...)` with no uniqueness check at all;
`yaml_serde::Mapping::insert` silently overwrites. One entire proposed record type -- and every record
under it -- disappears from the generated `.urzua/config.yaml` with no error, and the config still
loads and passes cleanly, leaving that corpus subtree ungoverned from the very first `init` run.

## Why this needs a decision, not a one-line fix

The narrow fix (assert final-name uniqueness across all proposed types, after disambiguation) is
straightforward in principle, but `detect_record_types` currently returns `Vec<ProposedRecordType>`
unconditionally -- it cannot report a collision without either becoming fallible (a signature change
touching all 4 call sites, 3 of them tests) or the caller re-deriving the same uniqueness check `run()`
already trusts `detect_record_types` to have done. And once a genuine post-disambiguation collision is
detected, there's a real question of what `init` should do about it: refuse to adopt at all until the
adopter reorganizes, or find a further disambiguation strategy (a numeric suffix?) that has its own
edge cases. Not rushed here.

## Fix

The signature question turned out to be moot: `run()` already holds the fully-disambiguated
`Vec<ProposedRecordType>` after calling `detect_record_types`, so the uniqueness check does not need
`detect_record_types` itself to become fallible -- it runs in `run()`, over data it already has,
before `render_config_yaml` is called. On a genuine post-disambiguation collision, `init` now refuses
(`CouldNotRun`, exit 2) naming both colliding directories and the shared name, rather than picking a
further disambiguation strategy -- consistent with this command's existing refusals (an existing
config, zero record-shaped files) and with `ADR-55`: silently dropping a proposed type is a worse
outcome than asking the adopter to reorganize or hand-write the config for this one case.

## References

- The "duplicated type name" comment already in `detect_record_types`, which handles the *first-order*
  collision (shared base name) this bug is the same failure one level deeper on (the disambiguated
  name colliding with something else).
- `ADR-55` -- the defect class this is: `init` reports success (`status: ok`, a config that loads)
  without having actually looked at whether its own output is internally consistent.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** found by a full-release code review; verified `render_config_yaml`'s `types.insert` has no uniqueness guard and the existing disambiguation logic only checks base-name collisions, not the disambiguated name it produces. Left `Status: Open` -- the fix needs deciding `detect_record_types`'s error-reporting shape and what `init` does once a genuine collision is found, not just adding an assertion. | **substantive** |
> | 2026-09-23 | Fixed. The signature concern was overstated: `run()` already has the disambiguated `Vec<ProposedRecordType>` in hand, so the uniqueness check lives there, not in `detect_record_types`. `init` refuses with a named-directories error rather than choosing a further disambiguation strategy. Regression test verified genuinely failing pre-fix (config written with one type silently dropped, exit 0) and passing after. | **substantive** |
