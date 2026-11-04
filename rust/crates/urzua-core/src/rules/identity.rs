use super::*;

/// Every record indexed by its normalized identifier (BUG-0002: keyed by
/// numeric value, not the raw string, so a reference's padding never has to
/// match the filename's exactly). Shared by `pointer_resolution`,
/// `narrative_field_stale`, and `graph()` -- previously three near-identical
/// copies of this same loop, one of which (`graph()`'s) never normalized at
/// all (BUG-0011).
pub fn build_normalized_index(records: &[Record]) -> HashMap<crate::values::RecordId, &Record> {
    build_index_reporting_collisions(records).0
}

/// Two records resolving to one identifier is a corpus error the index cannot
/// represent: it keeps one and the other stops existing for every rule that
/// resolves a reference. Reported here rather than papered over, because the
/// tool knows both records are there (BUG-79).
pub fn identity_collision(
    records: &[Record],
    collisions: &[IdentifierCollision<'_>],
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_IDENTITY_COLLISION;

    // A record whose filename yields no identifier is eligible and unexamined:
    // it was handed to the rule, and the rule has no identity to collide.
    let (population, examined_records) = census_records(
        PopulationUnit::Record,
        records.iter().collect(),
        |record| record.path.clone(),
        |record| {
            if record_id(record).is_some() {
                Outcome::Examined
            } else {
                Outcome::OutOfScope
            }
        },
    );

    let mut findings = Vec::new();
    for (id, claimants) in collisions {
        let mut paths: Vec<String> = claimants
            .iter()
            .map(|r| r.path.display().to_string())
            .collect();
        paths.sort();
        for record in claimants.iter() {
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Error,
                file: record.path.clone(),
                line: None,
                waived: None,
                declared_severity: None,
                message: format!(
                    "identifier {id} is claimed by {} records ({}) -- a reference to it resolves to only one of them",
                    claimants.len(),
                    paths.join(", ")
                ),
            });
        }
    }

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

/// Records by normalized identifier.
pub type RecordIndex<'a> = HashMap<crate::values::RecordId, &'a Record>;

/// One identifier and every record claiming it.
pub type IdentifierCollision<'a> = (crate::values::RecordId, Vec<&'a Record>);

/// The index plus the identifiers more than one record claims. Collecting
/// straight into a map kept whichever record sorted last and made the loser
/// invisible to every rule that resolves a reference -- so the verdict on a
/// corpus depended on filename order (BUG-79).
pub fn build_index_reporting_collisions(
    records: &[Record],
) -> (RecordIndex<'_>, Vec<IdentifierCollision<'_>>) {
    use crate::values::RecordId;
    let mut index: HashMap<RecordId, &Record> = HashMap::new();
    let mut claimants: HashMap<RecordId, Vec<&Record>> = HashMap::new();
    for record in records {
        let Some(id) = record_id(record) else {
            continue;
        };
        let key = RecordId::new(&id);
        claimants.entry(key.clone()).or_default().push(record);
        index.entry(key).or_insert(record);
    }
    let mut collisions: Vec<IdentifierCollision<'_>> = claimants
        .into_iter()
        .filter(|(_, rs)| rs.len() > 1)
        .collect();
    collisions.sort_by(|a, b| a.0.cmp(&b.0));
    (index, collisions)
}

/// One `Notice` per collision `build_index_reporting_collisions` found,
/// disclosed regardless of whether `identity.collision` is enabled: the
/// index every reference-resolving rule reads silently keeps one of several
/// colliding records (first-seen-wins), so a rule resolving that identifier
/// may examine the arbitrary winner, not the record a reference actually
/// meant. This is the engine's own computation being honest about an
/// ambiguity it had to resolve (`ADR-55`), not a judgment about the corpus
/// `ADR-53` reserves for a declared rule (`BUG-131`) --
/// `identity.collision`'s own blocking `Finding` stays exactly as opt-in as
/// before. Shared by `check`, `audit` and `graph`, each of which reads this
/// same index.
pub fn identity_collision_notices(collisions: &[IdentifierCollision<'_>]) -> Vec<Notice> {
    collisions
        .iter()
        .map(|(id, claimants)| {
            let mut paths: Vec<String> = claimants
                .iter()
                .map(|r| r.path.display().to_string())
                .collect();
            paths.sort();
            Notice {
                severity: NoticeSeverity::Warning,
                subject: RULE_IDENTITY_COLLISION.to_string(),
                message: format!(
                    "{id} resolves to more than one record ({}) -- any rule resolving a reference to it may examine an arbitrary one of them, not necessarily the one a reference meant",
                    paths.join(", ")
                ),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::*;

    #[test]
    fn a_five_digit_filename_still_resolves_bug_0002_observed_failing() {
        // Before the fix, record_id() rejected any numeric prefix that
        // wasn't exactly 4 digits -- past 9999 records of one type,
        // references would have silently stopped resolving.
        let target = record(
            "docs/rfc/RFC-10000-x.md",
            "rfc",
            "---\nStatus: Draft\n---\n",
        );
        let source = record(
            "docs/specs/SPEC-1-x.md",
            "spec",
            "---\nImplements: RFC-10000\n---\n",
        );
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
        )]);
        let records = [target, source];
        let index = build_normalized_index(&records);
        let (_, findings) = pointer_resolution(&records, &config, &index);
        // Resolution succeeding reports nothing (MILE-80). a_dangling_parent_pointer_is_an_error
        // is the control that keeps this from passing on a rule that never fires.
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_reference_resolves_regardless_of_zero_padding() {
        // ADR-0034 (the filename's own padding) and a hand-typed ADR-34
        // reference must resolve to the same record (BUG-0002).
        let target = record(
            "docs/adr/ADR-0034-x.md",
            "adr",
            "---\nStatus: Accepted\n---\n",
        );
        let source = record(
            "docs/specs/SPEC-1-y.md",
            "spec",
            "---\nImplements: ADR-34\n---\n",
        );
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
        )]);
        let records = [target, source];
        let index = build_normalized_index(&records);
        let (_, findings) = pointer_resolution(&records, &config, &index);
        // Resolution succeeding reports nothing (MILE-80). a_dangling_parent_pointer_is_an_error
        // is the control that keeps this from passing on a rule that never fires.
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_legacy_pre_type_prefix_filename_no_longer_resolves() {
        // ADR-36's amendment (BUG-9): the pre-type-prefix `NNNN-slug.md`
        // shape was never a real external-adopter case, only this repo's own
        // pre-ADR-36 history, and that history no longer exists in the
        // corpus. A reference to a filename in that shape is now correctly
        // dangling, not silently resolved.
        let legacy_style = record("docs/adr/0037-y.md", "adr", "---\nStatus: Accepted\n---\n");
        let source = record(
            "docs/specs/SPEC-1-z.md",
            "spec",
            "---\nImplements: ADR-37\n---\n",
        );
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
        )]);
        let records = [legacy_style, source];
        let index = build_normalized_index(&records);
        let (_, findings) = pointer_resolution(&records, &config, &index);
        assert_eq!(findings.len(), 1, "unexpected findings: {findings:?}");
        assert_eq!(findings[0].severity, FindingSeverity::Error);
    }

    #[test]
    fn a_configured_type_prefix_resolves_independently_of_the_type_name() {
        // A type's filename/ID prefix can be configured shorter than its own
        // name (e.g. `milestone` type configured with `prefix = "MILE"`) --
        // record_id must key off the configured prefix, not a derived
        // upper-cased type name, so `Implements: MILE-1` resolves.
        let milestone = record_with_prefix(
            "docs/milestones/MILE-1-x.md",
            "milestone",
            "MILE",
            "---\nStatus: Planned\n---\n",
        );
        let source = record(
            "docs/adr/ADR-1-y.md",
            "adr",
            "---\nImplements: MILE-1\n---\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
        )]);
        let records = [milestone, source];
        let index = build_normalized_index(&records);
        let (_, findings) = pointer_resolution(&records, &config, &index);
        assert!(findings.is_empty(), "{findings:?}");
    }

    fn declared(path: &str, pattern: &str) -> Record {
        let m = crate::document_model::RecordModel {
            identity: Some(
                crate::document_model::CompiledIdentity::compile(
                    &crate::document_model::IdentityDecl {
                        from: crate::document_model::IdentitySource::Filename,
                        pattern: pattern.to_string(),
                    },
                )
                .unwrap(),
            ),
            fields: None,
        };
        Record::parse_with_model(
            std::path::PathBuf::from(path),
            "adr".to_string(),
            "# 1\n",
            crate::header::HeaderShape::default(),
            "ADR".to_string(),
            &m,
        )
    }

    const DECLARED: &str = r"^(?P<prefix>ADR)-(?P<number>\d+)-.+$";

    /// MILE-98: a declared pattern keeps the file's padding in the id, and
    /// the id resolves like any padded reference.
    #[test]
    fn a_declared_pattern_gives_a_padded_filename_an_identity() {
        let r = declared("docs/adr/ADR-0173-x.md", DECLARED);
        assert_eq!(record_id(&r).as_deref(), Some("ADR-0173"));
        let index = build_normalized_index(std::slice::from_ref(&r));
        assert!(index.contains_key(&crate::values::RecordId::new("ADR-173")));
    }

    /// MILE-126 / RFC-46 (rejected): a bare numeric filename has no identity,
    /// declared pattern or not.
    #[test]
    fn a_bare_numeric_filename_has_no_identity_even_with_a_declaration() {
        let r = declared("docs/adr/0173-x.md", DECLARED);
        assert_eq!(record_id(&r), None);
        let r = record("docs/adr/0173-x.md", "adr", "# 173\n");
        assert_eq!(record_id(&r), None);
    }

    #[test]
    fn a_declared_pattern_replaces_the_type_prefix_convention() {
        let r = declared(
            "docs/adr/ADR-9-x.md",
            r"^(?P<prefix>ADR)-(?P<number>\d+)-zzz$",
        );
        assert_eq!(
            record_id(&r),
            None,
            "ADR-9-x.md does not match the declared pattern"
        );
    }
}
