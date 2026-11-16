use super::*;

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

/// The Unix time at the start of a `YYYY-MM-DD` date (UTC), or why the value
/// is not one. Shared by the CLI, which dates a record's last audit from it
/// (`MILE-166`), and by `embodiment.consistency`, which reports a bad one.
pub fn date_unix(value: &str) -> Result<i64, &'static str> {
    let value = value.trim();
    if let Some(problem) = date_problem(value) {
        return Err(problem);
    }
    let num = |r: std::ops::Range<usize>| i64::from(value[r].parse::<u32>().unwrap_or(0));
    let (year, month, day) = (num(0..4), num(5..7), num(8..10));
    // Days from civil (proleptic Gregorian), 1970-01-01 = 0.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Ok((era * 146_097 + doe - 719_468) * 86_400)
}

/// A last-audit date as `drift_audit_field` reads it: a real date that is not
/// after `now_unix`'s UTC day. A date written forward would put the end of the
/// grace window out of reach and hold drift back for ever, so it is reported and
/// ignored, as a malformed one is (`MILE-166`).
pub fn audit_date_at(value: &str, now_unix: i64) -> Result<i64, &'static str> {
    let unix = date_unix(value)?;
    if unix.div_euclid(86_400) > now_unix.div_euclid(86_400) {
        return Err("is in the future (a date written forward would hold drift back for ever)");
    }
    Ok(unix)
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
                findings.push(Finding::new(
                    RULE_ID,
                    FindingSeverity::Error,
                    record.path.clone(),
                    ActionSource::new("date-invalid").field(field.as_str()),
                    format!("{field} '{}' {problem}", value.trim()),
                ));
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution::ran(RULE_ID, population, examined_records),
        findings,
    )
}

/// `date.field-order` (`MILE-157`): for each `date_order` entry
/// `Earlier <= Later`, a record's `Earlier` date may not be later than its
/// `Later`. Well-formedness stays `date.field-valid`'s: a value that is not a
/// present, real date is not compared.
pub fn date_field_order(records: &[Record], config: &Config) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_DATE_FIELD_ORDER;
    let orders: Vec<(String, String)> = config
        .rules
        .get(RULE_ID)
        .and_then(|s| s.date_order.as_ref())
        .into_iter()
        .flatten()
        .map(|e| (e.earlier.trim().to_string(), e.later.trim().to_string()))
        .collect();
    let declared = declared_fields_by_type(config);
    let statuses = statuses_by_type(config);
    let mut slots: Vec<(&Record, &(String, String))> = Vec::new();
    for record in records {
        let Some(fields) = declared.get(&record.record_type) else {
            continue;
        };
        for order in &orders {
            let has = |f: &str| fields.contains(&crate::values::FieldName::from(f));
            if has(&order.0) && has(&order.1) {
                slots.push((record, order));
            }
        }
    }
    let mut findings = Vec::new();
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |(record, _)| *record,
        |(record, (earlier, later))| {
            let decl = statuses.get(&record.record_type);
            let mut dates = Vec::new();
            for field in [earlier, later] {
                let value = match declared_value(record, field) {
                    Ok(v) => v,
                    Err(outcome) => return outcome,
                };
                if classify_with(Some(value), decl) != FieldState::Present
                    || date_problem(value.trim()).is_some()
                {
                    return Outcome::Absent;
                }
                dates.push(value.trim());
            }
            // `YYYY-MM-DD` orders lexically once both are real dates.
            if dates[0] > dates[1] {
                findings.push(Finding::new(
                    RULE_ID,
                    FindingSeverity::Error,
                    record.path.clone(),
                    ActionSource::new("date-order").field(later.as_str()),
                    format!(
                        "{earlier} '{}' is later than {later} '{}', but {earlier} must not be later than {later}",
                        dates[0], dates[1]
                    ),
                ));
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution::ran(RULE_ID, population, examined_records),
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

    const VALID: &str =
        "  date.field-valid:\n    level: error\n    date_fields: [Date, Accepted-on]\n";

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

    const ORDER: &str =
        "  date.field-order:\n    level: error\n    date_order: [{earlier: Date, later: Accepted-on}]\n";

    #[test]
    fn an_acceptance_date_before_the_record_date_is_reported_observed_failing() {
        let c = config("", ORDER);
        let (exec, f) = date_field_order(
            &[adr(
                "Accepted",
                "Date: 2026-03-02\nAccepted-on: 2026-03-01\n",
            )],
            &c,
        );
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("2026-03-02") && f[0].message.contains("Accepted-on"));
        assert_eq!(examined(&exec), 1);
    }

    #[test]
    fn equal_and_later_acceptance_dates_pass() {
        let c = config("", ORDER);
        for later in ["2026-03-02", "2026-03-03", "2027-01-01"] {
            let rec = adr(
                "Accepted",
                &format!("Date: 2026-03-02\nAccepted-on: {later}\n"),
            );
            let (_, f) = date_field_order(&[rec], &c);
            assert!(f.is_empty(), "{later}: {f:?}");
        }
    }

    #[test]
    fn an_absent_placeholder_or_unparseable_date_is_left_to_the_field_rules() {
        let c = config("", ORDER);
        for extra in [
            "Date: 2026-03-02\n",
            "Date: 2026-03-02\nAccepted-on: YYYY-MM-DD\n",
            "Date: 2026-03-02\nAccepted-on: nope\n",
            "Date: 2026-02-30\nAccepted-on: 2026-01-01\n",
            "Accepted-on: 2026-01-01\n",
        ] {
            let (exec, f) = date_field_order(&[adr("Accepted", extra)], &c);
            assert!(f.is_empty(), "{extra}: {f:?}");
            let p = exec.population.unwrap();
            assert_eq!((p.eligible(), p.examined()), (1, 0), "{extra}");
        }
    }

    #[test]
    fn a_type_declaring_only_one_of_the_fields_is_not_examined() {
        let c = config("", ORDER);
        let mut rec = adr("Accepted", "Date: 2026-03-02\nAccepted-on: 2026-03-01\n");
        rec.record_type = "other".to_string();
        let (exec, f) = date_field_order(&[rec], &c);
        assert!(f.is_empty());
        assert_eq!(exec.population.unwrap().eligible(), 0);
    }

    #[test]
    fn date_order_loads_only_on_its_rule_and_is_validated() {
        let bad = |rules: &str| {
            let yaml = format!("schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    required_fields: [Status]\n    known_fields: [Date, Accepted-on]\nrules:\n{rules}");
            crate::config::parse(&yaml).err().map(|e| e.to_string())
        };
        assert!(bad("  date.field-order: error\n")
            .unwrap()
            .contains("date_order"));
        assert!(bad("  date.field-valid:\n    level: error\n    date_fields: [Date]\n    date_order: [{earlier: Date, later: Accepted-on}]\n").unwrap().contains("date_order"));
        for entry in [
            "\"Date <= Accepted-on\"",
            "{earlier: Date}",
            "{earlier: Date, later: \"\"}",
            "{earlier: Date, later: Date}",
            "{earlier: Date, later: Nope}",
            "{earlier: Date, later: Accepted-on, extra: x}",
        ] {
            let rules =
                format!("  date.field-order:\n    level: error\n    date_order: [{entry}]\n");
            assert!(bad(&rules).is_some(), "{entry} must be refused");
        }
        assert!(bad(ORDER).is_none());
        assert!(bad("  date.field-order: off\n").is_none());
    }

    #[test]
    fn a_date_fields_entry_naming_no_declared_field_is_refused_observed_failing() {
        let load = |entry: &str| {
            let yaml = format!("schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    required_fields: [Status]\n    known_fields: [Date]\nrules:\n  date.field-valid:\n    level: error\n    date_fields: [{entry}]\n");
            crate::config::parse(&yaml).err().map(|e| e.to_string())
        };
        assert!(load("Date").is_none());
        assert!(load("Typo").unwrap().contains("date_fields"));
        assert!(load("Date, \"\"").unwrap().contains("blank"));
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
        assert!(
            bad("  field.quality:\n    level: error\n    date_fields: [X]\n")
                .unwrap()
                .contains("date_fields")
        );
        assert!(bad("  date.field-valid: off\n").is_none());
    }

    #[test]
    fn date_unix_converts_real_dates_and_refuses_the_rest() {
        assert_eq!(date_unix("1970-01-01"), Ok(0));
        assert_eq!(date_unix("2000-03-01"), Ok(951_868_800));
        assert_eq!(date_unix(" 2026-11-04 "), Ok(1_793_750_400));
        assert!(date_unix("2026-02-30").is_err());
        assert!(date_unix("soon").is_err());
    }
}
