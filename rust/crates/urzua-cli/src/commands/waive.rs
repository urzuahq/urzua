//! `urzua waive --from-report` (ADR-91, SPEC-25): drafts one pending waiver per
//! rule and file for the current findings. It runs `check` in process, so a
//! draft is never built from a stale report, and this build only previews: no
//! file is written (`--apply` is a later change to `MILE-205`).
//!
//! The judging (which findings, how they group, what the `Reason` is) is pure
//! and lives in `urzua_core::waive`; the numbering and the scaffold are the
//! ones `urzua new` uses. Each function stays under 100 lines.

use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use urzua_core::config::RecordTypeConfig;
use urzua_core::report::{
    CouldNotRun, Failure, ReportStatus, WaiveDraft, WaiveFindings, WaiveReport,
};
use urzua_core::waive::{self, Filters, Group, PendingChoice};
use urzua_core::waiver::SuppressionType;

use super::check::{self, CheckArgs};
use super::new::{build_filename, disabled_pull_report, lookup_record_type, number_record};
use crate::emit;
use crate::session::Session;

/// The one header field the draft writes only when the waiver template lists it
/// and the type declares it (`MILE-205` decision of 2026-11-13).
const OWNER_FIELD: &str = "Owner";
const RULE_FIELD: &str = "Rule";
const SCOPE_FIELD: &str = "Scope";
/// A draft's title is cut here so a long path cannot make an over-long file name.
const MAX_TITLE_CHARS: usize = 100;

pub fn run(
    config_path: Option<PathBuf>,
    rules: Vec<String>,
    paths: Vec<PathBuf>,
    by: Option<String>,
) -> ExitCode {
    match execute(config_path, rules, paths, by) {
        Ok(report) => emit(&report),
        Err(e) => emit(&CouldNotRun::from(e)),
    }
}

/// Everything a draft is built from, resolved once.
struct Ctx {
    session: Session,
    type_name: String,
    st: SuppressionType,
    pending: PendingChoice,
    template: Option<String>,
    author: String,
    owner: bool,
}

fn type_config(ctx: &Ctx) -> &RecordTypeConfig {
    &ctx.session.config.record_types[&ctx.type_name]
}

fn execute(
    config_path: Option<PathBuf>,
    rules: Vec<String>,
    paths: Vec<PathBuf>,
    by: Option<String>,
) -> Result<WaiveReport, Failure> {
    let ctx = open_ctx(config_path.clone(), by)?;
    let filters = filters(&ctx.session.repo_root, rules, &paths)?;
    let report = check::execute(config_path.clone(), CheckArgs::default())?;
    if report.status == ReportStatus::NotRun {
        return Err("`urzua check` established nothing (no rule examined a record), so there is no report to draft from".into());
    }
    let today = urzua_io::today();
    let waivers = urzua_core::waiver::load_suppressions(&ctx.session.records, &ctx.st);
    let plan = waive::plan(&report.findings, report.fail_on, &waivers, &today, &filters);
    let (groups, resolved) = without_resolved(&ctx, config_path, plan.groups)?;
    let mut skipped = plan.skipped;
    skipped.extend(resolved);
    let drafts = build(&ctx, &groups)?.into_iter().map(|b| b.draft).collect();
    Ok(WaiveReport {
        status: ReportStatus::Ok,
        mode: "dry-run",
        pending_value: ctx.pending.value,
        pending_source: ctx.pending.source,
        drafts,
        skipped,
        unmatched_filters: plan.unmatched,
    })
}

/// Opens the repository and proves the draft can be written and would be
/// ineffective, before `check` runs: every refusal is exit 2 with nothing written.
fn open_ctx(config_path: Option<PathBuf>, by: Option<String>) -> Result<Ctx, Failure> {
    let session = Session::open(config_path)?;
    let type_name = session.config.suppression_type().to_string();
    let type_config = lookup_record_type(&session.config, &type_name, &session.config_path)?;
    require_fields_admitted(&type_name, type_config)?;
    let template_path = session
        .repo_root
        .join(".urzua/templates")
        .join(format!("{type_name}.md"));
    let template = urzua_io::read_to_string(&template_path).ok();
    let mut st = SuppressionType::from_config(&session.config);
    if let Some(text) = &template {
        st = st.with_template(text);
    }
    let pending = waive::choose_pending(&st)?;
    let identity = urzua_io::resolve_identity(by.as_deref(), &session.repo_root)
        .map_err(|e| format!("could not resolve an identity: {e}"))?;
    if let Some(warning) = identity.warning {
        eprintln!("urzua: {warning}");
    }
    let owner = owner_declared(type_config, template.as_deref());
    Ok(Ctx {
        session,
        type_name,
        st,
        pending,
        template,
        author: identity.name,
        owner,
    })
}

/// A closed field set that omits a field the draft writes would be reported by
/// `header.field-set-consistency` on every draft; refuse instead.
fn require_fields_admitted(type_name: &str, type_config: &RecordTypeConfig) -> Result<(), String> {
    if type_config.known_fields.is_none() {
        return Ok(());
    }
    let declared = type_config.declared_fields();
    for field in [RULE_FIELD, SCOPE_FIELD, waive::REASON_FIELD] {
        if !declared.contains(&urzua_core::values::FieldName::new(field)) {
            return Err(format!(
                "the {type_name} type declares a closed field set (`known_fields`) that does not list {field}, which every draft carries; add it to `required_fields` or `known_fields`"
            ));
        }
    }
    Ok(())
}

fn owner_declared(type_config: &RecordTypeConfig, template: Option<&str>) -> bool {
    let listed = template
        .map(urzua_core::new_record::template_header_fields)
        .unwrap_or_default()
        .iter()
        .any(|f| f == OWNER_FIELD);
    listed
        && (type_config.known_fields.is_none()
            || type_config
                .declared_fields()
                .contains(&urzua_core::values::FieldName::new(OWNER_FIELD)))
}

/// `--rule` ids this build ships, and `--path` values as repository-relative paths.
fn filters(repo_root: &Path, rules: Vec<String>, paths: &[PathBuf]) -> Result<Filters, String> {
    let mut out = Filters::default();
    for id in rules {
        let id = id.trim().to_string();
        if id.is_empty() {
            return Err("urzua waive --rule: an empty rule id (a stray comma?)".to_string());
        }
        if !urzua_core::rules::ALL_RULES.contains(&id.as_str()) {
            return Err(format!(
                "urzua waive --rule: '{id}' is not a rule this build ships (`urzua rules` lists them)"
            ));
        }
        if !out.rules.contains(&id) {
            out.rules.push(id);
        }
    }
    for p in paths {
        let rel: PathBuf = p
            .strip_prefix(repo_root)
            .unwrap_or(p)
            .components()
            .filter(|c| !matches!(c, Component::CurDir))
            .collect();
        if !out.paths.contains(&rel) {
            out.paths.push(rel);
        }
    }
    Ok(out)
}

/// One draft: its report entry and the record text it would be written as.
struct Built {
    draft: WaiveDraft,
    path: PathBuf,
    content: String,
}

/// Drops the findings the drafts themselves resolve, judged on the tree with
/// the drafts in memory (`ADR-91`, amendment of 2026-11-08).
fn without_resolved(
    ctx: &Ctx,
    config_path: Option<PathBuf>,
    groups: Vec<Group>,
) -> Result<(Vec<Group>, Vec<waive::Skipped>), Failure> {
    if groups.is_empty() {
        return Ok((groups, Vec::new()));
    }
    let added = build(ctx, &groups)?
        .into_iter()
        .map(|b| (b.path, b.content))
        .collect();
    let after = check::execute(
        config_path,
        CheckArgs {
            added_records: added,
            ..Default::default()
        },
    )?;
    Ok(waive::drop_resolved(groups, &after.findings))
}

/// The drafts for `groups`, numbered consecutively from the next free number.
fn build(ctx: &Ctx, groups: &[Group]) -> Result<Vec<Built>, String> {
    if groups.is_empty() {
        return Ok(Vec::new());
    }
    let type_config = type_config(ctx);
    let session = &ctx.session;
    let mut pull = disabled_pull_report();
    let mut notices = Vec::new();
    let first = number_record(
        &session.repo_root,
        &ctx.type_name,
        type_config,
        &session.config.new,
        &mut pull,
        &mut notices,
    )?;
    for n in &notices {
        eprintln!("urzua: {}", n.message);
    }
    let today = urzua_io::today();
    let type_prefix = type_config
        .prefix
        .clone()
        .unwrap_or_else(|| ctx.type_name.to_ascii_uppercase());
    let mut out = Vec::new();
    for (offset, group) in groups.iter().enumerate() {
        let number = first.display_number + offset as u32;
        let title: String = format!("{} in {}", group.rule, group.scope)
            .chars()
            .take(MAX_TITLE_CHARS)
            .collect();
        let (_, filename) = build_filename(
            &ctx.type_name,
            type_config,
            &title,
            number,
            first.declared.as_ref().zip(first.compiled.as_ref()),
        )?;
        let content = scaffold(
            ctx,
            group,
            &urzua_core::new_record::ScaffoldSpec::plain(
                type_config,
                first.compiled.as_ref(),
                &type_prefix,
                &filename,
                number,
                &title,
                "",
                &ctx.author,
                &today,
            ),
        );
        let path = PathBuf::from(&type_config.dir).join(&filename);
        out.push(Built {
            draft: draft_entry(&path, group),
            path,
            content,
        });
    }
    Ok(out)
}

/// The record text: the type's scaffold with the finding's rule and scope, the
/// pending `Reason`, the findings as the body and a revision-log row. `Expires`,
/// `Category` and `Justified-by` stay as the template leaves them.
fn scaffold(ctx: &Ctx, group: &Group, base: &urzua_core::new_record::ScaffoldSpec) -> String {
    let stable_id = urzua_id::StableId::generate();
    let mut fields: Vec<(&str, &str)> =
        vec![(RULE_FIELD, &group.rule), (SCOPE_FIELD, &group.scope)];
    if ctx.owner {
        fields.push((OWNER_FIELD, &ctx.author));
    }
    let body = findings_body(group);
    let note = format!(
        "Drafted by `urzua waive --from-report` (run by {}) from {} finding(s) of `{}` in `{}`; the {} is pending. **Why:** an unwaived finding at the time of drafting; a person writes the {} and the date.",
        ctx.author,
        group.findings.len(),
        group.rule,
        group.scope,
        waive::REASON_FIELD,
        waive::REASON_FIELD,
    );
    urzua_core::new_record::scaffold_record(
        &urzua_core::new_record::ScaffoldSpec {
            stable_id: stable_id.as_str(),
            reason: Some(&ctx.pending.value),
            header_fields: &fields,
            extra_body: Some(&body),
            log_row: Some(&note),
            ..*base
        },
        ctx.template.as_deref(),
    )
}

/// The finding messages, verbatim, one list item each.
fn findings_body(group: &Group) -> String {
    let items: Vec<String> = group
        .lines()
        .iter()
        .map(|l| format!("- {}", l.replace('\n', "\n  ")))
        .collect();
    format!("## Findings\n\n{}\n", items.join("\n"))
}

fn draft_entry(path: &Path, group: &Group) -> WaiveDraft {
    WaiveDraft {
        path: path.to_path_buf(),
        stable_id: None,
        rule: group.rule.clone(),
        scope: group.scope.clone(),
        findings: WaiveFindings {
            count: group.findings.len(),
            lines: group.lines(),
        },
        written: false,
    }
}
