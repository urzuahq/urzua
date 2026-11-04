//! `urzua index` (`MILE-152`): regenerate the index artifacts a repository
//! declares under `index.freshness`'s `indexes` from the records.
//!
//! Detect mode (the default) is read-only and says which artifacts differ;
//! `--apply` writes them. Regeneration is deterministic, so a second run
//! reports every artifact `current` and writes nothing.

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{
    CouldNotRun, IndexOutcome, IndexReport, IndexResult, IndexStatus, Notice, NoticeSeverity,
};
use urzua_core::rules::{regenerate_index, render_index_entries, IndexContent};

use crate::emit;
use crate::session::{Session, Workspace};

pub fn run(config_path: Option<PathBuf>, apply: bool) -> ExitCode {
    let workspace = match Workspace::open(&[], config_path) {
        Ok(session) => session,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };
    let config = &workspace.config;
    let specs: Vec<urzua_core::config::IndexSpec> = config
        .enabled_rule_opts::<urzua_core::rules::IndexOpts>(urzua_core::rules::RULE_INDEX_FRESHNESS)
        .map_or_else(Vec::new, |o| o.indexes);
    if specs.is_empty() {
        return emit(&CouldNotRun::from(
            "urzua index: no index is declared; enable index.freshness with an `indexes` list",
        ));
    }

    let Session {
        repo_root,
        records,
        full_text,
        ..
    } = match Session::load(workspace, |_, _, _| Ok(())) {
        Ok(session) => session,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let artifacts = match crate::discovery::read_index_artifacts(&repo_root, &specs) {
        Ok(a) => a,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let mut results = Vec::new();
    for artifact in &artifacts {
        let spec = &artifact.spec;
        let entries = render_index_entries(spec, &records, &full_text)
            .map(|e| e.len())
            .unwrap_or(0);
        let fail = |error: String| IndexResult {
            path: PathBuf::from(&spec.path),
            outcome: IndexOutcome::Failed,
            entries,
            error: Some(error),
        };
        let wanted = match regenerate_index(spec, &records, &full_text, &artifact.content) {
            Ok(text) => text,
            Err(e) => {
                results.push(fail(e));
                continue;
            }
        };
        let existing = match &artifact.content {
            IndexContent::Text(text) => Some(text.as_str()),
            _ => None,
        };
        let outcome = match (existing, apply) {
            (Some(text), _) if text == wanted => IndexOutcome::Current,
            (Some(_), false) => IndexOutcome::Stale,
            (None, false) => IndexOutcome::Missing,
            (current, true) => {
                let full = repo_root.join(&spec.path);
                if let Some(parent) = full.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        results.push(fail(format!("could not create {}: {e}", parent.display())));
                        continue;
                    }
                }
                match std::fs::write(&full, &wanted) {
                    Ok(()) if current.is_some() => IndexOutcome::Written,
                    Ok(()) => IndexOutcome::Created,
                    Err(e) => {
                        results.push(fail(format!("could not write {}: {e}", spec.path)));
                        continue;
                    }
                }
            }
        };
        results.push(IndexResult {
            path: PathBuf::from(&spec.path),
            outcome,
            entries,
            error: None,
        });
    }

    let failed = results.iter().any(|r| r.outcome == IndexOutcome::Failed);
    let changed = results.iter().any(|r| {
        matches!(
            r.outcome,
            IndexOutcome::Stale
                | IndexOutcome::Missing
                | IndexOutcome::Written
                | IndexOutcome::Created
        )
    });
    let status = if failed {
        IndexStatus::PartialFailure
    } else if !changed {
        IndexStatus::Ok
    } else if apply {
        IndexStatus::Applied
    } else {
        IndexStatus::RegenerationAvailable
    };
    let notices = if apply || !changed {
        Vec::new()
    } else {
        vec![Notice {
            severity: NoticeSeverity::Info,
            subject: "index".to_string(),
            message: "run `urzua index --apply` to write the regenerated artifacts".to_string(),
        }]
    };
    emit(&IndexReport {
        status,
        indexes: results,
        notices,
    })
}
