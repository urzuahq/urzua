use super::*;

/// Rule (MILE-0083, generalized to any `narrative_fields` entry by
/// MILE-0090/ADR-0044 -- renamed from `blocked_on_stale`): a narrative-field
/// pointer whose target has reached a terminal status is a staleness
/// signal, distinct from `pointer_resolution`'s generic "resolves; target
/// Status = X" noise -- a blocker (or any other narrative-tolerant
/// relationship) that has actually cleared deserves its own actionable
/// finding, not one more line among many routine ones. Terminal-status sets
/// are a small, hardcoded MVP per record type (ADR-18's own "ship the MVP,
/// extend once a real case demands more" precedent), not a config surface --
/// nothing has asked for one yet. A dangling reference is deliberately not
/// this rule's job; that's `pointer_resolution`'s error case, reused rather
/// than duplicated here.
pub fn narrative_field_stale(
    records: &[Record],
    config: &Config,
    terminal_statuses: &[String],
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_NARRATIVE_FIELD_STALE;
    let mut findings = Vec::new();

    let declared_by_type = declared_fields_by_type(config);
    let declared_by_type = &declared_by_type;
    let statuses = statuses_by_type(config);
    let narrative_fields_by_type = fields_with_capability(config, |k| k.check_target_staleness);

    // Declared narrative slots. A slot the record did not write, or wrote as
    // prose yielding no reference, is eligible and unexamined: BUG-39 was the
    // extractor silently returning nothing on prose, and its fix was verified
    // by watching this count move from 7 to 8. Counting a slot as examined
    // merely because it is present would pin the number and retire the
    // instrument that measured that fix.
    let slots = field_slots(records, &narrative_fields_by_type);

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |(record, _)| *record,
        |(record, field_name)| {
            let references = match declared_field_references(record, field_name.as_str()) {
                Ok(refs) => refs,
                Err(outcome) => return outcome,
            };

            for reference in references {
                let Some(target) = index.get(&crate::values::RecordId::new(&reference)) else {
                    continue; // pointer_resolution already reports a dangling reference
                };
                let target_status_field = relation_field_name(
                    config,
                    &target.record_type,
                    crate::config::RelationRole::Status,
                );
                let Ok(status) =
                    declared_cross_record_value(declared_by_type, target, target_status_field)
                else {
                    continue;
                };
                if is_finished(&statuses, &target.record_type, status, terminal_statuses) {
                    let source_status_field = relation_field_name(
                        config,
                        &record.record_type,
                        crate::config::RelationRole::Status,
                    );
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Warning,
                        file: record.path.clone(),
                        line: None,
                        waived: None,
                        declared_severity: None,
                        suggested_action: None,
                        action_source: ActionSource::new("narrative-field-stale").field(field_name.as_str()),
                        message: format!(
                            "{field_name}: {reference} has reached a terminal status ({status}) -- re-examine whether this record's {source_status_field}/{field_name} should update"
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

    /// BUG-178: a target type that declares its terminals is judged by them,
    /// not by the rule's hand-copied list, which lacks `Not a bug`.
    #[test]
    fn narrative_field_stale_reads_the_declared_terminals_observed_failing() {
        let yaml = "schema_version: 2\nrecord_types:\n  bug:\n    dir: docs/bugs\n    required_fields: [Status]\n    known_fields: [Blocked-on]\n    pointer_fields: []\n    narrative_fields: [Blocked-on]\n    statuses:\n      values: [Open, Fixed, Not a bug]\n      success: [Fixed]\n      failure: [Not a bug]\n";
        let config = crate::config::parse(yaml).unwrap();
        let blocked = record(
            "docs/bugs/BUG-1-x.md",
            "bug",
            "---\nBlocked-on: BUG-2\n---\n",
        );
        let target = record(
            "docs/bugs/BUG-2-y.md",
            "bug",
            "---\nStatus: Not a bug\n---\n",
        );
        let records = [blocked, target];
        let index = build_normalized_index(&records);
        let list = vec!["Fixed".to_string()];
        let (_, findings) = narrative_field_stale(&records, &config, &list, &index);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn narrative_field_stale_judges_a_type_the_engine_has_never_heard_of() {
        // The compiled-in table knew five type names and returned "not
        // terminal" for every other, so the rule was inert for any corpus that
        // named its types differently -- which is every adopter (BUG-59).
        let blocked = record(
            "docs/decisions/DEC-1-x.md",
            "dec",
            "---\nBlocked-on: DEC-2\n---\n",
        );
        let target = record(
            "docs/decisions/DEC-2-y.md",
            "dec",
            "---\nStatus: Ratified\n---\n",
        );
        let config = config_with_types(vec![(
            "dec",
            type_config_pointer(&["Status"], None, None, Some(&["Blocked-on"])),
        )]);

        let records = [blocked, target];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            narrative_field_stale(&records, &config, &["Ratified".to_string()], &index);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(population.unit(), PopulationUnit::Field);
        assert_eq!(population.eligible(), 2, "both records declare the slot");
        assert_eq!(
            population.examined(),
            1,
            "only DEC-1 wrote the slot, so only DEC-1 reaches a verdict"
        );
        assert_eq!(findings.len(), 1, "DEC-2 is terminal by declaration");
        assert!(findings[0].message.contains("DEC-2"));
    }

    #[test]
    fn a_blocked_on_pointer_to_a_terminal_bug_is_a_stale_finding() {
        // The exact live case this rule was built for: MILE-2/3's Blocked-on
        // named BUG-3, which had already shipped (Status: Fixed) with nothing
        // catching it, since Blocked-on used to be unchecked free-text prose.
        let bug = record("docs/bugs/BUG-3-x.md", "bug", "---\nStatus: Fixed\n---\n");
        let milestone = record(
            "docs/milestones/MILE-2-x.md",
            "milestone",
            "---\nStatus: Planned\nBlocked-on: BUG-3\n---\n",
        );
        let config = config_with_types(vec![
            (
                "milestone",
                type_config_pointer(&[], None, None, Some(&["Blocked-on"])),
            ),
            ("bug", type_config_pointer(&["Status"], None, None, None)),
        ]);
        let records = [bug, milestone];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            narrative_field_stale(&records, &config, &terminal_for_tests(), &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("BUG-3"));
        assert!(findings[0].message.contains("Fixed"));
        assert_eq!(findings[0].severity, FindingSeverity::Warning);
    }

    #[test]
    fn a_blocked_on_pointer_to_a_non_terminal_record_is_not_stale() {
        let bug = record("docs/bugs/BUG-4-x.md", "bug", "---\nStatus: Open\n---\n");
        let milestone = record(
            "docs/milestones/MILE-9-x.md",
            "milestone",
            "---\nStatus: Planned\nBlocked-on: BUG-4\n---\n",
        );
        let config = config_with_types(vec![(
            "milestone",
            type_config_pointer(&[], None, None, Some(&["Blocked-on"])),
        )]);
        let records = [bug, milestone];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            narrative_field_stale(&records, &config, &terminal_for_tests(), &index);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn blocked_on_free_text_with_no_reference_is_skipped() {
        let milestone = record(
            "docs/milestones/MILE-5-x.md",
            "milestone",
            "---\nStatus: Planned\nBlocked-on: a decision that hasn't been made\n---\n",
        );
        let config = config_with_types(vec![(
            "milestone",
            type_config_pointer(&[], None, None, Some(&["Blocked-on"])),
        )]);
        let records = [milestone];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            narrative_field_stale(&records, &config, &terminal_for_tests(), &index);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(population.eligible(), 1, "the slot is declared and written");
        assert_eq!(
            population.examined(),
            0,
            "prose yielding no reference is handed but not judged -- the BUG-39 instrument"
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn a_dangling_blocked_on_reference_is_not_this_rule_s_job() {
        let milestone = record(
            "docs/milestones/MILE-6-x.md",
            "milestone",
            "---\nStatus: Planned\nBlocked-on: BUG-9999\n---\n",
        );
        let config = config_with_types(vec![(
            "milestone",
            type_config_pointer(&[], None, None, Some(&["Blocked-on"])),
        )]);
        let records = [milestone];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            narrative_field_stale(&records, &config, &terminal_for_tests(), &index);
        assert_eq!(examined(&exec), 1);
        assert!(
            findings.is_empty(),
            "a dangling reference is pointer_resolution's error case, not this rule's"
        );
    }

    #[test]
    fn narrative_field_stale_generalizes_beyond_blocked_on() {
        // MILE-90: the rule now reads whatever the type's own
        // narrative_fields declares, not a hardcoded "Blocked-on" literal --
        // proven with a differently-named field.
        let bug = record("docs/bugs/BUG-7-x.md", "bug", "---\nStatus: Fixed\n---\n");
        let rfc = record(
            "docs/rfc/RFC-1-x.md",
            "rfc",
            "---\nStatus: Draft\nMotivated-by: BUG-7\n---\n",
        );
        let config = config_with_types(vec![
            (
                "rfc",
                type_config_pointer(&[], None, None, Some(&["Motivated-by"])),
            ),
            ("bug", type_config_pointer(&["Status"], None, None, None)),
        ]);
        let records = [bug, rfc];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            narrative_field_stale(&records, &config, &terminal_for_tests(), &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.starts_with("Motivated-by:"));
    }

    #[test]
    fn narrative_field_stale_reads_a_custom_status_field_name_observed_failing() {
        let bug = record("docs/bugs/BUG-8-x.md", "bug", "---\nState: Fixed\n---\n");
        let rfc = record(
            "docs/rfc/RFC-2-x.md",
            "rfc",
            "---\nStatus: Draft\nMotivated-by: BUG-8\n---\n",
        );
        let config = config_with_types(vec![
            (
                "rfc",
                type_config_pointer(&[], None, None, Some(&["Motivated-by"])),
            ),
            (
                "bug",
                crate::config::RecordTypeConfig {
                    exclude_files: Vec::new(),
                    relation_fields: Some(crate::config::RelationFields {
                        status: Some("State".to_string()),
                        ..Default::default()
                    }),
                    ..type_config_pointer(&["State"], None, None, None)
                },
            ),
        ]);
        let records = [bug, rfc];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            narrative_field_stale(&records, &config, &terminal_for_tests(), &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Fixed"), "{findings:?}");
    }
}
