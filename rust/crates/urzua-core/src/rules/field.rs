use super::*;

/// Rule 3 (Phase 6): a required field's *quality*, not just its presence.
/// `header.required-fields` only asks "is the key there"; a field holding
/// unedited template text or an explicit pending marker passes that check
/// and still isn't a real value. Blank is reported here too (redundantly
/// with Rule 1) so this rule's own report is self-contained.
/// Split out of `field.quality` (BUG-38). A required field marked `Pending`
/// says someone decided the work is unfinished; a `Blank` one says someone
/// forgot. A repository that does not declare this rule is not told about its
/// own deliberate markers, which is the right default -- it wrote them.
pub fn field_pending(records: &[Record], config: &Config) -> (RuleExecution, Vec<Finding>) {
    let required_by_type = required_fields_by_type(config);
    let required_by_type = &required_by_type;
    let statuses = statuses_by_type(config);
    const RULE_ID: &str = RULE_FIELD_PENDING;
    let mut findings = Vec::new();

    // Same population as `field.quality`: one declared slot per required field.
    let slots = field_slots(records, required_by_type);

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |(record, _)| *record,
        |(record, field)| {
            // Unreadable, not absent -- the same reason as `field.quality`.
            if record.header.is_unreadable() {
                return Outcome::Unreadable;
            }
            let declared = statuses.get(&record.record_type);
            if classify_with(record.header.get(field.as_str()), declared) == FieldState::Pending {
                let declared_severity = declared_field_severity(config, RULE_ID, field.as_str());
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: declared_severity.unwrap_or(FindingSeverity::Warning),
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity,
                    message: format!(
                        "field '{field}' is marked pending -- work declared unfinished"
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

/// A declared field whose value carries leading or trailing whitespace.
///
/// Every other rule compares a value exactly (`ADR-57`), so `"Superseded   "`
/// is not `Superseded` and a status rule configured to report it stays silent.
/// Trimming inside those rules would paper over it; this reports it instead --
/// the split `yamllint` makes between formatting and meaning.
///
/// YAML trims an unquoted scalar, so it takes a quoted value to carry the
/// space this far.
pub fn field_untrimmed_value(records: &[Record], config: &Config) -> (RuleExecution, Vec<Finding>) {
    let declared_by_type = declared_fields_by_type(config);
    let declared_by_type = &declared_by_type;
    const RULE_ID: &str = RULE_FIELD_UNTRIMMED_VALUE;
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
            let Some(value) = record.header.get(field.as_str()) else {
                return Outcome::Absent;
            };
            if value != value.trim() {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Warning,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    message: format!(
                        "field '{field}' is {value:?} -- the surrounding whitespace is part of the value, so any exact comparison against '{}' fails",
                        value.trim()
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

/// Characters YAML reserves as scalar indicators: a plain (unquoted) scalar
/// cannot begin with one. `BUG-18` found `@` doing exactly this to every
/// hand-typed `Author`/`Deciders` value -- decorative, forcing a quoting
/// workaround for a character that carried no meaning anywhere in this
/// codebase. Deliberately excludes backtick: this rule's own first run
/// found nine `Subject` values quoted only because they open with a
/// backtick-quoted command name -- meaningful Markdown, not decoration, so
/// including it would make the rule permanently noisy on legitimate content
/// rather than catching the next `BUG-18`. Also excludes `:`, `-`, `#`,
/// which YAML only treats as indicators in specific positions and would
/// false-positive on ordinary prose-like values.
const RESERVED_LEADING_CHARS: &[char] = &['@', '*', '&', '!', '%', '|', '>'];

/// A declared field's value starting with a reserved YAML indicator
/// character (`BUG-18`'s defect class, generalized past `Author`/`Deciders`):
/// the character forces quoting to parse at all, and a hand-typed value only
/// needs that quoting *because* the character is there. This does not assert
/// the character is meaningless -- only that it costs a reader nothing to
/// confirm, the same way `BUG-18`'s investigation had to grep the whole
/// codebase by hand to learn `@` was decorative before removing it. Distinct
/// from `BUG-12`'s defect (an *unquoted* reserved character breaking the
/// header's own parse, surfaced reactively via the real YAML error): this
/// rule only ever examines a value the header already parsed successfully.
pub fn field_leading_reserved_indicator(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    let declared_by_type = declared_fields_by_type(config);
    let declared_by_type = &declared_by_type;
    const RULE_ID: &str = RULE_FIELD_LEADING_RESERVED_INDICATOR;
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
            let Some(value) = record.header.get(field.as_str()) else {
                return Outcome::Absent;
            };
            if let Some(first) = value.chars().next() {
                if RESERVED_LEADING_CHARS.contains(&first) {
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Warning,
                        file: record.path.clone(),
                        line: None,
                        waived: None,
                        declared_severity: None,
                        message: format!(
                            "field '{field}' is {value:?} -- it needs quoting only because it starts with '{first}', a reserved YAML indicator character; confirm the character is meaningful, or drop it (BUG-18)"
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

pub fn field_quality(records: &[Record], config: &Config) -> (RuleExecution, Vec<Finding>) {
    let required_by_type = required_fields_by_type(config);
    let required_by_type = &required_by_type;
    const RULE_ID: &str = RULE_FIELD_QUALITY;
    let mut findings = Vec::new();

    // The population is the declared slot: one `(record, required field)` pair
    // per field the record's *type* declares. Built here, before the rule runs,
    // so `eligible` is this list's length and cannot disagree with what the
    // body does. It is far larger than the record count and always was
    // (`BUG-40`); the unit is what makes that legible rather than alarming.
    let slots = field_slots(records, required_by_type);

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |(record, _)| *record,
        |(record, field)| {
            // Unreadable, not absent: the header did not parse, so this slot
            // has no value to classify. Calling it Blank reports something
            // untrue about a file that may well set the field, and a rule whose
            // `examined` can never fall short of `eligible` can never be caught
            // not looking (ADR-55). `header.required-fields` reports the parse
            // error itself, once per record rather than once per slot.
            if record.header.is_unreadable() {
                return Outcome::Unreadable;
            }
            let state = classify(record.header.get(field.as_str()));
            // `Pending` is `field.pending`'s subject, not this rule's: it means
            // someone declared the work unfinished, where Blank and Placeholder
            // mean someone forgot. One rule carries one declared level, so
            // keeping both here left no setting that was correct (BUG-38).
            if matches!(state, FieldState::Blank | FieldState::Placeholder) {
                let declared_severity = declared_field_severity(config, RULE_ID, field.as_str());
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: declared_severity.unwrap_or(FindingSeverity::Error),
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity,
                    message: format!("field '{field}' is {state:?} -- not a real, present value"),
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

    /// Full builder for the config-level pointer/narrative-field validation
    /// tests (MILE-90) -- the two helpers above default both new fields to
    /// `None`, which isn't useful for testing them directly.
    /// BUG-65: a type that declares its pending vocabulary is judged by whole
    /// values; a value merely mentioning the word is a real value.
    #[test]
    fn a_declared_pending_vocabulary_matches_whole_values_observed_failing() {
        let yaml = "schema_version: 2\nrecord_types:\n  bug:\n    dir: docs/bugs\n    header_shape: yaml-frontmatter\n    required_fields: [Status, Regression-test]\n    statuses:\n      values: [Open, Fixed]\n      pending: [\"not yet written\"]\n";
        let config = crate::config::parse(yaml).unwrap();
        let mk = |v: &str| {
            Record::parse_with_shape_and_prefix(
                PathBuf::from("docs/bugs/BUG-1-x.md"),
                "bug".to_string(),
                &format!("---\nStatus: Open\nRegression-test: \"{v}\"\n---\n# 1 - X\n"),
                crate::header::HeaderShape::YamlFrontmatter,
                "BUG".to_string(),
            )
        };
        let (_, findings) = field_pending(&[mk("Not yet written")], &config);
        assert_eq!(
            findings.len(),
            1,
            "a declared whole value is pending: {findings:?}"
        );
        let (_, findings) =
            field_pending(&[mk("a_pending_field_belongs_to_field_pending")], &config);
        assert!(
            findings.is_empty(),
            "a mere mention is a real value: {findings:?}"
        );
        let (_, findings) = field_pending(&[mk("Pending review")], &config);
        assert!(
            findings.is_empty(),
            "the declared vocabulary replaces the token: {findings:?}"
        );
    }

    #[test]
    fn a_type_declaring_no_pending_vocabulary_keeps_the_substring_match() {
        let config = config_for_required(vec![("bug", vec!["Status"])]);
        let r = record(
            "docs/bugs/BUG-1-x.md",
            "bug",
            "---\nStatus: Pending review\n---\n",
        );
        let (_, findings) = field_pending(&[r], &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_slot_behind_an_unparsed_header_is_not_judged_and_not_reported_on() {
        // The record plainly sets `Status`, and the header does not parse, so
        // the rule cannot read either slot. Calling both Blank reports
        // something untrue about the file, and pinning `examined` to `eligible`
        // makes the rule invisible to the one check ADR-55 asks for: a rule
        // whose population can never show a gap can never be caught not
        // looking. `header.required-fields` reports the parse error itself.
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nStatus: Accepted\n  Date  :: nope\n---\n",
        );
        let config = config_for_required(vec![("adr", vec!["Status", "Date"])]);

        let (exec, findings) = field_quality(&[r.clone()], &config);
        let population = exec.population.expect("the rule carries a population");
        assert_eq!(
            (population.eligible(), population.examined()),
            (2, 0),
            "both declared slots are unreadable, not judged"
        );
        assert!(
            findings.is_empty(),
            "no finding about a slot the rule could not read: {findings:?}"
        );

        let (exec, findings) = field_pending(&[r], &config);
        let population = exec.population.expect("the rule carries a population");
        assert_eq!((population.eligible(), population.examined()), (2, 0));
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_value_with_trailing_whitespace_is_reported_observed_failing() {
        let r = Record::parse_with_shape_and_prefix(
            PathBuf::from("docs/adr/ADR-1-x.md"),
            "adr".to_string(),
            "---\nStatus: \"Superseded   \"\n---\n# 1 — X\n",
            crate::header::HeaderShape::YamlFrontmatter,
            "ADR".to_string(),
        );
        let config = config_for_required(vec![("adr", vec!["Status"])]);

        let (exec, findings) = field_untrimmed_value(&[r], &config);
        assert_eq!(examined(&exec), 1, "the slot is written, so it is judged");
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Superseded"));
    }

    /// The declared-field set `field_untrimmed_value` iterates
    /// (`declared_fields_by_type`) is a `BTreeSet`, not a `HashSet` -- two
    /// untrimmed fields on one record must report in the same order every
    /// run, not whichever order a randomized hasher seed happened to yield.
    #[test]
    fn two_untrimmed_fields_report_in_a_deterministic_order_observed_failing() {
        let r = Record::parse_with_shape_and_prefix(
            PathBuf::from("docs/adr/ADR-1-x.md"),
            "adr".to_string(),
            "---\nZeta: \"z   \"\nAlpha: \"a   \"\n---\n# 1 — X\n",
            crate::header::HeaderShape::YamlFrontmatter,
            "ADR".to_string(),
        );
        let config = config_for_required(vec![("adr", vec!["Zeta", "Alpha"])]);

        let (_, findings) = field_untrimmed_value(&[r], &config);
        assert_eq!(findings.len(), 2, "{findings:?}");
        assert!(
            findings[0].message.contains("Alpha") && findings[1].message.contains("Zeta"),
            "findings must report in a fixed (sorted) order regardless of declaration order: {findings:?}"
        );
    }

    #[test]
    fn a_clean_value_is_not_reported_as_untrimmed() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nStatus: Superseded\n---\n",
        );
        let config = config_for_required(vec![("adr", vec!["Status"])]);
        let (exec, findings) = field_untrimmed_value(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// `BUG-18`: a hand-typed `Author: '@beauwilliams'` only needs quoting
    /// because of the decorative `@` -- the same defect this rule generalizes
    /// past `Author`/`Deciders` to any declared field.
    #[test]
    fn a_value_starting_with_a_reserved_yaml_indicator_is_reported_observed_failing() {
        let r = Record::parse_with_shape_and_prefix(
            PathBuf::from("docs/adr/ADR-1-x.md"),
            "adr".to_string(),
            "---\nAuthor: '@beauwilliams'\n---\n# 1 — X\n",
            crate::header::HeaderShape::YamlFrontmatter,
            "ADR".to_string(),
        );
        let config = config_for_required(vec![("adr", vec!["Author"])]);

        let (exec, findings) = field_leading_reserved_indicator(&[r], &config);
        assert_eq!(examined(&exec), 1, "the slot is written, so it is judged");
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains('@'));
        assert!(findings[0].message.contains("BUG-18"));
    }

    #[test]
    fn a_value_with_no_reserved_leading_character_is_not_reported() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nAuthor: beauwilliams\n---\n",
        );
        let config = config_for_required(vec![("adr", vec!["Author"])]);
        let (exec, findings) = field_leading_reserved_indicator(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// An em dash, this corpus's own "no value" convention, must not be
    /// mistaken for a reserved indicator -- it isn't one, and `—` is a
    /// deliberate, common, legitimate value across many declared fields.
    #[test]
    fn the_no_value_em_dash_sentinel_is_not_reported() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nSupersedes / Superseded-by: —\n---\n",
        );
        let config = config_for_required(vec![("adr", vec!["Supersedes / Superseded-by"])]);
        let (_, findings) = field_leading_reserved_indicator(&[r], &config);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_status_carrying_whitespace_does_not_silently_match_and_is_reported() {
        // ADR-57 compares exactly, so "Superseded   " is not Superseded and the
        // status rule is right to stay silent. Trimming inside it would paper
        // over a value the author did not write; field.untrimmed-value reports
        // the whitespace instead, which is where the adopter can act on it.
        let yaml = |path: &str, content: &str| {
            Record::parse_with_shape_and_prefix(
                PathBuf::from(path),
                "adr".to_string(),
                content,
                crate::header::HeaderShape::YamlFrontmatter,
                "ADR".to_string(),
            )
        };
        let target = yaml(
            "docs/adr/ADR-1-x.md",
            "---\nStatus: \"Superseded   \"\n---\n# 1 — X\n",
        );
        let source = yaml(
            "docs/adr/ADR-2-y.md",
            "---\nDerives-from: ADR-1\n---\n# 2 — Y\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&["Status"], None, Some(&["Derives-from"]), Some(&[])),
        )]);

        let target_status_records = [target.clone(), source];
        let (_, status_findings) = pointer_target_status(
            &target_status_records,
            &config,
            &["Superseded".to_string()],
            None,
            &build_normalized_index(&target_status_records),
        );
        assert!(
            status_findings.is_empty(),
            "the value is not that status: {status_findings:?}"
        );

        let (_, whitespace) = field_untrimmed_value(&[target], &config);
        assert_eq!(
            whitespace.len(),
            1,
            "and the whitespace must be reported, or nothing tells the author"
        );
    }

    #[test]
    fn field_quality_flags_placeholder_not_just_absence() {
        // A field present per Rule 1 (the key exists) but never actually
        // filled in -- Rule 1 alone would pass this record.
        let r = record("docs/adr/0001-x.md", "adr", "---\nAuthor: name\n---\n");
        let config = config_for_required(vec![("adr", vec!["Author"])]);

        let (exec, findings) = field_quality(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Placeholder"));
    }

    /// `BUG-125`: a rule's silence about an unreadable header must not depend
    /// on `header.required-fields` being separately enabled to disclose it --
    /// a pairing `ADR-53` never guarantees. Each rule's own `Population`
    /// discloses `unreadable` on its own, regardless of what else is enabled.
    #[test]
    fn field_quality_discloses_unreadable_on_its_own_observed_failing() {
        let r = record("docs/adr/0001-x.md", "adr", "this is not a header at all");
        let config = config_for_required(vec![("adr", vec!["Status"])]);

        let (exec, findings) = field_quality(&[r], &config);
        assert_eq!(examined(&exec), 0, "{exec:?}");
        assert_eq!(unreadable(&exec), 1, "{exec:?}");
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_pending_field_belongs_to_field_pending_not_field_quality() {
        // BUG-38, observed on this repo's own corpus: `field.quality` held both
        // states, so a repository declaring it `error` had a deliberate
        // `Pending` marker block CI, and declaring it `warn` stopped a genuinely
        // blank required field from blocking. No setting was correct.
        let r = record("docs/adr/0001-x.md", "adr", "---\nDeciders: Pending\n---\n");
        let config = config_for_required(vec![("adr", vec!["Deciders"])]);

        let (_, quality) = field_quality(std::slice::from_ref(&r), &config);
        assert!(quality.is_empty(), "{quality:?}");

        let (_, pending) = field_pending(std::slice::from_ref(&r), &config);
        assert_eq!(pending.len(), 1);
        assert!(pending[0].message.contains("pending"));

        // The forgotten case stays with field.quality, and field.pending
        // must not claim it.
        let blank = record("docs/adr/0002-y.md", "adr", "---\nDeciders:\n---\n");
        let (_, q2) = field_quality(std::slice::from_ref(&blank), &config);
        assert_eq!(q2.len(), 1);
        assert_eq!(q2[0].severity, FindingSeverity::Error);
        let (_, p2) = field_pending(&[blank], &config);
        assert!(p2.is_empty(), "{p2:?}");
    }

    #[test]
    fn field_quality_passes_a_real_value() {
        let r = record("docs/adr/0001-x.md", "adr", "---\nAuthor: someone\n---\n");
        let config = config_for_required(vec![("adr", vec!["Author"])]);

        let (_, findings) = field_quality(&[r], &config);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn field_quality_flags_a_retired_placeholder_as_an_error() {
        // BUG-5: (project lead) used to be this project's own placeholder
        // convention and classified as Present -- field.quality never had a
        // chance to catch a regression back to it (the ADR-38 incident).
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nAuthor: (project lead)\n---\n",
        );
        let config = config_for_required(vec![("adr", vec!["Author"])]);

        let (_, findings) = field_quality(&[r], &config);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Error);
    }

    /// `ADR-50`: a `header_shape: none` type has nowhere for a header to be,
    /// so `is_unreadable()` is always true for it by construction -- not a
    /// parse failure. `header_required_fields`'s own record-scoped loop and
    /// `migrate::schema_report` (`BUG-137`) both gate on `has_no_header()`
    /// before checking `is_unreadable()`; `field_quality`'s population loop
    /// did not.
    #[test]
    fn a_header_none_type_is_out_of_scope_not_unreadable_for_field_quality_observed_failing() {
        let r = record("code/src.rs", "code", "fn main() {}\n");
        let config = config_with_types(vec![(
            "code",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                required_fields: vec!["Reviewers".to_string()],
                ..type_config_with_shape(crate::header::HeaderShape::None)
            },
        )]);

        let (exec, findings) = field_quality(&[r], &config);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(
            (population.examined(), population.unreadable(), population.out_of_scope()),
            (0, 0, 1),
            "a header-less type's declared field is never examinable, but it is not a parse failure either: {population:?}"
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// Same gap, `field_pending`'s own population loop.
    #[test]
    fn a_header_none_type_is_out_of_scope_not_unreadable_for_field_pending_observed_failing() {
        let r = record("code/src.rs", "code", "fn main() {}\n");
        let config = config_with_types(vec![(
            "code",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                required_fields: vec!["Reviewers".to_string()],
                ..type_config_with_shape(crate::header::HeaderShape::None)
            },
        )]);

        let (exec, findings) = field_pending(&[r], &config);
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
    fn a_header_none_type_is_out_of_scope_not_unreadable_for_field_untrimmed_value_observed_failing(
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

        let (exec, findings) = field_untrimmed_value(&[r], &config);
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

    /// `MILE-140`: one rule, two fields, two severities.
    fn levelled(rule: &str, levels: &str) -> Config {
        crate::config::parse(&format!(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    required_fields: [Deciders, Reviewers]\nrules:\n  {rule}:\n    level: warn\n    field_levels: {levels}\n"
        ))
        .expect("config loads")
    }

    #[test]
    fn field_quality_reports_each_field_at_its_own_declared_level_observed_failing() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nDeciders:\nReviewers:\n---\n",
        );
        let config = levelled("field.quality", "{Deciders: error, Reviewers: warn}");
        let (_, findings) = field_quality(&[r], &config);
        let by = |f: &str| {
            findings
                .iter()
                .find(|x| x.message.contains(&format!("'{f}'")))
                .unwrap_or_else(|| panic!("no finding for {f}: {findings:?}"))
                .declared_severity
        };
        assert_eq!(by("Deciders"), Some(FindingSeverity::Error));
        assert_eq!(by("Reviewers"), Some(FindingSeverity::Warning));
    }

    #[test]
    fn an_unlisted_field_declares_no_severity_of_its_own_observed_failing() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nDeciders:\nReviewers:\n---\n",
        );
        let config = levelled("field.quality", "{Deciders: error}");
        let (_, findings) = field_quality(&[r], &config);
        let reviewers = findings
            .iter()
            .find(|x| x.message.contains("'Reviewers'"))
            .unwrap();
        assert_eq!(reviewers.declared_severity, None, "the rule level applies");
    }

    #[test]
    fn without_the_option_no_finding_declares_a_severity() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nDeciders:\nReviewers:\n---\n",
        );
        let config = config_for_required(vec![("adr", vec!["Deciders", "Reviewers"])]);
        let (_, findings) = field_quality(&[r], &config);
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().all(|f| f.declared_severity.is_none()));
    }

    #[test]
    fn field_pending_reports_each_field_at_its_own_declared_level_observed_failing() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nDeciders: Pending\nReviewers: Pending\n---\n",
        );
        let config = levelled("field.pending", "{Deciders: error}");
        let (_, findings) = field_pending(&[r], &config);
        let d = findings
            .iter()
            .find(|x| x.message.contains("'Deciders'"))
            .unwrap();
        let rv = findings
            .iter()
            .find(|x| x.message.contains("'Reviewers'"))
            .unwrap();
        assert_eq!(d.declared_severity, Some(FindingSeverity::Error));
        assert_eq!(rv.declared_severity, None);
    }
}
