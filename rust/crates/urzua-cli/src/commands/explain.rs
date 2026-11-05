//! `urzua explain` (SPEC-13): "which decisions govern this file" -- every
//! record whose `Realized-by` names `path` as evidence (ADR-0024). Given
//! `path#scope` it also resolves the scope and prints the span and its hash,
//! the authoring route for a scoped pin (ADR-74).

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::locator::{self, FileText};
use urzua_core::report::{ExplainReport, ReportStatus, ScopeExplanation};

use crate::emit;
use crate::session::{emit_or_not_run, Session};

pub fn run(config_path: Option<PathBuf>, path: String) -> ExitCode {
    emit_or_not_run(run_inner(config_path, path))
}

fn run_inner(config_path: Option<PathBuf>, path: String) -> Result<ExitCode, String> {
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
        ));
    }
    let governing_records = urzua_core::graph::explain(&records, &parsed.path, &config);
    let scope = match &parsed.scope {
        None => None,
        Some(scope) => Some(explain_scope(&repo_root, &config, &parsed, scope)?),
    };

    Ok(emit(&ExplainReport {
        status: ReportStatus::Ok,
        path: parsed.path.clone(),
        governing_records,
        scope,
        notices: Vec::new(),
    }))
}

// Implements: ADR-74 (explain is the authoring route for a scoped pin)
fn explain_scope(
    repo_root: &std::path::Path,
    config: &urzua_core::config::Config,
    parsed: &locator::Locator,
    scope: &locator::Scope,
) -> Result<ScopeExplanation, String> {
    let texts = crate::file_texts::FileTexts::new(repo_root.to_path_buf());
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
