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
use urzua_core::rules;

use crate::discovery::{
    compute_drifted_records, path_is_inside_repo, read_claim_files, read_index_artifacts,
    scope_to_requested_paths,
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
    back_pointer_sources: Option<rules::BackPointerSources>,
    decision_gate_sources: Option<rules::BackPointerSources>,
    back_pointed: rules::BackPointerEvidence,
    drift: crate::discovery::DriftedRecords,
    claim_ages: HashMap<PathBuf, u64>,
    refs_snapshot: Result<RefsSnapshot, String>,
    record_index: rules::RecordIndex<'a>,
    collisions: Vec<rules::IdentifierCollision<'a>>,
    untracked_records: rules::UntrackedRecords,
    live_paths: Vec<PathBuf>,
    changed_paths: Option<Vec<PathBuf>>,
    committed_statuses: Option<HashMap<PathBuf, rules::CommittedStatus>>,
    repo_root: PathBuf,
    base_text: HashMap<PathBuf, String>,
    templates: urzua_core::new_record::TemplateValues,
    /// A proposed record is being judged (`--stdin-path`).
    record_only: bool,
}

/// `revision-log.version-bump-required` judges a changed record against the
/// merge base's copy, so that copy is read here (`BUG-51`) and handed over as
/// data. Only the changed records of a type declaring a bump are read; a
/// record new in the diff has no merge-base copy and gets no entry.
fn load_base_text(
    session: &Session,
    changes: Option<&urzua_io::GitChanges>,
    notices: &mut Vec<Notice>,
) -> Result<HashMap<PathBuf, String>, String> {
    let mut out = HashMap::new();
    let (Some(changes), Some(_)) = (
        changes,
        enabled_setting(
            &session.config,
            rules::RULE_REVISION_LOG_VERSION_BUMP_REQUIRED,
        ),
    ) else {
        return Ok(out);
    };
    for record in &session.records {
        let declares = session
            .config
            .record_types
            .get(&record.record_type)
            .is_some_and(|t| t.version_bump().is_some());
        if !declares || !changes.changed.contains(&record.path) {
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
                subject: rules::RULE_REVISION_LOG_VERSION_BUMP_REQUIRED.to_string(),
                message: format!(
                    "{}: could not read the merge base's copy, so the record is not examined: {e}",
                    record.path.display()
                ),
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
fn load_templates(
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
            back_pointer_sources: self.back_pointer_sources.as_ref(),
            decision_gate_sources: self.decision_gate_sources.as_ref(),
            back_pointed: &self.back_pointed,
            drifted: &self.drift.drifted,
            fresh: &self.drift.fresh,
            claim_ages: &self.claim_ages,
            base_text: &self.base_text,
            changed_paths: self.changed_paths.as_deref(),
            committed_statuses: self.committed_statuses.as_ref(),
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

pub fn run(
    config_path: Option<PathBuf>,
    paths: Vec<PathBuf>,
    base: Option<String>,
    fail_on: Option<urzua_core::config::FailOn>,
    stdin_path: Option<PathBuf>,
) -> ExitCode {
    match execute(config_path, paths, base, fail_on, stdin_path) {
        Ok(report) => emit(&report),
        Err(e) => emit(&CouldNotRun::from(e)),
    }
}

fn execute(
    config_path: Option<PathBuf>,
    paths: Vec<PathBuf>,
    base: Option<String>,
    fail_on: Option<urzua_core::config::FailOn>,
    stdin_path: Option<PathBuf>,
) -> Result<CheckReport, String> {
    if stdin_path.is_some() && (!paths.is_empty() || base.is_some()) {
        return Err(
            "urzua check --stdin-path judges one record's proposed content and reports on it alone: it cannot be combined with PATHS or --base"
                .to_string(),
        );
    }
    let opened = open_session(&paths, config_path, base.as_deref(), stdin_path.as_deref())?;
    let session = &opened.session;
    let scope = narrow(
        session,
        opened.requested_scopes,
        opened.changed,
        opened.prospective,
    )?;
    let (context, mut notices) = load_context(session, &scope)?;
    let (mut executions, mut findings) = run_rules(&context);
    scope_findings(session, &scope, &mut findings);
    let waivers = urzua_core::waiver::load_suppressions(
        &session.records,
        &urzua_core::waiver::SuppressionType::from_config(&session.config),
    );
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
    Ok(CheckReport {
        status,
        files_examined,
        // Counted over the same set `files_examined` counts (`BUG-40`). Rules
        // run over the whole corpus because a reference resolves against
        // records outside the scope (`BUG-67`); what is *reported on* is the
        // scope.
        records_read_by_any_rule: urzua_core::report::records_read_by_any_rule_within(
            &executions,
            &scope.in_scope.iter().map(|p| p.as_path()).collect(),
        ),
        rules_executed: executions,
        scope: ScopeInfo {
            source: if scope.prospective.is_some() {
                urzua_core::report::ScopeSource::Stdin
            } else {
                crate::discovery::scope_source(session.discovered.source)
            },
            base,
            record_types: session.config.record_types.keys().cloned().collect(),
            diff: scope.diff,
        },
        blocking,
        findings,
        notices,
        // The flag wins over the configuration; either only moves the exit code.
        fail_on: fail_on.unwrap_or(session.config.fail_on),
    })
}

/// Stage 1a: discovery, with argv paths overriding it (SPEC-2, BUG-24) and
/// `--base` computing the changed set against the full tracked set.
fn open_session(
    paths: &[PathBuf],
    config_path: Option<PathBuf>,
    base: Option<&str>,
    stdin_path: Option<&Path>,
) -> Result<Opened, String> {
    let mut requested_scopes = Vec::new();
    let mut changed = None;
    let session = Session::open_with(paths, config_path, |repo_root, _config, discovered| {
        requested_scopes = crate::discovery::relative_scopes(repo_root, paths)?;
        if !requested_scopes.is_empty() {
            let extra = crate::discovery::resolve_argv_overrides(
                repo_root,
                &requested_scopes,
                &discovered.paths,
            )?;
            if !extra.is_empty() && base.is_some() {
                // The diff lists tracked paths only, so a named file outside the
                // tracked set could never be examined: refusing is the alternative
                // to dropping it in silence (`BUG-24`'s failure shape).
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
            changed =
                Some(urzua_io::changed_paths_since(repo_root, base).map_err(|e| e.to_string())?);
            discovered.source = urzua_io::DiscoverySource::GitDiff;
        }
        Ok(())
    })?;
    let mut session = session;
    let prospective = match stdin_path {
        Some(p) => Some(propose(&mut session, p)?),
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
fn propose(session: &mut Session, stdin_path: &Path) -> Result<Prospective, String> {
    let rel = crate::discovery::relative_to_root(&session.repo_root, stdin_path)?;
    let mut content = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut content)
        .map_err(|e| format!("could not read the proposed content from stdin: {e}"))?;
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
    prospective: Option<&Prospective>,
) -> (crate::discovery::DriftedRecords, HashMap<PathBuf, u64>) {
    let (repo_root, config) = (&session.repo_root, &session.config);
    if enabled_setting(config, rules::RULE_EMBODIMENT_CONSISTENCY).is_none() {
        return (Default::default(), HashMap::new());
    }
    let only: Vec<urzua_core::record::Record> = prospective
        .into_iter()
        .flat_map(|p| session.records.iter().find(|r| r.path == p.path).cloned())
        .collect();
    let records = if prospective.is_some() {
        only.as_slice()
    } else {
        session.records.as_slice()
    };
    let mut drift = match prospective {
        Some(p) => crate::discovery::compute_drifted_records_for_content(
            repo_root, records, config, &p.content,
        ),
        None => compute_drifted_records(repo_root, records, config),
    };
    let (ages, problems) =
        crate::discovery::compute_claim_ages(repo_root, records, config, urzua_io::now_unix());
    for problem in problems {
        if !drift.git_problems.contains(&problem) {
            drift.git_problems.push(problem);
        }
    }
    (drift, ages)
}

/// Stage 2: read everything the rules are handed.
fn load_context<'a>(
    session: &'a Session,
    scope: &ReportScope,
) -> Result<(Context<'a>, Vec<Notice>), String> {
    let (repo_root, config, discovered) =
        (&session.repo_root, &session.config, &session.discovered);
    let mut notices: Vec<Notice> = Vec::new();

    let claim_opts =
        config.enabled_rule_opts::<rules::ClaimOpts>(rules::RULE_CLAIM_STATUS_AGREEMENT);
    if let Some(opts) = &claim_opts {
        notices.extend(claim_path_notices(repo_root, opts)?);
    }
    let claims = read_claim_files(
        repo_root,
        &claim_opts.map_or_else(Vec::new, |o| o.claim_paths),
    )?;

    let boundary_opts: rules::PublishedPathOpts = config
        .enabled_rule_opts(rules::RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE)
        .unwrap_or_default();
    let boundary_paths: Vec<String> = boundary_opts
        .boundaries
        .iter()
        .flat_map(|b| b.paths.iter().cloned())
        .collect();
    let published_files = crate::discovery::read_files_matching(
        repo_root,
        discovered,
        &boundary_paths,
        rules::RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE,
    )?;
    let (boundaries, ignore_marker) = (boundary_opts.boundaries, boundary_opts.ignore_marker);

    let index_specs: Vec<urzua_core::config::IndexSpec> = config
        .enabled_rule_opts::<rules::IndexOpts>(rules::RULE_INDEX_FRESHNESS)
        .map_or_else(Vec::new, |o| o.indexes);
    let index_artifacts = read_index_artifacts(repo_root, &index_specs)?;

    let back_pointer_sources =
        crate::discovery::read_back_pointer_sources(repo_root, discovered, config)?;
    let decision_gate_sources =
        crate::discovery::read_decision_gate_sources(repo_root, discovered, config)?;

    let (drift, claim_ages) = load_drift(session, scope.prospective.as_ref());
    notices.extend(crate::discovery::drift_notices(
        drift.shallow,
        &drift.git_problems,
    ));

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

    // A declared non-reference id that now matches a real record is disclosed
    // (`ADR-75`), only when `citation.resolution` is on.
    if enabled_setting(config, rules::RULE_CITATION_RESOLUTION).is_some() {
        notices.extend(rules::non_reference_notices(
            &session.records,
            config,
            &record_index,
        ));
    }

    let back_pointed = cited_records(back_pointer_sources.as_ref(), config, &record_index);

    let refs_snapshot = load_refs_with_notice(config, repo_root, &mut notices);
    // The diff rules judge a changed record against a committed copy. A
    // proposed record is the one changed record, and its committed copy is
    // HEAD's (`MILE-191`): status transitions and version bumps are then
    // judged on a single-record edit instead of reported not-applicable.
    let head_changes = scope
        .prospective
        .as_ref()
        .map(|p| head_changes_for(repo_root, &p.path));
    let changes = scope.changed.as_ref().or(head_changes.as_ref());
    let base_text = load_base_text(session, changes, &mut notices)?;
    let templates = load_templates(repo_root, config, &mut notices)?;

    Ok((
        Context {
            session,
            claims,
            published_files,
            boundaries,
            ignore_marker,
            index_artifacts,
            back_pointer_sources,
            decision_gate_sources,
            back_pointed,
            drift,
            claim_ages,
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
            // The diff rules judge the changed set itself; `None` without `--base`.
            changed_paths: changes.map(|c| c.changed.iter().cloned().collect()),
            committed_statuses: changes.map(|c| {
                crate::diff_scope::committed_statuses(
                    repo_root,
                    &session.config,
                    &session.records,
                    c,
                )
            }),
            repo_root: repo_root.clone(),
            base_text,
            templates,
            record_only: scope.prospective.is_some(),
        },
        notices,
    ))
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
    }
}

/// Stage 3a: the `Main` and `AfterRules` phases. `AfterRules` judges whether
/// the configuration reached what it declared (`MILE-106`), which is only
/// answerable once every other rule has reported.
fn run_rules(context: &Context) -> (Vec<RuleExecution>, Vec<Finding>) {
    let (may_exist, dir_exists) = probes(&context.repo_root);
    let texts = crate::file_texts::FileTexts::new(context.repo_root.clone());
    let read_text = |p: &str| texts.get(p);
    let mut executions = Vec::new();
    let mut findings = Vec::new();
    let main = context.rule_ctx(
        &may_exist,
        &dir_exists,
        &read_text,
        rules::RunState::empty(),
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
    if context.record_only {
        rules::run_phase_for_record(phase, ctx)
    } else {
        rules::run_phase(phase, ctx)
    }
}

/// The two filesystem probes a rule may make.
fn probes(repo_root: &Path) -> (impl Fn(&Path) -> bool, impl Fn(&str) -> bool) {
    let (a, b) = (repo_root.to_path_buf(), repo_root.to_path_buf());
    (
        move |p: &Path| a.join(p).try_exists().unwrap_or(true),
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
    let texts = crate::file_texts::FileTexts::new(context.repo_root.clone());
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
