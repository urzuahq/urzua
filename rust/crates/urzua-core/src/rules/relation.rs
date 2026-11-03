use super::*;

/// Rule 5 (Phase 6): `Supersedes`/`Superseded-by` reciprocity. Checking only
/// the forward claim leaves the reverse unguarded -- a record can claim to
/// supersede something that doesn't reciprocally point back, sending a
/// reader of the *target* to a record that denies the relation.
pub fn supersession_reciprocity(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_RELATION_SUPERSESSION_RECIPROCITY;
    let mut findings = Vec::new();

    // A record whose filename yields no id is eligible but unexaminable: the
    // reciprocity test is "does the target name *me* back", which needs an id.
    let slots = declared_slots_for_roles(
        records,
        config,
        &[crate::config::RelationRole::Supersession],
    );

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |record| *record,
        |record| {
            let field = relation_field_name(
                config,
                &record.record_type,
                crate::config::RelationRole::Supersession,
            );
            let Some(id) = record_id(record) else {
                return Outcome::OutOfScope;
            };
            let normalized_id = crate::values::RecordId::new(&id);
            // Unreadable, not absent: the header did not parse, so this slot
            // has no value to classify (BUG-125's shape, `field_quality`'s
            // same guard).
            if record.header.is_unreadable() {
                return Outcome::Unreadable;
            }
            let Some(value) = record.header.get(field) else {
                return Outcome::Absent;
            };
            // `—` is this corpus's written "nothing supersedes this", so the slot
            // was answered. Skipping before counting left the reciprocating half of
            // a correct pair uncounted.
            if crate::values::is_no_value_sentinel(value) {
                return Outcome::Examined;
            }

            for reference in extract_references(value) {
                let Some(target) = index.get(&crate::values::RecordId::new(&reference)) else {
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Error,
                        file: record.path.clone(),
                        line: None,
                        waived: None,
                        message: format!(
                            "{field}: {reference} does not resolve to any discovered record"
                        ),
                    });
                    continue;
                };
                // The target's header not parsing is its own defect, already
                // disclosed by that record's own `Unreadable` population entry
                // (and `header.required-fields`) -- `.get()` returns `None` for
                // an unreadable header exactly as it would for a genuinely
                // absent field, so reading it unguarded would blame the citer
                // for a failure that is entirely the target's.
                if target.header.is_unreadable() {
                    continue;
                }
                let target_field = relation_field_name(
                    config,
                    &target.record_type,
                    crate::config::RelationRole::Supersession,
                );
                let target_value = target.header.get(target_field).unwrap_or("");
                let target_names_back = extract_references(target_value)
                    .iter()
                    .any(|r| crate::values::RecordId::new(r) == normalized_id);
                if !target_names_back {
                    findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: None,
 waived: None,
                    message: format!(
                        "{field} claims a relation with {reference}, but {reference}'s {target_field} does not reciprocally name {id}"
                    ),
                });
                }
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// Rule (`RFC-45`/`ADR-63`): a resolved pointer/narrative reference's target
/// may not declare `Status` at all (`ADR-60`'s gate then silently excludes
/// it from every rule that reads it -- `pointer.target-status`,
/// `narrative-field.stale`). The one place this is diagnosed, rather than
/// each consuming rule inventing its own check, the same shape
/// `header.field-case-mismatch` already is for a declared-field case
/// mismatch (`RFC-40`/`ADR-58`).
pub fn relation_target_status_undeclared(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_RELATION_TARGET_STATUS_UNDECLARED;
    let mut findings = Vec::new();

    let pointer_fields_by_type = pointer_fields_by_type(config);
    let pointer_fields_by_type = &pointer_fields_by_type;
    let narrative_fields_by_type = narrative_fields_by_type(config);
    let narrative_fields_by_type = &narrative_fields_by_type;
    let declared_by_type = declared_fields_by_type(config);
    let declared_by_type = &declared_by_type;

    let slots =
        pointer_and_narrative_slots(records, pointer_fields_by_type, narrative_fields_by_type);

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |(record, _)| *record,
        |(record, field_name)| {
            let result = for_each_resolved_status_target(
                record,
                field_name.as_str(),
                config,
                index,
                |reference, target, target_status_field| {
                    let declared = declared_by_type
                        .get(&target.record_type)
                        .is_some_and(|fields| fields.contains(target_status_field));
                    if !declared {
                        findings.push(Finding {
                            rule: RULE_ID.to_string(),
                            severity: FindingSeverity::Warning,
                            file: record.path.clone(),
                            line: None,
                            waived: None,
                            message: format!(
                                "{field_name}: {reference} resolves, but its type '{}' does not declare {target_status_field} -- its status can never be checked",
                                target.record_type
                            ),
                        });
                    }
                },
            );
            match result {
                Ok(()) => Outcome::Examined,
                Err(outcome) => outcome,
            }
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::*;

    #[test]
    fn a_supersession_slot_declared_only_as_required_is_still_declared() {
        let old = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "> Supersedes / Superseded-by: —\n",
        );
        let new = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "> Supersedes / Superseded-by: ADR-0001\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&["Supersedes / Superseded-by"], None, None, None),
        )]);

        let records = [old, new];
        let index = build_normalized_index(&records);
        let (exec, findings) = supersession_reciprocity(&records, &config, &index);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(population.eligible(), 2, "both records declare the slot");
        assert_eq!(findings.len(), 1, "the claim is one-directional");
    }

    #[test]
    fn a_one_directional_supersession_claim_is_a_reciprocity_violation() {
        // 0002 claims to supersede 0001, but 0001 doesn't reciprocally name
        // 0002 -- a reader of 0001 would never know it was superseded.
        let old = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "> Supersedes / Superseded-by: —\n",
        );
        let new = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "> Supersedes / Superseded-by: ADR-0001\n",
        );

        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], Some(&["Supersedes / Superseded-by"]), None, None),
        )]);
        let records = [old, new];
        let index = build_normalized_index(&records);
        let (exec, findings) = supersession_reciprocity(&records, &config, &index);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(population.eligible(), 2, "both records declare the slot");
        assert_eq!(
            population.examined(),
            2,
            "the reciprocating half answered its slot with the em dash"
        );
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("ADR-0001"));
    }

    #[test]
    fn a_reciprocated_supersession_produces_no_finding() {
        let old = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "> Supersedes / Superseded-by: ADR-0002\n",
        );
        let new = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "> Supersedes / Superseded-by: ADR-0001\n",
        );

        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], Some(&["Supersedes / Superseded-by"]), None, None),
        )]);
        let records = [old, new];
        let index = build_normalized_index(&records);
        let (exec, findings) = supersession_reciprocity(&records, &config, &index);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!((population.eligible(), population.examined()), (2, 2));
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    /// `BUG-125`'s shape, a second instance found alongside
    /// `header_pointer_field_clean`'s by the same review: an unparsed header
    /// must disclose `unreadable`, not fold silently into `absent`.
    #[test]
    fn supersession_reciprocity_discloses_unreadable_on_its_own_observed_failing() {
        let r = record("docs/adr/ADR-1-x.md", "adr", "this is not a header at all");
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], Some(&["Supersedes / Superseded-by"]), None, None),
        )]);
        let records = [r];
        let index = build_normalized_index(&records);
        let (exec, findings) = supersession_reciprocity(&records, &config, &index);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(population.examined(), 0, "{population:?}");
        assert_eq!(population.unreadable(), 1, "{population:?}");
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// The citing record's own header being unreadable is already covered
    /// (`supersession_reciprocity_discloses_unreadable_on_its_own_observed_failing`).
    /// This is the *target*'s header being unreadable: `target.header.get()`
    /// returns `None` for that just as readily as for a genuinely absent
    /// field, so an un-gated `.unwrap_or("")` blamed the citing record for
    /// the target's own parse failure instead of skipping a judgment neither
    /// side's data supports.
    #[test]
    fn a_target_with_an_unreadable_header_is_not_blamed_for_failing_to_reciprocate_observed_failing(
    ) {
        let citer = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "> Supersedes / Superseded-by: ADR-0002\n",
        );
        let target = record("docs/adr/ADR-2-y.md", "adr", "this is not a header at all");

        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], Some(&["Supersedes / Superseded-by"]), None, None),
        )]);
        let records = [citer, target];
        let index = build_normalized_index(&records);
        let (_exec, findings) = supersession_reciprocity(&records, &config, &index);
        assert!(
            findings.is_empty(),
            "the target's own parse failure is not the citer's fault: {findings:?}"
        );
    }

    #[test]
    fn a_type_declaring_a_custom_supersession_field_name_is_read_by_that_name_observed_failing() {
        let old = record("docs/adr/ADR-1-x.md", "adr", "> Replaces: —\n");
        let new = record("docs/adr/ADR-2-y.md", "adr", "> Replaces: ADR-0001\n");

        let config = config_with_types(vec![(
            "adr",
            crate::config::RecordTypeConfig {
                relation_fields: Some(crate::config::RelationFields {
                    supersession: Some("Replaces".to_string()),
                    ..Default::default()
                }),
                ..type_config_pointer(&[], Some(&["Replaces"]), None, None)
            },
        )]);
        let records = [old, new];
        let index = build_normalized_index(&records);
        let (exec, findings) = supersession_reciprocity(&records, &config, &index);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(population.eligible(), 2, "both records declare the slot");
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("ADR-0001"));
    }

    #[test]
    fn an_em_dash_supersession_value_answers_its_slot() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "> Supersedes / Superseded-by: —\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], Some(&["Supersedes / Superseded-by"]), None, None),
        )]);
        let records = [r];
        let index = build_normalized_index(&records);
        let (exec, findings) = supersession_reciprocity(&records, &config, &index);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(
            (population.eligible(), population.examined()),
            (1, 1),
            "the em dash is a written answer, not an unfilled slot"
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn relation_target_status_undeclared_flags_a_resolving_reference_to_an_undeclaring_type_observed_failing(
    ) {
        let target = record("docs/rfc/RFC-1-x.md", "rfc", "> Date: 2026-01-01\n");
        let source = record("docs/specs/SPEC-1-x.md", "spec", "> Implements: RFC-1\n");
        let config = config_with_types(vec![
            (
                "spec",
                type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
            ),
            // "rfc" declares no Status at all -- ADR-60's gate would silently
            // exclude any rule reading it, which is exactly what this rule
            // exists to surface.
            ("rfc", type_config_pointer(&["Date"], None, None, None)),
        ]);
        let records = [target, source];
        let index = build_normalized_index(&records);

        let (exec, findings) = relation_target_status_undeclared(&records, &config, &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("RFC-1"), "{findings:?}");
        assert!(findings[0].message.contains("Status"), "{findings:?}");
    }

    #[test]
    fn relation_target_status_undeclared_is_silent_when_the_target_declares_status() {
        let target = record("docs/rfc/RFC-1-x.md", "rfc", "> Status: Draft\n");
        let source = record("docs/specs/SPEC-1-x.md", "spec", "> Implements: RFC-1\n");
        let config = config_with_types(vec![
            (
                "spec",
                type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
            ),
            ("rfc", type_config_pointer(&["Status"], None, None, None)),
        ]);
        let records = [target, source];
        let index = build_normalized_index(&records);

        let (exec, findings) = relation_target_status_undeclared(&records, &config, &index);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
    }
}
