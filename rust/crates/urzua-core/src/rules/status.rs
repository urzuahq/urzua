use super::*;
use crate::status_model::StatusDecl;

/// A type's validated `statuses` declaration, by type name, only for a type
/// that declares one (`MILE-132`): an undeclared type must stay absent from
/// the map so every rule reading it keeps its prior behaviour.
pub(super) fn statuses_by_type(config: &Config) -> HashMap<String, StatusDecl> {
    config
        .record_types
        .iter()
        .filter_map(|(name, cfg)| cfg.status_decl().map(|d| (name.clone(), d)))
        .collect()
}

/// Whether `status` finishes a record of `record_type`: the declared success
/// and failure terminals when the type declares them, otherwise the rule's own
/// `fallback` list (`BUG-178`).
pub(super) fn is_finished(
    declared: &HashMap<String, StatusDecl>,
    record_type: &str,
    status: &str,
    fallback: &[String],
) -> bool {
    match declared.get(record_type) {
        Some(decl) => decl.is_terminal(status),
        None => fallback.iter().any(|s| s == status),
    }
}

/// What `judge` is handed for one candidate record.
struct Judged<'a> {
    record: &'a Record,
    decl: &'a StatusDecl,
    record_type: &'a str,
    status_field: &'a str,
    status: &'a str,
}

fn run_status_rule(
    records: &[Record],
    config: &Config,
    rule_id: &'static str,
    eligible: impl Fn(&StatusDecl) -> bool,
    judge: impl Fn(&Judged) -> Vec<String>,
) -> (RuleExecution, Vec<Finding>) {
    let declared = statuses_by_type(config);
    let candidates: Vec<&Record> = records
        .iter()
        .filter(|r| declared.get(&r.record_type).is_some_and(&eligible))
        .collect();
    let mut findings = Vec::new();
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Record,
        candidates,
        |record| *record,
        |record| {
            let decl = &declared[&record.record_type];
            let status_field = relation_field_name(
                config,
                &record.record_type,
                crate::config::RelationRole::Status,
            );
            let status = match declared_value(record, status_field) {
                Ok(v) if !v.trim().is_empty() => v,
                Ok(_) => return Outcome::Absent,
                Err(outcome) => return outcome,
            };
            let judged = Judged {
                record,
                decl,
                record_type: &record.record_type,
                status_field,
                status,
            };
            for message in judge(&judged) {
                findings.push(Finding {
                    rule: rule_id.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    message,
                });
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution {
            rule: rule_id.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// A record's `Status` outside the values its type declares.
pub fn status_undeclared_value(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    run_status_rule(
        records,
        config,
        RULE_STATUS_UNDECLARED_VALUE,
        |_| true,
        |j| {
            if j.decl.declares(j.status) {
                return Vec::new();
            }
            vec![format!(
                "{} '{}' is not declared for type '{}' (declared: {})",
                j.status_field,
                j.status,
                j.record_type,
                j.decl.values.join(", ")
            )]
        },
    )
}

/// A field the record's status requires, blank or unedited template text.
pub fn status_required_field_missing(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    run_status_rule(
        records,
        config,
        RULE_STATUS_REQUIRED_FIELD_MISSING,
        |d| !d.require.is_empty(),
        |j| {
            j.decl
                .required_at(j.status)
                .into_iter()
                .filter_map(|field| {
                    let state = classify(j.record.header.get(field));
                    matches!(state, FieldState::Blank | FieldState::Placeholder).then(|| {
                        format!(
                            "{} is {}, which requires '{field}', but it is {state:?} -- not a real, present value",
                            j.status_field, j.status
                        )
                    })
                })
                .collect()
        },
    )
}

/// A field the record's status forbids, carrying a value.
pub fn status_forbidden_field_present(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    run_status_rule(
        records,
        config,
        RULE_STATUS_FORBIDDEN_FIELD_PRESENT,
        |d| !d.forbid.is_empty(),
        |j| {
            j.decl
                .forbidden_at(j.status)
                .into_iter()
                .filter(|field| classify(j.record.header.get(field)) != FieldState::Blank)
                .map(|field| {
                    format!(
                        "{} is {}, which forbids '{field}', but the record carries it",
                        j.status_field, j.status
                    )
                })
                .collect()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::*;

    fn typed(statuses: &str) -> Config {
        let yaml = format!(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n    known_fields: [Deciders, Accepted-on]\n    statuses:\n{statuses}\nrules:\n  identity.collision: error\n"
        );
        crate::config::parse(&yaml).expect("config loads")
    }

    fn adr(status: &str, extra: &str) -> Record {
        Record::parse_with_shape_and_prefix(
            PathBuf::from("docs/adr/ADR-1-x.md"),
            "adr".to_string(),
            &format!("---\nStatus: {status}\n{extra}---\n# 1 - X\n"),
            crate::header::HeaderShape::YamlFrontmatter,
            "ADR".to_string(),
        )
    }

    const VOCAB: &str = "      values: [Proposed, Accepted, Rejected]\n      success: [Accepted]\n      failure: [Rejected]\n      require:\n        - {when: [Accepted], fields: [Deciders]}\n      forbid:\n        - {when: [Proposed, Rejected], fields: [Accepted-on]}\n";

    #[test]
    fn a_status_outside_the_declared_set_is_reported_observed_failing() {
        let config = typed(VOCAB);
        let (exec, findings) = status_undeclared_value(&[adr("Acceptd", "")], &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Acceptd"), "{findings:?}");
        assert!(findings[0].message.contains("Proposed"), "{findings:?}");
        assert_eq!(examined(&exec), 1);
    }

    #[test]
    fn a_declared_status_is_not_reported() {
        let config = typed(VOCAB);
        let (exec, findings) = status_undeclared_value(&[adr("Accepted", "")], &config);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(examined(&exec), 1);
    }

    #[test]
    fn a_type_declaring_no_statuses_is_not_examined() {
        let config = config_for_required(vec![("adr", vec!["Status"])]);
        let (exec, findings) = status_undeclared_value(&[adr("Whatever", "")], &config);
        assert!(findings.is_empty());
        let p = exec.population.unwrap();
        assert_eq!((p.eligible(), p.examined()), (0, 0));
    }

    #[test]
    fn an_accepted_record_without_deciders_fails_and_a_proposed_one_passes_observed_failing() {
        let config = typed(VOCAB);
        let (_, findings) = status_required_field_missing(&[adr("Accepted", "")], &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Deciders"), "{findings:?}");
        let (_, findings) = status_required_field_missing(&[adr("Proposed", "")], &config);
        assert!(findings.is_empty(), "{findings:?}");
        let (_, findings) =
            status_required_field_missing(&[adr("Accepted", "Deciders: Ana\n")], &config);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_blank_or_template_value_does_not_satisfy_a_requirement() {
        let config = typed(VOCAB);
        let (_, findings) =
            status_required_field_missing(&[adr("Accepted", "Deciders: name\n")], &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_forbidden_field_on_the_wrong_status_is_reported_observed_failing() {
        let config = typed(VOCAB);
        let carrying = "Accepted-on: 2026-01-01\n";
        let (_, findings) = status_forbidden_field_present(&[adr("Proposed", carrying)], &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Accepted-on"), "{findings:?}");
        let (_, findings) = status_forbidden_field_present(&[adr("Accepted", carrying)], &config);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn the_declared_terminals_replace_the_fallback_list_and_an_undeclared_type_keeps_it() {
        let config = typed(VOCAB);
        let declared = statuses_by_type(&config);
        let fallback = vec!["Done".to_string()];
        assert!(is_finished(&declared, "adr", "Rejected", &fallback));
        assert!(!is_finished(&declared, "adr", "Done", &fallback));
        assert!(!is_finished(&declared, "adr", "Proposed", &fallback));
        assert!(is_finished(&declared, "other", "Done", &fallback));
    }
}
