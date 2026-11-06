use super::*;

/// A `dir` selects that directory and not its subtree (RFC-35). A
/// record-shaped file one level below a declared `dir` is therefore owned by no
/// type and drops out of the corpus entirely -- `check` reports a smaller
/// `files_examined` and says nothing, so a configuration written before RFC-35
/// loses coverage on upgrade with no diagnostic (BUG-62).
///
/// `candidates` is every tracked path the caller considered; `claimed` is the
/// subset some type took.
pub fn type_record_outside_declared_dir(
    config: &Config,
    candidates: &[std::path::PathBuf],
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR;
    let mut findings = Vec::new();

    let dirs: Vec<std::path::PathBuf> = config
        .record_types
        .values()
        .flat_map(|t| t.all_dirs())
        .map(std::path::PathBuf::from)
        .collect();

    // Eligible: every tracked, record-shaped path below a declared dir. A
    // markdown file elsewhere in the repo is a document, not an ungoverned
    // record, and saying otherwise would flag every README.
    let eligible_paths: Vec<&std::path::PathBuf> = candidates
        .iter()
        .filter(|path| {
            let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                return false;
            };
            if !crate::record::is_governed_record_filename(file_name) {
                return false;
            }
            dirs.iter().any(|d| path.starts_with(d))
        })
        .collect();

    let mut unowned: Vec<&std::path::PathBuf> = Vec::new();
    let population = census(PopulationUnit::Path, eligible_paths, |path| {
        // Ownership is decided by the path, which is what RFC-35 defines. It
        // was decided by the loaded record set, which also excludes a file the
        // loader could not read -- so a staged deletion directly inside a
        // declared dir was reported as sitting outside it (BUG-71).
        if !dirs.iter().any(|d| path.parent() == Some(d.as_path())) {
            unowned.push(path);
        }
        Outcome::Examined
    });

    unowned.sort();
    for path in unowned {
        findings.push(Finding {
            rule: RULE_ID.to_string(),
            severity: FindingSeverity::Warning,
            file: path.clone(),
            line: None,
            message: "sits below a declared record type's dir but not directly in it, so no type \
                      owns it and no rule examines it"
                .to_string(),
            waived: None,
            declared_severity: None,
            suggested_action: None,
            action_source: ActionSource::new("record-outside-declared-dir"),
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

/// Rule (MILE-0077 follow-up, ADR-0043): unlike every other rule, this one
/// examines `Config` directly rather than `&[Record]` -- it's an inventory
/// of the *schema* itself, not a per-record check. "Does this type need a
/// spec" stays editorial (ADR-0041 rejected a mechanical formula for that);
/// this only makes visible which configured types currently have no `spec`
/// pointer declared at all, the same "declared, not voted" shape as
/// `known_fields`. A type with none declared is a real,
/// permanently valid state under ADR-0041 -- this is a signal to review,
/// never a mandate to write one.
/// A declared record type whose `dir` matches no record.
///
/// `RFC-35` defines `dir` as *that directory* rather than that subtree, which
/// is unambiguous but unforgiving: a `dir` one level off matches nothing at
/// all. Alone that degrades to `not-run`, which is visible; mixed with any
/// working type it is silent -- `check` reports `ok` and exit 0 over a corpus
/// it never examined. `claim_paths` has a load-time guard for the same hazard
/// (`BUG-56`); `dir` has none.
///
/// Also reports an `exclude_files` entry that names no tracked file directly in
/// the type's `dir` (`MILE-150`): a stale exclusion is the same hazard as a
/// misdeclared `dir`, a declaration that silently does nothing.
///
/// `matched` and `tracked` (every live tracked path, excluded files included)
/// are supplied by the caller, which is where discovery happens.
pub fn type_dir_matches_nothing(
    config: &Config,
    config_path: &std::path::Path,
    matched: &HashMap<String, usize>,
    dir_exists: &dyn Fn(&str) -> bool,
    tracked: &[std::path::PathBuf],
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_TYPE_DIR_MATCHES_NOTHING;
    let mut findings = Vec::new();

    let names = config.sorted_type_names();

    let population = census(PopulationUnit::RecordType, names, |name| {
        let type_config = &config.record_types[*name];
        for entry in &type_config.exclude_files {
            let present = tracked.iter().any(|p| {
                type_config.owns_dir(p.parent())
                    && p.file_name().is_some_and(|n| n == entry.as_str())
            });
            if !present {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Warning,
                    file: config_path.to_path_buf(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new("exclude-files-matches-nothing").key_path(["record_types", name, "exclude_files"]),
                    message: format!(
                        "record type '{name}' declares exclude_files entry '{entry}', but no such file is \
                         directly in {} -- the exclusion does nothing; remove it or correct the name",
                        type_config
                            .all_dirs()
                            .map(|d| format!("'{d}'"))
                            .collect::<Vec<_>>()
                            .join(" or ")
                    ),
                });
            }
        }
        // A declared extra directory that is not there is a declaration that
        // silently does nothing, even when `dir` still holds records
        // (`MILE-130`).
        for dir in &type_config.extra_dirs {
            if !dir_exists(dir) {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Warning,
                    file: config_path.to_path_buf(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new("extra-dir-missing").key_path(["record_types", name, "extra_dirs"]),
                    message: format!(
                        "record type '{name}' declares extra_dirs entry '{dir}', which does not exist -- \
                         its records are not examined and its ids are not counted in the type's namespace"
                    ),
                });
            }
        }
        if matched.get(*name).copied().unwrap_or(0) == 0 {
            let dir = &config.record_types[*name].dir;
            // A directory that exists and is empty is a type declared and not
            // yet used, which is a legitimate state -- some types are expected
            // to hold nothing most of the time. An *absent* directory is the
            // misdeclaration this rule exists for. Git does not track empty
            // directories, so declaring a type means committing a placeholder
            // alongside it.
            if !dir_exists(dir) {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Warning,
                    file: config_path.to_path_buf(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new("dir-missing").key_path([
                        "record_types",
                        name,
                        "dir",
                    ]),
                    message: format!(
                        "record type '{name}' declares dir '{dir}', which does not exist -- \
                         every rule for this type examines nothing"
                    ),
                });
            }
        }
        Outcome::Examined
    });

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

/// A config-schema inventory rule: examines `Config::record_types` itself,
/// one type at a time, never a record. Shared by every rule below that
/// reports a contradiction or omission in the schema's own declarations --
/// several of these functions' own doc comments already named each other as
/// "the same shape," which is what this extracts. `type_dir_matches_nothing`
/// doesn't fit: it needs `matched`/`dir_exists`, data this shape has no room
/// for.
pub(super) fn schema_inventory_rule(
    config: &Config,
    rule_id: &'static str,
    mut check: impl FnMut(&str, &crate::config::RecordTypeConfig, &mut Vec<Finding>),
) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();
    let type_names = config.sorted_type_names();
    let population = census(PopulationUnit::RecordType, type_names, |type_name| {
        check(type_name, &config.record_types[*type_name], &mut findings);
        Outcome::Examined
    });
    (
        RuleExecution {
            rule: rule_id.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

pub fn type_no_declared_spec(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    schema_inventory_rule(
        config,
        RULE_TYPE_NO_DECLARED_SPEC,
        |type_name, type_config, findings| {
            if type_config.spec.is_none() {
                findings.push(Finding {
                    rule: RULE_TYPE_NO_DECLARED_SPEC.to_string(),
                    severity: FindingSeverity::Warning,
                    file: config_path.to_path_buf(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new("no-declared-spec"),
                    message: format!(
                        "record type '{type_name}' has no declared spec -- add `spec = \"SPEC-N\"` once one exists, or leave undeclared if ADR-41's editorial judgment says one isn't warranted"
                    ),
                });
            }
        },
    )
}

/// A declared rule whose candidates the configuration could not reach
/// (`MILE-106`).
///
/// Fires on `out_of_scope > 0`, never on a merely empty result. Those are
/// different problems with opposite fixes: a corpus that has not written a
/// revision log yet is doing nothing wrong and time resolves it, while a
/// `prefix` that matches no filename will examine zero records forever
/// (`BUG-61`). `type.dir-matches-nothing` draws the same line between an
/// absent directory and an empty one, and is the precedent this follows.
///
/// Deliberately silent on `Unreadable`: `header.required-fields` already
/// reports the parse error, per record, with the parser's own message.
///
/// Judges the run rather than the corpus, so it reads the executions the other
/// rules produced and never sees a record. It excludes itself -- judging its
/// own execution while producing it is not a verdict anyone can act on.
pub fn config_scope_matches_nothing(
    executed: &[RuleExecution],
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_CONFIG_SCOPE_MATCHES_NOTHING;
    let mut findings = Vec::new();

    let candidates: Vec<&RuleExecution> = executed
        .iter()
        .filter(|e| e.rule != RULE_ID && e.status == RuleStatus::Ran)
        .collect();

    let population = census(PopulationUnit::Rule, candidates, |exec| {
        let Some(population) = &exec.population else {
            return Outcome::Absent;
        };
        if population.out_of_scope() == 0 {
            return Outcome::Examined;
        }
        findings.push(Finding {
            rule: RULE_ID.to_string(),
            severity: FindingSeverity::Warning,
            file: config_path.to_path_buf(),
            line: None,
            waived: None,
            declared_severity: None,
            suggested_action: None,
            action_source: ActionSource::new("scope-matches-nothing")
                .key_path(["rules", exec.rule.as_str()]),
            message: format!(
                "'{}' was handed {} candidate(s) the configuration does not reach -- \
                 {} of {} are outside what this repository declared, so the rule cannot \
                 apply to them however the records are edited",
                exec.rule,
                population.out_of_scope(),
                population.out_of_scope(),
                population.eligible()
            ),
        });
        Outcome::Examined
    });

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

    #[test]
    fn a_type_with_no_declared_spec_is_a_finding_observed_failing() {
        // The exact live case this rule was built to catch: before this
        // session declared any `spec` pointer, every configured type
        // (including milestone/bug/waiver, which already had SPEC-6/9/10)
        // had none wired into config at all.
        let mut record_types = std::collections::BTreeMap::new();
        record_types.insert("milestone".to_string(), type_config(None));
        let config = Config {
            schema_version: 2,
            roadmap: Default::default(),
            relation_pairs: Vec::new(),
            fail_on: Default::default(),
            embodiment: Default::default(),
            locator_resolvers: Default::default(),
            upgraded_from: None,
            override_log: None,
            block_on: None,
            attestation: Default::default(),
            harness: None,
            authority: None,
            rules: std::collections::BTreeMap::new(),
            record_types,
        };

        let (exec, findings) =
            type_no_declared_spec(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("milestone"));
    }

    #[test]
    fn a_declared_spec_pointer_produces_no_finding() {
        let mut record_types = std::collections::BTreeMap::new();
        record_types.insert("milestone".to_string(), type_config(Some("SPEC-6")));
        let config = Config {
            schema_version: 2,
            roadmap: Default::default(),
            relation_pairs: Vec::new(),
            fail_on: Default::default(),
            embodiment: Default::default(),
            locator_resolvers: Default::default(),
            upgraded_from: None,
            override_log: None,
            block_on: None,
            attestation: Default::default(),
            harness: None,
            authority: None,
            rules: std::collections::BTreeMap::new(),
            record_types,
        };

        let (exec, findings) =
            type_no_declared_spec(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_scope_reaching_nothing_is_reported_observed_failing() {
        // BUG-61's shape: init wrote a prefix matching no filename, so the rule
        // examines zero records forever and no edit to a record changes that.
        let executed = [exec_with("identity.collision", 2, 0, 2)];
        let (exec, findings) =
            config_scope_matches_nothing(&executed, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1, "the rule must judge the execution");
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("identity.collision"));
    }

    #[test]
    fn the_finding_names_the_configuration_the_caller_selected() {
        // check's scoped filter keeps a config finding only when its path equals
        // the selected config, so a hardcoded path makes this finding vanish
        // under `--config` -- a finding that disappears rather than one that is
        // wrong.
        let executed = [exec_with("identity.collision", 2, 0, 2)];
        let chosen = std::path::Path::new("custom/urzua.yaml");
        let (_, findings) = config_scope_matches_nothing(&executed, chosen);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, chosen);
    }

    #[test]
    fn a_rule_with_nothing_written_yet_is_not_reported() {
        // Identical eligible/examined to the case above. The corpus simply has
        // no revision logs, which time resolves and config cannot.
        let executed = [exec_with("revision-log.change-class-required", 2, 0, 0)];
        let (exec, findings) =
            config_scope_matches_nothing(&executed, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(
            findings.is_empty(),
            "an empty corpus is not a misconfiguration: {findings:?}"
        );
    }

    #[test]
    fn it_does_not_judge_its_own_execution() {
        let executed = [exec_with(RULE_CONFIG_SCOPE_MATCHES_NOTHING, 3, 0, 3)];
        let (exec, findings) =
            config_scope_matches_nothing(&executed, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(
            exec.population.expect("carries a population").eligible(),
            0,
            "its own execution is not a candidate"
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_rule_that_did_not_run_is_not_a_candidate() {
        let executed = [RuleExecution {
            rule: "field.quality".to_string(),
            population: None,
            status: RuleStatus::NotEnabled,
            examined_records: Vec::new(),
        }];
        let (exec, findings) =
            config_scope_matches_nothing(&executed, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(exec.population.expect("carries a population").eligible(), 0);
        assert!(findings.is_empty());
    }
}
