use super::*;
use crate::waiver::Suppression;
use std::path::Path;

/// Source files a back-pointer rule scans, with the `back_pointer_paths` that
/// select them and the declared test class (a subset of those paths).
#[derive(Default)]
pub struct BackPointerSources {
    pub files: Vec<PublishedFile>,
    pub back_pointer_paths: Vec<String>,
    pub test_paths: Vec<String>,
    /// `Some` on a path-scoped run (`MILE-207`): what it left unread, and the
    /// scope it reads for. `None` reads every file, as a whole-corpus run does.
    pub scoped: Option<ScopedRead>,
}

/// What a path-scoped run reads of the source files and for whom (`MILE-207`).
/// A file is read when it lies under a requested path (the run reports on it),
/// or when a comment in it cites a record the run reports on (evidence and
/// decision findings about that record). Every other tracked file under the
/// declared paths is `unread`: counted in the rule's population, never dropped.
#[derive(Debug, Clone, Default)]
pub struct ScopedRead {
    /// The requested path prefixes, relative to the repository root.
    pub paths: Vec<PathBuf>,
    /// The records the run reports on.
    pub records: HashSet<PathBuf>,
    /// Tracked files under the declared paths that were not read.
    pub unread: Vec<PathBuf>,
}

impl ScopedRead {
    /// Whether the run reports on `file`: it lies under a requested path.
    pub fn reports(&self, file: &Path) -> bool {
        self.paths.iter().any(|p| file.starts_with(p))
    }
}

impl BackPointerSources {
    /// The tracked files a path scope left unread; empty on a whole-corpus run.
    pub fn unread(&self) -> &[PathBuf] {
        self.scoped.as_ref().map_or(&[], |s| s.unread.as_slice())
    }
}

/// What a rule in a later [`Phase`] reads of the run so far. Empty while the
/// `Main` rules run.
pub struct RunState<'a> {
    /// Every rule's execution so far.
    pub executions: &'a [RuleExecution],
    /// The findings left after scoping, before waivers apply.
    pub findings: &'a [Finding],
    pub waivers: &'a [Suppression],
    /// The run covers the whole corpus: no path scope, no `--base`.
    pub whole_corpus: bool,
    /// Whether a path is inside what the run reports on.
    pub reached: &'a dyn Fn(&Path) -> bool,
    pub today: &'a str,
    /// The configuration file as a finding on it names it: repository-relative.
    /// A waiver scoped to it must narrow (`waiver.matches-nothing`, `SPEC-10`).
    pub config_file: &'a Path,
}

impl RunState<'_> {
    pub fn empty() -> RunState<'static> {
        RunState {
            executions: &[],
            findings: &[],
            waivers: &[],
            whole_corpus: false,
            reached: &|_| true,
            today: "",
            config_file: Path::new(""),
        }
    }
}

/// Everything any rule reads, built once per run. A rule's table entry picks
/// what it needs; nothing here is specific to one rule.
pub struct RuleCtx<'a> {
    pub config: &'a Config,
    pub config_path: &'a Path,
    pub records: &'a [Record],
    pub full_text: &'a HashMap<PathBuf, String>,
    /// Each type's template header (`field -> value`), read by the caller from
    /// `.urzua/templates/<type>.md`; empty unless `field.quality` declares
    /// `template_placeholders`.
    pub templates: &'a crate::new_record::TemplateValues,
    /// Each type's template text, read by the caller from
    /// `.urzua/templates/<type>.md`; empty unless `section.order` or
    /// `header.scaffold-mismatch` is enabled (`MILE-104`, `MILE-103`).
    pub template_texts: &'a HashMap<String, String>,
    /// Resolves a reference to a record; built once for every rule that resolves one.
    pub index: &'a RecordIndex<'a>,
    pub collisions: &'a [IdentifierCollision<'a>],
    /// Record files on disk that git does not track, by id (`BUG-13`); read by
    /// the rules that report an unresolved reference.
    pub untracked_records: &'a UntrackedRecords,
    /// Claim files read from the declared `claim_paths`, as (path, text).
    pub claims: &'a [(String, String)],
    pub published_files: &'a [PublishedFile],
    pub boundaries: &'a [ResolvedBoundary],
    pub ignore_marker: Option<&'a str>,
    pub index_artifacts: &'a [IndexArtifact],
    /// The declared override log, read by the caller; `None` when `override_log`
    /// is not declared (`MILE-188`).
    pub override_log: Option<&'a OverrideLog>,
    /// `None` when the rule is absent or off.
    pub back_pointer_sources: Option<&'a BackPointerSources>,
    pub decision_gate_sources: Option<&'a BackPointerSources>,
    /// The source files `code.dangling-bug-reference` reads, filtered to its
    /// declared extensions; `None` when the rule is absent or off.
    pub code_citation_sources: Option<&'a BackPointerSources>,
    pub back_pointed: &'a BackPointerEvidence,
    pub drifted: &'a HashSet<PathBuf>,
    pub fresh: &'a HashSet<PathBuf>,
    pub claim_ages: &'a HashMap<PathBuf, u64>,
    /// The commit time of each record's `Realized-by` line, read only while
    /// `embodiment.claim-expired` is enabled (`ADR-97`).
    pub claim_line_times: &'a HashMap<PathBuf, i64>,
    /// Each record's first-cycle offset seed (`ADR-97` section 2).
    pub claim_seeds: &'a HashMap<PathBuf, u64>,
    /// Now, in seconds since the epoch: the one clock the expiry rule reads.
    pub now_unix: i64,
    /// The merge base's text of each changed record (`--base`), read by the
    /// caller; a record new in the diff has no entry. Empty without `--base`.
    pub base_text: &'a HashMap<PathBuf, String>,
    /// The locators (`category:locator` or `category:ID`) of each changed record
    /// that had drifted at the merge base, read by the caller for the records
    /// that gained a drift-acknowledging `Re-verified` row. Empty without `--base`.
    pub base_drift: &'a HashMap<PathBuf, HashSet<String>>,
    /// Per changed record, the UTC day (start, unix seconds) of the commit that
    /// added each `Re-verified` row of its text (`--base`); empty without one.
    pub added_row_days: &'a HashMap<PathBuf, super::RowWindows>,
    /// `None` on a run with no `--base`.
    pub changed_paths: Option<&'a [PathBuf]>,
    /// The subset of `changed_paths` that did not exist at the merge base; `None`
    /// on a run with no `--base`.
    pub added_paths: Option<&'a [PathBuf]>,
    /// The subset of `added_paths` that is a rename: a record whose `Stable-Id`
    /// a deleted record carried at the merge base. Empty unless
    /// `header.scaffold-mismatch` is enabled on a `--base` run.
    pub renamed_paths: &'a HashSet<PathBuf>,
    /// What the fetched pull-request refs hold that matters to this diff; empty
    /// unless `identity.open-pr-collision` is enabled on a `--base` run.
    pub pull_ref_claims: &'a PullRefClaims,
    /// Each changed record's `Status` at the merge base, for the types that
    /// declare `transitions`; `None` on a run with no `--base` (`ADR-72`).
    pub committed_statuses: Option<&'a HashMap<PathBuf, CommittedStatus>>,
    /// The records the diff deletes that are not a rename, for the types an
    /// authority status gate names (`diff.decider-required`).
    pub deleted_records: &'a [DeletedRecord],
    /// The identities the host supplied (`urzua check --actors`); `None` when
    /// no file was given (`MILE-200`).
    pub actors: Option<&'a crate::authority::Actors>,
    /// The base revision's `authority` table (`authority.decider_table`);
    /// `Absent` on a run with no `--base`.
    pub base_authority: &'a crate::authority::BaseAuthority,
    pub refs_snapshot: &'a Result<crate::refs_snapshot::RefsSnapshot, String>,
    /// Every discovered path, staged deletions included.
    pub tracked_paths: &'a [PathBuf],
    /// Discovered paths less staged deletions.
    pub live_paths: &'a [PathBuf],
    /// False only on a definite absence: an unreadable path counts as present,
    /// so a blocking finding is never raised on a guess.
    pub may_exist: &'a dyn Fn(&Path) -> bool,
    pub dir_exists: &'a dyn Fn(&str) -> bool,
    /// A file's text for a scoped locator to resolve in; read once per path.
    pub read_text: &'a dyn Fn(&str) -> crate::locator::SharedText,
    pub run: RunState<'a>,
}

/// A rule the repository has not declared does not run at all -- it is not run
/// and then filtered, because an opt-in rule that still costs its own
/// execution is opt-in in name only. The declared `level` replaces whatever
/// severity the rule body chose: the severity is the repository's call, not the
/// rule's (`MILE-80`).
///
/// Shared rather than owned by `check`: `audit` ran two rules directly and so
/// could not be declined by any configuration (`BUG-42`).
pub fn gate_and_run(
    config: &Config,
    id: &str,
    run: impl FnOnce() -> (RuleExecution, Vec<Finding>),
) -> (RuleExecution, Vec<Finding>) {
    use crate::config::RuleLevel;

    match config.rules.get(id).map(|s| s.level) {
        None | Some(RuleLevel::Off) => (RuleExecution::not_enabled(id), Vec::new()),
        Some(level) => {
            let severity = level
                .severity()
                .expect("a level other than off has a severity");
            let (exec, mut findings) = run();
            resolve_actions(id, &mut findings);
            for f in &mut findings {
                // A level the rule declared for this finding's own field
                // (`field_levels`, `MILE-140`) or boundary outranks the rule
                // level, except that a rule set to `info` is advisory
                // whatever a declaration says (`ADR-87` point 10); every
                // other finding takes the rule level.
                // Implements: ADR-87 (a declared severity never lifts a rule set to info)
                f.severity = match (level, f.declared_severity) {
                    (RuleLevel::Info, _) => severity,
                    (_, Some(declared)) => declared,
                    (_, None) => severity,
                };
                // A variant's declared floor outranks every setting above: the
                // repository's level applies to the rule's other variants, and
                // cannot lower this one (`ADR-96`).
                if let Some(floor) = variant_declaration(f).and_then(|v| v.severity_floor) {
                    f.severity = f.severity.at_least(floor);
                }
            }
            (exec, findings)
        }
    }
}

/// One rule through the gate.
pub fn run_rule(def: &RuleDef, ctx: &RuleCtx<'_>) -> (RuleExecution, Vec<Finding>) {
    gate_and_run(ctx.config, def.id, || (def.run)(ctx))
}

/// Every rule of `phase`, in table order.
///
/// A rule with a companion population (`MILE-186`) contributes its second
/// execution straight after its first, so `rules_executed` keeps table order.
pub fn run_phase(phase: Phase, ctx: &RuleCtx<'_>) -> Vec<(RuleExecution, Vec<Finding>)> {
    run_phase_with(phase, ctx, &PhasePlan::default())
}

/// How a run restricts the rules of a phase.
#[derive(Default)]
pub struct PhasePlan<'a> {
    /// The run reports on one record's proposed content (`check --stdin-path`,
    /// `MILE-191`). A rule whose findings are anchored outside the records (the
    /// configuration, a source file, an index, a diff) cannot report on that
    /// record, so an enabled one is not run and says so: `not-applicable`,
    /// never `ran` with nothing found.
    pub record_only: bool,
    /// `check --rules` (`ADR-79`): only these rule ids are run. The others are
    /// not run and not listed; selection narrows what runs and enables nothing.
    pub selected: Option<&'a HashSet<String>>,
}

/// [`run_phase`] under a [`PhasePlan`].
pub fn run_phase_with(
    phase: Phase,
    ctx: &RuleCtx<'_>,
    plan: &PhasePlan<'_>,
) -> Vec<(RuleExecution, Vec<Finding>)> {
    let mut out = Vec::new();
    for d in RULE_TABLE.iter().filter(|d| d.phase == phase) {
        if plan.selected.is_some_and(|s| !s.contains(d.id)) {
            continue;
        }
        let can_report_on_a_record =
            matches!(d.anchor, Anchor::Record | Anchor::RecordOrDeclaredFile(_));
        if plan.record_only && !can_report_on_a_record {
            out.push(gate_and_run(ctx.config, d.id, || not_applicable(d.id)));
            continue;
        }
        out.push(run_rule(d, ctx));
        if let Some((_, companion)) = d.companion {
            // Never called for a rule the repository has not enabled.
            let enabled = ctx
                .config
                .rules
                .get(d.id)
                .is_some_and(|s| s.level != crate::config::RuleLevel::Off);
            if let Some(extra) = enabled.then(|| companion(ctx)).flatten() {
                out.push(gate_and_run(ctx.config, d.id, || extra));
            }
        }
    }
    out
}

/// How a run narrows what it reports on.
pub enum Narrowing<'a> {
    /// Path arguments, repository-relative. Matched against the prefixes
    /// themselves, never the discovered set: that holds tracked files only, and
    /// a rule may report on a file git does not track (a claim file is read
    /// straight off disk) (`BUG-67`).
    Paths(&'a [PathBuf]),
    /// `--base`: the records the change reached, and the paths it changed.
    Diff {
        examined: &'a dyn Fn(&Path) -> bool,
        changed: &'a HashSet<PathBuf>,
    },
}

/// Whether a scoped run reports `finding`, decided by the anchor its rule
/// declares (`Anchor`) and nothing else.
pub fn finding_in_scope(config: &Config, finding: &Finding, narrowing: &Narrowing<'_>) -> bool {
    let anchor = rule_def(&finding.rule).map_or(Anchor::Record, |d| d.anchor);
    match anchor {
        Anchor::Config | Anchor::Outside => return true,
        Anchor::RecordOrDeclaredFile(declared) => {
            if declared(config).as_deref() == Some(finding.file.as_path()) {
                return true;
            }
        }
        Anchor::Record | Anchor::ChangedPath | Anchor::Directory => {}
    }
    match narrowing {
        Narrowing::Paths(prefixes) => prefixes.iter().any(|p| finding.file.starts_with(p)),
        Narrowing::Diff { examined, changed } => {
            examined(&finding.file)
                || match anchor {
                    // Not a loaded record, so never in the examined set; judged
                    // by path, so a changed one keeps its finding.
                    Anchor::ChangedPath => changed.contains(&finding.file),
                    // A whole-corpus finding on a type's directory stays when any
                    // changed path is inside it.
                    Anchor::Directory => changed.iter().any(|p| p.starts_with(&finding.file)),
                    _ => false,
                }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(declared: Option<FindingSeverity>) -> Finding {
        Finding {
            rule: "x".to_string(),
            severity: FindingSeverity::Error,
            file: PathBuf::from("a.md"),
            line: None,
            waived: None,
            declared_severity: declared,
            suggested_action: None,
            action_source: ActionSource::default(),
            message: String::new(),
        }
    }

    fn executed() -> RuleExecution {
        RuleExecution {
            rule: "header.required-fields".to_string(),
            population: None,
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        }
    }

    fn config_with(rule_yaml: &str) -> Config {
        crate::config::parse(&format!(
            "schema_version: 2\nrecord_types: {{}}\nrules: {{{rule_yaml}}}\n"
        ))
        .unwrap()
    }

    #[test]
    fn a_rule_the_repository_has_not_enabled_is_never_run() {
        for config in [config_with(""), config_with("header.required-fields: off")] {
            let (exec, findings) = gate_and_run(&config, "header.required-fields", || {
                panic!("a rule that is not enabled must not run")
            });
            assert_eq!(exec.status, RuleStatus::NotEnabled);
            assert!(findings.is_empty());
        }
    }

    #[test]
    fn the_declared_level_replaces_the_severity_the_rule_chose() {
        let config = config_with("header.required-fields: warn");
        let (_, findings) = gate_and_run(&config, "header.required-fields", || {
            (executed(), vec![finding(None)])
        });
        assert_eq!(findings[0].severity, FindingSeverity::Warning);
    }

    #[test]
    fn a_level_declared_for_the_finding_outranks_the_rule_level() {
        let config = config_with("header.required-fields: error");
        let (_, findings) = gate_and_run(&config, "header.required-fields", || {
            (executed(), vec![finding(Some(FindingSeverity::Warning))])
        });
        assert_eq!(findings[0].severity, FindingSeverity::Warning);
    }

    /// `rules_executed` lists rules in table order within a phase and phases
    /// in order, so `urzua rules` lists them as `check` reports them.
    #[test]
    fn the_table_lists_phases_in_the_order_they_run() {
        let phases: Vec<Phase> = RULE_TABLE.iter().map(|d| d.phase).collect();
        let mut sorted = phases.clone();
        sorted.sort_by_key(|p| match p {
            Phase::Main => 0,
            Phase::AfterRules => 1,
            Phase::AfterScoping => 2,
        });
        assert_eq!(phases, sorted);
    }

    fn on(rule: &str, file: &str) -> Finding {
        Finding {
            rule: rule.to_string(),
            file: PathBuf::from(file),
            ..finding(None)
        }
    }

    fn unconfigured() -> Config {
        config_with("")
    }

    #[test]
    fn a_path_scope_keeps_what_is_not_a_record_whatever_the_prefix() {
        let config = unconfigured();
        let prefixes = [PathBuf::from("docs/adr")];
        let narrowing = Narrowing::Paths(&prefixes);
        let keeps = |f: Finding| finding_in_scope(&config, &f, &narrowing);

        assert!(keeps(on(RULE_HEADER_REQUIRED_FIELDS, "docs/adr/ADR-1.md")));
        assert!(!keeps(on(RULE_HEADER_REQUIRED_FIELDS, "docs/rfc/RFC-1.md")));
        // Anchored on the config or outside the corpus: every scope keeps it.
        assert!(keeps(on(
            RULE_CONFIG_POINTER_FIELD_NOT_KNOWN,
            ".urzua/config.yaml"
        )));
        assert!(keeps(on(RULE_CLAIM_STATUS_AGREEMENT, ".changeset/c.md")));
        assert!(keeps(on(
            RULE_EMBODIMENT_BACK_POINTER_RESOLUTION,
            "src/lib.rs"
        )));
        // A path-anchored rule is judged by prefix under a path scope.
        assert!(!keeps(on(
            RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR,
            "docs/rfc/x.md"
        )));
        assert!(keeps(on(
            RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR,
            "docs/adr/x.md"
        )));
    }

    #[test]
    fn a_diff_run_keeps_a_path_anchored_finding_only_for_a_changed_path() {
        let config = unconfigured();
        let changed: HashSet<PathBuf> = [PathBuf::from("docs/adr/stray.md")].into();
        let examined = |p: &Path| p == Path::new("docs/adr/ADR-1.md");
        let narrowing = Narrowing::Diff {
            examined: &examined,
            changed: &changed,
        };
        let keeps = |f: Finding| finding_in_scope(&config, &f, &narrowing);

        assert!(keeps(on(RULE_HEADER_REQUIRED_FIELDS, "docs/adr/ADR-1.md")));
        assert!(!keeps(on(RULE_HEADER_REQUIRED_FIELDS, "docs/adr/ADR-2.md")));
        assert!(keeps(on(
            RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR,
            "docs/adr/stray.md"
        )));
        assert!(!keeps(on(
            RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR,
            "docs/adr/other.md"
        )));
        // A finding on a type's directory stays when a changed path is inside it.
        assert!(keeps(on(RULE_FILENAME_IDENTITY_PATTERN, "docs/adr")));
        assert!(!keeps(on(RULE_FILENAME_IDENTITY_PATTERN, "docs/rfc")));
        assert!(keeps(on(RULE_INDEX_FRESHNESS, "docs/adr/README.md")));
    }

    #[test]
    fn a_declared_file_is_kept_only_for_the_rule_that_declares_it() {
        let config = crate::config::parse(
            "schema_version: 2\nrecord_types: {}\nrules:\n  citation.issue-exists:\n    level: error\n    reference_sections: [References]\n    issue_refs: {snapshot: .urzua/known-refs.json, repository: o/r, forms: [number]}\n",
        )
        .unwrap();
        let prefixes = [PathBuf::from("docs")];
        let narrowing = Narrowing::Paths(&prefixes);
        assert!(finding_in_scope(
            &config,
            &on(RULE_CITATION_ISSUE_EXISTS, ".urzua/known-refs.json"),
            &narrowing
        ));
        assert!(!finding_in_scope(
            &config,
            &on(RULE_CITATION_ISSUE_EXISTS, "other/a.md"),
            &narrowing
        ));
        assert!(!finding_in_scope(
            &config,
            &on(RULE_CITATION_RESOLUTION, ".urzua/known-refs.json"),
            &narrowing
        ));
    }
}
