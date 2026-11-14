//! `urzua check` (SPEC-2): the validator.
//!
//! `run` composes four stages, each a function of at most 100 lines:
//!
//! 1. `open_session` and `narrow`: find the repository, load the corpus whole,
//!    and decide what the run reports on (path scopes, `--base`).
//! 2. `load_context`: read everything a rule is handed rather than opens
//!    (claim, published, index and source files, drift, the refs snapshot).
//! 3. `run_rules`: the three rule phases, with `scope_findings` between the
//!    second and the third.
//! 4. `build_report`: waivers, then the status from the pure `derive_status`.
//!
//! Every filesystem read lives in `discovery`; `urzua-core` stays pure.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::config::{Config, RuleLevel, RuleSetting};
use urzua_core::refs_snapshot::RefsSnapshot;
use urzua_core::report::{BaselineInfo, DetailValue, Failure};
use urzua_core::report::{
    CheckReport, CouldNotRun, DiffScope, Finding, FindingSeverity, Notice, NoticeSeverity,
    RuleExecution, ScopeInfo,
};
use urzua_core::rules;

use crate::discovery::{
    path_is_inside_repo, read_claim_files, read_index_artifacts, scope_to_requested_paths,
};
use crate::emit;
use crate::gate::derive_status;
use crate::session::Session;

/// What the command line asked to narrow the run to.
struct Opened {
    session: Session,
    requested_scopes: Vec<PathBuf>,
    changed: Option<urzua_io::GitChanges>,
    prospective: Option<Prospective>,
}

/// One record's proposed content (`--stdin-path`, `MILE-191`). It replaces the
/// record's on-disk copy (or adds a record) in the session before any rule
/// runs, so `urzua-core` only ever sees records and text, never a path to read.
struct Prospective {
    path: PathBuf,
    content: String,
}

/// What the run reports on. The corpus a pointer resolves against stays whole
/// (`BUG-60`, `MILE-123`); this only narrows what is *reported on*.
struct ReportScope {
    requested: Vec<PathBuf>,
    changed: Option<urzua_io::GitChanges>,
    diff: Option<DiffScope>,
    in_scope: HashSet<PathBuf>,
    /// How many records the diff reached before a path argument narrowed it.
    reached_before_narrowing: usize,
    prospective: Option<Prospective>,
}

impl ReportScope {
    fn whole_corpus(&self) -> bool {
        self.requested.is_empty() && self.diff.is_none()
    }
}

/// Everything a rule is handed rather than opens, read once.
struct Context<'a> {
    session: &'a Session,
    claims: Vec<(String, String)>,
    published_files: Vec<rules::PublishedFile>,
    boundaries: Vec<rules::ResolvedBoundary>,
    ignore_marker: Option<String>,
    index_artifacts: Vec<rules::IndexArtifact>,
    override_log: Option<rules::OverrideLog>,
    back_pointer_sources: Option<rules::BackPointerSources>,
    decision_gate_sources: Option<rules::BackPointerSources>,
    code_citation_sources: Option<rules::BackPointerSources>,
    back_pointed: rules::BackPointerEvidence,
    drift: crate::discovery::DriftedRecords,
    claim_ages: HashMap<PathBuf, u64>,
    /// `embodiment.claim-expired`'s inputs (`ADR-97`): the commit time of each
    /// record's `Realized-by` line and its first-cycle offset seed.
    claim_line_times: HashMap<PathBuf, i64>,
    claim_seeds: HashMap<PathBuf, u64>,
    now_unix: i64,
    refs_snapshot: Result<RefsSnapshot, String>,
    record_index: rules::RecordIndex<'a>,
    collisions: Vec<rules::IdentifierCollision<'a>>,
    untracked_records: rules::UntrackedRecords,
    live_paths: Vec<PathBuf>,
    changed_paths: Option<Vec<PathBuf>>,
    /// The changed paths that did not exist at the merge base; `None` without `--base`.
    added_paths: Option<Vec<PathBuf>>,
    /// The fetched pull-request refs' holders of the ids this diff adds
    /// (`identity.open-pr-collision`); empty unless that rule is enabled.
    pull_ref_claims: rules::PullRefClaims,
    committed_statuses: Option<HashMap<PathBuf, rules::CommittedStatus>>,
    deleted_records: Vec<rules::DeletedRecord>,
    /// The identities from `--actors` (`MILE-200`).
    actors: Option<urzua_core::authority::Actors>,
    /// The base revision's `authority` table (`authority.decider_table`).
    base_authority: urzua_core::authority::BaseAuthority,
    /// The checkout's config with the base revision's stricter decider-gate
    /// settings applied (the rule's level and options, an `authority` the head
    /// dropped); `None` when the head's own config is what runs.
    gate_config: Option<Config>,
    /// The exit threshold in force: the stricter of the head's `fail_on` and,
    /// under `authority.decider_table: base`/`both`, the base's.
    fail_on: urzua_core::config::FailOn,
    repo_root: PathBuf,
    base_text: HashMap<PathBuf, String>,
    base_drift: HashMap<PathBuf, HashSet<String>>,
    added_row_days: HashMap<PathBuf, rules::RowWindows>,
    templates: urzua_core::new_record::TemplateValues,
    /// `section.order`'s template texts, by type (`MILE-104`).
    template_texts: HashMap<String, String>,
    /// A proposed record is being judged (`--stdin-path`).
    record_only: bool,
    /// `--rules`: the only rules to run (`ADR-79`); `None` runs every declared rule.
    selected: Option<HashSet<String>>,
}

/// `revision-log.version-bump-required` judges a changed record against the
/// merge base's copy, so that copy is read here (`BUG-51`) and handed over as
/// data; so does `embodiment.claim-renewal-unattested` (`ADR-97`), for the
/// records of the types `embodiment.claim-expired` lists. Only the changed
/// records one of them reads are read; a record new in the diff has no
/// merge-base copy and gets no entry.
fn load_base_text(
    session: &Session,
    changes: Option<&urzua_io::GitChanges>,
    notices: &mut Vec<Notice>,
) -> Result<HashMap<PathBuf, String>, String> {
    let mut out = HashMap::new();
    let Some(changes) = changes else {
        return Ok(out);
    };
    let config = &session.config;
    let bump = enabled_setting(config, rules::RULE_REVISION_LOG_VERSION_BUMP_REQUIRED).is_some();
    let renewal =
        enabled_setting(config, rules::RULE_EMBODIMENT_CLAIM_RENEWAL_UNATTESTED).is_some();
    if !bump && !renewal {
        return Ok(out);
    }
    let expiry: rules::ClaimExpiredOpts = config.rule_opts(rules::RULE_EMBODIMENT_CLAIM_EXPIRED);
    for record in &session.records {
        let wanted_by_bump = bump
            && config
                .record_types
                .get(&record.record_type)
                .is_some_and(|t| t.version_bump().is_some());
        let wanted_by_renewal = renewal
            && expiry
                .periods
                .iter()
                .any(|p| p.record_type == record.record_type);
        if !(wanted_by_bump || wanted_by_renewal) || !changes.changed.contains(&record.path) {
            continue;
        }
        // A path added since the merge base (a rename's new name too) has no
        // copy there: not an error, and not examined. Any other failure to read
        // the merge base's copy is disclosed, since the record then goes
        // unexamined.
        if changes.added.contains(&record.path) {
            continue;
        }
        match urzua_io::read_at_commit(&session.repo_root, &changes.merge_base, &record.path) {
            Ok(text) => {
                out.insert(record.path.clone(), text);
            }
            Err(e) => notices.push(Notice {
                severity: NoticeSeverity::Warning,
                subject: if wanted_by_bump {
                    rules::RULE_REVISION_LOG_VERSION_BUMP_REQUIRED
                } else {
                    rules::RULE_EMBODIMENT_CLAIM_RENEWAL_UNATTESTED
                }
                .to_string(),
                message: format!(
                    "{}: could not read the merge base's copy, so the record is not examined: {e}",
                    record.path.display()
                ),
                detail: None,
            }),
        }
    }
    Ok(out)
}

/// The one change a proposed record is: its path, against HEAD's copy. A path
/// HEAD does not have is an addition, so it has nothing to compare with.
fn head_changes_for(repo_root: &Path, path: &Path) -> urzua_io::GitChanges {
    let mut changes = urzua_io::GitChanges {
        merge_base: "HEAD".to_string(),
        changed: HashSet::from([path.to_path_buf()]),
        deleted: HashSet::new(),
        added: HashSet::new(),
    };
    if urzua_io::read_at_commit(repo_root, "HEAD", path).is_err() {
        changes.added.insert(path.to_path_buf());
    }
    changes
}

/// `field.quality`'s `template_placeholders` (`BUG-22`): each declared type's
/// template header, read from `.urzua/templates/<type>.md`. A type with no
/// template is simply absent. Declaring the option when no type has one is a
/// warning notice rather than a rule that quietly does nothing (the same
/// treatment as an absent `claim_paths` entry).
pub(crate) fn load_templates(
    repo_root: &Path,
    config: &Config,
    notices: &mut Vec<Notice>,
) -> Result<urzua_core::new_record::TemplateValues, String> {
    let declared = config
        .enabled_rule_opts::<rules::FieldQualityOpts>(rules::RULE_FIELD_QUALITY)
        .is_some_and(|o| o.template_placeholders);
    let mut templates = urzua_core::new_record::TemplateValues::new();
    if !declared {
        return Ok(templates);
    }
    for record_type in config.record_types.keys() {
        let path = repo_root
            .join(".urzua/templates")
            .join(format!("{record_type}.md"));
        match urzua_io::read_to_string(&path) {
            Ok(text) => {
                templates.insert(
                    record_type.clone(),
                    urzua_core::new_record::template_header_values(&text),
                );
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(format!(
                    "field.quality: template '{}' could not be read: {e}",
                    path.display()
                ))
            }
        }
    }
    if templates.is_empty() {
        notices.push(Notice {
            severity: NoticeSeverity::Warning,
            subject: rules::RULE_FIELD_QUALITY.to_string(),
            message: "field.quality: template_placeholders is declared but no \
                      .urzua/templates/<type>.md exists for any record type -- the option reads nothing"
                .to_string(),
                detail: None,
        });
    }
    Ok(templates)
}

/// `section.order`'s template text (`MILE-104`): `.urzua/templates/<type>.md`
/// for each type declaring `sections.order: {from: template}`, read only while
/// the rule is enabled. A missing template is simply absent from the map; the
/// rule reports a missing or section-less template as a finding on each record
/// of the type, so it gates a run under `fail_on`.
pub(crate) fn load_template_texts(
    repo_root: &Path,
    config: &Config,
) -> Result<HashMap<String, String>, String> {
    let mut texts = HashMap::new();
    if config
        .rules
        .get(rules::RULE_SECTION_ORDER)
        .is_none_or(|s| s.level == urzua_core::config::RuleLevel::Off)
    {
        return Ok(texts);
    }
    for (record_type, _) in rules::types_reading_a_template_order(config) {
        let path = repo_root
            .join(".urzua/templates")
            .join(format!("{record_type}.md"));
        match urzua_io::read_to_string(&path) {
            Ok(text) => {
                texts.insert(record_type, text);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(format!(
                    "{}: template '{}' could not be read: {e}",
                    rules::RULE_SECTION_ORDER,
                    path.display()
                ))
            }
        }
    }
    Ok(texts)
}

impl<'a> Context<'a> {
    fn rule_ctx(
        &'a self,
        may_exist: &'a dyn Fn(&Path) -> bool,
        dir_exists: &'a dyn Fn(&str) -> bool,
        read_text: &'a dyn Fn(&str) -> urzua_core::locator::SharedText,
        run: rules::RunState<'a>,
    ) -> rules::RuleCtx<'a> {
        let s = self.session;
        rules::RuleCtx {
            config: self.gate_config.as_ref().unwrap_or(&s.config),
            config_path: &s.config_path,
            records: &s.records,
            full_text: &s.full_text,
            templates: &self.templates,
            template_texts: &self.template_texts,
            index: &self.record_index,
            collisions: &self.collisions,
            untracked_records: &self.untracked_records,
            claims: &self.claims,
            published_files: &self.published_files,
            boundaries: &self.boundaries,
            ignore_marker: self.ignore_marker.as_deref(),
            index_artifacts: &self.index_artifacts,
            override_log: self.override_log.as_ref(),
            back_pointer_sources: self.back_pointer_sources.as_ref(),
            decision_gate_sources: self.decision_gate_sources.as_ref(),
            code_citation_sources: self.code_citation_sources.as_ref(),
            back_pointed: &self.back_pointed,
            drifted: &self.drift.drifted,
            fresh: &self.drift.fresh,
            claim_ages: &self.claim_ages,
            claim_line_times: &self.claim_line_times,
            claim_seeds: &self.claim_seeds,
            now_unix: self.now_unix,
            base_text: &self.base_text,
            base_drift: &self.base_drift,
            added_row_days: &self.added_row_days,
            changed_paths: self.changed_paths.as_deref(),
            added_paths: self.added_paths.as_deref(),
            pull_ref_claims: &self.pull_ref_claims,
            committed_statuses: self.committed_statuses.as_ref(),
            deleted_records: &self.deleted_records,
            actors: self.actors.as_ref(),
            base_authority: &self.base_authority,
            refs_snapshot: &self.refs_snapshot,
            tracked_paths: &s.discovered.paths,
            live_paths: &self.live_paths,
            may_exist,
            dir_exists,
            read_text,
            run,
        }
    }
}

/// `rule`'s setting, only when the rule is enabled.
fn enabled_setting<'c>(config: &'c Config, rule: &str) -> Option<&'c RuleSetting> {
    config.rules.get(rule).filter(|s| s.level != RuleLevel::Off)
}

/// The `urzua check` command line, gathered so `run` stays one argument wide.
#[derive(Default)]
pub struct CheckArgs {
    pub paths: Vec<PathBuf>,
    pub base: Option<String>,
    pub fail_on: Option<urzua_core::config::FailOn>,
    pub stdin_path: Option<PathBuf>,
    /// The proposed content for `stdin_path`, when the caller has it already
    /// (`urzua harness run edit` judges in process); otherwise it is read from
    /// stdin.
    pub proposed: Option<String>,
    /// `--rules`: empty means every declared rule.
    pub rules: Vec<String>,
    /// `--actors`: the file the gate action writes (`MILE-200`).
    pub actors: Option<PathBuf>,
    /// Notices the invoking command adds to the report (`audit`'s deprecation).
    pub extra_notices: Vec<Notice>,
    /// Records that exist only in memory, as `(path, content)`, added to the
    /// whole-corpus run as if written (`waive --from-report` judges the tree
    /// with its drafts). Unlike `stdin_path`, the run still reports on every record.
    pub added_records: Vec<(PathBuf, String)>,
}

pub fn run(config_path: Option<PathBuf>, args: CheckArgs) -> ExitCode {
    match execute(config_path, args) {
        Ok(report) => emit(&report),
        Err(e) => emit(&CouldNotRun::from(e)),
    }
}

/// `--rules` against what the configuration declares (`ADR-79`): an id this
/// build does not ship and an id the configuration does not declare are both
/// mistakes in the command, not findings about the corpus. Selection narrows
/// what runs and never enables a rule, so a declared-`off` rule is accepted
/// and reports `not-enabled`. Returns the ids in the order given, without
/// repeats; `None` when no selection was asked for.
fn validate_selection(
    config: &Config,
    requested: &[String],
) -> Result<Option<Vec<String>>, String> {
    if requested.is_empty() {
        return Ok(None);
    }
    let mut out: Vec<String> = Vec::new();
    for id in requested {
        let id = id.trim();
        if id.is_empty() {
            return Err("urzua check --rules: an empty rule id (a stray comma?)".to_string());
        }
        if !rules::ALL_RULES.contains(&id) {
            return Err(format!(
                "urzua check --rules: '{id}' is not a rule this build ships (`urzua rules` lists them)"
            ));
        }
        if !config.rules.contains_key(id) {
            return Err(format!(
                "urzua check --rules: '{id}' is not declared in the configuration; --rules narrows the declared rules and never enables one"
            ));
        }
        if !out.iter().any(|o| o == id) {
            out.push(id.to_string());
        }
    }
    Ok(Some(out))
}

/// Judges one record's proposed content in process, as `check --stdin-path`
/// does, for the harness cores. The path is relative to the repository root.
pub(crate) fn judge_proposed(
    config_path: Option<PathBuf>,
    path: PathBuf,
    content: String,
) -> Result<CheckReport, Failure> {
    execute(
        config_path,
        CheckArgs {
            stdin_path: Some(path),
            proposed: Some(content),
            ..Default::default()
        },
    )
}

/// Runs `check PATHS` in process, as the post-command check does: a named file
/// is examined whether or not git tracks it (`BUG-24`), and nothing is staged.
pub(crate) fn check_paths(
    config_path: Option<PathBuf>,
    paths: Vec<PathBuf>,
) -> Result<CheckReport, Failure> {
    execute(
        config_path,
        CheckArgs {
            paths,
            ..Default::default()
        },
    )
}

/// Why the rules that judge a change were deferred on a run with no diff.
fn why_deferred(scope: &ReportScope, base: Option<&str>) -> String {
    if scope.prospective.is_some() {
        "not the proposed record".to_string()
    } else {
        base.map_or("no --base".to_string(), |b| format!("unchanged since {b}"))
    }
}

/// The report's account of what the run covered.
fn scope_info(session: &Session, scope: ReportScope, base: Option<String>) -> ScopeInfo {
    ScopeInfo {
        source: if scope.prospective.is_some() {
            urzua_core::report::ScopeSource::Stdin
        } else {
            crate::discovery::scope_source(session.discovered.source)
        },
        base,
        record_types: session.config.record_types.keys().cloned().collect(),
        include_untracked: session.config.discovery.include_untracked,
        untracked_included: session.discovered.untracked_included.len(),
        diff: scope.diff,
    }
}

/// The flag combinations `check` refuses, before anything is read.
fn reject_flag_conflicts(args: &CheckArgs) -> Result<(), Failure> {
    if args.stdin_path.is_some() && (!args.paths.is_empty() || args.base.is_some()) {
        return Err(
            "urzua check --stdin-path judges one record's proposed content and reports on it alone: it cannot be combined with PATHS or --base"
                .into(),
        );
    }
    if args.stdin_path.is_some() && args.actors.is_some() {
        return Err(
            "urzua check --actors names who made a --base change: it cannot be combined with --stdin-path"
                .into(),
        );
    }
    Ok(())
}

/// The waivers in the records, and the notices reading them owes: an unreadable
/// waiver template and the count of permanent waivers.
fn load_waivers(
    session: &Session,
    notices: &mut Vec<Notice>,
) -> Vec<urzua_core::waiver::Suppression> {
    let mut suppression_type = urzua_core::waiver::SuppressionType::from_config(&session.config);
    // The suppression type's template says what its placeholder text is.
    let template_path = session
        .repo_root
        .join(".urzua/templates")
        .join(format!("{}.md", suppression_type.name));
    match urzua_io::read_to_string(&template_path) {
        Ok(text) => suppression_type = suppression_type.with_template(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => notices.push(Notice {
            severity: NoticeSeverity::Warning,
            subject: "waiver.template".to_string(),
            message: format!(
                "the waiver template '{}' could not be read ({e}); a Reason still holding the template's text is not recognised as a placeholder",
                template_path.display()
            ),
            detail: None,
        }),
    }
    let waivers = urzua_core::waiver::load_suppressions(&session.records, &suppression_type);
    notices.extend(permanent_waivers_notice(&suppression_type, &waivers));
    waivers
}

pub(crate) fn execute(
    config_path: Option<PathBuf>,
    args: CheckArgs,
) -> Result<CheckReport, Failure> {
    reject_flag_conflicts(&args)?;
    let CheckArgs {
        paths,
        base,
        fail_on,
        stdin_path,
        proposed,
        rules: requested_rules,
        actors: actors_path,
        extra_notices,
        added_records,
    } = args;
    let actors = actors_path.as_deref().map(read_actors).transpose()?;
    let mut opened = open_session(
        &paths,
        config_path,
        base.as_deref(),
        stdin_path.as_deref(),
        proposed,
    )?;
    add_records(&mut opened.session, added_records)?;
    let session = &opened.session;
    let selection = validate_selection(&session.config, &requested_rules)?;
    let baseline_commit = resolve_drift_baseline(session, selection.as_deref())?;
    let scope = narrow(
        session,
        opened.requested_scopes,
        opened.changed,
        opened.prospective,
    )?;
    let (mut context, mut notices) = load_context(session, &scope, selection.as_deref())?;
    let baseline = hold_out_baseline(
        baseline_commit,
        session,
        &scope,
        (&mut context, &mut notices),
    );
    context.actors = actors;
    notices.extend(session_notices(
        session,
        &scope,
        &context,
        (base.is_some(), selection.as_deref()),
    ));
    notices.extend(extra_notices);
    let (mut executions, mut findings) = run_rules(&context);
    notices.extend(scope_changed_notices(
        &executions,
        &why_deferred(&scope, base.as_deref()),
    ));
    notices.extend(excused_line_notices(&executions));
    scope_findings(session, &scope, &mut findings);
    let waivers = effective_waivers(session, &context, &mut notices);
    let today = urzua_io::today();
    run_after_scoping(
        &context,
        &scope,
        (&waivers, &today),
        &mut executions,
        &mut findings,
    );
    urzua_core::waiver::apply_suppressions(&mut findings, &waivers, &today);
    let (status, files_examined) = status_and_examined(
        session,
        &scope,
        &executions,
        &findings,
        &mut notices,
        base.as_deref(),
    );
    let blocking = findings
        .iter()
        .any(|f| f.waived.is_none() && f.severity == FindingSeverity::Error);
    // Counted over the same set `files_examined` counts (`BUG-40`). Rules run
    // over the whole corpus because a reference resolves against records outside
    // the scope (`BUG-67`); what is *reported on* is the scope.
    let records_read_by_any_rule = urzua_core::report::records_read_by_any_rule_within(
        &executions,
        &scope.in_scope.iter().map(|p| p.as_path()).collect(),
    );
    Ok(CheckReport {
        as_of: crate::as_of(),
        status,
        files_examined,
        records_read_by_any_rule,
        rules_executed: executions,
        rules_selected: selection,
        scope: scope_info(session, scope, base),
        baseline,
        blocking,
        findings,
        notices,
        // The flag wins over the configuration; either only moves the exit code.
        fail_on: fail_on.unwrap_or(context.fail_on),
    })
}

/// The notices a run raises from its session, scope and context before any rule
/// runs; `with_base` says whether `--base` was given.
fn session_notices(
    session: &Session,
    scope: &ReportScope,
    context: &Context<'_>,
    (with_base, selection): (bool, Option<&[String]>),
) -> Vec<Notice> {
    let mut notices = untracked_notices(session, scope, &context.drift);
    notices.extend(crate::discovery::grace_notice(
        &context.drift,
        &session.records,
        &session.config,
        &context.back_pointed,
    ));
    notices.extend(harness_key_notices(session.config.harness.as_ref()));
    notices.extend(decider_gate_notices(
        &session.config,
        context,
        (with_base, context.actors.is_some()),
        selection,
    ));
    notices
}

/// Holds the drift a `drift_baseline` names out of `context` and discloses it
/// in `notices`; `None` when no baseline applies to this run.
fn hold_out_baseline(
    resolved: Option<(String, String)>,
    session: &Session,
    scope: &ReportScope,
    (context, notices): (&mut Context<'_>, &mut Vec<Notice>),
) -> Option<BaselineInfo> {
    let (git_ref, commit) = resolved?;
    let known_problems = context.drift.git_problems.len();
    let split = crate::discovery::apply_drift_baseline(
        &mut context.drift,
        &session.repo_root,
        &session.config,
        &commit,
    );
    // A git failure met while attributing drift is disclosed like any other.
    notices.extend(crate::discovery::drift_notices(
        false,
        &context.drift.git_problems[known_problems..],
    ));
    notices.extend(baseline_notices(
        &git_ref,
        &commit,
        &split,
        scope.whole_corpus(),
    ));
    Some(baseline_info(git_ref, commit, &split))
}

/// Notice subject for the drift a `drift_baseline` holds out of the gate.
const NOTICE_DRIFT_BASELINED: &str = "embodiment-drift-baselined";

/// Notice subject for a baseline that holds out nothing.
const NOTICE_DRIFT_BASELINE_UNUSED: &str = "embodiment-drift-baseline-unused";

/// The records a notice or the report names; the count says the rest.
const BASELINE_RECORDS_SHOWN: usize = 20;

/// The ref and commit of `embodiment.consistency`'s `drift_baseline`, when the
/// rule is on, declares one and the run reaches it. A ref that names no commit
/// stops the run (`MILE-49`): a baseline that cannot be read must never read as
/// "no pre-existing drift", which would put the whole backlog back on the gate.
fn resolve_drift_baseline(
    session: &Session,
    selection: Option<&[String]>,
) -> Result<Option<(String, String)>, Failure> {
    let Some(consistency) = enabled_setting(&session.config, rules::RULE_EMBODIMENT_CONSISTENCY)
    else {
        return Ok(None);
    };
    let Some(baseline) = consistency.drift_baseline.as_ref() else {
        return Ok(None);
    };
    if !selection_wants(selection, &[rules::RULE_EMBODIMENT_CONSISTENCY]) {
        return Ok(None);
    }
    let git_ref = baseline.git_ref.trim().to_string();
    match urzua_io::resolve_commit(&session.repo_root, &git_ref) {
        Ok(commit) => Ok(Some((git_ref, commit))),
        Err(e) => Err(format!(
            "{}.drift_baseline.ref: {e}; fix the ref or remove drift_baseline (a shallow clone may lack the commit: fetch full history)",
            rules::RULE_EMBODIMENT_CONSISTENCY
        )
        .into()),
    }
}

fn baseline_info(
    git_ref: String,
    commit: String,
    split: &crate::discovery::BaselineSplit,
) -> urzua_core::report::BaselineInfo {
    urzua_core::report::BaselineInfo {
        git_ref,
        commit,
        pre_existing: split.pre_existing.len(),
        pre_existing_records: split
            .pre_existing
            .iter()
            .take(BASELINE_RECORDS_SHOWN)
            .map(|p| p.display().to_string())
            .collect(),
        introduced: split.introduced.len(),
    }
}

/// The info notice for pre-existing drift held out of the gate, or, when none
/// is held out on a whole-corpus run, the nudge to remove the baseline. A scoped
/// run proves nothing about the rest of the corpus, so it gives no nudge.
fn baseline_notices(
    git_ref: &str,
    commit: &str,
    split: &crate::discovery::BaselineSplit,
    whole_corpus: bool,
) -> Option<Notice> {
    let n = split.pre_existing.len();
    let mut detail = std::collections::BTreeMap::new();
    detail.insert("ref".to_string(), DetailValue::Text(git_ref.to_string()));
    detail.insert("commit".to_string(), DetailValue::Text(commit.to_string()));
    if n == 0 {
        if !whole_corpus {
            return None;
        }
        return Some(Notice {
            severity: NoticeSeverity::Info,
            subject: NOTICE_DRIFT_BASELINE_UNUSED.to_string(),
            message: format!(
                "drift_baseline '{git_ref}' holds out no drift: no record's drift predates it, so the baseline can be removed from {}",
                rules::RULE_EMBODIMENT_CONSISTENCY
            ),
            detail: Some(detail),
        });
    }
    detail.insert("count".to_string(), DetailValue::Text(n.to_string()));
    detail.insert(
        "records".to_string(),
        DetailValue::List(
            split
                .pre_existing
                .iter()
                .take(BASELINE_RECORDS_SHOWN)
                .map(|p| p.display().to_string())
                .collect(),
        ),
    );
    Some(Notice {
        severity: NoticeSeverity::Info,
        subject: NOTICE_DRIFT_BASELINED.to_string(),
        message: format!(
            "{n} record{} drifted before drift_baseline '{git_ref}' ({}) and {} not reported as drift; drift introduced since it is",
            if n == 1 { "" } else { "s" },
            &commit[..commit.len().min(12)],
            if n == 1 { "is" } else { "are" },
        ),
        detail: Some(detail),
    })
}

/// One warning notice per key the `harness` configuration carries that this
/// build does not read (`ADR-89` amendment of 2026-11-07: reported, not an
/// error). Each names where the key sits (`subject`), the key, and the nearest
/// valid key when one is close, as `detail` fields a reader can branch on.
fn harness_key_notices(harness: Option<&urzua_core::harness::HarnessDecl>) -> Vec<Notice> {
    let Some(harness) = harness else {
        return Vec::new();
    };
    harness
        .unknown_keys
        .iter()
        .map(|u| {
            let hint = u
                .nearest
                .as_ref()
                .map(|n| format!("; did you mean '{n}'?"))
                .unwrap_or_default();
            let mut detail = std::collections::BTreeMap::new();
            detail.insert("key".to_string(), DetailValue::Text(u.key.clone()));
            if let Some(n) = &u.nearest {
                detail.insert("nearest".to_string(), DetailValue::Text(n.clone()));
            }
            detail.insert("valid".to_string(), DetailValue::List(u.valid.clone()));
            Notice {
                severity: NoticeSeverity::Warning,
                subject: u.subject.clone(),
                message: format!(
                    "{} has the key '{}', which this build does not read{hint} (it reads: {}); the setting has no effect",
                    u.subject,
                    u.key,
                    u.valid.join(", ")
                ),
                detail: Some(detail),
            }
        })
        .collect()
}

/// What the untracked-records option owes the reader (`ADR-92`). Without it, one
/// info notice naming the untracked records under a declared directory that the
/// run did not examine; with it, one counting the included records whose
/// `Realized-by` locator drift could not judge for want of history. Neither is
/// emitted for a proposed record (`--stdin-path`), which is judged alone.
fn untracked_notices(
    session: &Session,
    scope: &ReportScope,
    drift: &crate::discovery::DriftedRecords,
) -> Vec<Notice> {
    if scope.prospective.is_some() {
        return Vec::new();
    }
    if session.config.discovery.include_untracked {
        return crate::discovery::no_history_notice(&drift.no_history)
            .into_iter()
            .collect();
    }
    let skipped =
        match crate::discovery::untracked_record_files(&session.repo_root, &session.config) {
            Ok(listed) => listed,
            Err(e) => {
                return vec![Notice {
                    severity: NoticeSeverity::Warning,
                    subject: crate::discovery::NOTICE_UNTRACKED_SKIPPED.to_string(),
                    message: format!("could not list the untracked records: {e}"),
                    detail: None,
                }]
            }
        };
    // A record named on argv was examined, and one outside the requested paths
    // was never in question.
    let skipped: Vec<PathBuf> = skipped
        .into_iter()
        .filter(|p| !session.discovered.paths.contains(p))
        .filter(|p| scope.requested.is_empty() || scope.requested.iter().any(|r| p.starts_with(r)))
        .collect();
    crate::discovery::skipped_untracked_notice(&skipped)
        .into_iter()
        .collect()
}

// ADR-80 (amendment 2026-11-08): a declared ignore marker's excused lines are counted, never hidden.
/// One info notice per rule whose declared `ignore_marker` excused lines this
/// run, so an adopter reads how much a marker is hiding (`ADR-80`).
fn excused_line_notices(executions: &[RuleExecution]) -> Vec<Notice> {
    executions
        .iter()
        .filter_map(|e| {
            let excused = e.population.as_ref()?.excused_lines();
            (excused > 0).then(|| Notice {
                severity: NoticeSeverity::Info,
                subject: e.rule.clone(),
                message: format!(
                    "ignore_marker -- {excused} line{} excused from this rule; a line carrying the declared marker is neither reported nor counted as evidence",
                    if excused == 1 { "" } else { "s" }
                ),
                detail: None,
            })
        })
        .collect()
}

// ADR-90: a rule that declares scope: changed says how many slots it did not judge, and why.
/// One info notice per rule that declared `scope: changed` and left slots
/// unjudged (`ADR-90`): the rule, the option, how many slots and why. Without
/// it a plain `check` would show a nudge rule examining nothing with no cause.
fn scope_changed_notices(executions: &[RuleExecution], why: &str) -> Vec<Notice> {
    let mut notices: Vec<Notice> = executions
        .iter()
        .filter(|e| !rules::PATH_SCOPED_SOURCE_RULES.contains(&e.rule.as_str()))
        .filter_map(|e| {
            let deferred = e.population.as_ref()?.deferred_by_scope();
            (deferred > 0).then(|| Notice {
                severity: NoticeSeverity::Info,
                subject: e.rule.clone(),
                message: format!(
                    "scope: changed -- {deferred} slot{} not judged ({why}); this rule judges only the records a --base diff adds or changes",
                    if deferred == 1 { "" } else { "s" }
                ),
                detail: None,
            })
        })
        .collect();
    notices.extend(source_scope_notices(executions));
    notices
}

/// One info notice per source-reading rule that a path scope left files unread
/// for (`MILE-207`): how many of the files under its declared paths it did not
/// read, and why, so a rule that read little is not read as one that found
/// little.
fn source_scope_notices(executions: &[RuleExecution]) -> Vec<Notice> {
    executions
        .iter()
        .filter(|e| rules::PATH_SCOPED_SOURCE_RULES.contains(&e.rule.as_str()))
        .filter_map(|e| {
            let p = e.population.as_ref()?;
            let (deferred, eligible) = (p.deferred_by_scope(), p.eligible());
            (deferred > 0).then(|| Notice {
                severity: NoticeSeverity::Info,
                subject: e.rule.clone(),
                message: format!(
                    "path scope -- {deferred} of {eligible} source file{} under back_pointer_paths not judged: \
                     this run judges the files under the requested paths, and the source comments that cite a record in scope; \
                     a run with no path argument reads them all",
                    if eligible == 1 { "" } else { "s" }
                ),
                detail: None,
            })
        })
        .collect()
}

// Implements: ADR-86 (the report counts permanent waivers by category)
/// The permanent waivers in effect, counted by category (`ADR-86`), when the
/// suppression type declares `permanent`. Said on every run, including with
/// none: a repository that governs permanence is shown how much of it there
/// is, and a run that found nothing is distinguishable from one that did not
/// look. Counted over the whole corpus, whatever the scope of the run, so the
/// number does not change with the paths asked about.
fn permanent_waivers_notice(
    suppression_type: &urzua_core::waiver::SuppressionType,
    waivers: &[urzua_core::waiver::Suppression],
) -> Option<Notice> {
    suppression_type.permanent.as_ref()?;
    let mut by_category: std::collections::BTreeMap<&str, usize> = Default::default();
    for category in waivers.iter().filter_map(|w| w.permanent_category()) {
        *by_category.entry(category).or_default() += 1;
    }
    let total: usize = by_category.values().sum();
    let plural = if total == 1 { "" } else { "s" };
    let breakdown = if by_category.is_empty() {
        String::new()
    } else {
        let parts: Vec<String> = by_category
            .iter()
            .map(|(category, n)| format!("{category}: {n}"))
            .collect();
        format!(" ({})", parts.join(", "))
    };
    Some(Notice {
        severity: NoticeSeverity::Info,
        subject: crate::NOTICE_PERMANENT_WAIVERS.to_string(),
        message: format!("{total} permanent waiver{plural} in effect{breakdown}"),
        detail: None,
    })
}

/// Stage 1a: discovery, with argv paths overriding it (SPEC-2, BUG-24) and
/// `--base` computing the changed set against the full tracked set.
fn open_session(
    paths: &[PathBuf],
    config_path: Option<PathBuf>,
    base: Option<&str>,
    stdin_path: Option<&Path>,
    proposed: Option<String>,
) -> Result<Opened, Failure> {
    let mut requested_scopes = Vec::new();
    let mut changed = None;
    let session = Session::open_with(paths, config_path, |repo_root, _config, discovered| {
        requested_scopes = crate::discovery::relative_scopes(repo_root, paths)?;
        if !requested_scopes.is_empty() {
            // What git tracks or has staged. A record the option included is
            // not in it, so a file named on argv keeps its `BUG-24` treatment
            // whatever the option (`ADR-92` clause 5).
            let tracked: Vec<PathBuf> = discovered
                .paths
                .iter()
                .filter(|p| !discovered.untracked_included.contains(*p))
                .cloned()
                .collect();
            let extra =
                crate::discovery::resolve_argv_overrides(repo_root, &requested_scopes, &tracked)?;
            // The diff lists tracked paths, and the included untracked records
            // are added to it below, so only a named file the option would not
            // include could never be examined: refusing is the alternative to
            // dropping it in silence (`BUG-24`'s failure shape).
            if base.is_some()
                && extra
                    .iter()
                    .any(|p| !discovered.untracked_included.contains(p))
            {
                return Err(
                    "--base cannot be combined with a named file git does not track; \
                 drop --base or track the file"
                        .to_string(),
                );
            }
            if !extra.is_empty() {
                discovered.paths.extend(extra);
                discovered.paths.sort();
                discovered.paths.dedup();
                discovered.source = urzua_io::DiscoverySource::Argv;
            }
        }
        if let Some(base) = base {
            let mut changes =
                urzua_io::changed_paths_since(repo_root, base).map_err(|e| e.to_string())?;
            // An included untracked record has no merge-base copy: it is new
            // by definition, so the diff rules judge it as an addition
            // (`ADR-92` clause 6).
            changes.add_untracked(&discovered.untracked_included);
            changed = Some(changes);
            discovered.source = urzua_io::DiscoverySource::GitDiff;
        }
        Ok(())
    })?;
    let mut session = session;
    let prospective = match stdin_path {
        Some(p) => Some(propose(&mut session, p, proposed)?),
        None => None,
    };
    if let Some(p) = &prospective {
        requested_scopes = vec![p.path.clone()];
    }
    Ok(Opened {
        session,
        requested_scopes,
        changed,
        prospective,
    })
}

/// Puts records that exist only in memory into the session, as `propose` does
/// for one, without narrowing what the run reports on.
fn add_records(session: &mut Session, added: Vec<(PathBuf, String)>) -> Result<(), String> {
    if added.is_empty() {
        return Ok(());
    }
    for (rel, content) in added {
        let record = crate::discovery::parse_prospective_record(&session.config, &rel, &content)?;
        session.records.retain(|r| r.path != rel);
        session.records.push(record);
        session.full_text.insert(rel.clone(), content);
        if !session.discovered.paths.contains(&rel) {
            session.discovered.paths.push(rel);
        }
    }
    session.discovered.paths.sort();
    Ok(())
}

/// Reads the proposed content from stdin and puts it in the session in place of
/// the record's on-disk copy, adding the record (and its path) when the file
/// does not exist yet. A path no record type governs is a could-not-run.
fn propose(
    session: &mut Session,
    stdin_path: &Path,
    proposed: Option<String>,
) -> Result<Prospective, String> {
    let rel = crate::discovery::relative_to_root(&session.repo_root, stdin_path)?;
    let content = match proposed {
        Some(content) => content,
        None => {
            let mut content = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut content)
                .map_err(|e| format!("could not read the proposed content from stdin: {e}"))?;
            content
        }
    };
    let record = crate::discovery::parse_prospective_record(&session.config, &rel, &content)?;
    session.records.retain(|r| r.path != rel);
    session.records.push(record);
    session.full_text.insert(rel.clone(), content.clone());
    if !session.discovered.paths.contains(&rel) {
        session.discovered.paths.push(rel.clone());
        session.discovered.paths.sort();
    }
    Ok(Prospective { path: rel, content })
}

/// Stage 1b: the records the run reports on. A diff run examines the changed
/// records and every record a change reaches, not the changed files alone
/// (`MILE-123`).
fn narrow(
    session: &Session,
    requested: Vec<PathBuf>,
    changed: Option<urzua_io::GitChanges>,
    prospective: Option<Prospective>,
) -> Result<ReportScope, String> {
    let mut scoped = scope_to_requested_paths(&session.discovered.paths, &requested);
    let mut diff = None;
    let mut reached_before_narrowing = 0usize;
    if let Some(changes) = &changed {
        let examination = crate::diff_scope::examine(
            &session.repo_root,
            &session.config,
            &session.config_path,
            &session.records,
            changes,
        )?;
        reached_before_narrowing = examination.examined.len();
        scoped.retain(|p| examination.examined.contains(p));
        let mut scope = examination.scope;
        let kept: HashSet<&Path> = scoped.iter().map(|p| p.as_path()).collect();
        scope.dependents.retain(|d| kept.contains(Path::new(d)));
        diff = Some(scope);
    }
    Ok(ReportScope {
        requested,
        changed,
        diff,
        in_scope: scoped.into_iter().collect(),
        reached_before_narrowing,
        prospective,
    })
}

/// A `claim_paths` entry that is absent is a `Notice` (the rule has no input);
/// one that is actively wrong aborts. Either is visible, where a silent
/// `ran` with zero findings read as a clean corpus (`BUG-56`).
fn claim_path_notices(repo_root: &Path, opts: &rules::ClaimOpts) -> Result<Vec<Notice>, String> {
    let mut notices = Vec::new();
    // Both sides canonicalised: the repo root may itself reach through a link
    // (macOS `/tmp`).
    let root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());
    for prefix in &opts.claim_paths {
        let resolved = repo_root.join(prefix).canonicalize();
        match &resolved {
            // Only `NotFound` is absence: git keeps no empty directory, so
            // `.changeset` ceases to exist when a release consumes its last
            // fragment. `PermissionDenied` is not absence and must not read
            // as it (`BUG-56`).
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => notices.push(Notice {
                severity: NoticeSeverity::Warning,
                subject: rules::RULE_CLAIM_STATUS_AGREEMENT.to_string(),
                message: format!(
                    "claim.status-agreement: claim_paths entry '{prefix}' does not exist -- the rule has no input"
                ),
                detail: None,
            }),
            Err(e) => {
                return Err(format!(
                    "claim.status-agreement: claim_paths entry '{prefix}' could not be read: {e}"
                ))
            }
            Ok(r) if !r.is_dir() => {
                return Err(format!(
                    "claim.status-agreement: claim_paths entry '{prefix}' is not a directory"
                ))
            }
            // A link is followed where it stays inside the repository
            // (`BUG-69`, `BUG-80`).
            Ok(r) if !path_is_inside_repo(&root, r) => {
                return Err(format!(
                    "claim.status-agreement: claim_paths entry '{prefix}' resolves outside the repository"
                ))
            }
            Ok(_) => {}
        }
    }
    Ok(notices)
}

/// Drift and claim ages cost several `git` subprocesses per `Realized-by`
/// record, so they are computed only when `embodiment.consistency` will read
/// them (`BUG-115`).
///
/// A proposed record (`--stdin-path`) is the only one reported on, so only it
/// is computed, from its proposed content (`MILE-191`, as `fix --stdin-path`).
fn load_drift(
    session: &Session,
    scope: &ReportScope,
    blame: &urzua_io::BlameCache,
) -> (crate::discovery::DriftedRecords, HashMap<PathBuf, u64>) {
    let (repo_root, config) = (&session.repo_root, &session.config);
    if enabled_setting(config, rules::RULE_EMBODIMENT_CONSISTENCY).is_none() {
        return (Default::default(), HashMap::new());
    }
    let prospective = scope.prospective.as_ref();
    // Under a path or diff scope only the records in scope are reported on, and
    // a record's drift reaches no other record's finding, so the others are not
    // blamed: that history is most of a run's cost.
    let only: Vec<urzua_core::record::Record> = match prospective {
        Some(p) => session
            .records
            .iter()
            .filter(|r| r.path == p.path)
            .cloned()
            .collect(),
        None if !scope.whole_corpus() => session
            .records
            .iter()
            .filter(|r| scope.in_scope.contains(&r.path))
            .cloned()
            .collect(),
        None => Vec::new(),
    };
    let records = if prospective.is_some() || !scope.whole_corpus() {
        only.as_slice()
    } else {
        session.records.as_slice()
    };
    let now = urzua_io::now_unix();
    let mut drift = match prospective {
        Some(p) => crate::discovery::compute_drifted_records_at(
            repo_root,
            records,
            config,
            now,
            Some(&p.content),
            blame,
        ),
        None => crate::discovery::compute_drifted_records_at(
            repo_root, records, config, now, None, blame,
        ),
    };
    let (ages, problems) = crate::discovery::compute_claim_ages(records, config, now, blame);
    for problem in problems {
        if !drift.git_problems.contains(&problem) {
            drift.git_problems.push(problem);
        }
    }
    (drift, ages)
}

/// `embodiment.claim-expired`'s measures for the records the run reports on
/// (`ADR-97`): under a path or diff scope the others are not blamed, and a
/// proposed record (`--stdin-path`) has no commit for its line to be dated by.
fn load_claim_measures(
    session: &Session,
    scope: &ReportScope,
    blame: &urzua_io::BlameCache,
    notices: &mut Vec<Notice>,
) -> (HashMap<PathBuf, i64>, HashMap<PathBuf, u64>) {
    let config = &session.config;
    let only: Vec<urzua_core::record::Record>;
    let records: &[urzua_core::record::Record] = if let Some(p) = scope.prospective.as_ref() {
        only = session
            .records
            .iter()
            .filter(|r| r.path == p.path)
            .cloned()
            .collect();
        &only
    } else if !scope.whole_corpus() {
        only = session
            .records
            .iter()
            .filter(|r| scope.in_scope.contains(&r.path))
            .cloned()
            .collect();
        &only
    } else {
        session.records.as_slice()
    };
    let measures =
        crate::discovery::compute_claim_measures(records, config, urzua_io::now_unix(), blame);
    notices.extend(crate::discovery::drift_notices(false, &measures.problems));
    (measures.line_times, measures.seeds)
}

/// Whether a run with this `--rules` selection (`None`: every enabled rule) runs
/// any of `ids`.
fn selection_wants(selection: Option<&[String]>, ids: &[&str]) -> bool {
    selection.is_none_or(|s| s.iter().any(|r| ids.contains(&r.as_str())))
}

/// The files a run reads for the rules that judge them: claims, published
/// files, index artifacts, the override log and the back-pointer sources.
struct SourceInputs {
    claims: Vec<(String, String)>,
    published_files: Vec<rules::PublishedFile>,
    boundaries: Vec<rules::ResolvedBoundary>,
    ignore_marker: Option<String>,
    index_artifacts: Vec<rules::IndexArtifact>,
    override_log: Option<rules::OverrideLog>,
    back_pointer_sources: Option<rules::BackPointerSources>,
    decision_gate_sources: Option<rules::BackPointerSources>,
    code_citation_sources: Option<rules::BackPointerSources>,
}

/// Stage 2a: read the source files the rules are handed.
fn read_source_inputs(
    session: &Session,
    scope: &ReportScope,
    selection: Option<&[String]>,
    notices: &mut Vec<Notice>,
) -> Result<SourceInputs, String> {
    let (repo_root, config, discovered) =
        (&session.repo_root, &session.config, &session.discovered);
    // A `--rules` selection reads only what its rules read: the source files
    // under `back_pointer_paths` and the published boundary files are most of a
    // narrow run's cost on a large tree, and a rule that was not selected
    // never looks at them.
    let wanted = |ids: &[&str]| selection_wants(selection, ids);

    let claim_opts =
        config.enabled_rule_opts::<rules::ClaimOpts>(rules::RULE_CLAIM_STATUS_AGREEMENT);
    if let Some(opts) = &claim_opts {
        notices.extend(claim_path_notices(repo_root, opts)?);
    }
    let claims = read_claim_files(
        repo_root,
        claim_opts.as_ref().map_or(&[][..], |o| &o.claim_paths),
    )?;
    if let Some(opts) = &claim_opts {
        notices.extend(rules::claim_field_notices(&claims, opts));
    }

    let boundary_opts: rules::PublishedPathOpts = config
        .enabled_rule_opts(rules::RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE)
        .unwrap_or_default();
    let boundary_paths: Vec<String> = boundary_opts
        .boundaries
        .iter()
        .flat_map(|b| b.paths.iter().cloned())
        .collect();
    let published_files = if wanted(&[rules::RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE]) {
        crate::discovery::read_files_matching(
            repo_root,
            discovered,
            &boundary_paths,
            rules::RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE,
        )?
    } else {
        Vec::new()
    };
    let (boundaries, ignore_marker) = (boundary_opts.boundaries, boundary_opts.ignore_marker);

    let index_specs: Vec<urzua_core::config::IndexSpec> = config
        .enabled_rule_opts::<rules::IndexOpts>(rules::RULE_INDEX_FRESHNESS)
        .map_or_else(Vec::new, |o| o.indexes);
    let index_artifacts = read_index_artifacts(repo_root, &index_specs)?;
    let override_log = crate::discovery::read_override_log(repo_root, config)?;

    let (back_pointer_sources, decision_gate_sources, code_citation_sources) =
        read_comment_sources(session, scope, selection)?;
    Ok(SourceInputs {
        claims,
        published_files,
        boundaries,
        ignore_marker,
        index_artifacts,
        override_log,
        back_pointer_sources,
        decision_gate_sources,
        code_citation_sources,
    })
}

/// What a path-scoped run reads source files for (`MILE-207`), or `None` for a
/// run that reads them all: a whole-corpus run, and a `--base` run, whose
/// reading scope is the corpus whatever it reports on.
fn path_scoped_read(scope: &ReportScope) -> Option<rules::ScopedRead> {
    (!scope.requested.is_empty() && scope.diff.is_none()).then(|| rules::ScopedRead {
        paths: scope.requested.clone(),
        records: scope.in_scope.clone(),
        unread: Vec::new(),
    })
}

/// The identifiers, as the record index keys them, of the records in scope.
fn scope_record_ids(
    session: &Session,
    scope: &ReportScope,
) -> HashSet<urzua_core::values::RecordId> {
    if scope.requested.is_empty() || scope.diff.is_some() {
        return HashSet::new();
    }
    let (index, _) = rules::build_index_reporting_collisions(&session.records);
    index
        .into_iter()
        .filter(|(_, record)| scope.in_scope.contains(&record.path))
        .map(|(id, _)| id)
        .collect()
}

/// The source files the three source-comment rules read, as each of them is
/// handed them. A path-scoped run reads the files under the requested paths and
/// those that cite a record in scope, not every file (`MILE-207`).
type CommentSources = (
    Option<rules::BackPointerSources>,
    Option<rules::BackPointerSources>,
    Option<rules::BackPointerSources>,
);

fn read_comment_sources(
    session: &Session,
    scope: &ReportScope,
    selection: Option<&[String]>,
) -> Result<CommentSources, String> {
    let (repo_root, config, discovered) =
        (&session.repo_root, &session.config, &session.discovered);
    let wanted = |ids: &[&str]| selection_wants(selection, ids);
    let scoped = path_scoped_read(scope);
    let ids = scope_record_ids(session, scope);
    let prefixes = rules::declared_prefixes(config);
    let need = |reported: bool| {
        scoped.as_ref().map(|scoped| crate::discovery::ScopeNeed {
            scoped,
            ids: &ids,
            prefixes: &prefixes,
            reported,
            cited: true,
        })
    };
    // The back-pointer evidence also feeds `embodiment.consistency` and
    // `status.delivery-agreement` (`back_pointed`).
    let back_pointer_sources = if wanted(&[
        rules::RULE_EMBODIMENT_BACK_POINTER_RESOLUTION,
        rules::RULE_EMBODIMENT_CONSISTENCY,
        rules::RULE_STATUS_DELIVERY_AGREEMENT,
    ]) {
        let reported = wanted(&[rules::RULE_EMBODIMENT_BACK_POINTER_RESOLUTION]);
        crate::discovery::read_back_pointer_sources(
            repo_root,
            discovered,
            config,
            need(reported).as_ref(),
        )?
    } else {
        None
    };
    let decision_gate_sources = if wanted(&[rules::RULE_EMBODIMENT_BACK_POINTER_DECISION_ACCEPTED])
    {
        crate::discovery::read_decision_gate_sources(
            repo_root,
            discovered,
            config,
            need(true).as_ref(),
        )?
    } else {
        None
    };
    let code_citation_sources = if wanted(&[rules::RULE_CODE_DANGLING_BUG_REFERENCE]) {
        crate::discovery::read_code_citation_sources(
            repo_root,
            discovered,
            config,
            scoped.as_ref(),
        )?
    } else {
        None
    };
    Ok((
        back_pointer_sources,
        decision_gate_sources,
        code_citation_sources,
    ))
}

/// Stage 2b: the index every reference resolves against, with the notices that
/// disclose what the corpus holds (`BUG-116`).
fn index_with_disclosures<'a>(
    session: &'a Session,
    notices: &mut Vec<Notice>,
) -> (rules::RecordIndex<'a>, Vec<rules::IdentifierCollision<'a>>) {
    let config = &session.config;
    // Built once and shared by every rule that resolves a reference against
    // the whole corpus (`BUG-116`).
    let (record_index, collisions) = rules::build_index_reporting_collisions(&session.records);
    notices.extend(rules::identity_collision_notices(&collisions));
    // A record satisfying a required section through an alias is disclosed
    // (`MILE-165`), only when `section.required` is on.
    if enabled_setting(config, rules::RULE_SECTION_REQUIRED).is_some() {
        notices.extend(rules::section_alias_notices(
            &session.records,
            config,
            &session.full_text,
        ));
    }

    // Every waived Done-means item is listed, by category, never omitted
    // (`ADR-96` section 3), whenever a rule that judges them is on.
    if enabled_setting(config, rules::RULE_STATUS_WAIVED_ITEM_STALE).is_some()
        || enabled_setting(config, rules::RULE_STATUS_DELIVERY_AGREEMENT).is_some()
    {
        notices.extend(rules::waived_item_notices(&session.records, config));
    }

    // A declared non-reference id that now matches a real record is disclosed
    // (`ADR-75`), only when `citation.resolution` is on.
    if enabled_setting(config, rules::RULE_CITATION_RESOLUTION).is_some() {
        notices.extend(rules::non_reference_notices(
            &session.records,
            config,
            &record_index,
        ));
    }
    (record_index, collisions)
}

/// What a diff run judges a change against: the committed copies, the added and
/// changed paths, the fetched pull-request claims and the templates.
struct DiffInputs {
    base_text: HashMap<PathBuf, String>,
    base_drift: HashMap<PathBuf, HashSet<String>>,
    added_row_days: HashMap<PathBuf, rules::RowWindows>,
    added_paths: Option<Vec<PathBuf>>,
    pull_ref_claims: rules::PullRefClaims,
    templates: urzua_core::new_record::TemplateValues,
    template_texts: HashMap<String, String>,
    changed_paths: Option<Vec<PathBuf>>,
    committed_statuses: Option<HashMap<PathBuf, rules::CommittedStatus>>,
    deleted_records: Vec<rules::DeletedRecord>,
    base_authority: urzua_core::authority::BaseAuthority,
    gate_config: Option<Config>,
    fail_on: urzua_core::config::FailOn,
}

/// What had drifted at the merge base in each changed record that gained a
/// drift-acknowledging `Re-verified` row, for
/// `embodiment.claim-renewal-unattested` to hold the row's named locators
/// against. Empty unless that rule is enabled on a `--base` run.
fn load_base_drift(
    session: &Session,
    changes: Option<&urzua_io::GitChanges>,
    base_text: &HashMap<PathBuf, String>,
) -> HashMap<PathBuf, HashSet<String>> {
    let mut out = HashMap::new();
    let (config, repo_root) = (&session.config, &session.repo_root);
    let Some(changes) = changes else {
        return out;
    };
    if enabled_setting(config, rules::RULE_EMBODIMENT_CLAIM_RENEWAL_UNATTESTED).is_none() {
        return out;
    }
    let cache = urzua_io::BlameCache::new(repo_root);
    let now = urzua_io::now_unix();
    for record in &session.records {
        let Some(then_text) = base_text.get(&record.path) else {
            continue;
        };
        let (id, shape) = (
            config.revision_log_id(&record.record_type),
            config.revision_log_shape(&record.record_type),
        );
        let Ok(now_text) = urzua_io::read_to_string(&repo_root.join(&record.path)) else {
            continue;
        };
        let before = urzua_core::reverify::acknowledgements(then_text, &id, &shape);
        let gained = urzua_core::reverify::acknowledgements(&now_text, &id, &shape)
            .into_iter()
            .any(|a| !before.iter().any(|b| b.row == a.row));
        if !gained {
            continue;
        }
        if let Some(drifted) = crate::discovery::compute_base_drift(
            repo_root,
            config,
            &changes.merge_base,
            &record.path,
            then_text,
            now,
            &cache,
        ) {
            out.insert(record.path.clone(), drifted);
        }
    }
    out
}

/// The window of UTC days (start, unix seconds) in which each `Re-verified`
/// row a changed record gained since the merge base may be dated, keyed by the
/// row, for `embodiment.claim-renewal-unattested` (`ADR-97` item 6): from the
/// author day of the branch's first commit to the later of the author and
/// committer day of the commit the
/// row is blamed to in the head (today for a row not yet committed). A row git
/// cannot date has no entry and is not judged on its date.
fn load_added_row_days(
    session: &Session,
    changes: Option<&urzua_io::GitChanges>,
    base_text: &HashMap<PathBuf, String>,
) -> HashMap<PathBuf, rules::RowWindows> {
    const DAY: i64 = 86_400;
    let mut out = HashMap::new();
    let (config, repo_root) = (&session.config, &session.repo_root);
    let Some(changes) = changes else {
        return out;
    };
    if enabled_setting(config, rules::RULE_EMBODIMENT_CLAIM_RENEWAL_UNATTESTED).is_none() {
        return out;
    }
    let cache = urzua_io::BlameCache::new(repo_root);
    let today = urzua_io::now_unix().div_euclid(DAY) * DAY;
    // One day of slack before the branch began: `reverify` may run just before
    // UTC midnight and the first commit be authored just after it. A branch
    // with no commit yet starts at its merge base, so a row written yesterday
    // and not yet committed passes. The merge-base fallback also fires when
    // `git log` itself errors (the window then starts at the merge base too).
    let branch_start = urzua_io::branch_first_author_unix_time(repo_root, &changes.merge_base)
        .or_else(|| urzua_io::commit_author_unix_time(repo_root, &changes.merge_base))
        .map(|t| t.div_euclid(DAY) * DAY - DAY);
    for record in &session.records {
        let Some(then_text) = base_text.get(&record.path) else {
            continue;
        };
        let (id, shape) = (
            config.revision_log_id(&record.record_type),
            config.revision_log_shape(&record.record_type),
        );
        let Ok(now_text) = urzua_io::read_to_string(&repo_root.join(&record.path)) else {
            continue;
        };
        let before = urzua_core::reverify::acknowledgements(then_text, &id, &shape);
        let mut days = HashMap::new();
        for ack in urzua_core::reverify::acknowledgements(&now_text, &id, &shape)
            .into_iter()
            .filter(|a| !before.iter().any(|b| b.row == a.row))
        {
            let Ok(Some(commit)) = cache.commit_for_line(&record.path, ack.line, None) else {
                continue;
            };
            let day = if commit.chars().all(|c| c == '0') {
                Some(today)
            } else {
                // The later of the author and committer day: an amend or a
                // fixup keeps the author time and moves the committer time.
                let (author, committer) = (
                    cache.commit_author_unix_time(&commit),
                    cache.commit_unix_time(&commit),
                );
                author
                    .into_iter()
                    .chain(committer)
                    .max()
                    .map(|t| t.div_euclid(DAY) * DAY)
            };
            if let Some(day) = day {
                days.insert(ack.row, (branch_start.map_or(day, |s| s.min(day)), day));
            }
        }
        if !days.is_empty() {
            out.insert(record.path.clone(), days);
        }
    }
    out
}

/// The types an authority status gate names (the head's and the base
/// revision's), read only when `diff.decider-required` will use them.
fn gated_type_names<'a>(
    rule_config: &'a Config,
    base_authority: &'a urzua_core::authority::BaseAuthority,
) -> std::collections::BTreeSet<&'a str> {
    if enabled_setting(rule_config, rules::RULE_DIFF_DECIDER_REQUIRED).is_none() {
        return Default::default();
    }
    let mut types = rule_config
        .authority
        .as_ref()
        .map(|a| a.gated_types())
        .unwrap_or_default();
    if let Some(base) = base_authority.declared() {
        types.extend(base.gated_types());
    }
    types
}

/// The records the diff deleted that are not a rename, for the gated types.
/// Under `base`/`both` a deleted file is a record of a gated type when the
/// base revision's declaration owns it too, so a pull request cannot exclude
/// or move the record in the change that deletes it.
fn load_deleted_records(
    session: &Session,
    changes: Option<&urzua_io::GitChanges>,
    base_authority: &urzua_core::authority::BaseAuthority,
    gated_types: &std::collections::BTreeSet<&str>,
    notices: &mut Vec<Notice>,
    warned: &mut HashSet<PathBuf>,
) -> Vec<rules::DeletedRecord> {
    let Some(c) = changes else {
        return Vec::new();
    };
    let base_types = match gate_mode(session, base_authority) {
        urzua_core::authority::DeciderTable::Base | urzua_core::authority::DeciderTable::Both
            if !gated_types.is_empty() =>
        {
            load_base_record_types(session, c, notices)
        }
        _ => None,
    };
    crate::diff_scope::deleted_gated_records(
        &session.repo_root,
        &session.config,
        base_types.as_ref(),
        &session.records,
        c,
        gated_types,
        notices,
        warned,
    )
}

/// Stage 2c: read what the diff rules judge a change against.
fn read_diff_inputs(
    session: &Session,
    scope: &ReportScope,
    notices: &mut Vec<Notice>,
) -> Result<DiffInputs, String> {
    let (repo_root, config) = (&session.repo_root, &session.config);
    // The diff rules judge a changed record against a committed copy. A
    // proposed record is the one changed record, and its committed copy is
    // HEAD's (`MILE-191`): status transitions and version bumps are then
    // judged on a single-record edit instead of reported not-applicable.
    let head_changes = scope
        .prospective
        .as_ref()
        .map(|p| head_changes_for(repo_root, &p.path));
    let changes = scope.changed.as_ref().or(head_changes.as_ref());
    let base_text = load_base_text(session, changes, notices)?;
    let base_drift = load_base_drift(session, changes, &base_text);
    let added_row_days = load_added_row_days(session, changes, &base_text);
    let added_paths: Option<Vec<PathBuf>> = changes.map(|c| {
        let mut added: Vec<PathBuf> = c.added.iter().cloned().collect();
        added.sort();
        added
    });
    let pull_ref_claims = load_pull_ref_claims(session, added_paths.as_deref(), notices);
    let templates = load_templates(repo_root, config, notices)?;
    let template_texts = load_template_texts(repo_root, config)?;
    // The diff rules judge the changed set itself; `None` without `--base`.
    let changed_paths = changes.map(|c| c.changed.iter().cloned().collect());
    let base_gate = load_base_gate(session, changes, notices);
    let base_authority = base_gate.authority.clone();
    let (gate_config, fail_on) =
        apply_base_gate(&session.config, &base_gate, changes.is_some(), notices);
    let rule_config = gate_config.as_ref().unwrap_or(&session.config);
    let gated_types = gated_type_names(rule_config, &base_authority);
    // One warning per unreadable deleted base copy, whichever reader reaches it
    // first; the deletion judging goes first so the decider rule owns it.
    let mut warned = HashSet::new();
    let deleted_records = load_deleted_records(
        session,
        changes,
        &base_authority,
        &gated_types,
        notices,
        &mut warned,
    );
    let committed_statuses = changes.map(|c| {
        crate::diff_scope::committed_statuses(
            repo_root,
            &session.config,
            &session.records,
            c,
            &gated_types,
            notices,
            &mut warned,
        )
    });
    Ok(DiffInputs {
        base_text,
        base_drift,
        added_row_days,
        added_paths,
        pull_ref_claims,
        templates,
        template_texts,
        changed_paths,
        committed_statuses,
        deleted_records,
        base_authority,
        gate_config,
        fail_on,
    })
}

/// Applies `authority.waivable` (`ADR-89` amendment of 2026-11-09, gate
/// waivers) to the waivers of `diff.decider-required`: under `never` none
/// counts, under `base` only one the diff leaves untouched (so the base revision
/// holds it as it is) counts, and a run with no diff cannot tell, so none does.
/// A waiver of any other rule is untouched. Each ignored waiver is named in an
/// info notice, so it is never silently dropped.
///
/// This relies on `Suppression.id` being the waiver's repo-relative path: the
/// diff's changed-path set is matched against it to tell a touched waiver from
/// an untouched one.
fn restrict_decider_waivers(
    waivers: Vec<urzua_core::waiver::Suppression>,
    waivable: urzua_core::authority::Waivable,
    changed: Option<&[PathBuf]>,
    notices: &mut Vec<Notice>,
) -> Vec<urzua_core::waiver::Suppression> {
    use urzua_core::authority::Waivable;
    let rule = rules::RULE_DIFF_DECIDER_REQUIRED;
    if waivable == Waivable::Any {
        return waivers;
    }
    let mut ignored: Vec<String> = Vec::new();
    let kept = waivers
        .into_iter()
        .filter(|w| {
            if w.rule != rule {
                return true;
            }
            let held_by_base = waivable == Waivable::Base
                && changed.is_some_and(|c| !c.iter().any(|p| p == std::path::Path::new(&w.id)));
            if !held_by_base {
                ignored.push(w.id.clone());
            }
            held_by_base
        })
        .collect();
    if !ignored.is_empty() {
        ignored.sort();
        notices.push(Notice {
            severity: NoticeSeverity::Info,
            subject: rule.to_string(),
            message: format!(
                "authority.waivable is '{}': {} ignored for {rule}, so a gated change is cleared by a decider, not a record ({}): {}",
                waivable.as_str(),
                if ignored.len() == 1 { "a waiver is" } else { "waivers are" },
                if waivable == Waivable::Never {
                    "no waiver of this rule counts"
                } else {
                    "only a waiver the base revision holds unchanged counts"
                },
                ignored.join(", ")
            ),
            detail: None,
        });
    }
    kept
}

/// What the base revision's config says about the decider gate
/// (`authority.decider_table`, `ADR-89` amendment of 2026-11-09): its
/// `authority` table, its `diff.decider-required` setting and its `fail_on`,
/// all from the same config text. Read on every run with a diff, whatever the
/// head declares, because a pull request can drop the head's `authority`, turn
/// the rule off or lower `fail_on`. A config the base does not have is
/// `Absent`; one that cannot be read or loaded is said in a warning notice
/// (only when the head declares the gate) and the head table is used.
fn load_base_gate(
    session: &Session,
    changes: Option<&urzua_io::GitChanges>,
    notices: &mut Vec<Notice>,
) -> urzua_core::authority::BaseGate {
    use urzua_core::authority::{BaseAuthority, BaseGate};
    let rule = rules::RULE_DIFF_DECIDER_REQUIRED;
    let Some(changes) = changes else {
        return BaseGate::absent();
    };
    let head_declares =
        session.config.authority.is_some() || enabled_setting(&session.config, rule).is_some();
    let unreadable = |why: String, notices: &mut Vec<Notice>| {
        if head_declares {
            notices.push(Notice {
                severity: NoticeSeverity::Warning,
                subject: rule.to_string(),
                message: format!("{why}, so authority.decider_table cannot read the base table and the head table is used"),
                detail: None,
            });
        }
        BaseGate {
            authority: BaseAuthority::Unreadable,
            ..BaseGate::absent()
        }
    };
    let rel = match crate::discovery::relative_to_root(&session.repo_root, &session.config_path) {
        Ok(rel) => rel,
        Err(e) => {
            return unreadable(
                format!("the config path is not inside the repository ({e})"),
                notices,
            )
        }
    };
    let text =
        match urzua_io::read_at_commit_if_present(&session.repo_root, &changes.merge_base, &rel) {
            Ok(text) => text,
            Err(e) => {
                return unreadable(
                    format!(
                        "{} could not be read at the merge base ({e})",
                        rel.display()
                    ),
                    notices,
                )
            }
        };
    urzua_core::authority::BaseGate::from_config_text(text.as_deref(), rule)
        .unwrap_or_else(|why| unreadable(why, notices))
}

/// The mode in force: the stricter of the head's `authority.decider_table`
/// and the base revision's.
fn gate_mode(
    session: &Session,
    base: &urzua_core::authority::BaseAuthority,
) -> urzua_core::authority::DeciderTable {
    urzua_core::authority::effective_mode(session.config.authority.as_ref(), base)
}

/// The base revision's `record_types`, read from its whole config. A config the
/// base lacks gives none; one that does not load is said in a warning notice
/// and the head's declarations alone are used.
fn load_base_record_types(
    session: &Session,
    changes: &urzua_io::GitChanges,
    notices: &mut Vec<Notice>,
) -> Option<std::collections::BTreeMap<String, urzua_core::config::RecordTypeConfig>> {
    let rel = crate::discovery::relative_to_root(&session.repo_root, &session.config_path).ok()?;
    let text = urzua_io::read_at_commit_if_present(&session.repo_root, &changes.merge_base, &rel)
        .ok()??;
    match urzua_core::config::parse(&text) {
        Ok(base) => Some(base.record_types),
        Err(e) => {
            notices.push(Notice {
                severity: NoticeSeverity::Warning,
                subject: rules::RULE_DIFF_DECIDER_REQUIRED.to_string(),
                message: format!("the base revision's config does not load ({e}), so a deleted record is matched to its type by the head's declarations only"),
                detail: None,
            });
            None
        }
    }
}

/// The notices the decider gate owes a run: the missing-actors one and the
/// "no --base" decider-table one.
fn decider_gate_notices(
    config: &Config,
    context: &Context<'_>,
    (has_base, has_actors): (bool, bool),
    selection: Option<&[String]>,
) -> Vec<Notice> {
    let mut out = actors_notices(
        context.gate_config.as_ref().unwrap_or(config),
        has_actors,
        has_base,
        selection,
    );
    out.extend(decider_table_notices(
        config, has_base, has_actors, selection,
    ));
    out
}

/// Whether a `diff.decider-required` setting asks for evidence.
fn wants_evidence(setting: &urzua_core::config::RuleSetting) -> bool {
    use urzua_core::config::RuleOpts;
    urzua_core::rules::DeciderRequiredOpts::from_setting(setting).require_evidence
}

/// The stricter of the head's and the base's `diff.decider-required` setting,
/// level and option by option: the higher level (with the setting that holds it),
/// and `require_evidence` when either side sets it, the rule's only option.
fn stricter_decider_setting(
    head: Option<&urzua_core::config::RuleSetting>,
    base: &urzua_core::config::RuleSetting,
) -> urzua_core::config::RuleSetting {
    let head_level = head.map_or(RuleLevel::Off, |s| s.level);
    let mut merged = if base.level > head_level {
        base.clone()
    } else {
        head.cloned().unwrap_or_default()
    };
    if wants_evidence(base) || head.is_some_and(wants_evidence) {
        merged.require_evidence = Some(true);
    }
    merged
}

/// The waivers in force: the records' waivers, less those `authority.waivable`
/// ignores for `diff.decider-required`.
fn effective_waivers(
    session: &Session,
    context: &Context<'_>,
    notices: &mut Vec<Notice>,
) -> Vec<urzua_core::waiver::Suppression> {
    let loaded = load_waivers(session, notices);
    let waivable = urzua_core::authority::effective_waivable(
        session.config.authority.as_ref(),
        &context.base_authority,
    );
    restrict_decider_waivers(loaded, waivable, context.changed_paths.as_deref(), notices)
}

/// Applies the base revision's decider-gate settings under
/// `authority.decider_table: base` or `both` (the mode in force is the
/// stricter of the head's and the base's): the rule's level and options and
/// `fail_on` are the stricter of the base's and the head's, and an `authority`
/// the head dropped is replaced by an empty one so the base table is still
/// judged. Returns the config to run with (`None`: the head's own) and the
/// exit threshold. A base with no `authority` cannot be judged against, so the
/// head table is, and a notice says so.
fn apply_base_gate(
    config: &Config,
    base: &urzua_core::authority::BaseGate,
    has_base: bool,
    notices: &mut Vec<Notice>,
) -> (Option<Config>, urzua_core::config::FailOn) {
    use urzua_core::authority::{effective_mode, AuthorityDecl, DeciderTable};
    let rule = rules::RULE_DIFF_DECIDER_REQUIRED.to_string();
    let mode = effective_mode(config.authority.as_ref(), &base.authority);
    if !has_base || mode == DeciderTable::Head {
        return (None, config.fail_on);
    }
    let note = |message: String| Notice {
        severity: NoticeSeverity::Info,
        subject: rule.clone(),
        message,
        detail: None,
    };
    if base.authority.is_absent() {
        notices.push(note(format!(
            "authority.decider_table is '{}' but the base revision has no config file or no authority section, so it names no decider and no gate: the head table is judged instead",
            mode.as_str()
        )));
    }
    let mut adjusted = config.clone();
    let mut changed = false;
    if adjusted.authority.is_none() {
        adjusted.authority = Some(AuthorityDecl::none());
        changed = true;
        notices.push(note(
            "the head dropped the authority section; the base revision's is still judged"
                .to_string(),
        ));
    }
    let head_setting = config.rules.get(&rule);
    let head_level = head_setting.map_or(RuleLevel::Off, |s| s.level);
    if let Some(base_rule) = base.rule.as_ref() {
        let merged = stricter_decider_setting(head_setting, base_rule);
        if head_setting != Some(&merged) {
            adjusted.rules.insert(rule.clone(), merged);
            changed = true;
            if base_rule.level > head_level {
                notices.push(note(format!(
                    "{rule} is '{}' at the base revision and '{}' here: the base's stricter setting applies",
                    base_rule.level.as_str(),
                    head_level.as_str()
                )));
            }
            if wants_evidence(base_rule) && !head_setting.is_some_and(wants_evidence) {
                notices.push(note(format!(
                    "{rule} sets require_evidence at the base revision and not here: the base's stricter option applies"
                )));
            }
        }
    }
    let fail_on = config.fail_on.max(base.fail_on.unwrap_or_default());
    if fail_on != config.fail_on {
        notices.push(note(
            "fail_on is 'warning' at the base revision and 'error' here: the base's stricter threshold applies".to_string(),
        ));
    }
    (changed.then_some(adjusted), fail_on)
}

/// `authority.decider_table: base` or `both` names a base revision, and a run
/// with no `--base` has none: the head table is what is read (the harness
/// guardrails always read it), and the report says so.
fn decider_table_notices(
    config: &Config,
    has_base: bool,
    has_actors: bool,
    selection: Option<&[String]>,
) -> Vec<Notice> {
    use urzua_core::authority::DeciderTable;
    let rule = rules::RULE_DIFF_DECIDER_REQUIRED;
    let mode = config.authority.as_ref().and_then(|a| a.decider_table);
    // Said only when `--actors` names who made a change there is no base to
    // judge: a plain local check has nothing for the rule to judge.
    if has_base
        || !has_actors
        || !matches!(mode, Some(DeciderTable::Base | DeciderTable::Both))
        || enabled_setting(config, rule).is_none()
        || selection.is_some_and(|s| !s.iter().any(|r| r == rule))
    {
        return Vec::new();
    }
    vec![Notice {
        severity: NoticeSeverity::Info,
        subject: rule.to_string(),
        message: format!(
            "authority.decider_table is '{}' but this run has no --base, so there is no base revision to read: the head table is used",
            mode.map_or("head", DeciderTable::as_str)
        ),
        detail: None,
    }]
}

/// Stage 2: read everything the rules are handed.
fn load_context<'a>(
    session: &'a Session,
    scope: &ReportScope,
    selection: Option<&[String]>,
) -> Result<(Context<'a>, Vec<Notice>), String> {
    let (repo_root, config, discovered) =
        (&session.repo_root, &session.config, &session.discovered);
    let mut notices: Vec<Notice> = Vec::new();
    let sources = read_source_inputs(session, scope, selection, &mut notices)?;
    // One blame per file for the whole run, shared by drift, claim ages and
    // claim measures.
    let blame = urzua_io::BlameCache::new(repo_root);

    // Drift is read only by `embodiment.consistency`, so a selection that leaves
    // it out skips the git history it would cost.
    let drift_wanted =
        selection.is_none_or(|s| s.iter().any(|r| r == rules::RULE_EMBODIMENT_CONSISTENCY));
    let (drift, claim_ages) = if drift_wanted {
        load_drift(session, scope, &blame)
    } else {
        (Default::default(), HashMap::new())
    };
    notices.extend(crate::discovery::missing_record_notices(
        &discovered.unstaged_deletions,
    ));
    notices.extend(crate::discovery::drift_notices(
        drift.shallow,
        &drift.git_problems,
    ));

    let claim_wanted = selection_wants(selection, &[rules::RULE_EMBODIMENT_CLAIM_EXPIRED])
        && enabled_setting(config, rules::RULE_EMBODIMENT_CLAIM_EXPIRED).is_some();
    let (claim_line_times, claim_seeds) = if claim_wanted {
        load_claim_measures(session, scope, &blame, &mut notices)
    } else {
        (HashMap::new(), HashMap::new())
    };

    let (record_index, collisions) = index_with_disclosures(session, &mut notices);
    let back_pointed = cited_records(sources.back_pointer_sources.as_ref(), config, &record_index);
    let refs_snapshot = load_refs_with_notice(config, repo_root, selection, &mut notices);
    let diff = read_diff_inputs(session, scope, &mut notices)?;

    Ok((
        Context {
            session,
            claims: sources.claims,
            published_files: sources.published_files,
            boundaries: sources.boundaries,
            ignore_marker: sources.ignore_marker,
            index_artifacts: sources.index_artifacts,
            override_log: sources.override_log,
            back_pointer_sources: sources.back_pointer_sources,
            decision_gate_sources: sources.decision_gate_sources,
            code_citation_sources: sources.code_citation_sources,
            back_pointed,
            drift,
            claim_ages,
            claim_line_times,
            claim_seeds,
            now_unix: urzua_io::now_unix(),
            refs_snapshot,
            record_index,
            collisions,
            untracked_records: crate::discovery::untracked_records(
                repo_root,
                config,
                &discovered.paths,
            ),
            live_paths: discovered
                .paths
                .iter()
                .filter(|p| !discovered.staged_deletions.contains(*p))
                .cloned()
                .collect(),
            changed_paths: diff.changed_paths,
            added_paths: diff.added_paths,
            pull_ref_claims: diff.pull_ref_claims,
            committed_statuses: diff.committed_statuses,
            deleted_records: diff.deleted_records,
            actors: None,
            base_authority: diff.base_authority,
            gate_config: diff.gate_config,
            fail_on: diff.fail_on,
            repo_root: repo_root.clone(),
            base_text: diff.base_text,
            base_drift: diff.base_drift,
            added_row_days: diff.added_row_days,
            templates: diff.templates,
            template_texts: diff.template_texts,
            record_only: scope.prospective.is_some(),
            selected: selection.map(|s| s.iter().cloned().collect()),
        },
        notices,
    ))
}

/// The refs `identity.open-pr-collision` compares against (`ADR-93` clause 7): read
/// only when the rule is enabled and the run has a diff, from what is already in
/// `refs/urzua/pr`. `check` never fetches (clause 8); with no refs the rule's
/// notice says how to fetch them, so a run over nothing is not read as clean.
fn load_pull_ref_claims(
    session: &Session,
    added: Option<&[PathBuf]>,
    notices: &mut Vec<Notice>,
) -> rules::PullRefClaims {
    let (Some(added), Some(_)) = (
        added,
        enabled_setting(&session.config, rules::RULE_IDENTITY_OPEN_PR_COLLISION),
    ) else {
        return Default::default();
    };
    let added: HashSet<&PathBuf> = added.iter().collect();
    let ids: HashSet<urzua_core::values::RecordId> = session
        .records
        .iter()
        .filter(|r| added.contains(&r.path))
        .filter_map(rules::record_id)
        .map(|id| urzua_core::values::RecordId::new(&id))
        .collect();
    match crate::discovery::read_pull_ref_claims(&session.repo_root, &session.config, &ids) {
        Ok(claims) => {
            if claims.refs.is_empty() {
                let pattern = session
                    .config
                    .new
                    .pull_ref_patterns
                    .first()
                    .map_or("refs/pull/*/head", String::as_str);
                notices.push(rules::open_pr_missing_refs_notice(
                    &session.config.new.remote,
                    pattern,
                ));
            }
            claims
        }
        Err(e) => {
            notices.push(Notice {
                severity: NoticeSeverity::Warning,
                subject: rules::RULE_IDENTITY_OPEN_PR_COLLISION.to_string(),
                message: format!(
                    "could not read the fetched pull request refs ({e}), so the rule compared against nothing"
                ),
                detail: None,
            });
            Default::default()
        }
    }
}

/// Reads the actors file the gate action writes (`MILE-200`): JSON,
/// `{"schema": 1, "pr_author": LOGIN, "approvers": [LOGIN, ...]}`. A file that
/// cannot be read, is not JSON or does not have that shape is a could-not-run:
/// a run told who made a change and unable to read it must not pass as if it
/// had not been told.
fn read_actors(path: &Path) -> Result<urzua_core::authority::Actors, Failure> {
    let said = |why: String| -> Failure {
        format!("urzua check --actors {}: {why}", path.display()).into()
    };
    let text =
        std::fs::read_to_string(path).map_err(|e| said(format!("could not be read: {e}")))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| said(format!("is not valid JSON: {e}")))?;
    if value.get("schema").and_then(|s| s.as_u64()) != Some(1) {
        return Err(said("must carry \"schema\": 1".to_string()));
    }
    let pr_author = match value.get("pr_author") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Some(_) => return Err(said("pr_author must be a login string or null".to_string())),
    };
    let approvers = match value.get("approvers") {
        None | Some(serde_json::Value::Null) => Vec::new(),
        Some(serde_json::Value::Array(list)) => list
            .iter()
            .map(|v| match v.as_str().map(str::trim) {
                Some(s) if !s.is_empty() => Ok(s.to_string()),
                _ => Err(said(
                    "approvers must be a list of login strings".to_string(),
                )),
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => {
            return Err(said(
                "approvers must be a list of login strings".to_string(),
            ))
        }
    };
    Ok(urzua_core::authority::Actors {
        pr_author,
        approvers,
    })
}

/// An actors file nothing reads is said, never ignored (`MILE-200`): the rule is
/// off, or the run has no `--base` for it to judge.
fn actors_notices(
    config: &Config,
    given: bool,
    has_base: bool,
    selection: Option<&[String]>,
) -> Vec<Notice> {
    if !given {
        return Vec::new();
    }
    let rule = rules::RULE_DIFF_DECIDER_REQUIRED;
    let why = if enabled_setting(config, rule).is_none() {
        format!("{rule} is not enabled in the configuration")
    } else if selection.is_some_and(|s| !s.iter().any(|r| r == rule)) {
        format!("--rules leaves {rule} out")
    } else if !has_base {
        format!("{rule} judges a --base diff and this run has no --base")
    } else {
        return Vec::new();
    };
    vec![Notice {
        severity: NoticeSeverity::Warning,
        subject: rule.to_string(),
        message: format!("the --actors file was read by no rule: {why}"),
        detail: None,
    }]
}

/// Records a source comment cites (`MILE-128`).
fn cited_records(
    sources: Option<&rules::BackPointerSources>,
    config: &Config,
    index: &rules::RecordIndex,
) -> rules::BackPointerEvidence {
    sources
        .map(|s| {
            rules::back_pointer_evidence(
                &s.files,
                &s.back_pointer_paths,
                &s.test_paths,
                config,
                index,
            )
        })
        .unwrap_or_default()
}

/// The committed snapshot of issue and pull request numbers (`MILE-143`): a
/// file read, never a request, so `check` stays offline.
fn load_refs_with_notice(
    config: &Config,
    repo_root: &Path,
    selection: Option<&[String]>,
    notices: &mut Vec<Notice>,
) -> Result<RefsSnapshot, String> {
    let refs_snapshot = load_refs_snapshot(config, repo_root);
    // Said only when the rule that reads the snapshot is part of the run.
    let wanted = selection_wants(selection, &[rules::RULE_CITATION_ISSUE_EXISTS]);
    if let (true, Some(refs), Ok(snapshot)) =
        (wanted, issue_refs_declaration(config), &refs_snapshot)
    {
        notices.push(refs_notice(&refs.snapshot, snapshot));
    }
    refs_snapshot
}

fn refs_notice(snapshot_path: &str, snapshot: &RefsSnapshot) -> Notice {
    Notice {
        severity: NoticeSeverity::Info,
        subject: rules::RULE_CITATION_ISSUE_EXISTS.to_string(),
        message: format!(
            "citations were checked against {}, generated {} for {} ({} issues, {} pull requests); run `urzua refs refresh` to update it",
            snapshot_path,
            snapshot.generated_at,
            snapshot.repository,
            snapshot.issues.len(),
            snapshot.pulls.len(),
        ),
        detail: None,
    }
}

/// Stage 3a: the `Main` and `AfterRules` phases. `AfterRules` judges whether
/// the configuration reached what it declared (`MILE-106`), which is only
/// answerable once every other rule has reported.
fn run_rules(context: &Context) -> (Vec<RuleExecution>, Vec<Finding>) {
    let (may_exist, dir_exists) = probes(&context.repo_root);
    let texts =
        crate::file_texts::FileTexts::new(context.repo_root.clone(), &context.session.config);
    let read_text = |p: &str| texts.get(p);
    let mut executions = Vec::new();
    let mut findings = Vec::new();
    // `status.waived-item-stale` judges an expiry date against the run's day
    // (`ADR-96` section 3).
    let today = urzua_io::today();
    let main = context.rule_ctx(
        &may_exist,
        &dir_exists,
        &read_text,
        rules::RunState {
            today: &today,
            ..rules::RunState::empty()
        },
    );
    for (exec, rule_findings) in run_phase(context, rules::Phase::Main, &main) {
        executions.push(exec);
        findings.extend(rule_findings);
    }
    let after_rules = context.rule_ctx(
        &may_exist,
        &dir_exists,
        &read_text,
        rules::RunState {
            executions: &executions,
            ..rules::RunState::empty()
        },
    );
    let results = run_phase(context, rules::Phase::AfterRules, &after_rules);
    for (exec, rule_findings) in results {
        executions.push(exec);
        findings.extend(rule_findings);
    }
    (executions, findings)
}

/// One phase of rules. A proposed record (`--stdin-path`) runs only the rules
/// that can report on a record (`MILE-191`).
fn run_phase(
    context: &Context,
    phase: rules::Phase,
    ctx: &rules::RuleCtx<'_>,
) -> Vec<(RuleExecution, Vec<Finding>)> {
    rules::run_phase_with(
        phase,
        ctx,
        &rules::PhasePlan {
            record_only: context.record_only,
            selected: context.selected.as_ref(),
        },
    )
}

/// The two filesystem probes a rule may make.
fn probes(repo_root: &Path) -> (impl Fn(&Path) -> bool, impl Fn(&str) -> bool) {
    let (a, b) = (repo_root.to_path_buf(), repo_root.to_path_buf());
    (
        // A path that resolves outside the repository is never probed: it
        // reads as absent, not as "exists but untracked".
        move |p: &Path| {
            urzua_io::confine_to_root(&a, p).is_ok() && a.join(p).try_exists().unwrap_or(true)
        },
        move |d: &str| b.join(d).is_dir(),
    )
}

/// Stage 3b: keep a finding by the anchor its rule declares
/// (`rules::Anchor`), under a path scope and under a diff, then relativize the
/// config path.
fn scope_findings(session: &Session, scope: &ReportScope, findings: &mut Vec<Finding>) {
    let config = &session.config;
    if !scope.requested.is_empty() {
        let narrowing = rules::Narrowing::Paths(&scope.requested);
        findings.retain(|f| rules::finding_in_scope(config, f, &narrowing));
    }
    if let (Some(_), Some(changes)) = (&scope.diff, &scope.changed) {
        let examined = |p: &Path| scope.in_scope.contains(p);
        let narrowing = rules::Narrowing::Diff {
            examined: &examined,
            changed: &changes.changed,
        };
        findings.retain(|f| rules::finding_in_scope(config, f, &narrowing));
    }
    // A proposed record reports the findings on that record alone: a finding
    // anchored to another file concerns something this edit does not touch.
    if let Some(p) = &scope.prospective {
        findings.retain(|f| f.file == p.path);
    }
    // The config rules were handed the loaded (absolute) path, so the same
    // finding differed by checkout (BUG-185). Relativized after the filters
    // above compare against the loaded path, and before waivers match on it.
    // A suggested edit to the configuration names the same file, so it is
    // relativized with the finding (`ADR-73`: `file` is repository-relative).
    for f in findings.iter_mut() {
        if f.file == session.config_path.as_path() {
            if let Ok(rel) = f.file.strip_prefix(&session.repo_root) {
                f.file = rel.to_path_buf();
            }
        }
        if let Some(action) = f.suggested_action.as_mut() {
            if let Some(file) = action.target.file.as_mut() {
                if file == session.config_path.as_path() {
                    if let Ok(rel) = file.strip_prefix(&session.repo_root) {
                        *file = rel.to_path_buf();
                    }
                }
            }
        }
    }
}

/// Stage 3c: the `AfterScoping` phase, judged against the findings as they
/// stand before any waiver applies and after every scope filter: a waiver is
/// stale only against a run that could have produced its finding (`MILE-168`).
fn run_after_scoping(
    context: &Context,
    scope: &ReportScope,
    (waivers, today): (&[urzua_core::waiver::Suppression], &str),
    executions: &mut Vec<RuleExecution>,
    findings: &mut Vec<Finding>,
) {
    let (may_exist, dir_exists) = probes(&context.repo_root);
    let texts =
        crate::file_texts::FileTexts::new(context.repo_root.clone(), &context.session.config);
    let read_text = |p: &str| texts.get(p);
    let whole_corpus = scope.whole_corpus();
    let reached = |p: &Path| whole_corpus || scope.in_scope.contains(p);
    let ctx = context.rule_ctx(
        &may_exist,
        &dir_exists,
        &read_text,
        rules::RunState {
            executions,
            findings,
            waivers,
            whole_corpus,
            reached: &reached,
            today,
        },
    );
    let results = run_phase(context, rules::Phase::AfterScoping, &ctx);
    for (exec, rule_findings) in results {
        executions.push(exec);
        findings.extend(rule_findings);
    }
}

/// The status and the examined count, with the notice that explains an empty
/// diff. A diff in which no governed record changed and nothing reaches one
/// was asked about nothing and found nothing: `ok`, not `not-run`.
fn status_and_examined(
    session: &Session,
    scope: &ReportScope,
    executions: &[RuleExecution],
    findings: &[Finding],
    notices: &mut Vec<Notice>,
    base: Option<&str>,
) -> (urzua_core::report::ReportStatus, usize) {
    let examined = session
        .records
        .iter()
        .filter(|r| scope.in_scope.contains(r.path.as_path()))
        .count();
    let empty_diff = scope
        .diff
        .as_ref()
        .is_some_and(|d| examined == 0 && d.whole_corpus_reason.is_none());
    if let Some(d) = scope.diff.as_ref().filter(|_| empty_diff) {
        notices.push(empty_diff_notice(
            scope.reached_before_narrowing,
            d.changed_paths.len(),
            base.unwrap_or_default(),
        ));
    }
    let active = findings.iter().filter(|f| f.waived.is_none());
    let blocking = active.clone().any(|f| f.severity == FindingSeverity::Error);
    let status = derive_status(blocking, empty_diff, examined, executions, active.count());
    (status, examined)
}

fn empty_diff_notice(reached_before_narrowing: usize, changed_paths: usize, base: &str) -> Notice {
    let message = if reached_before_narrowing > 0 {
        format!(
            "the path argument excluded all {reached_before_narrowing} record(s) the change since {base} reached (changed or dependent); nothing was examined"
        )
    } else {
        format!(
            "no governed record changed since {base} ({changed_paths} changed path(s), none a record or the configuration); nothing was examined"
        )
    };
    Notice {
        severity: NoticeSeverity::Info,
        subject: "scope".to_string(),
        message,
        detail: None,
    }
}

/// `citation.issue-exists`'s `issue_refs`, when the rule is enabled.
fn issue_refs_declaration(
    config: &urzua_core::config::Config,
) -> Option<&urzua_core::config::IssueRefs> {
    config
        .rules
        .get(rules::RULE_CITATION_ISSUE_EXISTS)
        .filter(|s| s.level != urzua_core::config::RuleLevel::Off)
        .and_then(|s| s.issue_refs.as_ref())
}

/// The snapshot `citation.issue-exists` judges against, or why it cannot be
/// used. When the rule is not enabled nothing is read.
fn load_refs_snapshot(
    config: &urzua_core::config::Config,
    repo_root: &std::path::Path,
) -> Result<urzua_core::refs_snapshot::RefsSnapshot, String> {
    let Some(refs) = issue_refs_declaration(config) else {
        return Err("not enabled".to_string());
    };
    let path = repo_root.join(&refs.snapshot);
    urzua_io::confine_to_root(repo_root, &path)?;
    let text = std::fs::read_to_string(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            format!("{} does not exist", refs.snapshot)
        } else {
            format!("{} could not be read: {e}", refs.snapshot)
        }
    })?;
    let parsed: urzua_core::refs_snapshot::RefsSnapshot = serde_json::from_str(&text)
        .map_err(|e| format!("{} is not a valid snapshot: {e}", refs.snapshot))?;
    urzua_core::refs_snapshot::validate_snapshot(parsed, &refs.repository)
        .map_err(|why| format!("{} {why}", refs.snapshot))
}

#[cfg(test)]
mod selection_tests {
    use super::selection_wants as wanted;

    #[test]
    fn a_selection_wants_only_the_rules_it_names_and_no_selection_wants_all() {
        let only = vec!["header.required-fields".to_string()];
        assert!(wanted(None, &["embodiment.back-pointer-resolution"]));
        assert!(!wanted(
            Some(&only),
            &[
                "embodiment.back-pointer-resolution",
                "embodiment.consistency"
            ]
        ));
        assert!(wanted(Some(&only), &["header.required-fields"]));
    }
}
