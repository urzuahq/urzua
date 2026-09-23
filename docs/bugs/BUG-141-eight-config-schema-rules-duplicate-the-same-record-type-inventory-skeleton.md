---
Stable-Id: 01M373DZ7A16RH97KE4VCPHT66
Status: Fixed
Found-in: "MILE-112's duplication sweep"
Regression-test: "none -- pure extraction, no behavior change; each of the 8 rules' own existing tests are unchanged and pass"
---
# 141 — eight config-schema rules duplicate the same record-type inventory skeleton

## What was wrong

Eight rules that examine `Config::record_types` itself rather than any record (`type_no_declared_spec`,
`header_deprecated_shape`, `config_pointer_declaration_missing`,
`config_known_fields_declaration_missing`, `config_pointer_field_not_known`,
`config_relation_field_not_known`, `config_pointer_narrative_overlap`,
`config_header_none_has_no_required_fields`) all shared the identical skeleton, differing only in the
rule id, the per-type check, and the message:

```rust
let type_names = config.sorted_type_names();
let population = census(PopulationUnit::RecordType, type_names, |type_name| {
    let type_config = &config.record_types[*type_name];
    /* differs: the check, pushing Finding(s) */
    Outcome::Examined
});
(RuleExecution { rule: RULE_ID.to_string(), population: Some(population), status: RuleStatus::Ran, examined_records: Vec::new() }, findings)
```

Six of these eight functions' own doc comments already say "the same shape as `X`" for another one in
this same list -- the duplication was self-acknowledged in six places and never extracted. Broader than
the already-tracked `RuleExecution`-construction-tail pattern: this is the whole loop body, not only its
tail.

## Why nothing caught it

Each rule was filed, reviewed, and landed independently across several rounds as its own config
validation, so no single review compared its shape against the seven siblings that came before or
after it.

## Fix

Extracted `schema_inventory_rule(config, rule_id, check: impl FnMut(&str, &RecordTypeConfig, &mut
Vec<Finding>)) -> (RuleExecution, Vec<Finding>)`; each of the 8 rules is now just its own check closure,
with the census/`RuleExecution` construction shared. `type_dir_matches_nothing` was not folded in --
it needs `matched`/`dir_exists`, data this shape has no room for, and forcing it in would either widen
the shared signature for one caller or leave it half-fitting.

No behavior change: the full test suite passes unchanged, and the real corpus reports the same 70
findings.

## References

- `BUG-139`/`BUG-140` -- the other two duplication instances the same sweep found and fixed.
- `SPEC-22` -- the duplication-on-second-occurrence rule this was eight occurrences past.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass, found by MILE-112's duplication sweep. **Why:** eight config-schema rules shared one skeleton, self-acknowledged as "the same shape" in six of their own doc comments but never extracted. No behavior change: full suite and real-corpus finding count unchanged. | **substantive** |
