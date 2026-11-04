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
                        declared_severity: None,
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
 declared_severity: None,
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
                            declared_severity: None,
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

/// One side of one declared relation pair on one record whose type declares
/// that side's field -- the population unit of both pair rules (`RFC-47`).
#[derive(Clone, Copy)]
struct PairSlot<'a> {
    record: &'a Record,
    pair: &'a crate::config::RelationPair,
    /// Is this the pair's `forward` side (else its `inverse`).
    forward: bool,
}

impl PairSlot<'_> {
    fn own_field(&self) -> &str {
        if self.forward {
            &self.pair.forward
        } else {
            &self.pair.inverse
        }
    }

    /// The field on the claim's target that must echo this side.
    fn echo_field(&self) -> &str {
        if self.forward {
            &self.pair.inverse
        } else {
            &self.pair.forward
        }
    }
}

/// Every `(record, pair, side)` where the record's type declares the side's
/// field, for the pairs `keep` selects. A type that does not declare a pair's
/// field is not that pair's business, the same gate `ADR-60` gives `Status`.
fn pair_slots<'a>(
    records: &'a [Record],
    config: &'a Config,
    keep: impl Fn(&crate::config::RelationPair) -> bool,
) -> Vec<PairSlot<'a>> {
    let declared_by_type = declared_fields_by_type(config);
    let mut slots = Vec::new();
    for pair in config.relation_pairs.iter().filter(|p| keep(p)) {
        for record in records {
            let Some(declared) = declared_by_type.get(&record.record_type) else {
                continue;
            };
            for forward in [true, false] {
                let slot = PairSlot {
                    record,
                    pair,
                    forward,
                };
                if declared.contains(&crate::values::FieldName::new(slot.own_field())) {
                    slots.push(slot);
                }
            }
        }
    }
    slots
}

/// The references a pair slot claims, or the `Outcome` that ends the slot:
/// no readable id, an unreadable header, an unwritten field (`Absent`), or
/// the written "nothing here" sentinel (answered, so `Examined`).
fn pair_slot_claims(slot: &PairSlot) -> Result<Vec<String>, Outcome> {
    if record_id(slot.record).is_none() {
        return Err(Outcome::OutOfScope);
    }
    if slot.record.header.is_unreadable() {
        return Err(Outcome::Unreadable);
    }
    let Some(value) = slot.record.header.get(slot.own_field()) else {
        return Err(Outcome::Absent);
    };
    if crate::values::is_no_value_sentinel(value) {
        return Err(Outcome::Examined);
    }
    Ok(extract_references(value))
}

fn status_field<'a>(record: &Record, config: &'a Config) -> &'a str {
    relation_field_name(
        config,
        &record.record_type,
        crate::config::RelationRole::Status,
    )
}

fn claimant_status<'a>(record: &'a Record, config: &Config) -> Option<&'a str> {
    record.header.get(status_field(record, config))
}

/// The severity a claim from `record` is held to when the pair declares a
/// provisional split and the claimant's status is one of its statuses.
/// `None` means the rule's own level applies (`MILE-154`).
fn provisional_severity(
    pair: &crate::config::RelationPair,
    record: &Record,
    config: &Config,
) -> Option<FindingSeverity> {
    let prov = pair.provisional.as_ref()?;
    let status = claimant_status(record, config)?;
    prov.statuses
        .iter()
        .any(|s| s == status)
        .then_some(match prov.level {
            crate::config::ProvisionalLevel::Warn => FindingSeverity::Warning,
            crate::config::ProvisionalLevel::Error => FindingSeverity::Error,
        })
}

/// Rule (`RFC-47`, `MILE-122`): reciprocity over every declared relation pair.
/// A claim `A.forward = B` needs `B.inverse` to name `A`, and `A.inverse = B`
/// needs `B.forward` to name `A`; direction is the field name, so two records
/// each claiming the forward side do not satisfy each other. A pair declaring
/// `cross_type: false` has its cross-type claims excluded here (counted out of
/// scope when every claim of a slot is) and reported by
/// `relation.cross-type-disallowed`.
pub fn relation_pair_reciprocity(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_RELATION_PAIR_RECIPROCITY;
    let mut findings = Vec::new();

    let slots = pair_slots(records, config, |p| {
        p.reciprocity == crate::config::PairReciprocity::Required
    });

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |slot| slot.record,
        |slot| {
            let references = match pair_slot_claims(slot) {
                Ok(references) => references,
                Err(outcome) => return outcome,
            };
            let record = slot.record;
            let field = slot.own_field();
            let echo = slot.echo_field();
            let id = record_id(record).unwrap_or_default();
            let normalized_id = crate::values::RecordId::new(&id);
            let mut excluded = 0;
            for reference in &references {
                let Some(target) = index.get(&crate::values::RecordId::new(reference)) else {
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Error,
                        file: record.path.clone(),
                        line: None,
                        waived: None,
                        declared_severity: None,
                        message: format!(
                            "{field}: {reference} does not resolve to any discovered record"
                        ),
                    });
                    continue;
                };
                // Its own `Unreadable` entry discloses this; reading it here
                // would blame the citer for the target's failure.
                if target.header.is_unreadable() {
                    continue;
                }
                if !slot.pair.cross_type && target.record_type != record.record_type {
                    excluded += 1;
                    continue;
                }
                let names_back = extract_references(target.header.get(echo).unwrap_or(""))
                    .iter()
                    .any(|r| crate::values::RecordId::new(r) == normalized_id);
                if !names_back {
                    // A claim from a record not yet at an accepted status is
                    // provisional (`MILE-154`): the pair's own level applies
                    // instead of the rule's. Anything else keeps the rule's.
                    let declared_severity = provisional_severity(slot.pair, record, config);
                    let note = if declared_severity.is_some() {
                        format!(
                            " (provisional: {} is '{}')",
                            status_field(record, config),
                            claimant_status(record, config).unwrap_or_default()
                        )
                    } else {
                        String::new()
                    };
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: declared_severity.unwrap_or(FindingSeverity::Error),
                        file: record.path.clone(),
                        line: None,
                        waived: None,
                        declared_severity,
                        message: format!(
                            "{field} claims a relation with {reference} (pair '{}'), but {reference}'s {echo} does not reciprocally name {id}{note}",
                            slot.pair.name
                        ),
                    });
                }
            }
            if !references.is_empty() && excluded == references.len() {
                Outcome::OutOfScope
            } else {
                Outcome::Examined
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

/// Rule (`RFC-47`, `MILE-122`): a claim crossing record types in a pair that
/// declares `cross_type: false`. The reciprocity rule excludes such a claim as
/// out of scope; this is where it is reported, so it is never a silent skip.
pub fn relation_cross_type_disallowed(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_RELATION_CROSS_TYPE_DISALLOWED;
    let mut findings = Vec::new();

    let slots = pair_slots(records, config, |p| !p.cross_type);

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |slot| slot.record,
        |slot| {
            let references = match pair_slot_claims(slot) {
                Ok(references) => references,
                Err(outcome) => return outcome,
            };
            for reference in references {
                // An unresolved reference is `relation.pair-reciprocity`'s.
                let Some(target) = index.get(&crate::values::RecordId::new(&reference)) else {
                    continue;
                };
                if target.record_type != slot.record.record_type {
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Error,
                        file: slot.record.path.clone(),
                        line: None,
                        waived: None,
                        declared_severity: None,
                        message: format!(
                            "{}: {reference} is a '{}' record, but pair '{}' declares cross_type: false and this record is '{}'",
                            slot.own_field(),
                            target.record_type,
                            slot.pair.name,
                            slot.record.record_type
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
                exclude_files: Vec::new(),
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

    // ---- relation pairs (RFC-47, MILE-122) ----

    fn pair(name: &str, fwd: &str, inv: &str, cross_type: bool) -> crate::config::RelationPair {
        crate::config::RelationPair {
            name: name.to_string(),
            forward: fwd.to_string(),
            inverse: inv.to_string(),
            reciprocity: crate::config::PairReciprocity::Required,
            cross_type,
            provisional: None,
        }
    }

    fn pair_config(types: Vec<(&str, &[&str])>, pairs: Vec<crate::config::RelationPair>) -> Config {
        let mut config = config_with_types(
            types
                .into_iter()
                .map(|(t, known)| (t, type_config_pointer(&[], Some(known), None, None)))
                .collect(),
        );
        config.relation_pairs = pairs;
        config
    }

    fn amend_config() -> Config {
        pair_config(
            vec![("rfc", &["Amends", "Amended-by"])],
            vec![pair("amendment", "Amends", "Amended-by", false)],
        )
    }

    fn run_pairs(config: &Config, records: &[Record]) -> (RuleExecution, Vec<Finding>) {
        let index = build_normalized_index(records);
        relation_pair_reciprocity(records, config, &index)
    }

    #[test]
    fn a_forward_claim_the_inverse_side_does_not_echo_is_a_finding() {
        let base = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "> Amended-by: —
",
        );
        let amender = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "> Amends: RFC-1
",
        );
        let records = [base, amender];
        let (exec, findings) = run_pairs(&amend_config(), &records);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, RULE_RELATION_PAIR_RECIPROCITY);
        assert!(findings[0].message.contains("RFC-1"), "{findings:?}");
        assert!(findings[0].message.contains("Amended-by"), "{findings:?}");
        assert_eq!(examined(&exec), 2, "both slots answered: {exec:?}");
    }

    #[test]
    fn a_reciprocated_pair_has_no_finding_and_both_slots_are_examined() {
        let base = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "> Amended-by: RFC-2
",
        );
        let amender = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "> Amends: RFC-1
",
        );
        let records = [base, amender];
        let (exec, findings) = run_pairs(&amend_config(), &records);
        assert!(findings.is_empty(), "{findings:?}");
        // 2 records x 2 sides declared; the two unwritten sides are absent.
        let pop = exec.population.unwrap();
        assert_eq!((pop.eligible(), pop.examined()), (4, 2), "{pop:?}");
    }

    #[test]
    fn an_inverse_claim_the_forward_side_does_not_echo_is_a_finding() {
        let base = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "> Amended-by: RFC-2
",
        );
        let other = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "> Amends: —
",
        );
        let records = [base, other];
        let (_, findings) = run_pairs(&amend_config(), &records);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Amends"), "{findings:?}");
    }

    /// The direction-blind rule passes this: each side names the other
    /// somewhere. With direction declared, `RFC-1`'s `Amends` claim needs
    /// `RFC-2`'s `Amended-by`, which is absent.
    #[test]
    fn two_records_each_claiming_the_forward_direction_are_not_reciprocal() {
        let a = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "> Amends: RFC-2
",
        );
        let b = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "> Amends: RFC-1
",
        );
        let records = [a, b];
        let (_, findings) = run_pairs(&amend_config(), &records);
        assert_eq!(findings.len(), 2, "{findings:?}");
    }

    #[test]
    fn a_pair_may_span_two_record_types() {
        let config = pair_config(
            vec![("adr", &["Resolves"]), ("bug", &["Resolved-by"])],
            vec![pair("resolution", "Resolves", "Resolved-by", true)],
        );
        let adr = record(
            "docs/adr/ADR-1-a.md",
            "adr",
            "> Resolves: BUG-7
",
        );
        let bug = record(
            "docs/bug/BUG-7-b.md",
            "bug",
            "> Resolved-by: ADR-1
",
        );
        let records = [adr.clone(), bug];
        let (_, findings) = run_pairs(&config, &records);
        assert!(findings.is_empty(), "{findings:?}");

        let bug = record(
            "docs/bug/BUG-7-b.md",
            "bug",
            "> Resolved-by: —
",
        );
        let records = [adr, bug];
        let (_, findings) = run_pairs(&config, &records);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_target_type_that_declares_no_echo_field_cannot_echo() {
        let config = pair_config(
            vec![("adr", &["Resolves"]), ("bug", &["Title"])],
            vec![pair("resolution", "Resolves", "Resolved-by", true)],
        );
        let adr = record(
            "docs/adr/ADR-1-a.md",
            "adr",
            "> Resolves: BUG-7
",
        );
        let bug = record("docs/bug/BUG-7-b.md", "bug", "> Title: x\n");
        let records = [adr, bug];
        let (_, findings) = run_pairs(&config, &records);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_cross_type_claim_in_a_same_type_pair_is_left_to_its_own_rule() {
        let config = pair_config(
            vec![
                ("rfc", &["Amends", "Amended-by"]),
                ("adr", &["Amends", "Amended-by"]),
            ],
            vec![pair("amendment", "Amends", "Amended-by", false)],
        );
        let rfc = record("docs/rfc/RFC-1-a.md", "rfc", "> Amends: ADR-2\n");
        let adr = record("docs/adr/ADR-2-b.md", "adr", "> Amended-by: —\n");
        let records = [rfc, adr];
        let index = build_normalized_index(&records);
        let (exec, findings) = relation_pair_reciprocity(&records, &config, &index);
        assert!(findings.is_empty(), "excluded, not judged: {findings:?}");
        assert_eq!(exec.population.unwrap().out_of_scope(), 1);
        let (cross_exec, cross) = relation_cross_type_disallowed(&records, &config, &index);
        assert_eq!(cross.len(), 1, "{cross:?}");
        assert_eq!(cross[0].rule, RULE_RELATION_CROSS_TYPE_DISALLOWED);
        assert!(examined(&cross_exec) >= 1);
    }

    #[test]
    fn a_pair_declaring_no_reciprocity_is_not_examined() {
        let mut p = pair("amendment", "Amends", "Amended-by", true);
        p.reciprocity = crate::config::PairReciprocity::None;
        let config = pair_config(vec![("rfc", &["Amends", "Amended-by"])], vec![p]);
        let a = record("docs/rfc/RFC-1-a.md", "rfc", "> Amends: RFC-2\n");
        let b = record("docs/rfc/RFC-2-b.md", "rfc", "> Amended-by: —\n");
        let records = [a, b];
        let (exec, findings) = run_pairs(&config, &records);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exec.population.unwrap().eligible(), 0);
    }

    #[test]
    fn a_pair_claim_naming_no_record_is_a_finding() {
        let a = record("docs/rfc/RFC-1-a.md", "rfc", "> Amends: RFC-99\n");
        let records = [a];
        let (_, findings) = run_pairs(&amend_config(), &records);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].message.contains("does not resolve"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_pair_discloses_an_unreadable_header() {
        let r = record("docs/rfc/RFC-1-a.md", "rfc", "this is not a header at all");
        let records = [r];
        let (exec, findings) = run_pairs(&amend_config(), &records);
        assert!(findings.is_empty(), "{findings:?}");
        let pop = exec.population.unwrap();
        assert_eq!((pop.examined(), pop.unreadable()), (0, 2), "{pop:?}");
    }

    #[test]
    fn an_unreadable_target_is_not_blamed_on_the_citer() {
        let a = record("docs/rfc/RFC-1-a.md", "rfc", "> Amends: RFC-2\n");
        let b = record("docs/rfc/RFC-2-b.md", "rfc", "this is not a header at all");
        let records = [a, b];
        let (_, findings) = run_pairs(&amend_config(), &records);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_claim_naming_several_targets_is_judged_per_target() {
        let one = record("docs/rfc/RFC-1-a.md", "rfc", "> Amended-by: RFC-3\n");
        let two = record("docs/rfc/RFC-2-b.md", "rfc", "> Amended-by: —\n");
        let three = record("docs/rfc/RFC-3-c.md", "rfc", "> Amends: RFC-1, RFC-2\n");
        let records = [one, two, three];
        let (exec, findings) = run_pairs(&amend_config(), &records);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("RFC-2's"), "{findings:?}");
        assert_eq!(findings[0].file, records[2].path);
        assert_eq!(examined(&exec), 3);
    }

    fn provisional_config() -> Config {
        let mut p = pair("amendment", "Amends", "Amended-by", false);
        p.provisional = Some(crate::config::ProvisionalClaim {
            statuses: vec!["Draft".to_string()],
            level: crate::config::ProvisionalLevel::Warn,
        });
        pair_config(
            vec![(
                "rfc",
                &["Status", "Amends", "Amended-by", "Amended-by-pending"],
            )],
            vec![p],
        )
    }

    #[test]
    fn a_draft_claim_echoed_only_in_a_pending_field_is_provisional_observed_failing() {
        let base = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "> Status: Accepted\n> Amended-by: —\n> Amended-by-pending: RFC-2\n",
        );
        let draft = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "> Status: Draft\n> Amends: RFC-1\n",
        );
        let (_, findings) = run_pairs(&provisional_config(), &[base, draft]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, FindingSeverity::Warning);
        assert_eq!(
            findings[0].declared_severity,
            Some(FindingSeverity::Warning)
        );
        assert!(findings[0].message.contains("provisional"), "{findings:?}");
    }

    #[test]
    fn an_accepted_claim_with_the_same_gap_keeps_the_full_severity_observed_failing() {
        let base = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "> Status: Accepted\n> Amended-by: —\n> Amended-by-pending: RFC-2\n",
        );
        let accepted = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "> Status: Accepted\n> Amends: RFC-1\n",
        );
        let (_, findings) = run_pairs(&provisional_config(), &[base, accepted]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, FindingSeverity::Error);
        assert_eq!(findings[0].declared_severity, None);
        assert!(!findings[0].message.contains("provisional"), "{findings:?}");
    }

    #[test]
    fn a_draft_claim_the_peer_settles_is_not_reported() {
        let base = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "> Status: Accepted\n> Amended-by: RFC-2\n",
        );
        let draft = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "> Status: Draft\n> Amends: RFC-1\n",
        );
        let (_, findings) = run_pairs(&provisional_config(), &[base, draft]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_pair_declaring_no_provisional_split_is_unchanged_by_status() {
        let base = record("docs/rfc/RFC-1-a.md", "rfc", "> Status: Accepted\n");
        let draft = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "> Status: Draft\n> Amends: RFC-1\n",
        );
        let (_, findings) = run_pairs(&amend_config(), &[base, draft]);
        assert_eq!(findings[0].severity, FindingSeverity::Error);
        assert_eq!(findings[0].declared_severity, None);
    }

    #[test]
    fn a_self_referencing_claim_is_judged_like_any_other() {
        let lone = record("docs/rfc/RFC-1-a.md", "rfc", "> Amends: RFC-1\n");
        let (_, findings) = run_pairs(&amend_config(), &[lone]);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }
}
