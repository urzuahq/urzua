//! `urzua waive --from-report` (ADR-91, SPEC-25): drafts one pending waiver per
//! rule and file for the current findings. It runs `check` in process, so a
//! draft is never built from a stale report. Without `--apply` it previews and
//! writes nothing; with it, it writes each draft as `urzua new` writes a record.
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
use urzua_core::waive::{self, Filters, Group, PendingChoice, SkipReason};
use urzua_core::waiver::SuppressionType;

use super::check::{self, CheckArgs};
use super::new::{
    build_filename, disabled_pull_report, lookup_record_type, number_record, scan_pull_requests,
    write_new_file, Numbered,
};
use crate::emit;
use crate::session::Session;

/// The one header field the draft writes only when the waiver template lists it
/// and the type declares it (`MILE-205` decision of 2026-11-13).
const OWNER_FIELD: &str = "Owner";
const RULE_FIELD: &str = "Rule";
const SCOPE_FIELD: &str = "Scope";
/// A draft's title is cut here so a long path cannot make an over-long file name.
const MAX_TITLE_CHARS: usize = 100;
/// How many times a run takes a number again because another file claimed it.
const MAX_RETAKES: usize = 5;

/// The `urzua waive` command line.
#[derive(clap::Args)]
pub struct WaiveArgs {
    /// Draft from the current findings (the only source today).
    #[arg(long, required = true)]
    from_report: bool,
    /// Only findings of these rules (comma-separated or repeated).
    #[arg(long, value_delimiter = ',')]
    rule: Vec<String>,
    /// Only findings in this file or under this directory (repeatable; a path, not a glob).
    #[arg(long)]
    path: Vec<PathBuf>,
    /// Author named in each draft's revision log; defaults to the git identity.
    #[arg(long)]
    by: Option<String>,
    /// Write the drafts. Without it nothing is written.
    #[arg(long)]
    pub apply: bool,
}

pub fn run(config_path: Option<PathBuf>, args: WaiveArgs) -> ExitCode {
    match execute(config_path, args) {
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

fn execute(config_path: Option<PathBuf>, args: WaiveArgs) -> Result<WaiveReport, Failure> {
    let WaiveArgs {
        rule: rules,
        path: paths,
        by,
        apply,
        from_report: _,
    } = args;
    let ctx = open_ctx(config_path.clone(), by)?;
    let filters = filters(&ctx.session.repo_root, rules, &paths)?;
    let report = check::execute(config_path.clone(), CheckArgs::default())?;
    if report.status == ReportStatus::NotRun {
        return Err("`urzua check` established nothing (no rule examined a record), so there is no report to draft from".into());
    }
    let today = urzua_io::today();
    let waivers = urzua_core::waiver::load_suppressions(&ctx.session.records, &ctx.st);
    let records = waiver_records(&ctx);
    let plan = waive::plan(
        &report.findings,
        report.fail_on,
        (&waivers, &records),
        &today,
        &filters,
    );
    let mut skipped = plan.skipped;
    let mut drafts = Vec::new();
    let mut error = None;
    if !plan.groups.is_empty() {
        // Held to the end of the run, as `new` holds it: numbering and writing
        // happen under the lock.
        let (_lock, mut numbering) = number(&ctx, apply)?;
        let (groups, resolved) = settle(&ctx, &numbering, config_path, plan.groups)?;
        skipped.extend(resolved);
        let mut built = build(&ctx, &numbering, &groups)?;
        if apply {
            error = write_all(&ctx, &mut numbering, &groups, &mut built).err();
        }
        drafts = built.into_iter().map(|b| b.draft).collect();
    }
    Ok(WaiveReport {
        status: if error.is_some() {
            ReportStatus::NotRun
        } else {
            ReportStatus::Ok
        },
        mode: if apply { "apply" } else { "dry-run" },
        pending_value: ctx.pending.value,
        pending_source: ctx.pending.source,
        drafts,
        skipped,
        unmatched_filters: plan.unmatched,
        error,
    })
}

/// The paths of the suppression type's records: a finding inside one is a draft's
/// own, and is never drafted for.
fn waiver_records(ctx: &Ctx) -> std::collections::HashSet<PathBuf> {
    ctx.session
        .records
        .iter()
        .filter(|r| r.record_type == ctx.type_name)
        .map(|r| r.path.clone())
        .collect()
}

/// The numbering lock (with `new.check_open_prs`, taken only for `--apply`) and
/// the next free number, built once: its notices print once.
fn number(
    ctx: &Ctx,
    apply: bool,
) -> Result<(Option<super::new_refs::NumberingLock>, Numbered), Failure> {
    let session = &ctx.session;
    let mut pull = disabled_pull_report();
    let mut notices = Vec::new();
    let lock = if apply {
        scan_pull_requests(
            &session.repo_root,
            &session.config.new,
            &mut pull,
            &mut notices,
        )?
    } else {
        None
    };
    let numbering = number_record(
        &session.repo_root,
        &ctx.type_name,
        type_config(ctx),
        &session.config.new,
        &mut pull,
        &mut notices,
    )?;
    for n in &notices {
        eprintln!("urzua: {}", n.message);
    }
    Ok((lock, numbering))
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
    /// Minted with the text; reported only once the record is written.
    stable_id: String,
}

/// Drops the findings the drafts themselves resolve, judged on the tree with
/// the drafts in memory (`ADR-91`, amendment of 2026-11-08), and judges the kept
/// set again once, so a finding resolved only by a draft that is then dropped is
/// drafted after all. When nothing remains drafted, what disappeared did so only
/// through its own draft: it persists, and is reported as that and not resolved.
fn settle(
    ctx: &Ctx,
    numbering: &Numbered,
    config_path: Option<PathBuf>,
    groups: Vec<Group>,
) -> Result<(Vec<Group>, Vec<waive::Skipped>), Failure> {
    let after = judged_with(ctx, numbering, config_path.clone(), &groups)?;
    let first = waive::drop_resolved(groups, &after);
    if first.resolved.is_empty() {
        return Ok((first.kept, Vec::new()));
    }
    if first.kept.is_empty() {
        let own = waive::skipped_groups(&first.resolved, SkipReason::ResolvedByOwnDraft);
        return Ok((Vec::new(), own));
    }
    let again = judged_with(ctx, numbering, config_path, &first.kept)?;
    let second = waive::drop_resolved(first.resolved, &again);
    let skipped = waive::skipped_groups(&second.resolved, SkipReason::ResolvedByDrafts);
    Ok((waive::merge_groups(first.kept, second.kept), skipped))
}

/// The findings of a `check` over the corpus with `groups`' drafts held in memory.
fn judged_with(
    ctx: &Ctx,
    numbering: &Numbered,
    config_path: Option<PathBuf>,
    groups: &[Group],
) -> Result<Vec<urzua_core::report::Finding>, Failure> {
    let added = build(ctx, numbering, groups)?
        .into_iter()
        .map(|b| (b.path, b.content))
        .collect();
    let report = check::execute(
        config_path,
        CheckArgs {
            added_records: added,
            ..Default::default()
        },
    )?;
    Ok(report.findings)
}

/// The drafts for `groups`, numbered consecutively from `numbering`'s number.
fn build(ctx: &Ctx, numbering: &Numbered, groups: &[Group]) -> Result<Vec<Built>, String> {
    let type_config = type_config(ctx);
    let today = urzua_io::today();
    let type_prefix = type_config
        .prefix
        .clone()
        .unwrap_or_else(|| ctx.type_name.to_ascii_uppercase());
    let mut out = Vec::new();
    for (offset, group) in groups.iter().enumerate() {
        let number = numbering.display_number + offset as u32;
        let title: String = format!("{} in {}", group.rule, group.scope)
            .chars()
            .take(MAX_TITLE_CHARS)
            .collect();
        let (_, filename) = build_filename(
            &ctx.type_name,
            type_config,
            &title,
            number,
            numbering.declared.as_ref().zip(numbering.compiled.as_ref()),
        )?;
        let base = urzua_core::new_record::ScaffoldSpec::plain(
            type_config,
            numbering.compiled.as_ref(),
            &type_prefix,
            &filename,
            number,
            &title,
            "",
            &ctx.author,
            &today,
        );
        let (content, stable_id) = scaffold(ctx, group, &base);
        let path = PathBuf::from(&type_config.dir).join(&filename);
        out.push(Built {
            draft: draft_entry(&path, group),
            path,
            content,
            stable_id,
        });
    }
    Ok(out)
}

/// Writes every draft, in order, as `new` writes a record (confined to the
/// repository, never overwriting). A number another file took since numbering is
/// taken again and the remaining drafts renumbered; any other error stops the run,
/// and the drafts not yet written stay `written: false`.
fn write_all(
    ctx: &Ctx,
    numbering: &mut Numbered,
    groups: &[Group],
    built: &mut Vec<Built>,
) -> Result<(), String> {
    let root = &ctx.session.repo_root;
    let dir = root.join(&type_config(ctx).dir);
    let mut retakes = 0;
    let mut i = 0;
    while i < built.len() {
        let b = &built[i];
        match write_new_file(root, &dir, &root.join(&b.path), &b.content) {
            Ok(()) => {
                built[i].draft.written = true;
                built[i].draft.stable_id = Some(built[i].stable_id.clone());
                i += 1;
            }
            Err(e) if e.contains("already exists") && retakes < MAX_RETAKES => {
                retakes += 1;
                *numbering = number(ctx, false)?.1;
                let rest = build(ctx, numbering, &groups[i..])?;
                built.truncate(i);
                built.extend(rest);
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// The record text: the type's scaffold with the finding's rule and scope, the
/// pending `Reason`, the findings as the body and a revision-log row. `Expires`,
/// `Category` and `Justified-by` stay as the template leaves them.
fn scaffold(
    ctx: &Ctx,
    group: &Group,
    base: &urzua_core::new_record::ScaffoldSpec,
) -> (String, String) {
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
    let content = urzua_core::new_record::scaffold_record(
        &urzua_core::new_record::ScaffoldSpec {
            stable_id: stable_id.as_str(),
            omit_fields: &[waive::EXPIRES_FIELD],
            reason: Some(&ctx.pending.value),
            header_fields: &fields,
            extra_body: Some(&body),
            log_row: Some(&note),
            ..*base
        },
        ctx.template.as_deref(),
    );
    (content, stable_id.as_str().to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn waiver_type(required: &[&str], known: Option<&[&str]>) -> RecordTypeConfig {
        RecordTypeConfig {
            required_fields: required.iter().map(|s| s.to_string()).collect(),
            known_fields: known.map(|k| k.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        }
    }

    #[test]
    fn a_closed_field_set_that_omits_a_field_the_draft_writes_is_refused_by_name() {
        for missing in [RULE_FIELD, SCOPE_FIELD, waive::REASON_FIELD] {
            let required: Vec<&str> = [RULE_FIELD, SCOPE_FIELD, waive::REASON_FIELD]
                .into_iter()
                .filter(|f| *f != missing)
                .collect();
            let t = waiver_type(&required, Some(&["Expires"]));
            let err = require_fields_admitted("waiver", &t).unwrap_err();
            assert!(err.contains(&format!("does not list {missing}")), "{err}");
        }
    }

    #[test]
    fn a_closed_field_set_that_lists_them_and_an_open_one_are_accepted() {
        let all = [RULE_FIELD, SCOPE_FIELD, waive::REASON_FIELD];
        assert!(require_fields_admitted("waiver", &waiver_type(&all, Some(&["Expires"]))).is_ok());
        // Listed under `known_fields` rather than `required_fields` is declared too.
        assert!(require_fields_admitted("waiver", &waiver_type(&[], Some(&all))).is_ok());
        // No `known_fields` is an open set: nothing to refuse.
        assert!(require_fields_admitted("waiver", &waiver_type(&[], None)).is_ok());
    }

    #[test]
    fn owner_is_declared_only_when_the_template_lists_it_and_the_type_declares_it() {
        let template = "---\nRule: null\nOwner: null\n---\n# N\n";
        let no_owner = "---\nRule: null\n---\n# N\n";
        let closed_without = waiver_type(&["Rule"], Some(&["Expires"]));
        let closed_with = waiver_type(&["Rule"], Some(&["Owner"]));
        let open = waiver_type(&["Rule"], None);
        assert!(owner_declared(&closed_with, Some(template)));
        assert!(owner_declared(&open, Some(template)));
        // The type does not declare it, or the template does not list it, or no template.
        assert!(!owner_declared(&closed_without, Some(template)));
        assert!(!owner_declared(&closed_with, Some(no_owner)));
        assert!(!owner_declared(&closed_with, None));
    }
}
