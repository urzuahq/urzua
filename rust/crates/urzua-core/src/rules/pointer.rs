use super::*;

/// Rule 2 (config-driven per type since MILE-0090/ADR-0044): a
/// `pointer_fields`/`narrative_fields` entry resolves to a real record when
/// it names one. The target's status is surfaced in the message, never
/// judged -- whether a `Draft` target is acceptable is a policy decision
/// (RFC-0012), not this checker's call. Both kinds resolve identically here;
/// existence-checking isn't a `FieldKindSpec` capability; only the clean-
/// format check and the staleness check differ per kind.
/// Split out of `pointer.resolution` (MILE-80), which emitted a finding for
/// *every* reference that resolved -- 172 of 213 on this repo's own corpus, all
/// saying a reference worked. One rule id cannot carry two severities, so the
/// dangling-reference error and this policy could never be levelled apart.
///
/// Fires only on the statuses a repository declares unacceptable. With none
/// declared it examines nothing and reports nothing, rather than falling back
/// to a built-in list: a table keyed on record type names makes the rule
/// silently stop applying to every corpus that names its types differently.
pub fn pointer_target_status(
    records: &[Record],
    config: &Config,
    not_in: &[String],
    fields: Option<&[String]>,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    let pointer_fields_by_type = pointer_fields_by_type(config);
    let pointer_fields_by_type = &pointer_fields_by_type;
    let narrative_fields_by_type = narrative_fields_by_type(config);
    let narrative_fields_by_type = &narrative_fields_by_type;
    let declared_by_type = declared_fields_by_type(config);
    let declared_by_type = &declared_by_type;
    const RULE_ID: &str = RULE_POINTER_TARGET_STATUS;
    let mut findings = Vec::new();

    // Declared slots: every pointer or narrative field the record's type
    // declares, whether or not the record wrote it. A slot left unwritten is
    // eligible and unexamined -- handed to the rule and not judged.
    let slots =
        pointer_and_narrative_slots(records, pointer_fields_by_type, narrative_fields_by_type);
    // A field the repository's `fields` option leaves out is not part of this
    // rule's population at all. It is not `OutOfScope`: that outcome means
    // "fix the configuration" and trips `config.scope-matches-nothing`, but
    // this exclusion is the configuration working as written. Not `Absent`
    // either, which says the record did not write a field it was handed.
    let slots: Vec<_> = slots
        .into_iter()
        .filter(|(_, field)| fields.is_none_or(|f| f.contains(field)))
        .collect();

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
                    // A lookup miss is unjudged, not a status match (BUG-100).
                    let Ok(status) =
                        declared_cross_record_value(declared_by_type, target, target_status_field)
                    else {
                        return;
                    };
                    if not_in.iter().any(|s| s == status) {
                        findings.push(Finding {
                            rule: RULE_ID.to_string(),
                            severity: FindingSeverity::Warning,
                            file: record.path.clone(),
                            line: None,
                            waived: None,
                            declared_severity: None,
                            suggested_action: None,
                            action_source: ActionSource::new("pointer-target-status").field(field_name.as_str()),
                            message: format!(
                                "{field_name}: {reference} resolves, but its {target_status_field} is {status}"
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

pub fn pointer_resolution(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
    untracked: &UntrackedRecords,
) -> (RuleExecution, Vec<Finding>) {
    let pointer_fields_by_type = pointer_fields_by_type(config);
    let pointer_fields_by_type = &pointer_fields_by_type;
    let narrative_fields_by_type = narrative_fields_by_type(config);
    let narrative_fields_by_type = &narrative_fields_by_type;
    const RULE_ID: &str = RULE_POINTER_RESOLUTION;
    let mut findings = Vec::new();

    // Declared slots: every pointer or narrative field the record's type
    // declares, whether or not the record wrote it. A slot left unwritten is
    // eligible and unexamined -- handed to the rule and not judged.
    let slots =
        pointer_and_narrative_slots(records, pointer_fields_by_type, narrative_fields_by_type);

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
                if index.contains_key(&crate::values::RecordId::new(&reference)) {
                    continue;
                }
                let (variant, message) = unresolved_reference(
                    field_name,
                    &reference,
                    untracked,
                    ("pointer-unresolved", "pointer-untracked"),
                    "",
                );
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new(variant).field(field_name.as_str()),
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

/// BUG-0007: a pointer field's value silently tolerating trailing prose after
/// the reference token (`extract_references` only reads the leading token,
/// discarding the rest) turned out to be a corpus-wide habit, not two
/// isolated mistakes -- `Derives-from: RFC-1 (Accepted)`'s status annotation
/// alone appears in 33 files. That annotation is redundant with
/// `pointer.resolution`'s own live status report and a staleness risk
/// (nothing re-verifies it once written), and `Parent`'s freeform prose in
/// `SPEC-2`/`SPEC-4` is worse -- neither was ever a deliberate schema
/// decision, `extract_references` just never rejected it.
///
/// This rule is a check on the raw field value, independent of
/// `pointer_resolution`: a field is clean only if every comma-separated
/// entry, once trimmed, is *exactly* a reference token (or the `—` no-value
/// placeholder) -- nothing before or after it. Applies only to fields whose
/// `FieldKindSpec` declares `enforce_clean_format` (i.e. `pointer_fields`,
/// config-driven since MILE-0090) -- `narrative_fields` are deliberately
/// exempt, the same exemption `Blocked-on` always had.
pub fn header_pointer_field_clean(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_HEADER_POINTER_FIELD_CLEAN;
    let mut findings = Vec::new();

    let clean_fields_by_type = fields_with_capability(config, |k| k.enforce_clean_format);

    // Declared slots: a field the type says carries clean references. A slot
    // the record did not write is eligible and unexamined -- handed to the rule
    // and not judged -- rather than outside its population.
    let slots = field_slots(records, &clean_fields_by_type);

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |(record, _)| *record,
        |(record, field_name)| {
            // Unreadable, not absent: the header did not parse, so this slot
            // has no value to classify (BUG-125's shape, `field_quality`'s
            // same guard).
            if record.header.is_unreadable() {
                return Outcome::Unreadable;
            }
            let Some(value) = record.header.get(field_name.as_str()) else {
                return Outcome::Absent;
            };
            if crate::values::is_no_value_sentinel(value) {
                return Outcome::Examined;
            }

            for entry in value.split(',') {
                let entry = entry.trim();
                let is_clean_reference =
                    is_record_reference(entry) || crate::values::is_no_value_sentinel(entry);
                if !is_clean_reference {
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Warning,
                        file: record.path.clone(),
                        line: None,
                        waived: None,
                        declared_severity: None,
                        suggested_action: None,
                        action_source: ActionSource::new("pointer-field-not-clean").field(field_name.as_str()),
                        message: format!(
                            "{field_name} entry {entry:?} isn't a clean reference -- pointer fields should hold only comma-separated reference IDs (e.g. \"RFC-1\", not \"RFC-1 (Accepted)\"); pointer.resolution already reports a resolved target's live Status"
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
    fn target_status_judges_only_the_statuses_a_repository_declared() {
        // Replaces a_resolving_pointer_surfaces_target_status_without_judging_it,
        // whose name was the defect: surfacing a status without judging it
        // produced 172 of 213 findings on this repo's own corpus.
        let target = record("docs/rfc/RFC-1-x.md", "rfc", "---\nStatus: Draft\n---\n");
        let source = record(
            "docs/specs/SPEC-1-x.md",
            "spec",
            "---\nImplements: RFC-0001\n---\n",
        );
        let config = config_with_types(vec![
            (
                "spec",
                type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
            ),
            ("rfc", type_config_pointer(&["Status"], None, None, None)),
        ]);
        let records = [target, source];
        let index = build_normalized_index(&records);

        let (exec, findings) =
            pointer_target_status(&records, &config, &["Draft".to_string()], None, &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Draft"));

        // Nothing declared unacceptable means nothing reported -- never a
        // built-in fallback list, which is how a rule silently stops applying.
        let (_, none_declared) = pointer_target_status(&records, &config, &[], None, &index);
        assert!(none_declared.is_empty(), "{none_declared:?}");

        // A status outside the declared set is not this rule's business.
        let (_, other) =
            pointer_target_status(&records, &config, &["Superseded".to_string()], None, &index);
        assert!(other.is_empty(), "{other:?}");
    }

    #[test]
    fn a_type_declaring_a_custom_status_field_name_is_read_by_that_name_observed_failing() {
        let target = record("docs/rfc/RFC-1-x.md", "rfc", "---\nState: Draft\n---\n");
        let source = record(
            "docs/specs/SPEC-1-x.md",
            "spec",
            "---\nImplements: RFC-0001\n---\n",
        );
        let config = config_with_types(vec![
            (
                "spec",
                type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
            ),
            (
                "rfc",
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
        let records = [target, source];
        let index = build_normalized_index(&records);

        let (exec, findings) =
            pointer_target_status(&records, &config, &["Draft".to_string()], None, &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Draft"));
    }

    #[test]
    fn a_reference_to_an_untracked_file_on_disk_is_its_own_variant_observed_failing() {
        // `BUG-13`: a record drafted beside the one citing it, not yet `git add`ed,
        // read exactly like a typo. The CLI supplies the on-disk fact; core stays pure.
        let source = record(
            "docs/specs/SPEC-1-x.md",
            "spec",
            "---\nImplements: RFC-9, RFC-8\n---\n",
        );
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
        )]);
        let records = [source];
        let index = build_normalized_index(&records);
        let untracked: UntrackedRecords = [(
            crate::values::RecordId::new("RFC-9"),
            PathBuf::from("docs/rfc/RFC-9-draft.md"),
        )]
        .into();
        let (_, findings) = pointer_resolution(&records, &config, &index, &untracked);
        assert_eq!(findings.len(), 2, "{findings:?}");
        let untracked_finding = findings
            .iter()
            .find(|f| f.message.contains("RFC-9"))
            .unwrap();
        assert_eq!(untracked_finding.action_source.variant, "pointer-untracked");
        assert!(
            untracked_finding
                .message
                .contains("docs/rfc/RFC-9-draft.md")
                && untracked_finding.message.contains("not tracked by git"),
            "{}",
            untracked_finding.message
        );
        let missing = findings
            .iter()
            .find(|f| f.message.contains("RFC-8"))
            .unwrap();
        assert_eq!(missing.action_source.variant, "pointer-unresolved");
        assert!(missing
            .message
            .ends_with("does not resolve to any discovered record"));
    }

    #[test]
    fn a_non_resolving_pointer_is_an_error() {
        let source = record(
            "docs/specs/SPEC-1-x.md",
            "spec",
            "---\nImplements: RFC-9999\n---\n",
        );
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
        )]);
        let records = [source];
        let index = build_normalized_index(&records);
        let (_, findings) = pointer_resolution(&records, &config, &index, &Default::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Error);
    }

    /// BUG-8: `pointer.resolution` and `header.pointer-field-clean` read only
    /// a hardcoded field list, so a pointer field a repository declared under
    /// another name was never resolved or checked for prose. Both read the
    /// declared `pointer_fields`, so `Feeds-into` is judged like `Implements`.
    #[test]
    fn a_declared_pointer_field_with_a_custom_name_is_resolved_and_checked_clean_observed_failing()
    {
        let dangling = record(
            "docs/specs/SPEC-1-x.md",
            "spec",
            "---\nFeeds-into: SPEC-9999\n---\n",
        );
        let prose = record(
            "docs/specs/SPEC-2-x.md",
            "spec",
            "---\nFeeds-into: SPEC-1 (the first one).\n---\n",
        );
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Feeds-into"]), Some(&[])),
        )]);
        let records = [dangling, prose];
        let index = build_normalized_index(&records);
        let (_, resolution) = pointer_resolution(&records, &config, &index, &Default::default());
        assert_eq!(resolution.len(), 1, "{resolution:?}");
        assert!(resolution[0].message.contains("Feeds-into: SPEC-9999"));
        let (_, clean) = header_pointer_field_clean(&records, &config);
        assert_eq!(clean.len(), 1, "{clean:?}");
        assert!(
            clean[0].file.to_string_lossy().contains("SPEC-2"),
            "{clean:?}"
        );
    }

    #[test]
    fn a_parent_pointer_resolves_the_same_as_implements() {
        // Every spec's `Parent: SPEC-N` pointer was completely unchecked
        // before Parent was added to this rule's scanned fields -- a typo'd
        // or dangling Parent would never have been caught.
        let parent = record(
            "docs/specs/SPEC-1-cli.md",
            "spec",
            "---\nStatus: Draft\n---\n",
        );
        let child = record(
            "docs/specs/SPEC-2-urzua-check.md",
            "spec",
            "---\nParent: SPEC-1 (v0 CLI). Extra trailing prose that isn't a reference.\n---\n",
        );
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Parent"]), Some(&[])),
        )]);
        let records = [parent, child];
        let index = build_normalized_index(&records);
        let (_, findings) = pointer_resolution(&records, &config, &index, &Default::default());
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_dangling_parent_pointer_is_an_error() {
        let child = record(
            "docs/specs/SPEC-2-x.md",
            "spec",
            "---\nParent: SPEC-9999\n---\n",
        );
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Parent"]), Some(&[])),
        )]);
        let records = [child];
        let index = build_normalized_index(&records);
        let (_, findings) = pointer_resolution(&records, &config, &index, &Default::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Error);
        assert!(findings[0].message.contains("Parent: SPEC-9999"));
    }

    #[test]
    fn a_derives_from_status_annotation_is_flagged_observed_failing() {
        // The exact live pattern found across 33 files: a hand-typed status
        // snapshot baked into the field value itself, redundant with
        // pointer.resolution's own live report and never re-verified.
        let r = record(
            "docs/adr/ADR-38-x.md",
            "adr",
            "---\nStatus: Accepted\nDerives-from: RFC-1 (Accepted)\n---\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], None, Some(&["Derives-from"]), None),
        )]);
        let (exec, findings) = header_pointer_field_clean(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("RFC-1 (Accepted)"));
        assert_eq!(findings[0].severity, FindingSeverity::Warning);
    }

    /// `BUG-125`'s shape, found by CodeRabbit in this same rule after the
    /// `field.quality` fix: an unparsed header must disclose `unreadable`,
    /// not fold silently into `absent`.
    #[test]
    fn header_pointer_field_clean_discloses_unreadable_on_its_own_observed_failing() {
        let r = record("docs/adr/ADR-1-x.md", "adr", "this is not a header at all");
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], None, Some(&["Derives-from"]), None),
        )]);
        let (exec, findings) = header_pointer_field_clean(&[r], &config);
        assert_eq!(examined(&exec), 0, "{exec:?}");
        assert_eq!(unreadable(&exec), 1, "{exec:?}");
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_parent_field_with_freeform_trailing_prose_is_flagged() {
        let r = record(
            "docs/specs/SPEC-2-x.md",
            "spec",
            "---\nStatus: Draft\nParent: SPEC-1 (v0 CLI). Cross-cutting rules stated there.\n---\n",
        );
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Parent"]), None),
        )]);
        let (_, findings) = header_pointer_field_clean(&[r], &config);
        assert_eq!(findings.len(), 1);
    }

    /// BUG-72: an empty prefix passed the uppercase test vacuously, so a bare
    /// hyphen-number token was read as a reference. `header.pointer-field-clean`
    /// called a pointer field holding `-1` clean and `pointer.resolution`
    /// reported it as a reference that resolves to nothing.
    #[test]
    fn a_bare_hyphen_number_token_is_not_a_reference_observed_failing() {
        use crate::rules::shared::{extract_references, scan_references};
        assert!(extract_references("-1").is_empty());
        assert!(scan_references("see -1 and -22 for why").is_empty());

        let source = record(
            "docs/specs/SPEC-1-x.md",
            "spec",
            "---\nImplements: -1\n---\n",
        );
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
        )]);
        let (_, clean) = header_pointer_field_clean(std::slice::from_ref(&source), &config);
        assert_eq!(clean.len(), 1, "`-1` is not a clean reference: {clean:?}");

        let records = [source];
        let index = build_normalized_index(&records);
        let (_, resolution) = pointer_resolution(&records, &config, &index, &Default::default());
        assert!(
            resolution.is_empty(),
            "`-1` was never a reference to resolve: {resolution:?}"
        );
    }

    #[test]
    fn a_clean_bare_reference_is_not_flagged() {
        let r = record(
            "docs/adr/ADR-10-x.md",
            "adr",
            "---\nStatus: Accepted\nDerives-from: RFC-1\nParent: —\n---\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], None, Some(&["Derives-from", "Parent"]), None),
        )]);
        let (exec, findings) = header_pointer_field_clean(&[r], &config);
        // One record; two declared clean-format slots, both written.
        let population = exec.population.expect("the rule carries a population");
        assert_eq!(population.unit(), PopulationUnit::Field);
        assert_eq!(population.eligible(), 2);
        assert_eq!(population.examined(), 2);
        assert!(findings.is_empty());
    }

    #[test]
    fn multiple_clean_references_are_not_flagged() {
        let r = record(
            "docs/adr/ADR-8-x.md",
            "adr",
            "---\nStatus: Accepted\nDerives-from: RFC-8, ADR-15, ADR-19\n---\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], None, Some(&["Derives-from"]), None),
        )]);
        let (_, findings) = header_pointer_field_clean(&[r], &config);
        assert!(findings.is_empty());
    }

    #[test]
    fn an_empty_entry_between_commas_is_flagged_not_silently_skipped() {
        // A trailing/double comma (`RFC-1,,ADR-2`) produces an empty
        // split segment -- that must be flagged, not treated as a second
        // "no value" placeholder alongside the real one, "—".
        let r = record(
            "docs/adr/ADR-11-x.md",
            "adr",
            "---\nStatus: Accepted\nDerives-from: RFC-1,,ADR-2\n---\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], None, Some(&["Derives-from"]), None),
        )]);
        let (_, findings) = header_pointer_field_clean(&[r], &config);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn a_bare_placeholder_field_is_still_not_flagged() {
        let r = record(
            "docs/adr/ADR-12-x.md",
            "adr",
            "---\nStatus: Accepted\nParent: —\n---\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], None, Some(&["Parent"]), None),
        )]);
        let (_, findings) = header_pointer_field_clean(&[r], &config);
        assert!(findings.is_empty());
    }

    /// The doc comment above this rule says explicitly: "every comma-separated
    /// entry, once trimmed, is *exactly* a reference token (or the `—`
    /// no-value placeholder)" -- a per-entry claim, not only a whole-value
    /// one. The whole-value sentinel check alone missed a real reference
    /// mixed with the placeholder in the same field.
    #[test]
    fn a_placeholder_entry_mixed_with_a_real_reference_is_not_flagged_observed_failing() {
        let r = record(
            "docs/adr/ADR-12-x.md",
            "adr",
            "---\nStatus: Accepted\nDerives-from: RFC-1, —\n---\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], None, Some(&["Derives-from"]), None),
        )]);
        let (_, findings) = header_pointer_field_clean(&[r], &config);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn narrative_fields_are_excluded_even_with_trailing_prose() {
        // Blocked-on deliberately mixes free text with an optional embedded
        // reference (SPEC-6) -- must never be flagged by this rule, since
        // it's declared narrative, not pointer (MILE-90).
        let r = record(
            "docs/milestones/MILE-38-x.md",
            "milestone",
            "---\nStatus: Planned\nBlocked-on: MILE-38 (staleness detection) -- sequenced first.\n---\n",
        );
        let config = config_with_types(vec![(
            "milestone",
            type_config_pointer(&[], None, None, Some(&["Blocked-on"])),
        )]);
        let (exec, findings) = header_pointer_field_clean(&[r], &config);
        assert_eq!(examined(&exec), 0);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_header_none_type_is_out_of_scope_not_unreadable_for_header_pointer_field_clean_observed_failing(
    ) {
        let r = record("code/src.rs", "code", "fn main() {}\n");
        let config = config_with_types(vec![(
            "code",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                known_fields: Some(vec!["Derives-from".to_string()]),
                pointer_fields: Some(vec!["Derives-from".to_string()]),
                narrative_fields: Some(vec![]),
                ..type_config_with_shape(crate::header::HeaderShape::None)
            },
        )]);

        let (exec, findings) = header_pointer_field_clean(&[r], &config);
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
}
