//! The report that a record changed by a command or an edit contradicts the
//! state of the record it is part of (`status.children-agreement`). The
//! finding sits on the parent's file, which the command or edit did not touch,
//! so the report of the changed records never carries it.

use std::collections::HashSet;
use std::path::PathBuf;

use urzua_core::config::{Config, RelationRole};
use urzua_core::record::Record;
use urzua_core::rules::{effective_level, record_id, relation_field_name};
use urzua_core::view_model::{parent_standings, Disagreement};

use crate::session::Session;

/// Whether the repository has turned the rule on.
pub(super) fn enabled(config: &Config) -> bool {
    effective_level(config, urzua_core::rules::RULE_STATUS_CHILDREN_AGREEMENT)
        != urzua_core::config::RuleLevel::Off
}

/// One sentence per parent that a record in `changed` contradicts, skipping a
/// parent in `already` (reported as a finding of its own).
pub(super) fn lines(
    session: &Session,
    changed: &HashSet<PathBuf>,
    already: &HashSet<PathBuf>,
) -> Vec<String> {
    let status_of = |r: &Record| {
        let field = relation_field_name(&session.config, &r.record_type, RelationRole::Status);
        r.header
            .get(field)
            .map(str::trim)
            .unwrap_or_default()
            .to_string()
    };
    let named = |r: &Record| record_id(r).unwrap_or_else(|| r.path.display().to_string());
    let mut lines = Vec::new();
    for standing in parent_standings(&session.config, &session.records) {
        let Some(kind) = standing.disagreement else {
            continue;
        };
        if already.contains(&standing.parent.path) {
            continue;
        }
        let moved: Vec<&Record> = standing
            .contradicting
            .iter()
            .copied()
            .filter(|c| changed.contains(&c.path))
            .collect();
        let Some(first) = moved.first() else {
            continue;
        };
        let more = match moved.len() - 1 {
            0 => String::new(),
            n => format!(" (and {n} more of its children)"),
        };
        let (parent, status) = (named(standing.parent), standing.status.unwrap_or_default());
        lines.push(match kind {
            Disagreement::NotStartedButChildBegun => format!(
                "{parent} is still '{status}' while {} is now '{}'{more}; its status does not say work has begun",
                named(first),
                status_of(first)
            ),
            Disagreement::FinishedButChildOpen => format!(
                "{parent} is '{status}', which finishes it, while {} is now '{}', not finished or dropped{more}",
                named(first),
                status_of(first)
            ),
        });
    }
    lines
}
