use super::*;
use crate::status_model::StatusDecl;

/// A type's validated `statuses` declaration, by type name, only for a type
/// that declares one (`MILE-132`): an undeclared type must stay absent from
/// the map so every rule reading it keeps its prior behaviour.
pub fn statuses_by_type(config: &Config) -> HashMap<String, StatusDecl> {
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
    judge: impl Fn(&Judged) -> Vec<(Option<String>, String)>,
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
            for (field, message) in judge(&judged) {
                let declared_severity = field
                    .as_deref()
                    .and_then(|f| declared_field_severity(config, rule_id, f));
                findings.push(Finding {
                    rule: rule_id.to_string(),
                    severity: declared_severity.unwrap_or(FindingSeverity::Error),
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity,
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

/// How one record's `Status` stands against its delivery evidence
/// (`MILE-177`), shared by `status.delivery-agreement` and `urzua fix`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryJudgement {
    pub status_field: String,
    pub status: String,
    /// The tier computed from the record's evidence (drift is not read: it says
    /// the evidence is stale, not that nothing was delivered).
    pub tier: &'static str,
    /// What the evidence was, in words a repair row can cite.
    pub evidence: String,
    pub disagreement: Option<DeliveryDisagreement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryDisagreement {
    /// The evidence says delivered while the status is declared not done. The
    /// one correct value is the type's `set_status`.
    DeliveredButNotDone { set_status: String },
    /// A success status with evidence that says nothing was delivered. No
    /// single correct value exists: the repair is adding evidence.
    SuccessWithoutEvidence,
}

/// Judges one record of a type that declares `statuses.delivery`. `Err` is the
/// census outcome for a record that cannot be judged (no status, an unreadable
/// header).
pub fn judge_delivery(
    record: &Record,
    config: &Config,
    decl: &StatusDecl,
    back_pointed: &BackPointerEvidence,
) -> Result<DeliveryJudgement, Outcome> {
    use crate::config::RelationRole::{EmbodimentLocator, Status};
    let delivery = decl.delivery.as_ref().ok_or(Outcome::Absent)?;
    let status_field = relation_field_name(config, &record.record_type, Status);
    let locator_field = relation_field_name(config, &record.record_type, EmbodimentLocator);
    let status = match declared_value(record, status_field) {
        Ok(v) if !v.trim().is_empty() => v.trim(),
        Ok(_) => return Err(Outcome::Absent),
        Err(outcome) => return Err(outcome),
    };
    // A record with no evidence field is the common case, not an absence.
    let realized_by = match declared_value(record, locator_field) {
        Ok(v) => Some(v),
        Err(Outcome::Absent) => None,
        Err(outcome) => return Err(outcome),
    };
    let cited_by_code = back_pointed.code.contains(&record.path);
    let cited_by_test = back_pointed.test.contains(&record.path);
    let tier = compute_embodiment(
        &realized_by.map(parse_realized_by).unwrap_or_default(),
        false,
        cited_by_code,
        cited_by_test,
    );
    let evidence = evidence_description(
        locator_field,
        realized_by.is_some(),
        cited_by_code,
        cited_by_test,
    );
    let in_list = |list: &[String], v: &str| list.iter().any(|e| e == v);
    let disagreement =
        if in_list(&delivery.not_done, status) && in_list(&delivery.delivered_tiers, tier) {
            Some(DeliveryDisagreement::DeliveredButNotDone {
                set_status: delivery.set_status.clone(),
            })
        } else if in_list(&decl.success, status) && in_list(&delivery.unevidenced_tiers, tier) {
            Some(DeliveryDisagreement::SuccessWithoutEvidence)
        } else {
            None
        };
    Ok(DeliveryJudgement {
        status_field: status_field.to_string(),
        status: status.to_string(),
        tier,
        evidence,
        disagreement,
    })
}

/// A record's `Status` against its delivery evidence, both ways (`MILE-177`):
/// the evidence computes to a declared delivered tier while the status is
/// declared not done, and a success status whose evidence computes to a
/// declared unevidenced tier. The mapping is the type's `statuses.delivery`;
/// the rule hardcodes no status or type. A type declaring none is not examined.
pub fn status_delivery_agreement(
    records: &[Record],
    config: &Config,
    back_pointed: &BackPointerEvidence,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_STATUS_DELIVERY_AGREEMENT;
    let declared = statuses_by_type(config);
    let candidates: Vec<&Record> = records
        .iter()
        .filter(|r| {
            declared
                .get(&r.record_type)
                .is_some_and(|d| d.delivery.is_some())
        })
        .collect();
    let mut findings = Vec::new();
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Record,
        candidates,
        |record| *record,
        |record| {
            let judged = match judge_delivery(
                record,
                config,
                &declared[&record.record_type],
                back_pointed,
            ) {
                Ok(j) => j,
                Err(outcome) => return outcome,
            };
            let message = match &judged.disagreement {
                None => return Outcome::Examined,
                Some(DeliveryDisagreement::DeliveredButNotDone { set_status }) => format!(
                    "{} is '{}', but the evidence computes to '{}', from {}: the work reads as delivered; set {} to '{set_status}' (`urzua fix --apply` does)",
                    judged.status_field, judged.status, judged.tier, judged.evidence, judged.status_field
                ),
                Some(DeliveryDisagreement::SuccessWithoutEvidence) => format!(
                    "{} is '{}', but the evidence computes to '{}', from {}: add the evidence that delivered it",
                    judged.status_field, judged.status, judged.tier, judged.evidence
                ),
            };
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Error,
                file: record.path.clone(),
                line: None,
                waived: None,
                declared_severity: None,
                message,
            });
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
            vec![(
                None,
                format!(
                    "{} '{}' is not declared for type '{}' (declared: {})",
                    j.status_field,
                    j.status,
                    j.record_type,
                    j.decl.values.join(", ")
                ),
            )]
        },
    )
}

/// A field the record's status requires, blank, unedited template text or a
/// value from the type's declared `pending` vocabulary.
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
                    // A declared pending vocabulary (MILE-153) is the type's own
                    // placeholder list: its whole values are missing here too. A
                    // type declaring none keeps the old reading, so the compiled-in
                    // `contains("pending")` match never reaches this rule.
                    let state = classify_with(j.record.header.get(field), Some(j.decl));
                    let declared_pending =
                        state == FieldState::Pending && j.decl.pending.is_some();
                    (matches!(state, FieldState::Blank | FieldState::Placeholder)
                        || declared_pending)
                        .then(|| {
                        (
                            Some(field.to_string()),
                            format!(
                                "{} is {}, which requires '{field}', but it is {state:?} -- not a real, present value",
                                j.status_field, j.status
                            ),
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
                    (
                        None,
                        format!(
                            "{} is {}, which forbids '{field}', but the record carries it",
                            j.status_field, j.status
                        ),
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
    fn a_declared_pending_value_does_not_satisfy_a_requirement_observed_failing() {
        let with_pending = format!(
            "{VOCAB}      pending: [\"(pending review)\", \"none recorded\", \"—\", Unknown]\n"
        );
        let config = typed(&with_pending);
        for v in [
            "(pending review)",
            "none recorded",
            "—",
            "Unknown",
            "unknown",
        ] {
            let extra = format!("Deciders: '{v}'\n");
            let (_, findings) = status_required_field_missing(&[adr("Accepted", &extra)], &config);
            assert_eq!(findings.len(), 1, "{v}: {findings:?}");
            assert!(findings[0].message.contains("Deciders"), "{findings:?}");
        }
        // A real value that merely contains the word is not reported.
        for v in ["Unknown Person", "Ana (pending nothing)", "Ana"] {
            let extra = format!("Deciders: '{v}'\n");
            let (_, findings) = status_required_field_missing(&[adr("Accepted", &extra)], &config);
            assert!(findings.is_empty(), "{v}: {findings:?}");
        }
    }

    #[test]
    fn without_a_declared_pending_vocabulary_a_dash_or_pending_text_still_satisfies() {
        let config = typed(VOCAB);
        for v in ["—", "Unknown", "pending review"] {
            let extra = format!("Deciders: '{v}'\n");
            let (_, findings) = status_required_field_missing(&[adr("Accepted", &extra)], &config);
            assert!(findings.is_empty(), "{v}: {findings:?}");
        }
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

    #[test]
    fn a_missing_required_field_reports_at_its_own_declared_level_observed_failing() {
        let yaml = "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n    known_fields: [Deciders, Reviewers]\n    statuses:\n      values: [Proposed, Accepted]\n      success: [Accepted]\n      require:\n        - {when: [Accepted], fields: [Deciders, Reviewers]}\nrules:\n  status.required-field-missing:\n    level: warn\n    field_levels: {Deciders: error}\n";
        let config = crate::config::parse(yaml).expect("config loads");
        let (_, findings) = status_required_field_missing(&[adr("Accepted", "")], &config);
        let d = findings
            .iter()
            .find(|x| x.message.contains("'Deciders'"))
            .unwrap();
        let r = findings
            .iter()
            .find(|x| x.message.contains("'Reviewers'"))
            .unwrap();
        assert_eq!(d.declared_severity, Some(FindingSeverity::Error));
        assert_eq!(r.declared_severity, None);
    }
}
