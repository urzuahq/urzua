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
use urzua_core::report::{
    CheckReport, CouldNotRun, DiffScope, Finding, FindingSeverity, Notice, NoticeSeverity,
    RuleExecution, ScopeInfo,
};
use urzua_core::report::{DetailValue, Failure};
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
    /// The identities from `--actors` (`MILE-200`).
    actors: Option<urzua_core::authority::Actors>,
    repo_root: PathBuf,
    base_text: HashMap<PathBuf, String>,
    templates: urzua_core::new_record::TemplateValues,
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
            config: &s.config,
            config_path: &s.config_path,
            records: &s.records,
            full_text: &s.full_text,
            templates: &self.templates,
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
            back_pointed: &self.back_pointed,
            drifted: &self.drift.drifted,
            fresh: &self.drift.fresh,
            claim_ages: &self.claim_ages,
            claim_line_times: &self.claim_line_times,
            claim_seeds: &self.claim_seeds,
            now_unix: self.now_unix,
            base_text: &self.base_text,
            changed_paths: self.changed_paths.as_deref(),
            added_paths: self.added_paths.as_deref(),
            pull_ref_claims: &self.pull_ref_claims,
            committed_statuses: self.committed_statuses.as_ref(),
            actors: self.actors.as_ref(),
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

fn execute(config_path: Option<PathBuf>, args: CheckArgs) -> Result<CheckReport, Failure> {
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
    } = args;
    let actors = actors_path.as_deref().map(read_actors).transpose()?;
    let opened = open_session(
        &paths,
        config_path,
        base.as_deref(),
        stdin_path.as_deref(),
        proposed,
    )?;
    let session = &opened.session;
    let selection = validate_selection(&session.config, &requested_rules)?;
    let scope = narrow(
        session,
        opened.requested_scopes,
        opened.changed,
        opened.prospective,
    )?;
    let (mut context, mut notices) = load_context(session, &scope, selection.as_deref())?;
    notices.extend(untracked_notices(session, &scope, &context.drift));
    notices.extend(harness_key_notices(session.config.harness.as_ref()));
    notices.extend(actors_notices(
        &session.config,
        actors.is_some(),
        base.is_some(),
        selection.as_deref(),
    ));
    context.actors = actors;
    notices.extend(extra_notices);
    let (mut executions, mut findings) = run_rules(&context);
    notices.extend(scope_changed_notices(
        &executions,
        &why_deferred(&scope, base.as_deref()),
    ));
    notices.extend(excused_line_notices(&executions));
    scope_findings(session, &scope, &mut findings);
    let waivers = load_waivers(session, &mut notices);
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
        blocking,
        findings,
        notices,
        // The flag wins over the configuration; either only moves the exit code.
        fail_on: fail_on.unwrap_or(session.config.fail_on),
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
            })
        })
        .collect()
}

// ADR-90: a rule that declares scope: changed says how many slots it did not judge, and why.
/// One info notice per rule that declared `scope: changed` and left slots
/// unjudged (`ADR-90`): the rule, the option, how many slots and why. Without
/// it a plain `check` would show a nudge rule examining nothing with no cause.
fn scope_changed_notices(executions: &[RuleExecution], why: &str) -> Vec<Notice> {
    executions
        .iter()
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
}

/// Stage 2a: read the source files the rules are handed.
fn read_source_inputs(
    session: &Session,
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

    // The back-pointer evidence also feeds `embodiment.consistency` and
    // `status.delivery-agreement` (`back_pointed`).
    let back_pointer_sources = if wanted(&[
        rules::RULE_EMBODIMENT_BACK_POINTER_RESOLUTION,
        rules::RULE_EMBODIMENT_CONSISTENCY,
        rules::RULE_STATUS_DELIVERY_AGREEMENT,
    ]) {
        crate::discovery::read_back_pointer_sources(repo_root, discovered, config)?
    } else {
        None
    };
    let decision_gate_sources = if wanted(&[rules::RULE_EMBODIMENT_BACK_POINTER_DECISION_ACCEPTED])
    {
        crate::discovery::read_decision_gate_sources(repo_root, discovered, config)?
    } else {
        None
    };
    Ok(SourceInputs {
        claims,
        published_files,
        boundaries,
        ignore_marker,
        index_artifacts,
        override_log,
        back_pointer_sources,
        decision_gate_sources,
    })
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
    added_paths: Option<Vec<PathBuf>>,
    pull_ref_claims: rules::PullRefClaims,
    templates: urzua_core::new_record::TemplateValues,
    changed_paths: Option<Vec<PathBuf>>,
    committed_statuses: Option<HashMap<PathBuf, rules::CommittedStatus>>,
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
    let added_paths: Option<Vec<PathBuf>> = changes.map(|c| {
        let mut added: Vec<PathBuf> = c.added.iter().cloned().collect();
        added.sort();
        added
    });
    let pull_ref_claims = load_pull_ref_claims(session, added_paths.as_deref(), notices);
    let templates = load_templates(repo_root, config, notices)?;
    // The diff rules judge the changed set itself; `None` without `--base`.
    let changed_paths = changes.map(|c| c.changed.iter().cloned().collect());
    let committed_statuses = changes.map(|c| {
        // The types an authority status gate names are read too, but only
        // when `diff.decider-required` will use them.
        let gated = if enabled_setting(config, rules::RULE_DIFF_DECIDER_REQUIRED).is_some() {
            config
                .authority
                .as_ref()
                .map(|a| a.gated_types())
                .unwrap_or_default()
        } else {
            Default::default()
        };
        crate::diff_scope::committed_statuses(
            repo_root,
            &session.config,
            &session.records,
            c,
            &gated,
            notices,
        )
    });
    Ok(DiffInputs {
        base_text,
        added_paths,
        pull_ref_claims,
        templates,
        changed_paths,
        committed_statuses,
    })
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
    let sources = read_source_inputs(session, selection, &mut notices)?;
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
    let refs_snapshot = load_refs_with_notice(config, repo_root, &mut notices);
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
            actors: None,
            repo_root: repo_root.clone(),
            base_text: diff.base_text,
            templates: diff.templates,
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
    notices: &mut Vec<Notice>,
) -> Result<RefsSnapshot, String> {
    let refs_snapshot = load_refs_snapshot(config, repo_root);
    if let (Some(refs), Ok(snapshot)) = (issue_refs_declaration(config), &refs_snapshot) {
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
