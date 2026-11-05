use super::*;

/// The references a line actually claims to close: the run immediately
/// following a closing verb, ending at the first token that is neither a
/// reference nor a separator.
///
/// Deliberately narrow. A missed claim is recoverable; a false error on a
/// correct sentence teaches people to stop reading the output.
fn claimed_closed(line: &str) -> Vec<String> {
    const VERBS: [&str; 3] = ["closes", "fixes", "resolves"];
    let mut out = Vec::new();
    let mut tokens = line.split_whitespace().peekable();

    while let Some(token) = tokens.next() {
        let word = token
            .trim_matches(|c: char| !c.is_ascii_alphanumeric())
            .to_lowercase();
        if !VERBS.contains(&word.as_str()) {
            continue;
        }
        let mut expecting = true;
        let mut seen_one = false;
        while let Some(next) = tokens.peek() {
            // A conjunction bridges two references in one claim ("X and Y",
            // "X, and Y") but never starts one -- "Fixes and BUG-40" claims
            // nothing. Bridgeable from either state once a reference has been
            // seen, because an Oxford comma leaves the run open.
            if seen_one && next.eq_ignore_ascii_case("and") {
                tokens.next();
                expecting = true;
                continue;
            }
            if !expecting {
                break;
            }
            let refs = scan_references(next);
            if refs.is_empty() {
                break;
            }
            expecting = next.ends_with(',');
            seen_one = true;
            out.extend(refs);
            tokens.next();
        }
    }
    out
}

/// A file outside the corpus claiming a record is closed, while the record
/// itself says otherwise.
///
/// Written after a changeset in this repository announced that it closed
/// `BUG-36`. It did not -- it fixed a hazard recorded *beside* that bug -- and
/// nothing noticed, because the claim and the record it contradicted live in
/// different files and only one of them was ever read.
///
/// `closed_statuses` is declared, never inferred. A built-in list would make
/// this rule stop applying the moment a repository used a status the list did
/// not know, which is the failure direction this project treats as the worse
/// one.
///
/// Matches on verb and proximity, not meaning, so it can be wrong in both
/// directions: a present-tense sentence discussing someone else's claim will
/// match, and a claim phrased without one of the verbs will not. A waiver
/// (`ADR-11`) is the intended escape for the former -- deliberately a record,
/// so an exception is visible rather than a silent pattern tweak.
pub fn claim_status_agreement(
    claims: &[(String, String)],
    closed_statuses: &[String],
    config: &Config,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_CLAIM_STATUS_AGREEMENT;
    let mut findings = Vec::new();
    let declared_by_type = declared_fields_by_type(config);
    let declared_by_type = &declared_by_type;
    let statuses = statuses_by_type(config);

    // Present tense only: a claim is written in the present, while prose
    // *about* a past claim is written in the past. A narrowing, not a fix --
    // a present-tense sentence discussing a claim still matches.
    let mut occurrences: Vec<(&String, usize, String, String)> = Vec::new();
    for (path, content) in claims {
        for (idx, line) in content.lines().enumerate() {
            for reference in claimed_closed(line) {
                let normalized = crate::values::RecordId::new(&reference).to_string();
                occurrences.push((path, idx + 1, reference, normalized));
            }
        }
    }

    // One candidate per *distinct* claimed reference, deduped the same way the
    // verdict is: two changesets closing one record are one thing to judge.
    // Counting occurrences instead made the number exceed the corpus while
    // saying nothing about records (BUG-90).
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let candidates: Vec<&str> = occurrences
        .iter()
        .map(|(_, _, _, normalized)| normalized.as_str())
        .filter(|normalized| seen.insert(normalized))
        .collect();

    // A claimed reference resolving to no record is eligible and unexamined --
    // the claim was made and the engine could not judge it. It was a bare
    // `continue` that moved no number.
    let population = census(PopulationUnit::Claim, candidates, |normalized| {
        if index.contains_key(*normalized) {
            Outcome::Examined
        } else {
            Outcome::Absent
        }
    });

    // Findings are per occurrence, not per candidate: each place the claim is
    // written is its own thing to correct.
    for (path, line, reference, normalized) in occurrences {
        let Some(target) = index.get(normalized.as_str()) else {
            continue;
        };
        // A lookup miss is unjudged, not a status match (BUG-100).
        let status_field = relation_field_name(
            config,
            &target.record_type,
            crate::config::RelationRole::Status,
        );
        let Ok(status) = declared_cross_record_value(declared_by_type, target, status_field) else {
            continue;
        };
        if is_finished(&statuses, &target.record_type, status, closed_statuses) {
            continue;
        }
        findings.push(Finding {
            rule: RULE_ID.to_string(),
            severity: FindingSeverity::Error,
            file: PathBuf::from(path),
            line: Some(line),
            waived: None,
            declared_severity: None,
            suggested_action: None,
            action_source: ActionSource::new("claim-status-disagrees"),
            message: format!(
                "claims to close {reference}, but {reference} has {status_field} {status}"
            ),
        });
    }

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::*;

    /// BUG-178: `WontFix` is a finished bug, but the hand-copied list lacks it,
    /// so a changeset closing one was an error. Declared terminals fix that.
    #[test]
    fn a_claim_against_a_declared_failure_terminal_is_not_an_error_observed_failing() {
        let yaml = "schema_version: 2\nrecord_types:\n  bug:\n    dir: docs/bugs\n    required_fields: [Status]\n    statuses:\n      values: [Open, Fixed, WontFix]\n      success: [Fixed]\n      failure: [WontFix]\n";
        let config = crate::config::parse(yaml).unwrap();
        let bug = record(
            "docs/bugs/BUG-36-x.md",
            "bug",
            "---\nStatus: WontFix\n---\n",
        );
        let claims = vec![(
            ".changeset/x.md".to_string(),
            "which also closes BUG-36.".to_string(),
        )];
        let records = [bug];
        let index = build_normalized_index(&records);
        let closed = vec!["Fixed".to_string()];
        let (_, findings) = claim_status_agreement(&claims, &closed, &config, &index);
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// Observed failing against the real defect: a changeset in this repository
    /// announced it closed `BUG-36` while `BUG-36` said `Open`, and shipped.
    #[test]
    fn a_claim_to_close_an_open_record_is_an_error_observed_failing() {
        let bug = record("docs/bugs/BUG-36-x.md", "bug", "---\nStatus: Open\n---\n");
        let claims = vec![(
            ".changeset/x.md".to_string(),
            "rendered through a real serializer -- which also closes BUG-36, where a directory\nname escaped its value.\n".to_string(),
        )];
        let closed = vec!["Fixed".to_string()];

        let records = [bug];
        let index = build_normalized_index(&records);
        let config = config_for_required(vec![("bug", vec!["Status"])]);
        let (exec, findings) = claim_status_agreement(&claims, &closed, &config, &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Error);
        assert_eq!(findings[0].line, Some(1));
        assert!(findings[0].message.contains("Status Open"), "{findings:?}");
    }

    /// BUG-90: the count was per claim *file*, so one record claimed closed by
    /// three changesets reported three examined in a population that has one
    /// record. It is counted per distinct claimed record; each place the claim
    /// is written still gets its own finding.
    #[test]
    fn several_claim_files_closing_one_record_count_as_one_claim_observed_failing() {
        let bug = record("docs/bugs/BUG-36-x.md", "bug", "---\nStatus: Open\n---\n");
        let claims = vec![
            (".changeset/a.md".to_string(), "closes BUG-36.".to_string()),
            (
                ".changeset/b.md".to_string(),
                "also closes BUG-36.".to_string(),
            ),
            (
                ".changeset/c.md".to_string(),
                "which closes BUG-36.".to_string(),
            ),
        ];
        let closed = vec!["Fixed".to_string()];

        let records = [bug];
        let index = build_normalized_index(&records);
        let config = config_for_required(vec![("bug", vec!["Status"])]);
        let (exec, findings) = claim_status_agreement(&claims, &closed, &config, &index);
        let population = exec.population.expect("the rule carries a population");
        assert_eq!(
            (population.eligible(), population.examined()),
            (1, 1),
            "one record claimed three times is one claim, never more than the corpus holds"
        );
        assert_eq!(findings.len(), 3, "each claim site is its own finding");
    }

    /// BUG-100: a target record whose `Status` key is written under a
    /// different case is not judged, not judged-and-wrong. Before the fix
    /// this fabricated the sentinel `"(no Status field)"`, which matched no
    /// configured closed status and pushed a blocking `Error` on every claim.
    #[test]
    fn a_claim_against_a_case_differently_written_status_key_is_not_judged_observed_failing() {
        let bug = record("docs/bugs/BUG-36-x.md", "bug", "---\nstatus: Open\n---\n");
        let claims = vec![(".changeset/x.md".to_string(), "closes BUG-36.".to_string())];
        let closed = vec!["Fixed".to_string()];

        let records = [bug];
        let index = build_normalized_index(&records);
        let config = config_for_required(vec![("bug", vec!["Status"])]);
        let (exec, findings) = claim_status_agreement(&claims, &closed, &config, &index);
        assert_eq!(examined(&exec), 1);
        assert!(
            findings.is_empty(),
            "a lookup miss must not be reported as a status mismatch: {findings:?}"
        );
    }

    #[test]
    fn claim_status_agreement_reads_a_custom_status_field_name_observed_failing() {
        let bug = record("docs/bugs/BUG-36-x.md", "bug", "---\nState: Open\n---\n");
        let claims = vec![(
            ".changeset/x.md".to_string(),
            "which also closes BUG-36.".to_string(),
        )];
        let closed = vec!["Fixed".to_string()];

        let records = [bug];
        let index = build_normalized_index(&records);
        let config = config_with_types(vec![(
            "bug",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                relation_fields: Some(crate::config::RelationFields {
                    status: Some("State".to_string()),
                    ..Default::default()
                }),
                ..type_config_pointer(&["State"], None, None, None)
            },
        )]);
        let (exec, findings) = claim_status_agreement(&claims, &closed, &config, &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("State Open"), "{findings:?}");
    }

    /// `BUG-109`/`ADR-60`: a target record's `Status` is only judged when its
    /// own type declares `Status` -- the same rule `header.field-case-mismatch`
    /// already applies. A type that never declared it is not this rule's
    /// business, the same as any other undeclared field.
    #[test]
    fn a_claim_against_a_target_whose_type_never_declares_status_is_not_judged_observed_failing() {
        let bug = record("docs/bugs/BUG-36-x.md", "bug", "---\nStatus: Open\n---\n");
        let claims = vec![(".changeset/x.md".to_string(), "closes BUG-36.".to_string())];
        let closed = vec!["Fixed".to_string()];

        let records = [bug];
        let index = build_normalized_index(&records);
        // "bug" declares no fields at all -- Status included.
        let config = config_for_required(vec![("bug", vec![])]);
        let (exec, findings) = claim_status_agreement(&claims, &closed, &config, &index);
        assert_eq!(examined(&exec), 1);
        assert!(
            findings.is_empty(),
            "a type that never declared Status must not have it judged: {findings:?}"
        );
    }

    /// BUG-45, both shapes, each of which produced a blocking error on a
    /// correct sentence.
    #[test]
    fn a_verb_inside_another_word_is_not_a_claim_observed_failing() {
        assert!(claimed_closed("The path prefixes changed; see RFC-9 for why.").is_empty());
        assert!(claimed_closed("This discloses RFC-9's reasoning.").is_empty());
        assert!(claimed_closed("Suffixes and RFC-9 are unrelated.").is_empty());
    }

    /// `BUG-111`: `parse_record_filename` supports a hyphenated type prefix
    /// (`DOC-ADR-2-x.md`), but the reference recognizer used to split on only
    /// the first hyphen, rejecting `DOC-ADR-2` as not-a-reference everywhere
    /// it's cited.
    #[test]
    fn a_hyphenated_type_prefix_is_a_recognized_reference_observed_failing() {
        assert!(is_record_reference("DOC-ADR-2"));
        assert!(is_record_reference("RFC-9"));
        assert!(
            !is_record_reference("DOC-adr-2"),
            "a lowercase segment is not a prefix"
        );
        assert!(
            !is_record_reference("DOC-ADR-"),
            "nothing after the last hyphen is not a number"
        );
        assert_eq!(
            scan_references("see DOC-ADR-2 for context"),
            vec!["DOC-ADR-2".to_string()]
        );
        assert_eq!(
            extract_references("DOC-ADR-2, RFC-9"),
            vec!["DOC-ADR-2".to_string(), "RFC-9".to_string()]
        );
    }

    /// `BUG-114` loosened `parse_record_filename` to allow a digit-bearing
    /// prefix (e.g. a type declaring `prefix: "V2"`), but the reference
    /// recognizer wasn't updated to match -- a mention of such a record in
    /// prose or a pointer field never resolved.
    #[test]
    fn a_digit_bearing_prefix_reference_is_recognized_observed_failing() {
        assert!(is_record_reference("V2-3"));
        assert_eq!(
            extract_references("V2-3, RFC-9"),
            vec!["V2-3".to_string(), "RFC-9".to_string()]
        );
    }

    /// `is_record_reference` and `parse_record_filename` share their segment
    /// grammar (`new_record::{is_digit_segment, is_prefix_segment}`,
    /// `BUG-133`) rather than each re-implementing it, the exact duplication
    /// `BUG-111`/`BUG-114` drifted apart on. A prefix+number token this
    /// recognizes as a reference must also parse as a filename carrying the
    /// same prefix, by construction -- this pins that down rather than
    /// re-deriving it by hand for each case.
    #[test]
    fn a_recognized_reference_parses_as_the_same_filename_prefix() {
        for token in ["RFC-9", "DOC-ADR-2", "V2-3"] {
            assert!(is_record_reference(token), "{token}");
            let filename = format!("{token}-slug.md");
            let (prefix, num) = crate::new_record::parse_record_filename(&filename)
                .unwrap_or_else(|| panic!("{token}-slug.md must parse"));
            let expected_prefix = token.rsplit_once('-').unwrap().0;
            assert_eq!(prefix, Some(expected_prefix), "{token}");
            let expected_num: u32 = token.rsplit_once('-').unwrap().1.parse().unwrap();
            assert_eq!(num, expected_num, "{token}");
        }
    }

    #[test]
    fn only_the_reference_the_verb_governs_is_claimed_observed_failing() {
        // "Fixes BUG-39, which RFC-9 predicted" claimed RFC-9 too.
        assert_eq!(
            claimed_closed("Fixes BUG-39, which RFC-9 predicted."),
            vec!["BUG-39".to_string()]
        );
        // A comma-separated run after the verb is all claimed.
        // An Oxford comma leaves the run open, so `and` must be bridgeable
        // from either state -- but never directly after the verb.
        assert_eq!(
            claimed_closed("Fixes BUG-39, and BUG-40."),
            vec!["BUG-39".to_string(), "BUG-40".to_string()]
        );
        assert!(claimed_closed("Fixes and BUG-40 are unrelated.").is_empty());
        assert_eq!(
            claimed_closed("Closes BUG-39, BUG-40 and BUG-41 in one change."),
            vec![
                "BUG-39".to_string(),
                "BUG-40".to_string(),
                "BUG-41".to_string()
            ]
        );
        // The ordinary single case still works.
        assert_eq!(
            claimed_closed("which also closes BUG-36, where a directory name"),
            vec!["BUG-36".to_string()]
        );
    }

    #[test]
    fn a_claim_to_close_an_already_closed_record_is_silent() {
        let bug = record("docs/bugs/BUG-36-x.md", "bug", "---\nStatus: Fixed\n---\n");
        let claims = vec![(".changeset/x.md".to_string(), "closes BUG-36\n".to_string())];
        let records = [bug];
        let index = build_normalized_index(&records);
        let config = config_for_required(vec![("bug", vec!["Status"])]);
        let (_, findings) =
            claim_status_agreement(&claims, &["Fixed".to_string()], &config, &index);
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// Mentioning a record is not claiming to close it.
    #[test]
    fn a_reference_without_a_closing_verb_is_not_a_claim() {
        let bug = record("docs/bugs/BUG-36-x.md", "bug", "---\nStatus: Open\n---\n");
        let claims = vec![(
            ".changeset/x.md".to_string(),
            "see BUG-36 for the four hardcoded paths\n".to_string(),
        )];
        let records = [bug];
        let index = build_normalized_index(&records);
        let config = config_for_required(vec![("bug", vec!["Status"])]);
        let (_, findings) =
            claim_status_agreement(&claims, &["Fixed".to_string()], &config, &index);
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// `extract_references` reads the first token of each comma-separated entry,
    /// which is correct for a header value and silently empty on a sentence.
    /// BUG-39, live in this corpus: `MILE-13` declares
    /// `Blocked-on: RFC-9's own Q2`, and neither extractor could see `RFC-9`.
    #[test]
    fn a_possessive_does_not_hide_a_reference_observed_failing() {
        let value = "RFC-9's own Q2";
        assert_eq!(extract_references(value), vec!["RFC-9".to_string()]);
        assert_eq!(scan_references(value), vec!["RFC-9".to_string()]);

        // Narrow deliberately: a suffixed identifier is a different record, not
        // a possessive, and must stay unrecognised.
        assert!(scan_references("RFC-9a is unrelated").is_empty());
        assert!(scan_references("RFC-9s covers nine").is_empty());

        // Ordinary trailing punctuation was already handled; keep it asserted
        // so a narrower possessive rule cannot regress it.
        assert_eq!(
            scan_references("see RFC-9, then stop"),
            vec!["RFC-9".to_string()]
        );
        assert_eq!(scan_references("see RFC-9."), vec!["RFC-9".to_string()]);
    }

    #[test]
    fn scanning_prose_finds_references_that_extract_references_misses() {
        let line = "concatenation -- which also closes BUG-36, where a directory name escaped";
        assert!(extract_references(line).is_empty());
        assert_eq!(scan_references(line), vec!["BUG-36".to_string()]);
    }
}
