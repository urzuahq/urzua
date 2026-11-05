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
///
/// The second reading is exact (`ADR-85`). A repository that declares `closes`
/// has each claim file list, in a frontmatter key, the records it closes, and
/// the rule checks that data: each id resolves, and is at a status its type
/// declares terminal. Prose is never read for that claim; the closing-verb
/// reading above stays as it was, over the body of the file.
pub fn claim_status_agreement(
    claims: &[(String, String)],
    opts: &ClaimOpts,
    config: &Config,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_CLAIM_STATUS_AGREEMENT;
    let closed_statuses = opts.closed_statuses.as_slice();
    let mut findings = Vec::new();
    let declared_by_type = declared_fields_by_type(config);
    let declared_by_type = &declared_by_type;
    let statuses = statuses_by_type(config);

    // Present tense only: a claim is written in the present, while prose
    // *about* a past claim is written in the past. A narrowing, not a fix --
    // a present-tense sentence discussing a claim still matches.
    let mut occurrences: Vec<(&String, usize, String, String)> = Vec::new();
    for (path, content) in claims {
        for (idx, line) in verb_claim_lines(content, opts) {
            for reference in claimed_closed(line) {
                let normalized = crate::values::RecordId::new(&reference).to_string();
                occurrences.push((path, idx + 1, reference, normalized));
            }
        }
    }

    // What each file declares in the structured field, when one is declared.
    let mut listed: Vec<(&String, usize, String, String)> = Vec::new();
    let mut non_fragments: Vec<&String> = Vec::new();
    if let Some(key) = opts.closes.as_deref() {
        for (path, content) in claims {
            match read_claim_field(content, key) {
                ClaimField::NoFrontmatter => non_fragments.push(path),
                ClaimField::Missing => {
                    if opts.strict {
                        findings.push(field_finding(
                            path,
                            1,
                            "claim-field-missing",
                            format!(
                                "has no `{key}` field; declare the records this change closes as `{key}: [BUG-1, BUG-2]`, or `{key}: []` if it closes none"
                            ),
                        ));
                    }
                }
                ClaimField::Malformed { line, reason } => {
                    findings.push(field_finding(path, line, "claim-field-malformed", reason))
                }
                ClaimField::Declared { line, ids } => {
                    for id in ids {
                        let normalized = crate::values::RecordId::new(&id).to_string();
                        listed.push((path, line, id, normalized));
                    }
                }
            }
        }
    }

    // One candidate per *distinct* claimed reference, deduped the same way the
    // verdict is: two changesets closing one record are one thing to judge.
    // Counting occurrences instead made the number exceed the corpus while
    // saying nothing about records (BUG-90). A listed id is its own candidate,
    // because it is judged by a different reading than the same id in prose.
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut seen_listed: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut candidates: Vec<Candidate> = occurrences
        .iter()
        .map(|(_, _, _, normalized)| normalized.as_str())
        .filter(|normalized| seen.insert(normalized))
        .map(Candidate::Verb)
        .collect();
    candidates.extend(
        listed
            .iter()
            .map(|(_, _, _, normalized)| normalized.as_str())
            .filter(|normalized| seen_listed.insert(normalized))
            .map(|_| Candidate::Listed),
    );
    // A file with no frontmatter is not a fragment, so the field does not
    // apply to it: eligible, out of scope, and disclosed by `check`.
    candidates.extend(non_fragments.iter().map(|_| Candidate::NotAFragment));

    // A claimed reference resolving to no record is eligible and unexamined --
    // the claim was made and the engine could not judge it. It was a bare
    // `continue` that moved no number. A listed id that resolves to nothing is
    // judged (it is a finding), so it counts as examined.
    let population = census(
        PopulationUnit::Claim,
        candidates,
        |candidate| match candidate {
            Candidate::Verb(normalized) if !index.contains_key(*normalized) => Outcome::Absent,
            Candidate::NotAFragment => Outcome::OutOfScope,
            Candidate::Listed | Candidate::Verb(_) => Outcome::Examined,
        },
    );

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

    let key = opts.closes.as_deref().unwrap_or_default();
    for (path, line, id, normalized) in listed {
        let Some(target) = index.get(normalized.as_str()) else {
            findings.push(field_finding(
                path,
                line,
                "claim-id-unresolved",
                format!("lists {id} in `{key}`, which resolves to no record"),
            ));
            continue;
        };
        let status_field = relation_field_name(
            config,
            &target.record_type,
            crate::config::RelationRole::Status,
        );
        // Closed is the type's declared terminals alone: the rule's fallback
        // list is for the verb reading, and a type that declares none has no
        // declared close to compare a listed record against.
        let status = declared_cross_record_value(declared_by_type, target, status_field);
        if status.is_ok_and(|s| is_finished(&statuses, &target.record_type, s, &[])) {
            continue;
        }
        let state = match status {
            Ok(s) => format!("{status_field} {s}"),
            Err(_) => format!("no readable {status_field}"),
        };
        let mut finding = field_finding(
            path,
            line,
            "claim-closes-open",
            format!("lists {id} in `{key}` as closed, but {id} has {state}"),
        );
        finding.action_source = ActionSource::new("claim-closes-open")
            .file(target.path.clone())
            .field(status_field);
        findings.push(finding);
    }

    // The structured findings and the verb ones arrive in file and line order
    // whichever reading found them.
    if opts.closes.is_some() {
        findings.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
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

/// What the census counts: a distinct reference found by the closing-verb
/// reading, a distinct id listed in the structured field, or a file the field
/// does not apply to.
enum Candidate<'a> {
    Verb(&'a str),
    Listed,
    NotAFragment,
}

fn field_finding(path: &str, line: usize, variant: &'static str, message: String) -> Finding {
    Finding {
        rule: RULE_CLAIM_STATUS_AGREEMENT.to_string(),
        severity: FindingSeverity::Error,
        file: PathBuf::from(path),
        line: Some(line),
        waived: None,
        declared_severity: None,
        suggested_action: None,
        action_source: ActionSource::new(variant),
        // The finding names itself the way the decision does (`claim.id-unresolved`),
        // so it can be searched for; the variant stays the unit `urzua rules` lists.
        message: format!("{}: {message}", variant.replacen('-', ".", 1)),
    }
}

/// The lines the closing-verb reading looks at: every line of the file, less
/// the frontmatter block once the structured field is declared. A frontmatter
/// line is data, not prose, and `closes: [BUG-1]` would otherwise be read a
/// second time as the verb "closes".
fn verb_claim_lines<'a>(
    content: &'a str,
    opts: &ClaimOpts,
) -> impl Iterator<Item = (usize, &'a str)> {
    let skip = opts
        .closes
        .as_ref()
        .and_then(|_| frontmatter_lines(content))
        .unwrap_or(0..0);
    content
        .lines()
        .enumerate()
        .filter(move |(idx, _)| !skip.contains(idx))
}

/// What `check` tells a reader about the claim files the rule could not
/// judge by their field, and about the reading that is on its way out
/// (`ADR-85`). Empty unless the field is declared.
pub fn claim_field_notices(claims: &[(String, String)], opts: &ClaimOpts) -> Vec<Notice> {
    let Some(key) = opts.closes.as_deref() else {
        return Vec::new();
    };
    let mut notices = Vec::new();
    let not_fragments: Vec<&str> = claims
        .iter()
        .filter(|(_, text)| read_claim_field(text, key) == ClaimField::NoFrontmatter)
        .map(|(path, _)| path.as_str())
        .collect();
    if !not_fragments.is_empty() {
        notices.push(Notice {
            severity: NoticeSeverity::Info,
            subject: RULE_CLAIM_STATUS_AGREEMENT.to_string(),
            message: format!(
                "{} file(s) under claim_paths have no frontmatter block, so they are not fragments and `{key}` is not read from them (out of scope): {}",
                not_fragments.len(),
                not_fragments.join(", ")
            ),
        });
    }
    let verb_claims: usize = claims
        .iter()
        .flat_map(|(_, text)| verb_claim_lines(text, opts))
        .map(|(_, line)| claimed_closed(line).len())
        .sum();
    notices.push(Notice {
        severity: NoticeSeverity::Info,
        subject: RULE_CLAIM_STATUS_AGREEMENT.to_string(),
        message: format!(
            "the closing-verb reading found {verb_claims} closing-verb claim(s) in prose; with `{key}` declared it is a deprecated convenience, and no claim is read from prose by `{key}`"
        ),
    });
    notices
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::*;

    fn verb_only(closed: &[String]) -> ClaimOpts {
        ClaimOpts {
            closed_statuses: closed.to_vec(),
            ..Default::default()
        }
    }

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
        let (_, findings) = claim_status_agreement(&claims, &verb_only(&closed), &config, &index);
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
        let (exec, findings) =
            claim_status_agreement(&claims, &verb_only(&closed), &config, &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Error);
        assert_eq!(findings[0].line, Some(1));
        assert!(findings[0].message.contains("Status Open"), "{findings:?}");
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
        let (exec, findings) =
            claim_status_agreement(&claims, &verb_only(&closed), &config, &index);
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
        let (exec, findings) =
            claim_status_agreement(&claims, &verb_only(&closed), &config, &index);
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
        let (exec, findings) =
            claim_status_agreement(&claims, &verb_only(&closed), &config, &index);
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
            claim_status_agreement(&claims, &verb_only(&["Fixed".to_string()]), &config, &index);
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
            claim_status_agreement(&claims, &verb_only(&["Fixed".to_string()]), &config, &index);
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

    // ---- ADR-85: the structured `closes` field -------------------------------

    const WORLD: &str = "schema_version: 2\nrecord_types:\n  bug:\n    dir: docs/bugs\n    required_fields: [Status]\n    statuses:\n      values: [Open, Fixed, WontFix]\n      success: [Fixed]\n      failure: [WontFix]\n  note:\n    dir: docs/notes\n    required_fields: [Status]\n";

    fn world() -> (Config, Vec<Record>) {
        let config = crate::config::parse(WORLD).unwrap();
        let records = vec![
            record("docs/bugs/BUG-1-open.md", "bug", "---\nStatus: Open\n---\n"),
            record(
                "docs/bugs/BUG-2-fixed.md",
                "bug",
                "---\nStatus: Fixed\n---\n",
            ),
            record(
                "docs/bugs/BUG-3-wontfix.md",
                "bug",
                "---\nStatus: WontFix\n---\n",
            ),
            record(
                "docs/notes/NOTE-1-x.md",
                "note",
                "---\nStatus: Fixed\n---\n",
            ),
        ];
        (config, records)
    }

    fn declared(strict: bool) -> ClaimOpts {
        ClaimOpts {
            closed_statuses: vec!["Fixed".to_string()],
            closes: Some("closes".to_string()),
            strict,
            ..Default::default()
        }
    }

    fn run(files: &[(&str, &str)], opts: &ClaimOpts) -> (RuleExecution, Vec<Finding>) {
        let (config, records) = world();
        let index = build_normalized_index(&records);
        let claims: Vec<(String, String)> = files
            .iter()
            .map(|(p, t)| (p.to_string(), t.to_string()))
            .collect();
        claim_status_agreement(&claims, opts, &config, &index)
    }

    fn variants(findings: &[Finding]) -> Vec<&'static str> {
        findings.iter().map(|f| f.action_source.variant).collect()
    }

    fn fragment(closes: &str) -> String {
        format!("---\ndefault: patch\ncloses: {closes}\n---\n\nA change.\n")
    }

    /// `BUG-66`'s regression test: a fragment whose `closes` lists a record
    /// that is still open is an error, with no closing verb anywhere in it.
    #[test]
    fn a_fragment_listing_a_still_open_record_is_an_error_observed_failing() {
        let (exec, findings) = run(
            &[(".changeset/a.md", &fragment("[BUG-1]"))],
            &declared(false),
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, FindingSeverity::Error);
        assert_eq!(findings[0].line, Some(3));
        assert_eq!(findings[0].file, PathBuf::from(".changeset/a.md"));
        assert!(findings[0].message.contains("BUG-1"), "{findings:?}");
        assert!(findings[0].message.contains("Status Open"), "{findings:?}");
        assert_eq!(variants(&findings), ["claim-closes-open"]);
        // The suggested action is an edit of the listed record's Status.
        let target = &findings[0].action_source.target;
        assert_eq!(target.file, Some(PathBuf::from("docs/bugs/BUG-1-open.md")));
        assert_eq!(target.field.as_deref(), Some("Status"));
        assert_eq!(examined(&exec), 1);
    }

    #[test]
    fn a_fragment_listing_closed_records_is_clean_observed_failing() {
        let (exec, findings) = run(
            &[(".changeset/a.md", &fragment("[BUG-2, BUG-3]"))],
            &declared(true),
        );
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(examined(&exec), 2);
    }

    /// Closed is the type's declared terminals, not the rule's fallback list: a
    /// `note` declares no statuses, so `Fixed` there is not a declared close.
    #[test]
    fn the_fallback_closed_statuses_do_not_decide_a_listed_record_observed_failing() {
        let (_, findings) = run(
            &[(".changeset/a.md", &fragment("NOTE-1"))],
            &declared(false),
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(variants(&findings), ["claim-closes-open"]);
        assert!(findings[0].message.contains("Status Fixed"), "{findings:?}");
    }

    #[test]
    fn a_listed_id_that_resolves_to_nothing_is_an_error_but_a_verb_claim_still_is_not_observed_failing(
    ) {
        let (exec, findings) = run(
            &[(".changeset/a.md", &fragment("[BUG-99]"))],
            &declared(false),
        );
        assert_eq!(variants(&findings), ["claim-id-unresolved"], "{findings:?}");
        assert_eq!(findings[0].line, Some(3));
        assert!(findings[0].message.contains("BUG-99"), "{findings:?}");
        assert_eq!(examined(&exec), 1);

        // The verb reading keeps today's behaviour: skipped and counted.
        let (exec, findings) = run(
            &[(
                ".changeset/a.md",
                "---\ndefault: patch\ncloses: []\n---\n\nFixes BUG-99.\n",
            )],
            &declared(true),
        );
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(examined(&exec), 0);
        assert_eq!(exec.population.as_ref().unwrap().eligible(), 1);
    }

    #[test]
    fn an_undeclared_field_changes_nothing_observed_failing() {
        let block = "---\ndefault: patch\ncloses:\n  - BUG-9\n---\n\nNo verb here.\n";
        let (exec, findings) = run(
            &[(".changeset/a.md", block)],
            &verb_only(&["Fixed".to_string()]),
        );
        assert!(findings.is_empty(), "{findings:?}");
        let population = exec.population.unwrap();
        assert_eq!((population.eligible(), population.out_of_scope()), (0, 0));
        // No frontmatter, no field, no strictness: nothing to report either.
        let (exec, findings) = run(&[(".changeset/README.md", "# Notes\n")], &verb_only(&[]));
        assert!(findings.is_empty());
        assert_eq!(exec.population.unwrap().out_of_scope(), 0);
    }

    #[test]
    fn a_block_list_is_a_malformed_finding_naming_the_line_observed_failing() {
        let text = "---\ndefault: patch\ncloses:\n  - BUG-1\n---\n";
        let (_, findings) = run(&[(".changeset/a.md", text)], &declared(false));
        assert_eq!(
            variants(&findings),
            ["claim-field-malformed"],
            "{findings:?}"
        );
        assert_eq!(findings[0].line, Some(3));
        assert_eq!(findings[0].severity, FindingSeverity::Error);
        assert!(findings[0].message.contains("closes"), "{findings:?}");
        assert!(findings[0].message.contains("block list"), "{findings:?}");
    }

    #[test]
    fn a_value_that_is_no_id_is_malformed_and_never_guessed_at_observed_failing() {
        let (exec, findings) = run(
            &[(".changeset/a.md", &fragment("\"BUG-2, BUG-3\""))],
            &declared(false),
        );
        assert_eq!(
            variants(&findings),
            ["claim-field-malformed"],
            "{findings:?}"
        );
        // Nothing listed was judged: BUG-2 and BUG-3 are not guessed out of it.
        assert_eq!(examined(&exec), 0);
    }

    #[test]
    fn strict_asks_every_fragment_for_the_field_observed_failing() {
        let bare = "---\ndefault: patch\n---\n\nNames BUG-1 without saying whether it closes it.\n";
        let (_, findings) = run(&[(".changeset/a.md", bare)], &declared(true));
        assert_eq!(variants(&findings), ["claim-field-missing"], "{findings:?}");
        assert_eq!(findings[0].line, Some(1));
        assert!(findings[0].message.contains("closes: []"), "{findings:?}");
        // Not strict: the field is checked when present.
        let (_, findings) = run(&[(".changeset/a.md", bare)], &declared(false));
        assert!(findings.is_empty(), "{findings:?}");
        // `closes: []` is the declaration that it closes nothing.
        let (exec, findings) = run(&[(".changeset/a.md", &fragment("[]"))], &declared(true));
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(examined(&exec), 0);
    }

    #[test]
    fn a_file_with_no_frontmatter_is_out_of_scope_and_disclosed_observed_failing() {
        let (exec, findings) = run(
            &[
                (".changeset/README.md", "# What these are\n"),
                (".changeset/a.md", &fragment("[BUG-2]")),
            ],
            &declared(true),
        );
        assert!(findings.is_empty(), "{findings:?}");
        let population = exec.population.unwrap();
        assert_eq!(population.out_of_scope(), 1);
        assert_eq!(population.examined(), 1);
        assert_eq!(population.eligible(), 2);

        let claims = vec![(
            ".changeset/README.md".to_string(),
            "# What these are\n".to_string(),
        )];
        let notices = claim_field_notices(&claims, &declared(true));
        let text: Vec<&str> = notices.iter().map(|n| n.message.as_str()).collect();
        assert!(
            text.iter()
                .any(|m| m.contains(".changeset/README.md") && m.contains("no frontmatter")),
            "{text:?}"
        );
    }

    /// The field line is not prose: `closes: [BUG-1]` must not be read again
    /// as the closing verb "closes", which would report one claim twice.
    #[test]
    fn the_field_line_is_not_read_again_as_a_closing_verb_observed_failing() {
        let (_, findings) = run(
            &[(".changeset/a.md", &fragment("[BUG-1]"))],
            &declared(true),
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(variants(&findings), ["claim-closes-open"]);
    }

    #[test]
    fn closing_verbs_in_prose_are_still_read_and_counted_when_the_field_is_declared_observed_failing(
    ) {
        let text =
            "---\ndefault: patch\ncloses: [BUG-2]\n---\n\nFixes BUG-1, which is still open.\n";
        let (_, findings) = run(&[(".changeset/a.md", text)], &declared(true));
        assert_eq!(
            variants(&findings),
            ["claim-status-disagrees"],
            "{findings:?}"
        );
        assert_eq!(findings[0].line, Some(6));

        let claims = vec![(".changeset/a.md".to_string(), text.to_string())];
        let notices = claim_field_notices(&claims, &declared(true));
        assert!(
            notices
                .iter()
                .any(|n| n.message.contains("1 closing-verb claim")),
            "{notices:?}"
        );
        // Declared nothing: nothing to disclose.
        assert!(claim_field_notices(&claims, &verb_only(&[])).is_empty());
    }

    #[test]
    fn findings_come_back_in_file_and_line_order_observed_failing() {
        let a = "---\ndefault: patch\ncloses: [BUG-1]\n---\n\nFixes BUG-1.\n";
        let b = "---\ndefault: patch\n---\n";
        let (_, findings) = run(
            &[(".changeset/a.md", a), (".changeset/b.md", b)],
            &declared(true),
        );
        let order: Vec<(String, Option<usize>)> = findings
            .iter()
            .map(|f| (f.file.display().to_string(), f.line))
            .collect();
        assert_eq!(
            order,
            [
                (".changeset/a.md".to_string(), Some(3)),
                (".changeset/a.md".to_string(), Some(6)),
                (".changeset/b.md".to_string(), Some(1)),
            ]
        );
    }

    #[test]
    fn one_record_listed_by_two_fragments_is_one_candidate_and_two_findings_observed_failing() {
        let (exec, findings) = run(
            &[
                (".changeset/a.md", &fragment("[BUG-1]")),
                (".changeset/b.md", &fragment("BUG-1")),
            ],
            &declared(false),
        );
        assert_eq!(findings.len(), 2, "{findings:?}");
        let population = exec.population.unwrap();
        assert_eq!((population.eligible(), population.examined()), (1, 1));
    }
}
