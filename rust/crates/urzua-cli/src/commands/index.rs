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

use crate::discovery::{find_repo_root, load_config, load_records};
use crate::emit;

pub fn run(config_path: Option<PathBuf>, apply: bool) -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };
    let config_path =
        config_path.unwrap_or_else(|| crate::discovery::default_config_path(&repo_root));
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };
    let specs: Vec<urzua_core::config::IndexSpec> = config
        .rules
        .get(urzua_core::rules::RULE_INDEX_FRESHNESS)
        .filter(|s| s.level != urzua_core::config::RuleLevel::Off)
        .and_then(|s| s.indexes.clone())
        .unwrap_or_default();
    if specs.is_empty() {
        return emit(&CouldNotRun::from(
            "urzua index: no index is declared; enable index.freshness with an `indexes` list",
        ));
    }

    let discovered = match urzua_io::discover_tracked_files(&repo_root) {
        Ok(d) => d,
        Err(e) => return emit(&CouldNotRun::from(e.to_string())),
    };
    let (records, full_text) = match load_records(
        &repo_root,
        &discovered.paths,
        &discovered.staged_deletions,
        &config,
    ) {
        Ok(r) => r,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let artifacts = match super::check::read_index_artifacts(&repo_root, &specs) {
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
