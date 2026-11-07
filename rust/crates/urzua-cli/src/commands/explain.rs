//! `urzua explain` (SPEC-13): "which decisions govern this file" -- every
//! record whose `Realized-by` names `path` as evidence (ADR-0024). Given
//! `path#scope` it also resolves the scope and prints the span and its hash,
//! the authoring route for a scoped pin (ADR-74).
//!
//! `urzua explain --rule <rule-id>` is the other mode (`ADR-96` amendment of
//! 2026-11-07): what one rule is, as data, read from the same rule table
//! `urzua rules` prints. It is a flag and not a second positional form because
//! a path and a rule id cannot be told apart by their spelling: `status.x` is a
//! legal file name and a rule id may be a path.

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::config::LocatorKind;
use urzua_core::harness::nearest_name;
use urzua_core::locator::{self, FileText};
use urzua_core::report::{
    CoverageExplanation, ExplainReport, FindingSeverity, Notice, NoticeSeverity, Report,
    ReportStatus, ScopeExplanation,
};
use urzua_core::rules::{rule_def, RULE_TABLE};

use super::rules::RuleFacts;

use crate::emit;
use crate::session::{emit_or_not_run, Session};
use urzua_core::report::Failure;

pub fn run(config_path: Option<PathBuf>, path: Option<String>, rule: Option<String>) -> ExitCode {
    match (path, rule) {
        (Some(path), None) => emit_or_not_run(run_inner(config_path, path)),
        (None, Some(rule)) => emit_or_not_run(run_rule(config_path, &rule)),
        // clap rejects both and neither before this is reached.
        _ => emit_or_not_run(Err("give a path, or --rule <rule-id>".to_string().into())),
    }
}

/// One rule as `explain --rule` prints it: what `urzua rules` prints for it,
/// plus what this repository's configuration does with it.
#[derive(serde::Serialize)]
struct RuleExplanation {
    #[serde(flatten)]
    facts: RuleFacts,
    /// The level this repository sets; `off` when its configuration does not
    /// name the rule.
    level: &'static str,
    /// The strictest floor among the rule's variants: no level lowers a
    /// finding below it (`ADR-96`). Omitted when no variant has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    severity_floor: Option<FindingSeverity>,
    /// `false` when at least one variant cannot be waived by a rule-level
    /// waiver; the variants say which.
    waivable: bool,
    /// Every option the rule accepts, with the value this repository sets, or
    /// `null` when it sets none.
    options: serde_json::Map<String, serde_json::Value>,
}

#[derive(serde::Serialize)]
struct ExplainRuleReport {
    status: ReportStatus,
    rule: RuleExplanation,
}

impl Report for ExplainRuleReport {
    fn notices(&self) -> &[Notice] {
        &[]
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::SUCCESS
    }
}

fn run_rule(config_path: Option<PathBuf>, id: &str) -> Result<ExitCode, Failure> {
    let Some(def) = rule_def(id) else {
        let hint = nearest_name(id, RULE_TABLE.iter().map(|d| d.id))
            .map(|valid| format!("; did you mean '{valid}'?"))
            .unwrap_or_default();
        return Err(format!(
            "'{id}' is not a rule this build ships{hint} (`urzua rules` lists them)"
        )
        .into());
    };
    let Session { config, .. } = Session::open(config_path)?;
    let setting = config.rules.get(def.id);
    let configured = setting
        .and_then(|s| serde_json::to_value(s).ok())
        .and_then(|v| match v {
            serde_json::Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default();
    let options = def
        .options
        .iter()
        .map(|name| {
            (
                (*name).to_string(),
                configured
                    .get(*name)
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            )
        })
        .collect();
    let facts = RuleFacts::of(def);
    let severity_floor = def
        .actions
        .iter()
        .filter_map(|a| a.severity_floor)
        .max_by_key(|f| f.rank());
    let waivable = def.actions.iter().all(|a| a.waivable);
    Ok(emit(&ExplainRuleReport {
        status: ReportStatus::Ok,
        rule: RuleExplanation {
            facts,
            level: setting.map_or("off", |s| s.level.as_str()),
            severity_floor,
            waivable,
            options,
        },
    }))
}

fn run_inner(config_path: Option<PathBuf>, path: String) -> Result<ExitCode, Failure> {
    let Session {
        config,
        records,
        repo_root,
        ..
    } = Session::open(config_path)?;
    let parsed = locator::parse_locator(&path).map_err(|e| format!("'{path}': {e}"))?;
    if parsed.anchor.is_some() {
        return Err(format!(
            "'{path}' carries an anchor; give `path` or `path#scope`, the pin is printed for you"
        )
        .into());
    }
    crate::file_texts::FileTexts::new(repo_root.clone(), &config)
        .confined(&parsed.path)
        .map_err(|e| format!("'{path}': {e}"))?;
    let governing_records = urzua_core::graph::explain(&records, &parsed.path, &config);
    let (population, notice) = urzua_core::graph::read_population(&records, &config, "explain");
    let mut notices: Vec<Notice> = notice.into_iter().collect();
    let (scope, coverage) = match &parsed.scope {
        None => (
            None,
            explain_coverage(&repo_root, &config, &parsed, &mut notices)?,
        ),
        Some(scope) => (
            Some(explain_scope(&repo_root, &config, &parsed, scope)?),
            None,
        ),
    };

    Ok(emit(&ExplainReport {
        status: ReportStatus::Ok,
        path: parsed.path.clone(),
        governing_records,
        scope,
        coverage,
        population,
        notices,
    }))
}

// BUG-176: explain shows the files a directory or glob pin covers
/// The files a directory or glob argument covers and the pin over them. A file
/// argument, or a path that names nothing, has no coverage (the old query). A
/// kind the config does not accept is said so in a notice rather than shown, so
/// the output never offers a pin the check would reject.
fn explain_coverage(
    repo_root: &std::path::Path,
    config: &urzua_core::config::Config,
    parsed: &locator::Locator,
    notices: &mut Vec<Notice>,
) -> Result<Option<CoverageExplanation>, String> {
    use urzua_core::rules::{LocatorExistsOpts, RULE_EMBODIMENT_LOCATOR_EXISTS};
    let texts = crate::file_texts::FileTexts::new(repo_root.to_path_buf(), config);
    let located = texts
        .locator_files(&parsed.path)
        .map_err(|e| format!("'{}': the glob {e}", parsed.path))?;
    let Some(kind) = located.kind.filter(|k| *k != LocatorKind::File) else {
        return Ok(None);
    };
    let opts: LocatorExistsOpts = config.rule_opts(RULE_EMBODIMENT_LOCATOR_EXISTS);
    if !opts.locator_kinds.contains(&kind) {
        notices.push(Notice {
            severity: NoticeSeverity::Warning,
            subject: parsed.path.clone(),
            message: format!(
                "'{}' is a {kind} locator and `{RULE_EMBODIMENT_LOCATOR_EXISTS}` does not accept that kind; add `{kind}` to its `locator_kinds` option to cite it in `Realized-by`",
                parsed.path
            ),
        });
        return Ok(None);
    }
    if located.files.is_empty() {
        return Err(format!(
            "'{}' matches no tracked file in the working tree",
            parsed.path
        ));
    }
    let mut pairs = Vec::new();
    for file in &located.files {
        let hash = urzua_io::sha256_hex_of_file(&repo_root.join(file))
            .ok_or_else(|| format!("'{file}' could not be read"))?;
        pairs.push((file.clone(), hash));
    }
    let sha256 = urzua_io::sha256_hex_of_str(&locator::files_pin_text(&pairs));
    let pin = opts
        .pinnable
        .contains(&kind)
        .then(|| format!("{}@sha256:{sha256}", parsed.path));
    Ok(Some(CoverageExplanation {
        kind: kind.to_string(),
        files: located.files,
        sha256,
        pin,
    }))
}

// Implements: ADR-74 (explain is the authoring route for a scoped pin)
fn explain_scope(
    repo_root: &std::path::Path,
    config: &urzua_core::config::Config,
    parsed: &locator::Locator,
    scope: &locator::Scope,
) -> Result<ScopeExplanation, String> {
    let texts = crate::file_texts::FileTexts::new(repo_root.to_path_buf(), config);
    let text = match &*texts.get(&parsed.path) {
        FileText::Text(t) => t.clone(),
        FileText::NotUtf8 => {
            return Err(format!(
                "'{}': {}",
                parsed.path,
                locator::ScopeError::NotUtf8
            ))
        }
        FileText::Missing => return Err(format!("'{}' could not be read", parsed.path)),
    };
    let resolver = locator::select_resolver(&config.locator_resolvers, &parsed.path)
        .map_err(|e| format!("'{}': {e}", parsed.display()))?;
    let span = locator::resolve_scope(&text, resolver, scope)
        .map_err(|e| format!("'{}' does not resolve: {e}", parsed.display()))?;
    let pinned = locator::span_text(&text, span);
    let sha256 = urzua_io::sha256_hex_of_str(&pinned);
    Ok(ScopeExplanation {
        scope: scope.to_string(),
        resolver: match scope {
            locator::Scope::Symbol(_) => resolver.map(|r| r.name.clone()),
            locator::Scope::Range { .. } => None,
        },
        start: span.start,
        end: span.end,
        text: pinned,
        pin: format!("{}@sha256:{sha256}", parsed.display()),
        sha256,
    })
}
