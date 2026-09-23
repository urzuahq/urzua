//! `urzua graph` (SPEC-13): the full record-relationship graph, as data
//! (ADR-0024).

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{CouldNotRun, GraphReport, Notice, NoticeSeverity, ReportStatus};
use urzua_core::rules;

use crate::discovery::{find_repo_root, load_config, load_records};
use crate::emit;

pub fn run(config_path: Option<PathBuf>) -> ExitCode {
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

    let discovered = match urzua_io::discover_tracked_files(&repo_root) {
        Ok(d) => d,
        Err(e) => return emit(&CouldNotRun::from(e.to_string())),
    };

    let (records, _full_text) = match load_records(
        &repo_root,
        &discovered.paths,
        &discovered.staged_deletions,
        &config,
    ) {
        Ok(r) => r,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let mut pointer_fields_by_type = HashMap::new();
    let mut narrative_fields_by_type = HashMap::new();
    for (name, cfg) in &config.record_types {
        if let Some(pointer_fields) = &cfg.pointer_fields {
            pointer_fields_by_type.insert(name.clone(), pointer_fields.clone());
        }
        if let Some(narrative_fields) = &cfg.narrative_fields {
            narrative_fields_by_type.insert(name.clone(), narrative_fields.clone());
        }
    }

    let (edges, collisions) = urzua_core::graph::graph(
        &records,
        &config,
        &pointer_fields_by_type,
        &narrative_fields_by_type,
    );

    // The index this graph is built from silently keeps one of several
    // colliding records (first-seen-wins) -- an edge naming that identifier
    // may point at the arbitrary winner, not the record a reference actually
    // meant. Disclosed regardless of whether `identity.collision` is
    // enabled: this is the engine's own computation being honest about an
    // ambiguity it had to resolve (`ADR-55`), not a judgment about the
    // corpus `ADR-53` reserves for a declared rule.
    let mut notices: Vec<Notice> = Vec::new();
    for (id, claimants) in collisions {
        let mut paths: Vec<String> = claimants
            .iter()
            .map(|r| r.path.display().to_string())
            .collect();
        paths.sort();
        notices.push(Notice {
            severity: NoticeSeverity::Warning,
            subject: rules::RULE_IDENTITY_COLLISION.to_string(),
            message: format!(
                "{id} resolves to more than one record ({}) -- any edge naming it may point at an arbitrary one of them, not necessarily the one a reference meant",
                paths.join(", ")
            ),
        });
    }

    emit(&GraphReport {
        status: ReportStatus::Ok,
        edges,
        notices,
    })
}
