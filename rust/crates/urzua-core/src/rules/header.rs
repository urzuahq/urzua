use super::*;

pub fn header_required_fields(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    let required_by_type = required_fields_by_type(config);
    let required_by_type = &required_by_type;
    const RULE_ID: &str = RULE_HEADER_REQUIRED_FIELDS;
    let mut findings = Vec::new();

    // Record-scoped findings -- an unparsed header, a duplicated key -- are
    // about the record, not about any one slot, so they are emitted once per
    // record. Decision 1's rule against reporting on a candidate you did not
    // examine governs *per-slot* findings; a record-scoped finding explaining
    // why its slots are unreadable is the opposite of a contradiction.
    for record in records {
        let Some(required) = required_by_type.get(&record.record_type) else {
            continue;
        };
        // A type that declares no required fields has no slot in this rule's
        // population, so it is not judged here (ADR-55).
        if required.is_empty() {
            continue;
        }
        // A `none`-shaped type has nowhere for a header to be (`ADR-50`): a
        // missing region here is the declared truth, not a defect to report.
        if config
            .record_types
            .get(&record.record_type)
            .is_some_and(|t| t.has_no_header())
        {
            continue;
        }

        if record.header.is_unreadable() {
            let detail = match &record.header.parse_error {
                Some(e) => format!(" -- {e}"),
                None => String::new(),
            };
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Error,
                file: record.path.clone(),
                line: None,
 waived: None,
 declared_severity: None,
 suggested_action: None,
 action_source: ActionSource::new("header-region-missing"),
                message: format!(
                    "no header-shaped region found -- required fields {required:?} cannot be checked{detail}"
                ),
            });
            continue;
        }

        for dup in record.header.duplicate_keys() {
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Error,
                file: record.path.clone(),
                line: None,
 waived: None,
 declared_severity: None,
 suggested_action: None,
 action_source: ActionSource::new("duplicate-key").field(dup.as_str()),
                message: format!("header key '{dup}' appears more than once -- ambiguous which value is operative"),
            });
        }
    }

    // The population is the declared slot: one `(record, required field)` pair
    // per field the record's type declares.
    let slots = field_slots(records, required_by_type);

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |(record, _)| *record,
        |(record, field)| {
            // An unparsed header leaves every slot of that record unreadable: the
            // rule was handed them and could not judge them. The record-scoped
            // finding above says why.
            if record.header.is_unreadable() {
                return Outcome::Unreadable;
            }
            if record.header.get(field.as_str()).is_none() {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new("field-missing").field(field.as_str()),
                    message: format!(
                        "missing required header field '{field}' for record type '{}'",
                        record.record_type
                    ),
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

/// Rule (MILE-0081): every header field on a record belongs to its type's
/// declared `required_fields` or `known_fields`. `header.required-fields`
/// only catches a *missing* required field; nothing catches the opposite --
/// a field present on some records of a type but not others (found live:
/// SPEC-1 carries `Embodiment`/`Author`/`Derives-from` that SPEC-2 through
/// SPEC-6 don't). Declared, not voted, same principle as `required_fields`: a
/// type with no declared `known_fields` is skipped entirely, examined stays
/// 0 for it -- required so an existing corpus's legitimate optional fields
/// (`Stable-Id`, `Realized-by`, `Derives-from` on `adr`, none of which are
/// *required*) don't all become false positives the moment this rule ships.
/// Exact (`ADR-57`): matching case-insensitively would accept a name nobody
/// declared -- `Maße` and `Masse` are different words.
pub fn field_is_declared(
    key: &crate::values::FieldName,
    allowed: &std::collections::BTreeSet<crate::values::FieldName>,
) -> bool {
    allowed.contains(key)
}

/// A hint on a finding, never a verdict, so an approximate answer is safe here.
///
/// `min` rather than `find`: several declared names can differ only in case, and
/// a message that changes between runs on one corpus is `BUG-79`'s shape.
pub fn near_miss<'a>(
    key: &crate::values::FieldName,
    allowed: &'a std::collections::BTreeSet<crate::values::FieldName>,
) -> Option<&'a crate::values::FieldName> {
    let key = key.as_str().to_lowercase();
    allowed
        .iter()
        .filter(|d| d.as_str().to_lowercase() == key)
        .min()
}

/// `MILE-141`: a header key beginning `x-` in any case. Matched on the
/// first two bytes only, so a multi-byte key never panics on a char boundary.
fn is_extension_key(key: &str) -> bool {
    key.get(..2).is_some_and(|p| p.eq_ignore_ascii_case("x-"))
}

pub fn header_field_set_consistency(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    let allowed_by_type = known_fields_by_type(config);
    let allowed_by_type = &allowed_by_type;
    const RULE_ID: &str = RULE_HEADER_FIELD_SET_CONSISTENCY;
    let mut findings = Vec::new();
    // Off unless declared (`MILE-141`): without the option an `x-` key is an
    // undeclared key like any other, so existing corpora see no change.
    let allow_extension_prefix = config
        .rules
        .get(RULE_ID)
        .and_then(|s| s.allow_extension_prefix)
        .unwrap_or(false);

    // Eligible: records of a type declaring `known_fields`. An unparsed header
    // stays eligible and unexamined (BUG-78) rather than vanishing.
    let candidates: Vec<&Record> = records
        .iter()
        .filter(|r| allowed_by_type.contains_key(&r.record_type))
        .collect();

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Record,
        candidates,
        |record| *record,
        |record| {
            let Some(allowed) = allowed_by_type.get(&record.record_type) else {
                return Outcome::Absent;
            };
            // A record whose header region never parsed has an empty field list,
            // which is indistinguishable here from a record whose fields are all
            // allowed. Counting it as examined inflated the one signal ADR-55 makes
            // load-bearing; `header.required-fields` reports the unparsed header
            // (BUG-78).
            if record.header.is_unreadable() {
                return Outcome::Unreadable;
            }

            for field in &record.header.fields {
                if allow_extension_prefix && is_extension_key(field.key.as_str()) {
                    continue;
                }
                if !field_is_declared(&field.key, allowed) {
                    // A key that differs from a declared one only in case is a
                    // rename, never a removal: removing it would delete the value.
                    let near = near_miss(&field.key, allowed);
                    let source = match near {
                        Some(declared) => ActionSource::new("undeclared-field-case")
                            .field(field.key.as_str())
                            .text(declared.as_str()),
                        None => ActionSource::new("undeclared-field").field(field.key.as_str()),
                    };
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Warning,
                        file: record.path.clone(),
                        line: Some(field.line),
                        waived: None,
                        declared_severity: None,
                        suggested_action: None,
                        action_source: source,
                        message: match near {
                            Some(declared) => format!(
                                "field '{}' is not declared for record type '{}' -- the type declares '{declared}', which differs only in case; rename it",
                                field.key, record.record_type
                            ),
                            None => format!(
                                "field '{}' is not declared (required_fields or known_fields) for record type '{}'",
                                field.key, record.record_type
                            ),
                        },
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

/// A declared field written under a different case than its declaration
/// (`RFC-40`, `ADR-58`): `ADR-57` makes that a different name, so `read_declared`
/// treats it as absent everywhere else, and this is the one place that says
/// which absences are really this instead of non-adoption.
pub fn header_field_case_mismatch(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    let declared_by_type = declared_fields_by_type(config);
    let declared_by_type = &declared_by_type;
    const RULE_ID: &str = RULE_HEADER_FIELD_CASE_MISMATCH;
    let mut findings = Vec::new();

    let slots = field_slots(records, declared_by_type);

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |(record, _)| *record,
        |(record, field)| {
            if record.header.is_unreadable() {
                return Outcome::Unreadable;
            }
            if record.header.get(field.as_str()).is_some() {
                return Outcome::Examined;
            }
            // `.min_by`, not `.find`: several header keys can differ from the
            // declared name only in case, and a message that changes between
            // runs on one corpus is `near_miss`'s own shape (`BUG-79`).
            let Some(found) = record
                .header
                .fields
                .iter()
                .filter(|f| {
                    f.key.as_str().eq_ignore_ascii_case(field.as_str())
                        && f.key.as_str() != field.as_str()
                })
                .min_by(|a, b| a.key.cmp(&b.key))
            else {
                return Outcome::Absent;
            };
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Error,
                file: record.path.clone(),
                line: Some(found.line),
                waived: None,
                declared_severity: None,
                suggested_action: None,
                action_source: ActionSource::new("field-case-mismatch").field(found.key.as_str()).text(field.as_str()),
                message: format!(
                    "declared field '{field}' found as '{}' -- field names compare exactly (ADR-57); rename it",
                    found.key
                ),
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

/// One record's header writing two keys that differ only by case (`MILE-113`).
///
/// Additive and declaration-free: `ADR-57` keeps every field-name comparison
/// exact, so `Status` and `status` are two fields and nothing here merges
/// them, picks a winner, or touches `duplicate_keys()` (an exact repeat of one
/// spelling stays `header.required-fields`' finding). This only reports the
/// fact of the case-only difference, for a repository that wants it.
///
/// "Differing only by case" is plain `str::to_lowercase` equality: Unicode
/// lower-casing, not case folding, so `CAFÉ`/`café` are grouped while
/// `Maße`/`Masse` stay different names, as `ADR-57` decided. Normalisation
/// forms (NFC/NFD) are likewise different names. This is a reporting
/// heuristic over keys already present, so a mis-grouping cannot produce a
/// wrong verdict elsewhere.
pub fn header_case_variant_key(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_HEADER_CASE_VARIANT_KEY;
    let mut findings = Vec::new();

    let candidates: Vec<&Record> = records.iter().collect();
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Record,
        candidates,
        |record| *record,
        |record| {
            if record.header.is_unreadable() {
                return Outcome::Unreadable;
            }
            // Groups in order of first appearance, each holding its distinct
            // exact spellings (with the line of the first) in header order, so
            // the output is stable across runs.
            let mut groups: Vec<(String, Vec<(&str, usize)>)> = Vec::new();
            for field in &record.header.fields {
                let key = field.key.as_str();
                let lowered = key.to_lowercase();
                let group = match groups.iter().position(|(l, _)| *l == lowered) {
                    Some(i) => &mut groups[i].1,
                    None => {
                        groups.push((lowered, Vec::new()));
                        &mut groups.last_mut().expect("just pushed").1
                    }
                };
                if !group.iter().any(|(seen, _)| *seen == key) {
                    group.push((key, field.line));
                }
            }
            for (_, spellings) in groups.iter().filter(|(_, s)| s.len() > 1) {
                let named = spellings
                    .iter()
                    .map(|(key, line)| format!("'{key}' (line {line})"))
                    .collect::<Vec<_>>()
                    .join(", ");
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: Some(spellings[0].1),
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new("case-variant-key").field(spellings[0].0),
                    message: format!(
                        "header writes keys differing only by case: {named} -- field names compare exactly (ADR-57), so these are different fields; this reports the difference, not which spelling is right"
                    ),
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

    #[test]
    fn header_field_set_consistency_does_not_count_an_unparsed_header() {
        // An unparsed header leaves an empty field list, which is
        // indistinguishable here from a record whose fields are all allowed.
        // Counting it inflates the signal ADR-55 makes load-bearing (BUG-78).
        let (r, _) = yaml_record(
            "docs/adr/ADR-1-x.md",
            "---\nStatus: [unclosed\n---\n# 1 — X\n",
        );
        let config = config_for_known(vec![("adr", vec!["status"])]);
        let (exec, _) = header_field_set_consistency(&[r], &config);
        let population = exec.population.expect("the rule carries a population");
        assert_eq!(
            (population.eligible(), population.examined()),
            (1, 0),
            "the record is in the population and the rule never saw its fields"
        );
    }

    #[test]
    fn a_case_only_near_miss_suggests_a_rename_never_a_removal_observed_failing() {
        let (r, _) = yaml_record("docs/adr/ADR-1-x.md", "---\nstatus: Open\n---\n# 1 — X\n");
        let config = config_for_known(vec![("adr", vec!["Status"])]);
        let (_, findings) = header_field_set_consistency(&[r], &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
        let src = &findings[0].action_source;
        assert_eq!(src.variant, "undeclared-field-case");
        assert_eq!(src.target.field.as_deref(), Some("status"));
        assert!(matches!(&src.value, Some(crate::report::ActionValue::Text(t)) if t == "Status"));
    }

    #[test]
    fn a_headerless_record_of_a_type_declaring_no_required_fields_is_not_judged_observed_failing() {
        let a = record("docs/a/A-1-x.md", "a", "---\nStatus: Open\n---\n");
        let b = record("docs/b/B-1-y.md", "b", "No header here.\n");
        let config = config_for_required(vec![("a", vec!["Status"]), ("b", vec![])]);
        let (exec, findings) = header_required_fields(&[a, b], &config);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn missing_required_field_is_a_finding() {
        let r = record("docs/adr/0001-x.md", "adr", "---\nStatus: Accepted\n---\n");
        let config = config_for_required(vec![("adr", vec!["Status", "Deciders"])]);

        let (exec, findings) = header_required_fields(&[r], &config);
        assert_eq!(examined(&exec), 2, "both declared slots were read");
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Deciders"));
    }

    /// `BUG-98`/`ADR-50`: a type declaring `header_shape: none` has no
    /// header at all, by declaration -- a missing region there is the
    /// declared truth, not a defect to report.
    #[test]
    fn a_type_declaring_no_header_is_not_reported_as_missing_one_observed_failing() {
        let r = Record::parse_with_shape(
            PathBuf::from("docs/decisions/0001-x.md"),
            "dec".to_string(),
            "Just prose. No header block anywhere in this file.\n",
            crate::header::HeaderShape::None,
        );
        let config = config_with_types(vec![(
            "dec",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                header_shape: crate::header::HeaderShape::None,
                ..type_config(None)
            },
        )]);

        let (exec, findings) = header_required_fields(&[r], &config);
        assert_eq!(
            examined(&exec),
            0,
            "a type with no header has nothing for this rule to examine"
        );
        assert!(
            findings.is_empty(),
            "a declared absence of a header must not be reported as a missing one: {findings:?}"
        );
    }

    #[test]
    fn a_broken_yaml_header_surfaces_the_real_parse_error_observed_failing() {
        // BUG-0012: before this fix, a header that failed to parse as YAML
        // produced only "no header-shaped region found" -- the real
        // yaml_serde error (line, reason) was discarded, forcing manual
        // byte-level diagnosis every time this was actually hit.
        let r = Record::parse_with_shape(
            PathBuf::from("docs/adr/ADR-1-x.md"),
            "adr".to_string(),
            "---\nSubject: `oops\nStatus: Draft\n---\n# Title\n",
            crate::header::HeaderShape::YamlFrontmatter,
        );
        let config = config_for_required(vec![("adr", vec!["Status"])]);

        let (exec, findings) = header_required_fields(&[r], &config);
        let population = exec.population.expect("the rule carries a population");
        assert_eq!(
            (population.eligible(), population.examined()),
            (1, 0),
            "the declared slot survives a header that did not parse, unjudged"
        );
        assert_eq!(findings.len(), 1);
        assert!(
            findings[0].message.contains("YAML parse error"),
            "expected the real parse error surfaced, got: {}",
            findings[0].message
        );
    }

    #[test]
    fn a_present_required_field_produces_no_finding() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nStatus: Accepted\nDeciders: someone\n---\n",
        );
        let config = config_for_required(vec![("adr", vec!["Status", "Deciders"])]);

        let (exec, findings) = header_required_fields(&[r], &config);
        assert_eq!(examined(&exec), 2, "both declared slots were read");
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn a_planted_violation_is_observed_failing() {
        // The exact corpus defect this rule was chosen to catch: SPEC-0001's
        // shape lacks a field SPEC-0002-0005 share.
        let r = record(
            "docs/specs/0001-x.md",
            "spec",
            "---\nStatus: Draft\nEmbodiment: Not started\n---\n",
        );
        let config = config_for_required(vec![("spec", vec!["Version"])]);

        let (_, findings) = header_required_fields(&[r], &config);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Version"));
    }

    #[test]
    fn an_undeclared_field_is_a_finding() {
        // The exact corpus defect this rule was chosen to catch: SPEC-1
        // carries Embodiment/Author/Derives-from that SPEC-2 through SPEC-6
        // don't -- modeled here on the smaller adr case for a focused test.
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nStatus: Accepted\nDate: 2026-09-07\nSponsor: someone\n---\n",
        );
        let config = config_for_known(vec![("adr", vec!["Status", "Date"])]);

        let (exec, findings) = header_field_set_consistency(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Sponsor"));
    }

    #[test]
    fn a_known_field_produces_no_finding() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nStatus: Accepted\nRealized-by: code:x.rs\n---\n",
        );
        let config = config_for_known(vec![("adr", vec!["Status", "Realized-by"])]);

        let (exec, findings) = header_field_set_consistency(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_type_with_no_declared_known_fields_is_skipped_entirely() {
        let r = record("docs/specs/0001-x.md", "spec", "---\nVersion: 0.1\n---\n");
        let config = config_with_types(vec![]);

        let (exec, findings) = header_field_set_consistency(&[r], &config);
        assert_eq!(examined(&exec), 0);
        assert!(findings.is_empty());
    }

    /// BUG-101: a declared field written under a different case used to be
    /// silently absent everywhere. This rule is the one place that names it.
    #[test]
    fn a_declared_field_written_under_a_different_case_is_reported_observed_failing() {
        let r = record("docs/adr/ADR-1-x.md", "adr", "---\nstatus: Accepted\n---\n");
        let config = config_for_required(vec![("adr", vec!["Status"])]);
        let (exec, findings) = header_field_case_mismatch(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("'status'"), "{findings:?}");
        assert_eq!(findings[0].severity, FindingSeverity::Error);
    }

    #[test]
    fn a_declared_field_written_correctly_cased_is_not_reported() {
        let r = record("docs/adr/ADR-1-x.md", "adr", "---\nStatus: Accepted\n---\n");
        let config = config_for_required(vec![("adr", vec!["Status"])]);
        let (exec, findings) = header_field_case_mismatch(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_declared_field_never_written_at_all_is_absent_not_reported() {
        let r = record("docs/adr/ADR-1-x.md", "adr", "---\nAuthor: x\n---\n");
        let config = config_for_required(vec![("adr", vec!["Status"])]);
        let (exec, findings) = header_field_case_mismatch(&[r], &config);
        assert_eq!(
            examined(&exec),
            0,
            "genuine non-adoption is Absent, not Examined"
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// Same gap, `header_required_fields`'s own per-slot population loop --
    /// distinct from its record-scoped loop, which already gated correctly.
    #[test]
    fn a_header_none_type_is_out_of_scope_not_unreadable_for_header_required_fields_observed_failing(
    ) {
        let r = record("code/src.rs", "code", "fn main() {}\n");
        let config = config_with_types(vec![(
            "code",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                required_fields: vec!["Reviewers".to_string()],
                ..type_config_with_shape(crate::header::HeaderShape::None)
            },
        )]);

        let (exec, findings) = header_required_fields(&[r], &config);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(
            (
                population.examined(),
                population.unreadable(),
                population.out_of_scope()
            ),
            (0, 0, 1),
            "{population:?}"
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// Round 27's own sweep found the same `BUG-142`/`ADR-50` gap in three
    /// more rules that share the shape of reading `known_fields`-declared
    /// slots but not `header_required_fields`'s specific field list.
    #[test]
    fn a_header_none_type_is_out_of_scope_not_unreadable_for_header_field_set_consistency_observed_failing(
    ) {
        let r = record("code/src.rs", "code", "fn main() {}\n");
        let config = config_with_types(vec![(
            "code",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                known_fields: Some(vec!["Reviewers".to_string()]),
                ..type_config_with_shape(crate::header::HeaderShape::None)
            },
        )]);

        let (exec, findings) = header_field_set_consistency(&[r], &config);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(
            (
                population.examined(),
                population.unreadable(),
                population.out_of_scope()
            ),
            (0, 0, 1),
            "{population:?}"
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_header_none_type_is_out_of_scope_not_unreadable_for_header_field_case_mismatch_observed_failing(
    ) {
        let r = record("code/src.rs", "code", "fn main() {}\n");
        let config = config_with_types(vec![(
            "code",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                required_fields: vec!["Reviewers".to_string()],
                ..type_config_with_shape(crate::header::HeaderShape::None)
            },
        )]);

        let (exec, findings) = header_field_case_mismatch(&[r], &config);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(
            (
                population.examined(),
                population.unreadable(),
                population.out_of_scope()
            ),
            (0, 0, 1),
            "{population:?}"
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    // header.case-variant-key (MILE-113). Opt-in, never declaration-driven:
    // it reads the keys a record wrote, not the vocabulary its type declared.

    #[test]
    fn two_keys_differing_only_by_case_are_reported_observed_failing() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nStatus: Accepted\nstatus: Draft\n---\n",
        );
        let config = config_for_required(vec![("adr", vec!["Status"])]);
        let (exec, findings) = header_case_variant_key(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("'Status'"), "{findings:?}");
        assert!(findings[0].message.contains("'status'"), "{findings:?}");
        assert_eq!(findings[0].rule, RULE_HEADER_CASE_VARIANT_KEY);
        assert!(findings[0].line.is_some(), "{findings:?}");
    }

    #[test]
    fn the_rule_needs_no_field_declaration_at_all_observed_failing() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nStatus: Accepted\nstatus: Draft\n---\n",
        );
        let config = config_with_types(vec![("adr", type_config(None))]);
        let (exec, findings) = header_case_variant_key(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn three_spellings_of_one_key_are_one_finding_naming_all_three() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nStatus: A\nstatus: B\nSTATUS: C\n---\n",
        );
        let config = config_with_types(vec![("adr", type_config(None))]);
        let (_, findings) = header_case_variant_key(&[r], &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
        for spelling in ["'Status'", "'status'", "'STATUS'"] {
            assert!(findings[0].message.contains(spelling), "{findings:?}");
        }
    }

    #[test]
    fn two_independent_case_variant_pairs_are_two_findings_in_header_order() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nStatus: A\nAuthor: x\nstatus: B\nauthor: y\n---\n",
        );
        let config = config_with_types(vec![("adr", type_config(None))]);
        let (_, findings) = header_case_variant_key(&[r], &config);
        assert_eq!(findings.len(), 2, "{findings:?}");
        assert!(findings[0].message.contains("'Status'"), "{findings:?}");
        assert!(findings[1].message.contains("'Author'"), "{findings:?}");
    }

    #[test]
    fn a_record_with_no_case_variants_is_examined_and_clean() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nStatus: Accepted\nAuthor: x\n---\n",
        );
        let config = config_with_types(vec![("adr", type_config(None))]);
        let (exec, findings) = header_case_variant_key(&[r], &config);
        assert_eq!(
            examined(&exec),
            1,
            "a clean pass must say what it looked at"
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// A header with a repeated key. YAML frontmatter rejects one outright, so
    /// it is built by hand, as a `prefix-lines` header can carry it.
    fn record_with_fields(record_type: &str, fields: &[(&str, &str)]) -> Record {
        Record {
            path: PathBuf::from("docs/adr/ADR-1-x.md"),
            record_type: record_type.to_string(),
            type_prefix: "ADR".to_string(),
            header: crate::header::Header {
                fields: fields
                    .iter()
                    .enumerate()
                    .map(|(i, (k, v))| crate::header::HeaderField {
                        key: crate::values::FieldName::new(*k),
                        value: v.to_string(),
                        line: i + 2,
                    })
                    .collect(),
                region: Some((1, fields.len() + 2)),
                parse_error: None,
            },
            identity: None,
        }
    }

    /// An exact repeat is `duplicate_keys()`'s signal, reported by
    /// `header.required-fields`; this rule is composable alongside it, not a
    /// second report of the same fact.
    #[test]
    fn an_exact_repeat_is_not_this_rule_s_finding() {
        let r = record_with_fields("adr", &[("Status", "A"), ("Status", "B")]);
        let config = config_with_types(vec![("adr", type_config(None))]);
        let (exec, findings) = header_case_variant_key(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn an_exact_repeat_beside_a_case_variant_still_reports_the_variant_once() {
        let r = record_with_fields("adr", &[("Status", "A"), ("Status", "B"), ("status", "C")]);
        let config = config_with_types(vec![("adr", type_config(None))]);
        let (_, findings) = header_case_variant_key(&[r], &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
        let msg = &findings[0].message;
        assert_eq!(msg.matches("'Status'").count(), 1, "{msg}");
    }

    /// `ADR-57`: a different spelling is a different field, and the engine
    /// does not decide that `Masse` and `Maße` are one name. Only a
    /// lower-casing difference is reported.
    #[test]
    fn non_ascii_case_variants_are_reported_but_sharp_s_is_not_folded() {
        let variant = record("docs/adr/ADR-1-x.md", "adr", "---\nCAFÉ: a\ncafé: b\n---\n");
        let sharp_s = record(
            "docs/adr/ADR-2-x.md",
            "adr",
            "---\nMaße: a\nMasse: b\n---\n",
        );
        let config = config_with_types(vec![("adr", type_config(None))]);
        let (exec, findings) = header_case_variant_key(&[variant, sharp_s], &config);
        assert_eq!(examined(&exec), 2);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].file.ends_with("ADR-1-x.md"), "{findings:?}");
    }

    #[test]
    fn variants_in_two_different_records_are_not_one_pair() {
        let a = record("docs/adr/ADR-1-x.md", "adr", "---\nStatus: A\n---\n");
        let b = record("docs/adr/ADR-2-x.md", "adr", "---\nstatus: B\n---\n");
        let config = config_with_types(vec![("adr", type_config(None))]);
        let (exec, findings) = header_case_variant_key(&[a, b], &config);
        assert_eq!(examined(&exec), 2);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn works_on_yaml_frontmatter_headers_too() {
        let (r, _) = yaml_record(
            "docs/adr/ADR-1-x.md",
            "---\nStatus: A\nstatus: B\n---\n# 1 — X\n",
        );
        let config = config_with_types(vec![("adr", type_config(None))]);
        let (exec, findings) = header_case_variant_key(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn an_unreadable_header_is_unreadable_not_clean() {
        let (r, _) = yaml_record(
            "docs/adr/ADR-1-x.md",
            "---\nStatus: [unclosed\n---\n# 1 — X\n",
        );
        let config = config_with_types(vec![("adr", type_config(None))]);
        let (exec, findings) = header_case_variant_key(&[r], &config);
        let population = exec.population.expect("the rule carries a population");
        assert_eq!(
            (
                population.eligible(),
                population.examined(),
                population.unreadable()
            ),
            (1, 0, 1),
            "{population:?}"
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_header_none_type_is_out_of_scope_for_case_variant_key() {
        let r = record("code/src.rs", "code", "fn main() {}\n");
        let config = config_with_types(vec![(
            "code",
            type_config_with_shape(crate::header::HeaderShape::None),
        )]);
        let (exec, findings) = header_case_variant_key(&[r], &config);
        let population = exec.population.expect("the rule carries a population");
        assert_eq!(
            (
                population.examined(),
                population.unreadable(),
                population.out_of_scope()
            ),
            (0, 0, 1),
            "{population:?}"
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// `ADR-57` stays exactly as decided: this rule adds a signal and changes
    /// no exact-match verdict.
    #[test]
    fn it_leaves_the_exact_match_rules_verdicts_unchanged() {
        let config = config_for_required(vec![("adr", vec!["Status"])]);
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nStatus: A\nstatus: B\n---\n",
        );
        assert!(r.header.duplicate_keys().is_empty());
        assert_eq!(r.header.get("Status"), Some("A"));
        let (_, required) = header_required_fields(std::slice::from_ref(&r), &config);
        assert!(required.is_empty(), "{required:?}");
    }

    /// `MILE-141`: a repository declaring `allow_extension_prefix: true` lets
    /// `x-` keys (any case) through the closed key set, and no other key.
    fn field_set_config(allow: Option<bool>) -> Config {
        let option = match allow {
            Some(v) => format!("    allow_extension_prefix: {v}\n"),
            None => String::new(),
        };
        let yaml = format!(
            "schema_version: 2\nrules:\n  header.field-set-consistency:\n    level: warn\n{option}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields: [\"Status\"]\n"
        );
        crate::config::parse(&yaml).expect("the fixture config loads")
    }

    #[test]
    fn an_x_prefixed_key_is_exempt_when_the_option_is_declared_observed_failing() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nStatus: Accepted\nx-Local: 1\nX-Other: 2\nSponsor: someone\n---\n",
        );
        let (exec, findings) = header_field_set_consistency(&[r], &field_set_config(Some(true)));
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("'Sponsor'"), "{findings:?}");
    }

    #[test]
    fn an_unprefixed_unknown_key_stays_a_finding_when_the_option_is_declared() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nStatus: Accepted\nxlocal: 1\nx_local: 2\nLocal-x-: 3\n---\n",
        );
        let (_, findings) = header_field_set_consistency(&[r], &field_set_config(Some(true)));
        assert_eq!(findings.len(), 3, "{findings:?}");
    }

    #[test]
    fn an_x_prefixed_key_is_a_finding_when_the_option_is_absent_or_false() {
        for allow in [None, Some(false)] {
            let r = record(
                "docs/adr/0001-x.md",
                "adr",
                "---\nStatus: Accepted\nx-Local: 1\n---\n",
            );
            let (_, findings) = header_field_set_consistency(&[r], &field_set_config(allow));
            assert_eq!(findings.len(), 1, "allow={allow:?}: {findings:?}");
            assert!(findings[0].message.contains("'x-Local'"), "{findings:?}");
        }
    }
}
