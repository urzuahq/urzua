use super::*;
use crate::embodiment_model::{EmbodimentDecl, Evidence};

/// A record's `Realized-by` field, categorized (ADR-0018's MVP scope --
/// three flat lists, not RFC-0005's full claim graph). `spec:`/`code:`/
/// `test:` prefixes; an unrecognized prefix is dropped rather than reported,
/// since this field's closure isn't this rule's concern.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct RealizedBy {
    pub(crate) spec: Vec<String>,
    pub(crate) code: Vec<String>,
    pub(crate) test: Vec<String>,
}

/// Lexical normalization of a locator path: drops `.` segments and empty
/// ones (`//`), and resolves `x/..`. `./src/b.rs` and `src/../src/b.rs` name
/// the tracked file `src/b.rs`, which git's own pathspec resolution (drift
/// detection) already treated as the same path; membership against the
/// tracked set (`embodiment.locator-exists`) must too. Applied once, in
/// [`parse_realized_by`], so every rule reading a locator agrees. A leading
/// `..` is kept (it escapes the repository and matches nothing); an empty
/// locator stays empty so the rule can report it.
pub fn normalize_locator(raw: &str) -> String {
    let raw = raw.trim();
    // An absolute locator is never a repository path; keep it as written so
    // it cannot normalize into a tracked relative one.
    if raw.starts_with('/') {
        return raw.to_string();
    }
    let mut parts: Vec<&str> = Vec::new();
    for seg in raw.split('/') {
        match seg {
            "" | "." => {}
            ".." if parts.last().is_some_and(|l| *l != "..") => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    if parts.is_empty() && !raw.is_empty() {
        return ".".to_string();
    }
    parts.join("/")
}

/// A locator that parses is its normalised path; one that does not stays as
/// written, normalised, so every reader still has a path to name.
pub(crate) fn parse_realized_by(value: &str) -> RealizedBy {
    let mut result = RealizedBy::default();
    for (prefix, locator) in crate::locator::realized_by_entries(value) {
        let locator = match crate::locator::parse_locator(locator) {
            Ok(l) => l.path,
            Err(_) => normalize_locator(locator),
        };
        match prefix {
            "spec" => result.spec.push(locator),
            "code" => result.code.push(locator),
            "test" => result.test.push(locator),
            _ => {}
        }
    }
    result
}

/// Every locator path across all three categories, flattened -- what a
/// caller needs to walk each one's git history for drift detection
/// (ADR-0032). Order isn't meaningful; only membership is.
pub fn realized_by_locator_paths(value: &str) -> Vec<String> {
    let parsed = parse_realized_by(value);
    parsed
        .spec
        .into_iter()
        .chain(parsed.code)
        .chain(parsed.test)
        .collect()
}

/// ADR-0018's tier computation: the highest tier with a non-empty category.
/// The declared vocabulary (`MILE-144`, `Config::embodiment`) names the tiers
/// and their order; by default `test` outranks `code` outranks `spec` -- a
/// claim verified by a test is stronger evidence than a claim only pointed at
/// by a spec. `drifted` (ADR-0032) overrides every tier unconditionally -- the
/// drift state is an alarm reachable from any other state, not just another
/// tier to rank against the others.
/// `back_pointed` (`MILE-128`): a source comment cites the record, which is
/// code-tier evidence -- it lifts a lower tier to the code tier, and never
/// outranks a `test` locator or drift.
/// `test_pointed` (`MILE-146`): a test-class file cites it with `Verifies:`,
/// which is test-tier evidence, equal to a `test` locator.
pub(crate) fn compute_embodiment<'a>(
    decl: &'a EmbodimentDecl,
    realized_by: &RealizedBy,
    drifted: bool,
    back_pointed: bool,
    test_pointed: bool,
) -> &'a str {
    decl.state_for(drifted, |evidence| match evidence {
        Evidence::Test => !realized_by.test.is_empty() || test_pointed,
        Evidence::Code => !realized_by.code.is_empty() || back_pointed,
        Evidence::Spec => !realized_by.spec.is_empty(),
    })
}

/// The field name `record`'s own type uses for `role` (`RFC-42`/`ADR-61`),
/// or the role's pre-`RFC-42` default if the type isn't configured at all.
pub fn relation_field_name<'a>(
    config: &'a Config,
    record_type: &str,
    role: crate::config::RelationRole,
) -> &'a str {
    config
        .record_types
        .get(record_type)
        .map(|t| t.relation_field(role))
        .unwrap_or_else(|| role.default_field_name())
}

// Implements: ADR-74 (every scope failure is reported here, never passed)
/// A `Realized-by` locator that does not point at anything: a path that is
/// not in the working tree, an empty one, one that does not parse, or a scope
/// that does not resolve in the file (`ADR-74` point 6).
///
/// `Embodiment` is computed from these paths, so a locator naming nothing lets
/// a record claim verified work against a file that is not there -- and the
/// claim reads as stronger than `Not started`, not weaker.
///
/// A directory or glob locator is a locator when the repository accepts that
/// kind (`locator_kinds`) and it matches at least one tracked file that is
/// present; a pin on one needs `pinnable` to allow it (`BUG-176`).
///
/// `tracked`, `present`, `read` and `untracked_on_disk` are supplied by the caller: this
/// crate does not touch the filesystem. `read` returns a file's text for a
/// scope to resolve in. `untracked_on_disk` is true for a path that exists but
/// that git does not track, which is its own variant (`BUG-13`) because the
/// repair is to track the file, not to correct the locator.
pub fn embodiment_locator_exists(
    records: &[Record],
    config: &Config,
    tracked: &[PathBuf],
    present: &dyn Fn(&str) -> bool,
    read: &dyn Fn(&str) -> crate::locator::SharedText,
    untracked_on_disk: &dyn Fn(&str) -> bool,
) -> (RuleExecution, Vec<Finding>) {
    use crate::locator;
    const RULE_ID: &str = RULE_EMBODIMENT_LOCATOR_EXISTS;
    let mut findings = Vec::new();
    let env = LocatorEnv {
        config,
        opts: config.rule_opts(RULE_ID),
        tracked,
        present,
        read,
        untracked_on_disk,
        records,
    };

    // One candidate per declared slot, not per locator: the population is the
    // rule's input, not its work count.
    let slots = declared_slots_for_roles(
        records,
        config,
        &[crate::config::RelationRole::EmbodimentLocator],
    );

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |record| *record,
        |record| {
            let field = relation_field_name(
                config,
                &record.record_type,
                crate::config::RelationRole::EmbodimentLocator,
            );
            // A Done-means item clause (`ADR-96`) is evidence too, held to the
            // same test; a record with item clauses and no `Realized-by` is
            // still examined.
            let clauses: Vec<&crate::done_items::Clause> = record
                .items
                .iter()
                .flat_map(|i| i.items.iter())
                .filter_map(|i| i.evidence.as_ref())
                .filter(|c| c.valid)
                .collect();
            let value = match declared_value(record, field) {
                Ok(v) => Some(v),
                Err(Outcome::Absent) if !clauses.is_empty() => None,
                Err(outcome) => return outcome,
            };
            if let Some(value) = value {
                let mut report = |variant: &'static str, message: String| {
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Error,
                        file: record.path.clone(),
                        line: None,
                        waived: None,
                        declared_severity: None,
                        suggested_action: None,
                        action_source: ActionSource::new(variant).field(field),
                        message,
                    });
                };
                // Text the splitter drops is evidence that does not exist; say so.
                if let Some(lead) = locator::realized_by_unrecognised(value) {
                    report(
                        "locator-malformed",
                        format!(
                            "{field} holds '{lead}', which is not a recognised entry and counts as no evidence (grammar: {})",
                            locator::GRAMMAR
                        ),
                    );
                }
                for entry in locator::realized_by_locators(value) {
                    if let Some((variant, message)) = locator_defect(&entry, field, &env) {
                        report(variant, message);
                    }
                }
            }
            let section = if clauses.is_empty() {
                String::new()
            } else {
                items_section(config, &record.record_type)
            };
            for clause in clauses {
                let place = format!("a Done means item (line {})", clause.line);
                for entry in locator::realized_by_locators(&clause.text) {
                    if let Some((_, message)) = locator_defect(&entry, &place, &env) {
                        findings.push(Finding {
                            rule: RULE_ID.to_string(),
                            severity: FindingSeverity::Error,
                            file: record.path.clone(),
                            line: Some(clause.line),
                            waived: None,
                            declared_severity: None,
                            suggested_action: None,
                            action_source: ActionSource::new("item-locator-defect")
                                .section(section.clone())
                                .line(clause.line),
                            message,
                        });
                    }
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

/// What a locator is checked against: the tracked files, the working tree and
/// the repository's declared `locator_kinds` and `pinnable` (`BUG-176`). Core
/// touches no filesystem; the caller supplies each lookup.
pub struct LocatorEnv<'a> {
    pub config: &'a Config,
    pub opts: LocatorExistsOpts,
    pub tracked: &'a [PathBuf],
    pub present: &'a dyn Fn(&str) -> bool,
    pub read: &'a dyn Fn(&str) -> crate::locator::SharedText,
    pub untracked_on_disk: &'a dyn Fn(&str) -> bool,
    /// The records, for a Done-means item whose kind asks that the records it
    /// cites have reached a `success` status (`ADR-96`, amendment of
    /// 2026-11-07). Empty where nothing is judged against a cited record.
    pub records: &'a [Record],
}

impl<'a> LocatorEnv<'a> {
    /// Runs `f` over a working tree with no file in it: for a caller with no
    /// filesystem (a unit test, a property test).
    pub fn without_files<R>(config: &'a Config, f: impl FnOnce(&LocatorEnv<'_>) -> R) -> R {
        let present = |_: &str| false;
        let env = LocatorEnv {
            config,
            opts: config.rule_opts(RULE_EMBODIMENT_LOCATOR_EXISTS),
            tracked: &[],
            present: &present,
            read: &crate::locator::no_file_text,
            untracked_on_disk: &present,
            records: &[],
        };
        f(&env)
    }
}

/// Why one `Realized-by`-grammar locator does not point at anything: the
/// finding variant and message `embodiment.locator-exists` reports, or `None`
/// when it resolves. `field` names the place it was written, for the message.
/// Shared with the Done-means items (`ADR-96`), whose locators are held to the
/// same test.
pub(crate) fn locator_defect(
    entry: &crate::locator::LocatorEntry,
    field: &str,
    env: &LocatorEnv<'_>,
) -> Option<(&'static str, String)> {
    use crate::config::LocatorKind;
    use crate::locator::{self, Anchor, FileText, ScopeError};
    const RULE_ID: &str = RULE_EMBODIMENT_LOCATOR_EXISTS;
    let LocatorEnv {
        config,
        opts,
        tracked,
        present,
        read,
        untracked_on_disk,
        records: _,
    } = env;
    macro_rules! defect {
        ($variant:expr, $message:expr $(,)?) => {
            return Some(($variant, $message))
        };
    }
    let l = match &entry.parsed {
        Ok(l) => l,
        Err(e) => {
            defect!(
                "locator-malformed",
                format!(
                    "{field} names '{}', which does not parse: {e} (grammar: {})",
                    entry.raw,
                    locator::GRAMMAR
                ),
            );
        }
    };
    // An empty locator joins to the repository root, which exists --
    // so it would pass while still computing to `Implemented`.
    if l.path.trim().is_empty() {
        defect!("locator-empty", format!("{field} has an empty locator"));
    }
    let matched = match locator::match_locator_files(&l.path, tracked, present) {
        Ok(m) => m,
        Err(e) => {
            defect!(
                "locator-malformed",
                format!(
                    "{field} names '{}', whose glob {e} (grammar: {})",
                    l.path,
                    locator::GRAMMAR
                ),
            );
        }
    };
    let Some(kind) = matched.kind else {
        // A `#` that did not parse as a scope stayed in the path: it
        // is a malformed scope, not a file missing from the tree.
        if l.path.contains('#') {
            defect!("locator-malformed", format!(
                    "{field} names '{}', whose text after `#` is not a scope and which is not a file in the working tree (grammar: {})",
                    l.path,
                    locator::GRAMMAR
                ));
        } else if untracked_on_disk(&l.path) {
            defect!(
                "locator-untracked",
                format!(
                    "{field} names '{}', which exists on disk but is not tracked by git",
                    l.path
                ),
            );
        } else {
            defect!(
                "locator-missing",
                format!(
                    "{field} names '{}', which is not in the working tree",
                    l.path
                ),
            );
        }
    };
    // BUG-176: which kinds are legal, and which can be pinned, is config
    if !opts.locator_kinds.contains(&kind) {
        defect!(
                "locator-kind-undeclared",
                format!(
                    "{field} names '{}', a {kind} locator, and `{RULE_ID}` does not accept that kind: add `{kind}` to its `locator_kinds` option, or name a file",
                    l.path
                ),
            );
    }
    if kind != LocatorKind::File && matched.files.is_empty() {
        defect!(
            "locator-missing",
            format!(
                "{field} names '{}', a glob that matches no tracked file in the working tree",
                l.path
            ),
        );
    }
    if matches!(l.anchor, Some(Anchor::Commit(_))) {
        defect!(
            "locator-commit-anchor",
            format!(
                "{field} names '{}': {}",
                l.display(),
                ScopeError::CommitAnchor
            ),
        );
    }
    if matches!(l.anchor, Some(Anchor::Pin(_))) && !opts.pinnable.contains(&kind) {
        defect!(
                "locator-kind-unpinnable",
                format!(
                    "{field} names '{}' with a pin, and `{RULE_ID}` does not allow a {kind} locator to be pinned: add `{kind}` to its `pinnable` option, or remove the pin",
                    l.display()
                ),
            );
    }
    let Some(scope) = &l.scope else { return None };
    if kind != LocatorKind::File {
        defect!(
            "locator-scope-unresolved",
            format!(
                "{field} names '{}', whose scope does not resolve: {}",
                l.display(),
                ScopeError::Unsupported(format!(
                    "a scope applies to one file, and this locator names a {kind}"
                ))
            ),
        );
    }
    let resolved = match &*read(&l.path) {
        FileText::Text(text) => locator::select_resolver(&config.locator_resolvers, &l.path)
            .and_then(|resolver| locator::resolve_scope(text, resolver, scope)),
        FileText::NotUtf8 => Err(ScopeError::NotUtf8),
        FileText::Missing => Err(ScopeError::NotFound(
            "the file could not be read".to_string(),
        )),
    };
    match resolved {
            Err(e) => defect!("locator-scope-unresolved", format!(
                "{field} names '{}', whose scope does not resolve: {e}",
                l.display()
            )),
            Ok(_) if l.anchor.is_none() => defect!("locator-scope-unpinned", format!(
                "{field} names '{}': scope given without a pin; drift is judged on the whole file (add `@sha256:` from `urzua explain {}`)",
                l.display(),
                l.display()
            )),
            Ok(_) => {}
        }

    None
}

/// Rule (ADR-0018/ADR-0032): a record's stated `Embodiment` must agree with
/// what its own `Realized-by` locators compute to, including drift -- a
/// locator that changed, per git history, since the `Realized-by` line was
/// last touched. Both `Embodiment` and `Realized-by` have to be present --
/// a record with no `Realized-by` at all is eligible and unexamined on a
/// corpus that hasn't adopted the field, not a defect in the rule. `drifted` is
/// precomputed by the caller (git history is I/O, this function isn't --
/// same shape as `full_text` elsewhere in this module); so is `back_pointed`,
/// the records source comments cite at each tier (`MILE-128`, `MILE-146`),
/// from [`back_pointer_evidence`]. A record with no `Realized-by` field that
/// a back-pointer cites is judged on that evidence alone (`MILE-149`); one
/// with neither is judged only when it states `Verified` or `Implemented`, a
/// claim with no evidence (`MILE-161`), and otherwise unexamined. `claim_age_days` (`MILE-129`) is each record's
/// days since its `Realized-by` line was last touched, computed by the caller
/// and read only while `claim_max_age_days` is set: a claim older than that
/// draws a warning in addition to any disagreement. `fresh_drift` (`MILE-129`)
/// holds the records whose only drift is inside `drift_grace_days`: they
/// compute to their tier, and a stated `Drift detected` is accepted too.
pub fn embodiment_consistency(
    records: &[Record],
    config: &Config,
    drifted: &HashSet<PathBuf>,
    fresh_drift: &HashSet<PathBuf>,
    back_pointed: &BackPointerEvidence,
    claim_age_days: &HashMap<PathBuf, u64>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_EMBODIMENT_CONSISTENCY;
    let mut findings = Vec::new();
    let max_age = config.rules.get(RULE_ID).and_then(|s| s.claim_max_age_days);
    let sticky = sticky_states(config);
    let audit_field = config
        .rules
        .get(RULE_ID)
        .and_then(|s| s.drift_audit_field.as_deref())
        .map(str::trim);

    // The candidate is the *pair*: the rule needs both fields, so a record
    // carrying only one of them is handed to the rule and reaches no verdict.
    // Treating each field as its own slot would make that state unreportable.
    let slots = declared_slots_for_roles(
        records,
        config,
        &[
            crate::config::RelationRole::EmbodimentState,
            crate::config::RelationRole::EmbodimentLocator,
        ],
    );

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |record| *record,
        |record| {
            let state_field = relation_field_name(
                config,
                &record.record_type,
                crate::config::RelationRole::EmbodimentState,
            );
            let locator_field = relation_field_name(
                config,
                &record.record_type,
                crate::config::RelationRole::EmbodimentLocator,
            );
            let stated = match declared_value(record, state_field) {
                Ok(v) => v,
                Err(outcome) => return outcome,
            };
            // A sticky state is a human's, and no evidence can reproduce it
            // (`MILE-148`): examined, and neither disagreement nor age applies.
            if sticky.contains(stated.trim()) {
                return Outcome::Examined;
            }
            // A malformed last-audit date is reported, never ignored: the grace
            // window falls back to the file's last change for this record
            // (`MILE-166`).
            if let Some(field) = audit_field {
                if let Ok(value) = declared_value(record, field) {
                    if let Err(problem) = date_unix(value) {
                        findings.push(Finding {
                            rule: RULE_ID.to_string(),
                            severity: FindingSeverity::Warning,
                            file: record.path.clone(),
                            line: None,
                            waived: None,
                            declared_severity: None,
                            suggested_action: None,
                            action_source: ActionSource::new("last-audit-date-invalid").field(field),
                            message: format!(
                                "last-audit field {field} value '{}' {problem}, so drift_grace_days counts from the cited file's last change for this record",
                                value.trim()
                            ),
                        });
                    }
                }
            }
            let cited_by_code = back_pointed.code.contains(&record.path);
            let cited_by_test = back_pointed.test.contains(&record.path);
            // A missing `Realized-by` is judged when a back-pointer cites the
            // record (`MILE-149`), or when the record states a tier that no
            // evidence of either kind supports (`MILE-161`); one stating no
            // tier stays unexamined, and an unreadable header is never judged.
            let claims_a_tier = config.embodiment.claims_built(stated.trim());
            let realized_by_value = match evidence_value(record, locator_field) {
                Ok(v) => Some(v),
                Err(Outcome::Absent) if cited_by_code || cited_by_test || claims_a_tier => None,
                Err(outcome) => return outcome,
            };

            let computed = compute_embodiment(
                &config.embodiment,
                &realized_by_value
                    .as_deref()
                    .map(parse_realized_by)
                    .unwrap_or_default(),
                drifted.contains(&record.path),
                cited_by_code,
                cited_by_test,
            );
            let evidence = evidence_description(
                locator_field,
                realized_by_value.is_some(),
                cited_by_code,
                cited_by_test,
            );
            // Fresh drift (`drift_grace_days`) only holds back reporting: a
            // record that already says `Drift detected` is right, not wrong.
            let agrees_with_fresh_drift =
                fresh_drift.contains(&record.path) && config.embodiment.is_drift(stated.trim());
            if stated.trim() != computed && !agrees_with_fresh_drift {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Warning,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: if realized_by_value.is_some() || cited_by_code || cited_by_test { ActionSource::new("embodiment-drift").argv(["urzua", "fix", "--apply", "--ids", &record.path.display().to_string()]) } else { ActionSource::new("embodiment-unevidenced").field(state_field) },
                    message: format!(
                    "stated {state_field} '{}' disagrees with '{computed}', computed from {evidence}",
                    stated.trim()
                ),
                });
            }
            if let (Some(max), Some(age)) = (max_age, claim_age_days.get(&record.path)) {
                if *age > u64::from(max) {
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Warning,
                        file: record.path.clone(),
                        line: None,
                        waived: None,
                        declared_severity: None,
                        suggested_action: None,
                        action_source: ActionSource::new("claim-too-old"),
                        message: format!(
                            "stated {state_field} was last verified {age} days ago, longer than the configured claim_max_age_days of {max}"
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

/// The `Embodiment` values the repository declares as set by a human and not
/// computable (`MILE-148`): `sticky_states` of `embodiment.consistency`. Empty
/// when the rule or the option is absent, so every stated value is judged.
pub fn sticky_states(config: &Config) -> HashSet<&str> {
    config
        .rules
        .get(RULE_EMBODIMENT_CONSISTENCY)
        .and_then(|s| s.sticky_states.as_deref())
        .unwrap_or_default()
        .iter()
        .map(|s| s.trim())
        .collect()
}

/// What a consistency finding says its computation read: the `Realized-by`
/// field, the back-pointers that cited the record, or both (`MILE-149`).
pub(crate) fn evidence_description(
    locator_field: &str,
    has_locators: bool,
    cited_by_code: bool,
    cited_by_test: bool,
) -> String {
    let mut parts = Vec::new();
    if has_locators {
        parts.push(locator_field.to_string());
    }
    if cited_by_code {
        parts.push("source back-pointers".to_string());
    }
    if cited_by_test {
        parts.push("test back-pointers".to_string());
    }
    if parts.is_empty() {
        return format!(
            "no evidence (the record has no {locator_field} field and no back-pointer cites it)"
        );
    }
    let mut text = parts.join(" and ");
    if !has_locators {
        text.push_str(&format!(" (the record has no {locator_field} field)"));
    }
    text
}

/// Rule (ADR-0018): the same locator cited by more than one record's
/// `Realized-by` is a promotion candidate -- each record would otherwise
/// drift independently on what is really one piece of shared evidence (the
/// ADR-0072/0073/0074 shape RFC-0005 names). Reports only; a human runs the
/// actual promotion into a `claim` record, never this rule.
///
/// With `scope: changed` (`ADR-90`) the rule is a nudge for the moment someone
/// touches a record that shares a locator: a locator is reported only when a
/// record the diff adds or changes cites it, and the finding sits on that
/// changed record. The unchanged citers still count toward "cited by more than
/// one record" and are named in the message, so a changed record is nudged when
/// it joins a shared locator; the unchanged records are counted in the
/// population's `deferred_by_scope` and carry no finding.
pub fn embodiment_locator_promotion_candidate(
    records: &[Record],
    config: &Config,
    scope: Option<&ChangedOnly>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE;
    let mut findings = Vec::new();

    let mut citers: std::collections::BTreeMap<
        String,
        std::collections::BTreeSet<std::path::PathBuf>,
    > = std::collections::BTreeMap::new();
    // `scope: changed`: the citers that are changed records, per locator.
    let mut changed_citers: std::collections::BTreeMap<
        String,
        std::collections::BTreeSet<std::path::PathBuf>,
    > = std::collections::BTreeMap::new();

    let slots = declared_slots_for_roles(
        records,
        config,
        &[crate::config::RelationRole::EmbodimentLocator],
    );

    // Findings are emitted after the census, not inside it: this rule judges
    // locators across records, so no single candidate is at fault.
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |record| *record,
        |record| {
            let field = relation_field_name(
                config,
                &record.record_type,
                crate::config::RelationRole::EmbodimentLocator,
            );
            let realized_by_value = match evidence_value(record, field) {
                Ok(v) => v,
                Err(outcome) => return outcome,
            };

            let parsed = parse_realized_by(&realized_by_value);
            for locator in parsed
                .spec
                .iter()
                .chain(parsed.code.iter())
                .chain(parsed.test.iter())
            {
                // A BTreeSet, not a Vec: the same record citing one locator
                // under both `code:` and `test:` is still one record, not two
                // independent citers -- cross-record duplication is what needs
                // promotion, not cross-category duplication within one record.
                citers
                    .entry(locator.clone())
                    .or_default()
                    .insert(record.path.clone());
                if scope.is_some_and(|s| !s.defers(record)) {
                    changed_citers
                        .entry(locator.clone())
                        .or_default()
                        .insert(record.path.clone());
                }
            }
            // `scope: changed` (`ADR-90`): the slot is eligible and counted as a
            // citer above, and not judged for a record the diff did not change.
            if scope.is_some_and(|s| s.defers(record)) {
                return Outcome::DeferredByScope;
            }
            Outcome::Examined
        },
    );

    for (locator, paths) in citers {
        if paths.len() < 2 {
            continue;
        }
        let names: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
        // Under `scope: changed` (`ADR-90` amendment, point 5) one finding sits on
        // each changed citer, so `check <path>` and a file-scoped waiver see the
        // finding on the record they examine; a locator no changed record cites
        // is not reported. Without the option, one finding on the first citer.
        let anchors: Vec<std::path::PathBuf> = if scope.is_some() {
            changed_citers
                .get(&locator)
                .map(|c| c.iter().cloned().collect())
                .unwrap_or_default()
        } else {
            vec![paths.iter().next().expect("checked len >= 2 above").clone()]
        };
        for anchor in anchors {
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Warning,
                file: anchor,
                line: None,
                waived: None,
                declared_severity: None,
                suggested_action: None,
                action_source: ActionSource::new("locator-promotion-candidate"),
                message: format!(
                    "locator '{locator}' is cited by {} records ({}) -- consider promoting to a shared claim record",
                    paths.len(),
                    names.join(", ")
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

/// Rule (`MILE-138`): a record whose `Status` is not an accepted status cannot
/// state an `Embodiment` of `Implemented` or `Verified`. Accepted means the
/// type's declared `statuses.success`; a type declaring none falls back to the
/// rule's `accepted_statuses` option, and with neither it is not examined
/// (the narrowest reading: no guess at which status means accepted).
pub fn embodiment_requires_accepted_status(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_EMBODIMENT_REQUIRES_ACCEPTED_STATUS;
    use crate::config::RelationRole::{EmbodimentState, Status};
    let mut findings = Vec::new();
    let declared = statuses_by_type(config);
    let fallback: Vec<String> = config
        .rules
        .get(RULE_ID)
        .and_then(|s| s.accepted_statuses.clone())
        .unwrap_or_default();
    // A type's own success terminals win; the option only stands in for a type
    // declaring none.
    let accepted_for = |record_type: &str| -> Vec<String> {
        match declared.get(record_type) {
            Some(d) if !d.success.is_empty() => {
                let mut v = d.success.clone();
                v.extend(d.claim_retained.iter().cloned());
                v
            }
            _ => fallback.clone(),
        }
    };
    let candidates = declared_slots_for_roles(records, config, &[Status, EmbodimentState]);

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Record,
        candidates,
        |record| *record,
        |record| {
            let status_field = relation_field_name(config, &record.record_type, Status);
            let state_field = relation_field_name(config, &record.record_type, EmbodimentState);
            let status = match declared_value(record, status_field) {
                Ok(v) if !v.trim().is_empty() => v.trim(),
                Ok(_) => return Outcome::Absent,
                Err(outcome) => return outcome,
            };
            let stated = match declared_value(record, state_field) {
                Ok(v) if !v.trim().is_empty() => v.trim(),
                Ok(_) => return Outcome::Absent,
                Err(outcome) => return outcome,
            };
            let accepted = accepted_for(&record.record_type);
            if accepted.is_empty() {
                // No declared accepted status: eligible, but nothing to judge by.
                return Outcome::Absent;
            }
            if config.embodiment.claims_built(stated)
                && !accepted.iter().any(|a| a.trim() == status)
            {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new("embodiment-state-not-accepted").field(state_field),
                    message: format!(
                        "{state_field} '{stated}' cannot be stated while {status_field} is '{status}' (accepted: {})",
                        accepted.join(", ")
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
    fn other_readers_see_the_bare_path_whatever_the_pin_or_scope() {
        let h = "a".repeat(64);
        let value = format!("code:./src/a.rs#Foo@sha256:{h}, test:t.rs, spec:s.md@sha256:ABC");
        assert_eq!(parse_realized_by(&value).code, vec!["src/a.rs".to_string()]);
        assert!(realized_by_locator_paths(&value).contains(&"src/a.rs".to_string()));
        // A malformed pin is neither a pin nor a path; the existence rule
        // reports it, and every other reader still has a path to name.
        assert_eq!(
            parse_realized_by(&value).spec,
            vec!["s.md@sha256:ABC".to_string()]
        );
    }

    // ---- scoped locators (ADR-74) ------------------------------------------------

    fn scoped_config() -> Config {
        let mut config = embodiment_config();
        config.locator_resolvers = vec![yaml_serde::from_str(
            r#"
name: rust
glob: "**/*.rs"
start: '^\s*(pub\s+)?(fn|struct)\s+{symbol}\b'
end: brace_balanced
"#,
        )
        .unwrap()];
        config
    }

    /// The findings of one `Realized-by` value over files `a.rs` (valid) and
    /// `bin.rs` (not UTF-8).
    fn exists_messages(value: &str) -> Vec<String> {
        use crate::locator::FileText;
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            &format!("---\nRealized-by: '{value}'\n---\n"),
        );
        let present = |p: &str| matches!(p, "a.rs" | "bin.rs" | "weird#1.md" | "docs/c.md");
        let read = |p: &str| {
            std::rc::Rc::new(match p {
                "a.rs" => FileText::Text("fn one() {\n}\nfn two() {\n}\nfn two() {\n}\n".into()),
                "bin.rs" => FileText::NotUtf8,
                "docs/c.md" => FileText::Text("Foo\n".into()),
                _ => FileText::Missing,
            })
        };
        let (_, findings) =
            embodiment_locator_exists(&[r], &scoped_config(), &[], &present, &read, &|_| false);
        findings.into_iter().map(|f| f.message).collect()
    }

    #[test]
    fn a_pinned_scope_that_resolves_is_silent() {
        let h = "a".repeat(64);
        assert!(exists_messages(&format!(
            "code:a.rs#one@sha256:{h}, code:a.rs#L1-L2@sha256:{h}"
        ))
        .is_empty());
    }

    #[test]
    fn a_scope_without_a_pin_is_reported_and_names_the_authoring_route() {
        let m = exists_messages("code:a.rs#one");
        assert_eq!(m.len(), 1, "{m:?}");
        assert!(
            m[0].contains("without a pin") && m[0].contains("urzua explain"),
            "{m:?}"
        );
    }

    #[test]
    fn every_scope_failure_is_one_finding_with_its_reason() {
        let h = "b".repeat(64);
        let cases = [
            (format!("code:a.rs#missing@sha256:{h}"), "matches nothing"),
            (format!("code:a.rs#two@sha256:{h}"), "ambiguous"),
            (format!("code:a.rs#L1-L99@sha256:{h}"), "beyond the end"),
            (format!("code:bin.rs#L1@sha256:{h}"), "not valid UTF-8"),
            (
                format!("code:docs/c.md#Foo@sha256:{h}"),
                "no locator resolver",
            ),
            (
                "code:a.rs#one@abcdef1".to_string(),
                "commit anchor not yet supported",
            ),
            (format!("code:a.rs#one::<T>::f@sha256:{h}"), "unsupported"),
        ];
        for (value, reason) in cases {
            let m = exists_messages(&value);
            assert_eq!(m.len(), 1, "{value}: {m:?}");
            assert!(m[0].contains(reason), "{value}: {m:?}");
        }
    }

    #[test]
    fn a_locator_that_does_not_parse_is_a_malformed_finding_not_a_missing_path() {
        let h = "c".repeat(64);
        for value in [
            "code:a.rs#L0".to_string(),
            "code:a.rs#L40-L10".to_string(),
            format!("code:a.rs@abcdef1@sha256:{h}"),
            "spec:s.md@sha256:ABC".to_string(),
            "code:src/**#Foo".to_string(),
        ] {
            let m = exists_messages(&value);
            assert_eq!(m.len(), 1, "{value}: {m:?}");
            assert!(
                m[0].contains("does not parse") && m[0].contains("grammar"),
                "{value}: {m:?}"
            );
            assert!(!m[0].contains("not in the working tree"), "{value}: {m:?}");
        }
        // A `#` that is not a scope, on a file that is not there: a malformed
        // scope naming the grammar, not a plain missing path.
        let m = exists_messages("code:gone#1.md");
        assert!(
            m[0].contains("grammar") && !m[0].contains("not in the working tree"),
            "{m:?}"
        );
        // The same text naming a file that is there passes.
        assert!(exists_messages("code:weird#1.md").is_empty());
    }

    #[test]
    fn a_comma_inside_a_locator_path_is_part_of_the_path_observed_failing() {
        let r = parse_realized_by("code:src/weird,name.rs, test:tests/a, b.rs");
        assert_eq!(r.code, vec!["src/weird,name.rs".to_string()]);
        assert_eq!(r.test, vec!["tests/a, b.rs".to_string()]);
        let entries = crate::locator::realized_by_locators("spec:docs/x,y.md,code:src/a.rs");
        let paths: Vec<_> = entries
            .iter()
            .map(|e| e.parsed.as_ref().unwrap().path.clone())
            .collect();
        assert_eq!(paths, vec!["docs/x,y.md", "src/a.rs"]);
    }

    #[test]
    fn a_comma_followed_by_a_category_prefix_separates_entries() {
        let r = parse_realized_by("code:a.rs,test:b.rs ,  spec:c.md");
        assert_eq!(r.code, vec!["a.rs".to_string()]);
        assert_eq!(r.test, vec!["b.rs".to_string()]);
        assert_eq!(r.spec, vec!["c.md".to_string()]);
    }

    #[test]
    fn a_comma_followed_by_an_unknown_prefix_stays_in_the_path() {
        // The unknown prefix is no category, so the text is not an entry of its
        // own; the path names nothing and `embodiment.locator-exists` reports it.
        let r = parse_realized_by("code:a.rs, docs:b.md");
        assert_eq!(r.code, vec!["a.rs, docs:b.md".to_string()]);
    }

    #[test]
    fn a_claim_older_than_claim_max_age_days_warns_and_only_when_declared() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nEmbodiment: Implemented\nRealized-by: code:src/lib.rs\n---\n",
        );
        let ages: HashMap<PathBuf, u64> = [(r.path.clone(), 40)].into();
        let run = |config: &Config| {
            embodiment_consistency(
                std::slice::from_ref(&r),
                config,
                &HashSet::new(),
                &HashSet::new(),
                &Default::default(),
                &ages,
            )
            .1
        };
        assert!(run(&embodiment_config()).is_empty());
        let with_max = |days: u32| {
            let mut config = embodiment_config();
            let setting: crate::config::RuleSetting =
                yaml_serde::from_str(&format!("{{level: warn, claim_max_age_days: {days}}}"))
                    .unwrap();
            config
                .rules
                .insert(RULE_EMBODIMENT_CONSISTENCY.to_string(), setting);
            config
        };
        let findings = run(&with_max(30));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("40 days"), "{findings:?}");
        assert!(
            run(&with_max(40)).is_empty(),
            "exactly at the limit is not over it"
        );
    }

    #[test]
    fn a_slot_declared_only_as_required_is_still_a_declared_slot() {
        // `known_fields` is documented as the fields a type carries *beyond*
        // `required_fields`, so a type that requires `Realized-by` declares it
        // just as much as one that lists it as known. Selecting candidates from
        // `known_fields` alone handed these rules nothing, and a rule handed
        // nothing reports a clean `eligible: 0` -- the ADR-55 shape.
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nEmbodiment: Verified\nRealized-by: code:src/lib.rs\n---\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&["Embodiment", "Realized-by"], None, None, None),
        )]);

        let (exec, findings) = embodiment_consistency(
            &[r],
            &config,
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
            &HashMap::new(),
        );
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(
            (population.eligible(), population.examined()),
            (1, 1),
            "the pair is declared, so the record is a candidate the rule judged"
        );
        assert_eq!(findings.len(), 1, "Verified disagrees with Implemented");
    }

    /// `MILE-150`: an `exclude_files` entry that names no tracked file directly
    /// in the type's `dir` is a stale exclusion, reported rather than ignored.
    #[test]
    fn an_exclude_files_entry_matching_no_file_is_a_warning_observed_failing() {
        let yaml = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    exclude_files: [\"README.md\", \"gone.md\"]\n";
        let config = crate::config::parse(yaml).unwrap();
        let path = PathBuf::from(".urzua/config.yaml");
        let tracked = vec![
            PathBuf::from("docs/adr/README.md"),
            PathBuf::from("docs/adr/ADR-1-a.md"),
            // Same name one level down is not directly in `dir`.
            PathBuf::from("docs/adr/sub/gone.md"),
        ];
        let (_, findings) =
            type_dir_matches_nothing(&config, &path, &HashMap::new(), &|_| true, &tracked);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, FindingSeverity::Warning);
        assert_eq!(findings[0].file, path);
        assert!(findings[0].message.contains("gone.md"), "{findings:?}");
        assert!(findings[0].message.contains("'adr'"), "{findings:?}");
    }

    /// BUG-49, found live: `ADR-42` named `.urzua/config.toml` for two days
    /// after `ADR-52` deleted it, and nothing reported it.
    /// A declared type with no records may be exactly right, so the finding is
    /// an *absent* directory rather than an empty one.
    #[test]
    fn an_empty_declared_directory_is_not_a_finding() {
        let yaml = "schema_version: 2\nrecord_types:\n  present:\n    dir: \"docs/present\"\n  absent:\n    dir: \"docs/absent\"\n";
        let config = crate::config::parse(yaml).unwrap();
        let path = PathBuf::from(".urzua/config.yaml");

        // docs/present exists and is empty; docs/absent does not exist.
        let exists = |d: &str| d == "docs/present";
        let (exec, findings) =
            type_dir_matches_nothing(&config, &path, &HashMap::new(), &exists, &[]);

        assert_eq!(examined(&exec), 2, "both types are examined");
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("docs/absent"), "{findings:?}");
        assert!(!findings[0].message.contains("present"), "{findings:?}");
    }

    #[test]
    fn a_locator_naming_an_untracked_file_on_disk_is_its_own_variant_observed_failing() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nRealized-by: code:src/real.rs, code:src/drafted.rs, code:src/gone.rs\n---\n",
        );
        let present = |p: &str| p == "src/real.rs";
        let untracked_on_disk = |p: &str| p == "src/drafted.rs";
        let (_, findings) = embodiment_locator_exists(
            std::slice::from_ref(&r),
            &embodiment_config(),
            &[],
            &present,
            &crate::locator::no_file_text,
            &untracked_on_disk,
        );
        assert_eq!(findings.len(), 2, "{findings:?}");
        let drafted = findings
            .iter()
            .find(|f| f.message.contains("drafted"))
            .unwrap();
        assert_eq!(drafted.action_source.variant, "locator-untracked");
        assert!(
            drafted
                .message
                .contains("exists on disk but is not tracked by git"),
            "{}",
            drafted.message
        );
        let gone = findings
            .iter()
            .find(|f| f.message.contains("gone"))
            .unwrap();
        assert_eq!(gone.action_source.variant, "locator-missing");
        assert!(gone.message.contains("not in the working tree"));
    }

    #[test]
    fn realized_by_text_outside_a_recognised_entry_is_reported_observed_failing() {
        for value in [
            "src/a.rs",
            "Code:src/a.rs",
            "tests:src/a.rs",
            "src/a.rs, code:src/real.rs",
        ] {
            let r = record(
                "docs/adr/ADR-1-x.md",
                "adr",
                &format!("---\nRealized-by: {value}\n---\n"),
            );
            let (_, findings) = embodiment_locator_exists(
                std::slice::from_ref(&r),
                &embodiment_config(),
                &[],
                &|p: &str| p == "src/real.rs",
                &crate::locator::no_file_text,
                &|_| false,
            );
            assert_eq!(findings.len(), 1, "{value}: {findings:?}");
            assert_eq!(findings[0].action_source.variant, "locator-malformed");
            assert!(
                findings[0].message.contains("not a recognised entry"),
                "{value}"
            );
        }
    }

    #[test]
    fn a_locator_naming_a_missing_path_is_an_error_observed_failing() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nRealized-by: code:src/real.rs, code:src/gone.rs\n---\n",
        );
        let present = |p: &str| p == "src/real.rs";

        let (exec, findings) = embodiment_locator_exists(
            std::slice::from_ref(&r),
            &embodiment_config(),
            &[],
            &present,
            &crate::locator::no_file_text,
            &|_| false,
        );
        // One candidate per declared slot, whatever its locator count: the
        // population is the rule's input, not its work count.
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Error);
        assert!(findings[0].message.contains("src/gone.rs"), "{findings:?}");

        // Every locator present means silence, not a rule that cannot fire.
        let all_there = |_: &str| true;
        let (_, none) = embodiment_locator_exists(
            std::slice::from_ref(&r),
            &embodiment_config(),
            &[],
            &all_there,
            &crate::locator::no_file_text,
            &|_| false,
        );
        assert!(none.is_empty(), "{none:?}");

        // An empty locator names nothing, and the repo root exists -- so it
        // passed a raw existence check while still computing to `Implemented`.
        let empty = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "---\nRealized-by: 'code:'\n---\n",
        );
        let (_, findings) = embodiment_locator_exists(
            &[empty],
            &embodiment_config(),
            &[],
            &all_there,
            &crate::locator::no_file_text,
            &|_| false,
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].message.contains("empty locator"),
            "{findings:?}"
        );
    }

    /// Round 30 F3: `./src/b.rs` and `src/../src/b.rs` are the tracked file
    /// `src/b.rs`; the rule reported them "not in the working tree".
    #[test]
    fn a_dot_or_dotdot_spelled_locator_resolves_to_the_tracked_file_observed_failing() {
        let present = |p: &str| p == "src/b.rs";
        for spelled in ["./src/b.rs", "src/../src/b.rs", "src//b.rs", "src/./b.rs"] {
            let r = record(
                "docs/adr/ADR-1-x.md",
                "adr",
                &format!("---\nRealized-by: code:{spelled}\n---\n"),
            );
            let (_, findings) = embodiment_locator_exists(
                std::slice::from_ref(&r),
                &embodiment_config(),
                &[],
                &present,
                &crate::locator::no_file_text,
                &|_| false,
            );
            assert!(findings.is_empty(), "{spelled}: {findings:?}");
        }
        assert_eq!(normalize_locator("../x"), "../x");
        assert_eq!(normalize_locator("/src/b.rs"), "/src/b.rs");
        assert_eq!(normalize_locator("."), ".");
        assert_eq!(normalize_locator(" "), "");
    }

    #[test]
    fn a_type_declaring_a_custom_locator_field_name_is_read_by_that_name_observed_failing() {
        let config = config_with_types(vec![(
            "adr",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                relation_fields: Some(crate::config::RelationFields {
                    embodiment_locator: Some("Evidence".to_string()),
                    ..Default::default()
                }),
                ..type_config_pointer(&[], Some(&["Evidence"]), None, None)
            },
        )]);
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nEvidence: code:src/gone.rs\n---\n",
        );
        let present = |_: &str| false;

        let (exec, findings) = embodiment_locator_exists(
            std::slice::from_ref(&r),
            &config,
            &[],
            &present,
            &crate::locator::no_file_text,
            &|_| false,
        );
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("src/gone.rs"), "{findings:?}");
    }

    #[test]
    fn embodiment_disagreeing_with_realized_by_is_a_planted_violation_observed_failing() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nEmbodiment: Verified\nRealized-by: code:src/lib.rs\n---\n",
        );
        let (exec, findings) = embodiment_consistency(
            &[r],
            &embodiment_config(),
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
            &HashMap::new(),
        );
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Verified"));
        assert!(findings[0].message.contains("Implemented"));
    }

    #[test]
    fn embodiment_agreeing_with_realized_by_produces_no_finding() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nEmbodiment: Implemented\nRealized-by: code:src/lib.rs\n---\n",
        );
        let (_, findings) = embodiment_consistency(
            &[r],
            &embodiment_config(),
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
            &HashMap::new(),
        );
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn a_type_declaring_custom_embodiment_field_names_is_read_by_those_names_observed_failing() {
        let config = config_with_types(vec![(
            "adr",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                relation_fields: Some(crate::config::RelationFields {
                    embodiment_state: Some("Phase".to_string()),
                    embodiment_locator: Some("Evidence".to_string()),
                    ..Default::default()
                }),
                ..type_config_pointer(&[], Some(&["Phase", "Evidence"]), None, None)
            },
        )]);
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nPhase: Verified\nEvidence: code:src/lib.rs\n---\n",
        );
        let (exec, findings) = embodiment_consistency(
            &[r],
            &config,
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
            &HashMap::new(),
        );
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Phase"));
        assert!(findings[0].message.contains("Evidence"));
    }

    #[test]
    fn a_test_locator_outranks_a_code_locator() {
        let realized = parse_realized_by("code:src/lib.rs, test:tests/it.rs");
        assert_eq!(
            compute_embodiment(&EmbodimentDecl::default(), &realized, false, false, false),
            "Verified"
        );
    }

    #[test]
    fn a_drifted_record_computes_to_drift_detected_regardless_of_tier() {
        let realized = parse_realized_by("code:src/lib.rs, test:tests/it.rs");
        assert_eq!(
            compute_embodiment(&EmbodimentDecl::default(), &realized, true, false, false),
            "Drift detected"
        );
    }

    #[test]
    fn a_record_marked_drifted_produces_a_finding_even_when_verified_matches_its_tier() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nEmbodiment: Verified\nRealized-by: code:src/lib.rs, test:tests/it.rs\n---\n",
        );
        let mut drifted = HashSet::new();
        drifted.insert(r.path.clone());
        let (_, findings) = embodiment_consistency(
            &[r],
            &embodiment_config(),
            &drifted,
            &HashSet::new(),
            &Default::default(),
            &HashMap::new(),
        );
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Drift detected"));
    }

    #[test]
    fn a_record_missing_half_the_pair_is_handed_to_the_rule_and_not_judged() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nEmbodiment: Not started\n---\n",
        );
        let (exec, findings) = embodiment_consistency(
            &[r],
            &embodiment_config(),
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
            &HashMap::new(),
        );
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(
            (population.eligible(), population.examined()),
            (1, 0),
            "the type declares both fields, so the record is a candidate that reached no verdict"
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn realized_by_locator_paths_flattens_all_three_categories() {
        let paths =
            realized_by_locator_paths("spec:docs/rfc/0001-x.md, code:src/lib.rs, test:tests/it.rs");
        assert_eq!(
            paths,
            vec!["docs/rfc/0001-x.md", "src/lib.rs", "tests/it.rs"]
        );
    }

    #[test]
    fn a_locator_cited_by_two_records_is_a_promotion_candidate_observed_failing() {
        let a = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nRealized-by: code:src/shared.rs\n---\n",
        );
        let b = record(
            "docs/adr/0002-y.md",
            "adr",
            "---\nRealized-by: code:src/shared.rs\n---\n",
        );
        let (exec, findings) =
            embodiment_locator_promotion_candidate(&[a, b], &embodiment_config(), None);
        assert_eq!(examined(&exec), 2);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("src/shared.rs"));
    }

    #[test]
    fn one_record_citing_the_same_locator_under_two_categories_is_not_a_promotion_candidate() {
        // A single record's own code: and test: locators both naming the
        // same file is one record, not two independent citers -- promotion
        // exists for cross-record duplication, never cross-category
        // duplication within a single record's own Realized-by.
        let a = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nRealized-by: code:src/shared.rs, test:src/shared.rs\n---\n",
        );
        let (_, findings) =
            embodiment_locator_promotion_candidate(&[a], &embodiment_config(), None);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn a_locator_cited_by_only_one_record_is_not_a_promotion_candidate() {
        let a = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nRealized-by: code:src/a.rs\n---\n",
        );
        let (_, findings) =
            embodiment_locator_promotion_candidate(&[a], &embodiment_config(), None);
        assert!(findings.is_empty());
    }

    fn back_pointer_record(embodiment: &str, realized_by: &str) -> Record {
        record(
            "docs/adr/0001-x.md",
            "adr",
            &format!("---\nEmbodiment: '{embodiment}'\nRealized-by: '{realized_by}'\n---\n"),
        )
    }

    fn sticky_config(states: &str) -> Config {
        let mut config = embodiment_config();
        let setting: crate::config::RuleSetting = yaml_serde::from_str(&format!(
            "{{level: warn, claim_max_age_days: 10, sticky_states: {states}}}"
        ))
        .unwrap();
        config
            .rules
            .insert(RULE_EMBODIMENT_CONSISTENCY.to_string(), setting);
        config
    }

    fn sticky_findings(config: &Config, records: &[Record], age: u64) -> (usize, Vec<Finding>) {
        let ages: HashMap<PathBuf, u64> = records.iter().map(|r| (r.path.clone(), age)).collect();
        let (exec, findings) = embodiment_consistency(
            records,
            config,
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
            &ages,
        );
        (exec.population.unwrap().examined(), findings)
    }

    #[test]
    fn a_declared_sticky_state_is_not_drift_and_a_neighbour_still_is_observed_failing() {
        let mut deprecated = back_pointer_record("Deprecated", "code:src/x.rs");
        deprecated.path = PathBuf::from("docs/adr/0001-a.md");
        let mut inactive = back_pointer_record("Inactive", "spec:docs/x.md");
        inactive.path = PathBuf::from("docs/adr/0002-b.md");
        let mut neighbour = back_pointer_record("Verified", "spec:docs/x.md");
        neighbour.path = PathBuf::from("docs/adr/0003-c.md");
        let records = [deprecated, inactive, neighbour];
        let (examined, findings) =
            sticky_findings(&sticky_config("[Deprecated, Inactive]"), &records, 5);
        assert_eq!(examined, 3, "a sticky record is examined, not skipped");
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].file.ends_with("0003-c.md"), "{findings:?}");
    }

    #[test]
    fn with_no_sticky_states_declared_deprecated_is_drift_as_before() {
        let records = [back_pointer_record("Deprecated", "code:src/x.rs")];
        let (_, findings) = sticky_findings(&embodiment_config(), &records, 5);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn only_the_declared_states_are_sticky() {
        let records = [back_pointer_record("Inactive", "code:src/x.rs")];
        let (_, findings) = sticky_findings(&sticky_config("[Deprecated]"), &records, 5);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_sticky_state_is_not_a_verification_claim_so_it_draws_no_age_warning() {
        let records = [back_pointer_record("Deprecated", "code:src/x.rs")];
        let (_, findings) = sticky_findings(&sticky_config("[Deprecated]"), &records, 400);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_sticky_state_with_no_realized_by_is_examined_and_clean() {
        let (examined, findings) = sticky_findings(
            &sticky_config("[Deprecated]"),
            &[no_locators("Deprecated")],
            5,
        );
        assert_eq!((examined, findings.len()), (1, 0));
    }

    fn consistency(r: Record, code: bool, test: bool) -> Vec<Finding> {
        let evidence = BackPointerEvidence {
            code: if code {
                HashSet::from([r.path.clone()])
            } else {
                HashSet::new()
            },
            test: if test {
                HashSet::from([r.path.clone()])
            } else {
                HashSet::new()
            },
            ..Default::default()
        };
        let (_, findings) = embodiment_consistency(
            &[r],
            &embodiment_config(),
            &HashSet::new(),
            &HashSet::new(),
            &evidence,
            &HashMap::new(),
        );
        findings
    }

    fn no_locators(embodiment: &str) -> Record {
        record(
            "docs/adr/0001-x.md",
            "adr",
            &format!("---\nEmbodiment: '{embodiment}'\n---\n"),
        )
    }

    #[test]
    fn a_test_back_pointer_computes_verified_observed_failing() {
        let r = back_pointer_record("Verified", "code:src/x.rs");
        assert!(consistency(r, true, true).is_empty());
        let r = back_pointer_record("Implemented", "code:src/x.rs");
        let findings = consistency(r, true, true);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("'Verified'"));
        assert!(findings[0].message.contains("test back-pointers"));
    }

    #[test]
    fn a_code_only_back_pointer_does_not_reach_verified() {
        let r = back_pointer_record("Verified", "code:src/x.rs");
        assert_eq!(consistency(r, true, false).len(), 1);
    }

    #[test]
    fn a_record_with_no_realized_by_is_judged_when_cited_observed_failing() {
        let findings = consistency(no_locators("Not started"), true, false);
        assert_eq!(findings.len(), 1, "{findings:?}");
        let m = &findings[0].message;
        assert!(m.contains("'Implemented'"), "{m}");
        assert!(m.contains("source back-pointers"), "{m}");
        assert!(m.contains("no Realized-by field"), "{m}");
        assert!(consistency(no_locators("Implemented"), true, false).is_empty());
        assert!(consistency(no_locators("Verified"), false, true).is_empty());
    }

    #[test]
    fn a_tier_claim_with_no_evidence_of_either_kind_is_reported() {
        for tier in ["Verified", "Implemented"] {
            let findings = consistency(no_locators(tier), false, false);
            assert_eq!(findings.len(), 1, "{tier}: {findings:?}");
            let m = &findings[0].message;
            assert!(m.contains(&format!("stated Embodiment '{tier}'")), "{m}");
            assert!(m.contains("'Not started'"), "{m}");
            assert!(m.contains("no Realized-by field"), "{m}");
            assert!(m.contains("no back-pointer cites it"), "{m}");
        }
    }

    #[test]
    fn a_record_stating_no_tier_with_no_evidence_stays_unjudged_and_unexamined() {
        for stated in ["Not started", "Specified", "Deprecated"] {
            let r = no_locators(stated);
            assert!(consistency(r.clone(), false, false).is_empty(), "{stated}");
            let (exec, _) = embodiment_consistency(
                &[r],
                &embodiment_config(),
                &HashSet::new(),
                &HashSet::new(),
                &Default::default(),
                &HashMap::new(),
            );
            assert_eq!(exec.population.unwrap().examined(), 0, "{stated}");
        }
    }

    #[test]
    fn a_tier_claim_with_no_evidence_is_examined_in_the_census() {
        let r = no_locators("Verified");
        let (exec, _) = embodiment_consistency(
            &[r],
            &embodiment_config(),
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
            &HashMap::new(),
        );
        assert_eq!(exec.population.unwrap().examined(), 1);
    }

    #[test]
    fn a_finding_that_used_no_back_pointer_names_only_realized_by() {
        let findings = consistency(
            back_pointer_record("Verified", "spec:docs/x.md"),
            false,
            false,
        );
        assert!(
            findings[0].message.ends_with("computed from Realized-by"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_back_pointer_lifts_a_specified_record_to_implemented_observed_failing() {
        let r = back_pointer_record("Implemented", "spec:docs/x.md");
        let cited = HashSet::from([r.path.clone()]);
        let (_, findings) = embodiment_consistency(
            &[r],
            &embodiment_config(),
            &HashSet::new(),
            &HashSet::new(),
            &BackPointerEvidence {
                code: cited.clone(),
                ..Default::default()
            },
            &HashMap::new(),
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn without_a_back_pointer_the_same_record_still_computes_specified() {
        let r = back_pointer_record("Implemented", "spec:docs/x.md");
        let (_, findings) = embodiment_consistency(
            &[r],
            &embodiment_config(),
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
            &HashMap::new(),
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Specified"));
    }

    #[test]
    fn a_back_pointer_never_outranks_a_test_locator_or_a_drift() {
        let none = RealizedBy::default();
        assert_eq!(
            compute_embodiment(&EmbodimentDecl::default(), &none, false, true, false),
            "Implemented"
        );
        let verified = RealizedBy {
            test: vec!["t.rs".into()],
            ..Default::default()
        };
        assert_eq!(
            compute_embodiment(&EmbodimentDecl::default(), &verified, false, true, false),
            "Verified"
        );
        assert_eq!(
            compute_embodiment(&EmbodimentDecl::default(), &none, true, true, false),
            "Drift detected"
        );
    }

    #[test]
    fn fresh_drift_accepts_either_the_tier_or_drift_detected_observed_failing() {
        let fresh: HashSet<PathBuf> = [PathBuf::from("docs/adr/ADR-1-x.md")].into();
        let run = |stated: &str, fresh: &HashSet<PathBuf>| {
            let r = record(
                "docs/adr/ADR-1-x.md",
                "adr",
                &format!("---\nEmbodiment: '{stated}'\nRealized-by: code:src/lib.rs\n---\n"),
            );
            embodiment_consistency(
                &[r],
                &embodiment_config(),
                &HashSet::new(),
                fresh,
                &Default::default(),
                &HashMap::new(),
            )
            .1
        };
        assert!(run("Drift detected", &fresh).is_empty());
        assert!(run("Implemented", &fresh).is_empty());
        assert_eq!(run("Verified", &fresh).len(), 1);
        // Without the fresh marker a stated `Drift detected` still disagrees.
        assert_eq!(run("Drift detected", &HashSet::new()).len(), 1);
    }

    // ---- embodiment.requires-accepted-status (MILE-138)

    fn accepted_config(statuses: Option<&str>, option: Option<&str>) -> Config {
        let mut t = type_config_pointer(&[], Some(&["Status", "Embodiment"]), None, None);
        t.statuses = statuses.map(|y| yaml_serde::from_str(y).unwrap());
        let mut config = config_with_types(vec![("adr", t)]);
        let opt = option
            .map(|o| format!(", accepted_statuses: {o}"))
            .unwrap_or_default();
        let setting: crate::config::RuleSetting =
            yaml_serde::from_str(&format!("{{level: warn{opt}}}")).unwrap();
        config.rules.insert(
            RULE_EMBODIMENT_REQUIRES_ACCEPTED_STATUS.to_string(),
            setting,
        );
        config
    }

    const DECLARED: &str =
        "{values: [Proposed, Accepted, Rejected], success: [Accepted], failure: [Rejected]}";

    fn adr(status: &str, embodiment: &str) -> Record {
        record(
            "docs/adr/ADR-1-x.md",
            "adr",
            &format!("---\nStatus: '{status}'\nEmbodiment: '{embodiment}'\n---\n"),
        )
    }

    #[test]
    fn a_proposed_record_stating_implemented_or_verified_is_a_finding_observed_failing() {
        for stated in ["Implemented", "Verified"] {
            let (exec, findings) = embodiment_requires_accepted_status(
                &[adr("Proposed", stated)],
                &accepted_config(Some(DECLARED), None),
            );
            assert_eq!(examined(&exec), 1);
            assert_eq!(findings.len(), 1, "{stated}: {findings:?}");
            assert!(findings[0].message.contains("Proposed"));
            assert!(findings[0].message.contains(stated));
        }
    }

    #[test]
    fn a_claim_retained_status_keeps_its_claim_observed_failing() {
        let retained = "{values: [Proposed, Accepted, Superseded], success: [Accepted], claim_retained: [Superseded]}";
        let (exec, findings) = embodiment_requires_accepted_status(
            &[
                adr("Superseded", "Implemented"),
                adr("Proposed", "Implemented"),
            ],
            &accepted_config(Some(retained), None),
        );
        assert_eq!(examined(&exec), 2);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Proposed"));
        // Without the list the Superseded record is a finding.
        let (_, findings) = embodiment_requires_accepted_status(
            &[adr("Superseded", "Implemented")],
            &accepted_config(
                Some("{values: [Proposed, Accepted, Superseded], success: [Accepted]}"),
                None,
            ),
        );
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn the_same_record_accepted_is_not_a_finding() {
        let (exec, findings) = embodiment_requires_accepted_status(
            &[adr("Accepted", "Verified")],
            &accepted_config(Some(DECLARED), None),
        );
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn specified_and_inactive_on_a_proposed_record_are_not_flagged() {
        for stated in ["Specified", "Inactive", "Not started"] {
            let (exec, findings) = embodiment_requires_accepted_status(
                &[adr("Proposed", stated)],
                &accepted_config(Some(DECLARED), None),
            );
            assert_eq!(examined(&exec), 1);
            assert!(findings.is_empty(), "{stated}: {findings:?}");
        }
    }

    #[test]
    fn a_type_declaring_no_success_uses_the_option_and_without_it_is_not_examined() {
        let r = adr("Proposed", "Implemented");
        let with = accepted_config(None, Some("[Accepted]"));
        let (exec, findings) = embodiment_requires_accepted_status(&[r.clone()], &with);
        assert_eq!((examined(&exec), findings.len()), (1, 1));
        let ok = adr("Accepted", "Implemented");
        assert!(embodiment_requires_accepted_status(&[ok], &with)
            .1
            .is_empty());

        let (exec, findings) =
            embodiment_requires_accepted_status(&[r], &accepted_config(None, None));
        assert_eq!(examined(&exec), 0, "no accepted status is known");
        assert_eq!(exec.population.as_ref().unwrap().eligible(), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_declared_success_overrides_the_option() {
        let r = adr("Accepted", "Verified");
        let (_, findings) = embodiment_requires_accepted_status(
            &[r],
            &accepted_config(Some(DECLARED), Some("[Done]")),
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_malformed_last_audit_date_is_reported_and_a_good_or_absent_one_is_not() {
        let mut config = embodiment_config();
        let setting: crate::config::RuleSetting =
            yaml_serde::from_str("{level: warn, drift_grace_days: 7, drift_audit_field: Audited}")
                .unwrap();
        config
            .rules
            .insert(RULE_EMBODIMENT_CONSISTENCY.to_string(), setting);
        let with = |audited: &str| {
            record(
                "docs/adr/0001-x.md",
                "adr",
                &format!(
                    "---\nEmbodiment: 'Specified'\nRealized-by: 'spec:docs/x.md'\n{audited}---\n"
                ),
            )
        };
        let run = |r: Record| {
            embodiment_consistency(
                &[r],
                &config,
                &HashSet::new(),
                &HashSet::new(),
                &Default::default(),
                &HashMap::new(),
            )
            .1
        };
        let bad = run(with("Audited: '2026-13-40'\n"));
        assert_eq!(bad.len(), 1, "{bad:?}");
        assert!(bad[0].message.contains("Audited") && bad[0].message.contains("2026-13-40"));
        assert!(run(with("Audited: '2026-11-04'\n")).is_empty());
        assert!(run(with("")).is_empty());
    }
}
