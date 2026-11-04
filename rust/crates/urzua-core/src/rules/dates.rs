use super::*;
use crate::status_model::StatusDecl;

/// Why `value` is not a date, or `None` when it is a real `YYYY-MM-DD` one.
fn date_problem(value: &str) -> Option<&'static str> {
    let b = value.as_bytes();
    let shaped = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !shaped {
        return Some("is not a well-formed YYYY-MM-DD date");
    }
    let num = |r: std::ops::Range<usize>| value[r].parse::<u32>().unwrap_or(0);
    let (year, month, day) = (num(0..4), num(5..7), num(8..10));
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    (day < 1 || day > days).then_some("is not a real calendar date")
}

/// `date.field-valid` (`MILE-137`): every declared date field that carries a
/// value must hold a real `YYYY-MM-DD` date. Blank, placeholder and pending
/// values are not date values and are not examined.
pub fn date_field_valid(records: &[Record], config: &Config) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_DATE_FIELD_VALID;
    let date_fields: Vec<String> = config
        .rules
        .get(RULE_ID)
        .and_then(|s| s.date_fields.clone())
        .unwrap_or_default();
    let declared = declared_fields_by_type(config);
    let statuses = statuses_by_type(config);
    let mut slots: Vec<(&Record, &String)> = Vec::new();
    for record in records {
        let Some(fields) = declared.get(&record.record_type) else {
            continue;
        };
        for f in &date_fields {
            if fields.contains(&crate::values::FieldName::from(f.as_str())) {
                slots.push((record, f));
            }
        }
    }
    let mut findings = Vec::new();
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |(record, _)| *record,
        |(record, field)| {
            let value = match declared_value(record, field) {
                Ok(v) => v,
                Err(outcome) => return outcome,
            };
            let decl = statuses.get(&record.record_type);
            if classify_with(Some(value), decl) != FieldState::Present {
                return Outcome::Absent;
            }
            if let Some(problem) = date_problem(value.trim()) {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    message: format!("{field} '{}' {problem}", value.trim()),
                });
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

/// The statuses of `record_type` that are accepted-type: the declared success
/// terminals, else the rule's `accepted_statuses` option.
fn accepted_statuses(decl: Option<&StatusDecl>, option: &[String]) -> Vec<String> {
    match decl {
        Some(d) if !d.success.is_empty() => d.success.clone(),
        _ => option.to_vec(),
    }
}

/// `date.acceptance-by-status` (`MILE-137`): the acceptance date is present on
/// an accepted-type status and absent on every other.
pub fn date_acceptance_by_status(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_DATE_ACCEPTANCE_BY_STATUS;
    let setting = config.rules.get(RULE_ID);
    let field = setting
        .and_then(|s| s.acceptance_field.clone())
        .unwrap_or_default();
    let option = setting
        .and_then(|s| s.accepted_statuses.clone())
        .unwrap_or_default();
    let declared = declared_fields_by_type(config);
    let statuses = statuses_by_type(config);
    let field_name = crate::values::FieldName::from(field.as_str());
    let candidates: Vec<&Record> = records
        .iter()
        .filter(|r| {
            declared
                .get(&r.record_type)
                .is_some_and(|f| f.contains(&field_name))
                && !accepted_statuses(statuses.get(&r.record_type), &option).is_empty()
        })
        .collect();
    let mut findings = Vec::new();
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Record,
        candidates,
        |record| *record,
        |record| {
            let status_field = relation_field_name(
                config,
                &record.record_type,
                crate::config::RelationRole::Status,
            );
            let status = match declared_value(record, status_field) {
                Ok(v) if !v.trim().is_empty() => v.trim(),
                Ok(_) => return Outcome::Absent,
                Err(outcome) => return outcome,
            };
            let decl = statuses.get(&record.record_type);
            let accepted = accepted_statuses(decl, &option).iter().any(|s| s == status);
            let state = classify_with(record.header.get(&field), decl);
            let message = if accepted
                && matches!(
                    state,
                    FieldState::Blank | FieldState::Placeholder | FieldState::Pending
                ) {
                Some(format!(
                    "{status_field} is {status}, an accepted-type status, which requires '{field}', but it is {state:?} -- not a real date"
                ))
            } else if !accepted
                && !matches!(
                    state,
                    FieldState::Blank | FieldState::Placeholder | FieldState::Pending
                )
            {
                Some(format!(
                    "{status_field} is {status}, which is not an accepted-type status, but the record carries '{field}' -- an acceptance date reads as a decision nobody made"
                ))
            } else {
                None
            };
            if let Some(message) = message {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    message,
                });
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

    fn config(extra_type: &str, rules: &str) -> Config {
        let yaml = format!(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n    known_fields: [Date, Accepted-on]\n{extra_type}rules:\n{rules}"
        );
        crate::config::parse(&yaml).expect("config loads")
    }

    const STATUSES: &str = "    statuses:\n      values: [Proposed, Accepted, Superseded, Rejected]\n      success: [Accepted, Superseded]\n      failure: [Rejected]\n";
    const VALID: &str =
        "  date.field-valid:\n    level: error\n    date_fields: [Date, Accepted-on]\n";
    const ACCEPT: &str =
        "  date.acceptance-by-status:\n    level: error\n    acceptance_field: Accepted-on\n";

    fn adr(status: &str, extra: &str) -> Record {
        Record::parse_with_shape_and_prefix(
            PathBuf::from("docs/adr/ADR-1-x.md"),
            "adr".to_string(),
            &format!("---\nStatus: {status}\n{extra}---\n# 1 - X\n"),
            crate::header::HeaderShape::YamlFrontmatter,
            "ADR".to_string(),
        )
    }

    #[test]
    fn a_malformed_date_is_reported_observed_failing() {
        let c = config("", VALID);
        for bad in [
            "2026-13-45",
            "next week",
            "2026-1-5",
            "2026-02-30 ",
            "20260205",
        ] {
            let (_, f) = date_field_valid(&[adr("Proposed", &format!("Date: {bad}\n"))], &c);
            assert_eq!(f.len(), 1, "{bad}: {f:?}");
            assert!(f[0].message.contains("Date"), "{f:?}");
        }
    }

    #[test]
    fn an_impossible_calendar_date_is_reported_and_leap_days_pass() {
        let c = config("", VALID);
        for bad in [
            "2026-02-30",
            "2026-02-29",
            "2026-04-31",
            "2026-00-10",
            "2026-06-00",
        ] {
            let (_, f) = date_field_valid(&[adr("Proposed", &format!("Date: {bad}\n"))], &c);
            assert_eq!(f.len(), 1, "{bad}: {f:?}");
            assert!(f[0].message.contains("calendar"), "{f:?}");
        }
        for good in ["2026-02-28", "2028-02-29", "2000-02-29", "2026-12-31"] {
            let (_, f) = date_field_valid(&[adr("Proposed", &format!("Date: {good}\n"))], &c);
            assert!(f.is_empty(), "{good}: {f:?}");
        }
        let (_, f) = date_field_valid(&[adr("Proposed", "Date: 1900-02-29\n")], &c);
        assert_eq!(f.len(), 1, "1900 is not a leap year");
    }

    #[test]
    fn the_rule_counts_the_date_values_it_examined() {
        let c = config("", VALID);
        let rec = adr("Accepted", "Date: 2026-01-01\nAccepted-on: nope\n");
        let (exec, f) = date_field_valid(&[rec], &c);
        assert_eq!(examined(&exec), 2, "one per date value");
        let p = exec.population.unwrap();
        assert_eq!((p.eligible(), p.examined()), (2, 2), "two date values");
        assert_eq!(f.len(), 1);
        // blank, placeholder and absent are not date values
        let (exec, f) = date_field_valid(&[adr("Proposed", "Date: YYYY-MM-DD\n")], &c);
        assert!(f.is_empty(), "{f:?}");
        assert_eq!(exec.population.unwrap().examined(), 0);
    }

    #[test]
    fn an_accepted_record_without_the_date_fails_and_a_blank_or_placeholder_counts_as_missing() {
        let c = config(STATUSES, ACCEPT);
        for extra in ["", "Accepted-on: YYYY-MM-DD\n", "Accepted-on:\n"] {
            let (_, f) = date_acceptance_by_status(&[adr("Accepted", extra)], &c);
            assert_eq!(f.len(), 1, "{extra:?}: {f:?}");
            assert!(f[0].message.contains("Accepted-on"), "{f:?}");
        }
        let (_, f) = date_acceptance_by_status(&[adr("Superseded", "")], &c);
        assert_eq!(
            f.len(),
            1,
            "every declared success terminal is accepted-type"
        );
        let (exec, f) =
            date_acceptance_by_status(&[adr("Accepted", "Accepted-on: 2026-01-01\n")], &c);
        assert!(f.is_empty(), "{f:?}");
        assert_eq!(examined(&exec), 1);
    }

    #[test]
    fn any_other_status_carrying_a_date_fails_but_blank_or_placeholder_is_tolerated() {
        let c = config(STATUSES, ACCEPT);
        for status in ["Proposed", "Rejected"] {
            let (_, f) = date_acceptance_by_status(&[adr(status, "Accepted-on: 2026-01-01\n")], &c);
            assert_eq!(f.len(), 1, "{status}: {f:?}");
            for ok in ["", "Accepted-on:\n", "Accepted-on: YYYY-MM-DD\n"] {
                let (_, f) = date_acceptance_by_status(&[adr(status, ok)], &c);
                assert!(f.is_empty(), "{status} {ok:?}: {f:?}");
            }
        }
    }

    #[test]
    fn a_type_declaring_no_success_status_uses_the_rule_option() {
        let rules = format!("{ACCEPT}    accepted_statuses: [Accepted]\n");
        let c = config("", &rules);
        let (_, f) = date_acceptance_by_status(&[adr("Accepted", "")], &c);
        assert_eq!(f.len(), 1, "{f:?}");
        let (_, f) = date_acceptance_by_status(&[adr("Proposed", "")], &c);
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn with_no_accepted_statuses_declared_anywhere_nothing_is_examined() {
        let c = config("", ACCEPT);
        let (exec, f) = date_acceptance_by_status(&[adr("Accepted", "")], &c);
        assert!(f.is_empty(), "{f:?}");
        let p = exec.population.unwrap();
        assert_eq!((p.eligible(), p.examined()), (0, 0));
    }

    #[test]
    fn a_type_not_declaring_the_acceptance_field_is_not_examined() {
        let extra = "  bug:\n    dir: docs/bugs\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n    statuses:\n      values: [Open, Fixed]\n      success: [Fixed]\n";
        let c = config(extra, ACCEPT);
        let bug = Record::parse_with_shape_and_prefix(
            PathBuf::from("docs/bugs/BUG-1-x.md"),
            "bug".to_string(),
            "---\nStatus: Fixed\n---\n# 1 - X\n",
            crate::header::HeaderShape::YamlFrontmatter,
            "BUG".to_string(),
        );
        let (_, f) = date_acceptance_by_status(&[bug], &c);
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn the_options_load_only_on_their_rules_and_are_required() {
        let bad = |rules: &str| {
            let yaml = format!("schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    required_fields: [Status]\nrules:\n{rules}");
            crate::config::parse(&yaml).err().map(|e| e.to_string())
        };
        assert!(bad("  date.field-valid: error\n")
            .unwrap()
            .contains("date_fields"));
        assert!(bad("  date.acceptance-by-status: error\n")
            .unwrap()
            .contains("acceptance_field"));
        assert!(
            bad("  field.quality:\n    level: error\n    acceptance_field: X\n")
                .unwrap()
                .contains("acceptance_field")
        );
        assert!(
            bad("  field.quality:\n    level: error\n    date_fields: [X]\n")
                .unwrap()
                .contains("date_fields")
        );
        assert!(bad("  date.field-valid: off\n").is_none());
    }
}
