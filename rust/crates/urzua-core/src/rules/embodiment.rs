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

const PIN_MARKER: &str = "@sha256:";

/// Splits an optional content-hash pin off a locator (`MILE-129`):
/// `src/a.rs@sha256:<64 lowercase hex>` is the path `src/a.rs` pinned to that
/// digest. Anything else after the marker -- wrong length, uppercase, non-hex
/// -- is not a pin and stays part of the path, so a malformed pin surfaces as
/// a locator naming nothing (`embodiment.locator-exists`), never as a silently
/// unpinned one.
fn split_pin(locator: &str) -> (&str, Option<&str>) {
    let locator = locator.trim();
    match locator.rsplit_once(PIN_MARKER) {
        Some((path, hash))
            if hash.len() == 64 && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) =>
        {
            (path, Some(hash))
        }
        _ => (locator, None),
    }
}

/// Every locator with its optional pinned digest (`MILE-129`), in the order
/// written. The path is normalised exactly as [`parse_realized_by`] does.
pub fn realized_by_pinned_locators(value: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    for segment in value.split(',') {
        let Some((prefix, locator)) = segment.trim().split_once(':') else {
            continue;
        };
        if !matches!(prefix.trim(), "spec" | "code" | "test") {
            continue;
        }
        let (path, pin) = split_pin(locator);
        out.push((normalize_locator(path), pin.map(str::to_string)));
    }
    out
}

pub(crate) fn parse_realized_by(value: &str) -> RealizedBy {
    let mut result = RealizedBy::default();
    for segment in value.split(',') {
        let segment = segment.trim();
        let Some((prefix, locator)) = segment.split_once(':') else {
            continue;
        };
        let locator = normalize_locator(split_pin(locator).0);
        match prefix.trim() {
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

/// A `Realized-by` locator naming a path that is not in the working tree.
///
/// `Embodiment` is computed from these paths, so a locator naming nothing lets
/// a record claim verified work against a file that is not there -- and the
/// claim reads as stronger than `Not started`, not weaker.
///
/// `present` is supplied by the caller: this crate does not touch the
/// filesystem.
pub fn embodiment_locator_exists(
    records: &[Record],
    config: &Config,
    present: &dyn Fn(&str) -> bool,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_EMBODIMENT_LOCATOR_EXISTS;
    let mut findings = Vec::new();

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
            let value = match declared_value(record, field) {
                Ok(v) => v,
                Err(outcome) => return outcome,
            };
            let realized = parse_realized_by(value);
            for locator in realized
                .spec
                .iter()
                .chain(&realized.code)
                .chain(&realized.test)
            {
                // An empty locator joins to the repository root, which exists --
                // so it would pass while still computing to `Implemented`.
                if locator.trim().is_empty() {
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Error,
                        file: record.path.clone(),
                        line: None,
                        waived: None,
                        declared_severity: None,
                        message: format!("{field} has an empty locator"),
                    });
                    continue;
                }
                if present(locator) {
                    continue;
                }
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    message: format!("{field} names '{locator}', which is not in the working tree"),
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
            let realized_by_value = match declared_value(record, locator_field) {
                Ok(v) => Some(v),
                Err(Outcome::Absent) if cited_by_code || cited_by_test || claims_a_tier => None,
                Err(outcome) => return outcome,
            };

            let computed = compute_embodiment(
                &config.embodiment,
                &realized_by_value.map(parse_realized_by).unwrap_or_default(),
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
pub fn embodiment_locator_promotion_candidate(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE;
    let mut findings = Vec::new();

    let mut citers: std::collections::BTreeMap<
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
            let realized_by_value = match declared_value(record, field) {
                Ok(v) => v,
                Err(outcome) => return outcome,
            };

            let parsed = parse_realized_by(realized_by_value);
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
            }
            Outcome::Examined
        },
    );

    for (locator, paths) in citers {
        if paths.len() < 2 {
            continue;
        }
        let names: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
        let first_path = paths.iter().next().expect("checked len >= 2 above").clone();
        findings.push(Finding {
            rule: RULE_ID.to_string(),
            severity: FindingSeverity::Warning,
            file: first_path,
            line: None,
            waived: None,
            declared_severity: None,
            message: format!(
                "locator '{locator}' is cited by {} records ({}) -- consider promoting to a shared claim record",
                paths.len(),
                names.join(", ")
            ),
        });
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
    fn a_sha256_pin_is_split_off_the_locator_and_malformed_pins_are_not() {
        let h = "a".repeat(64);
        let value = format!("code:./src/a.rs@sha256:{h}, test:t.rs, spec:s.md@sha256:ABC");
        assert_eq!(
            realized_by_pinned_locators(&value),
            vec![
                ("src/a.rs".to_string(), Some(h.clone())),
                ("t.rs".to_string(), None),
                ("s.md@sha256:ABC".to_string(), None),
            ]
        );
        // Every other reader sees the bare path.
        assert_eq!(parse_realized_by(&value).code, vec!["src/a.rs".to_string()]);
        assert!(realized_by_locator_paths(&value).contains(&"src/a.rs".to_string()));
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
    fn a_locator_naming_a_missing_path_is_an_error_observed_failing() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nRealized-by: code:src/real.rs, code:src/gone.rs\n---\n",
        );
        let present = |p: &str| p == "src/real.rs";

        let (exec, findings) =
            embodiment_locator_exists(std::slice::from_ref(&r), &embodiment_config(), &present);
        // One candidate per declared slot, whatever its locator count: the
        // population is the rule's input, not its work count.
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Error);
        assert!(findings[0].message.contains("src/gone.rs"), "{findings:?}");

        // Every locator present means silence, not a rule that cannot fire.
        let all_there = |_: &str| true;
        let (_, none) =
            embodiment_locator_exists(std::slice::from_ref(&r), &embodiment_config(), &all_there);
        assert!(none.is_empty(), "{none:?}");

        // An empty locator names nothing, and the repo root exists -- so it
        // passed a raw existence check while still computing to `Implemented`.
        let empty = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "---\nRealized-by: 'code:'\n---\n",
        );
        let (_, findings) = embodiment_locator_exists(&[empty], &embodiment_config(), &all_there);
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
            let (_, findings) =
                embodiment_locator_exists(std::slice::from_ref(&r), &embodiment_config(), &present);
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

        let (exec, findings) =
            embodiment_locator_exists(std::slice::from_ref(&r), &config, &present);
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
            embodiment_locator_promotion_candidate(&[a, b], &embodiment_config());
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
        let (_, findings) = embodiment_locator_promotion_candidate(&[a], &embodiment_config());
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn a_locator_cited_by_only_one_record_is_not_a_promotion_candidate() {
        let a = record(
            "docs/adr/0001-x.md",
            "adr",
            "---\nRealized-by: code:src/a.rs\n---\n",
        );
        let (_, findings) = embodiment_locator_promotion_candidate(&[a], &embodiment_config());
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
