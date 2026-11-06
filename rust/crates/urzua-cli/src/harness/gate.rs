//! The acceptance and protected-path guardrails of the editor hook (`ADR-89`
//! section 4, `MILE-200`).
//!
//! For an agent, a Write, Edit or MultiEdit that moves a record of a gated type
//! to a gated status, or that writes a gated path, is denied outright. The
//! override of `ADR-78` is a name in an environment variable the agent controls,
//! so it is not accepted for this verdict. A person edits outside the agent
//! harness and is not asked.
//!
//! This is the whole claim: the edit tools are refused. A shell command that
//! writes the same file is not seen here (the post-command check reports it),
//! and in CI the evidence is authorship, not who decided (`diff.decider-required`).

use std::path::Path;

use urzua_core::authority::Gate;
use urzua_core::config::{Config, RelationRole};
use urzua_core::rules;

/// Why an agent's edit is refused, and by which guardrail.
pub(super) struct Denial {
    pub guardrail: &'static str,
    pub reason: String,
}

fn status_of(config: &Config, rel: &Path, text: &str) -> Option<(String, String)> {
    let record = crate::discovery::parse_prospective_record(config, rel, text).ok()?;
    let field = rules::relation_field_name(config, &record.record_type, RelationRole::Status);
    let status = record.header.get(field).map(str::trim)?.to_string();
    (!status.is_empty()).then_some((record.record_type.clone(), status))
}

/// The status the record holds before this edit: the copy on disk, else the
/// copy at `HEAD`, else none (a new record).
fn status_before(repo_root: &Path, config: &Config, rel: &Path) -> Option<Option<String>> {
    let text = match std::fs::read_to_string(repo_root.join(rel)) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            match urzua_io::read_at_commit_if_present(repo_root, "HEAD", rel) {
                Ok(t) => t,
                Err(_) => return None,
            }
        }
        Err(_) => return None,
    };
    match text {
        None => Some(None),
        Some(text) => Some(status_of(config, rel, &text).map(|(_, s)| s)),
    }
}

fn gate_summary(gate: &Gate) -> String {
    format!("gate: {}, by '{}'", gate.describe(), gate.by)
}

/// Judges one edit against the loaded `authority` table. `None` means the gates
/// have nothing to say; the ordinary editor-hook judgement still follows.
pub(super) fn judge(
    repo_root: &Path,
    config: &Config,
    rel: &Path,
    content: &str,
) -> Option<Denial> {
    let (harness, authority) = (config.harness.as_ref()?, config.authority.as_ref()?);
    if harness.enabled("protected-paths") {
        for gate in authority.gates.iter().filter(|g| g.is_path_gate()) {
            if gate.paths.iter().any(|e| rules::path_matches_entry(rel, e)) {
                return Some(Denial {
                    guardrail: "protected-paths",
                    reason: format!(
                        "{} is a protected path ({}); only that decider group may change it, and an agent's edit through the edit tools is refused",
                        rel.display(),
                        gate_summary(gate)
                    ),
                });
            }
        }
    }
    if harness.enabled("acceptance") && authority.has_status_gate() {
        let (record_type, status) = status_of(config, rel, content)?;
        let gate = authority.status_gates(&record_type, &status).next()?;
        let field = rules::relation_field_name(config, &record_type, RelationRole::Status);
        // An edit that leaves the status where it was is not a move; any other
        // edit to a gated status, a record filed straight at it included, is.
        let from = match status_before(repo_root, config, rel) {
            Some(Some(before)) if before == status => return None,
            Some(Some(before)) => format!("moves {} from {field} '{before}' to '{status}'", rel.display()),
            Some(None) => format!("files {} already at {field} '{status}'", rel.display()),
            None => format!(
                "sets {} to {field} '{status}' (the copy it replaces could not be read, so it is treated as a move)",
                rel.display()
            ),
        };
        return Some(Denial {
            guardrail: "acceptance",
            reason: format!(
                "this edit {from}, a move the authority table reserves for a decider ({}); an agent cannot make it through the edit tools",
                gate_summary(gate)
            ),
        });
    }
    None
}

/// What the raw configuration says about the gates when it does not load, read
/// leniently so a broken file does not silently turn a denying gate into an
/// allowing one (`ADR-89` section 3).
pub(super) struct RawGates {
    /// The protected-paths guardrail, when enabled and set to deny.
    pub protected: bool,
    /// The acceptance guardrail, when enabled and set to deny.
    pub acceptance: bool,
    pub paths: Vec<String>,
    /// The directories of the types a status gate names.
    pub gated_dirs: Vec<String>,
}

pub(super) fn raw_gates(config_text: &str) -> Option<RawGates> {
    let value: yaml_serde::Value = yaml_serde::from_str(config_text).ok()?;
    let guardrail = |name: &str| -> bool {
        let Some(g) = value
            .get("harness")
            .and_then(|h| h.get("guardrails"))
            .and_then(|g| g.get(name))
        else {
            return false;
        };
        let enabled = g.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
        let allow = g.get("on_unjudged").and_then(|o| o.as_str()) == Some("allow");
        enabled && !allow
    };
    let gates = value
        .get("authority")
        .and_then(|a| a.get("gates"))
        .and_then(|g| g.as_sequence())
        .cloned()
        .unwrap_or_default();
    let mut raw = RawGates {
        protected: guardrail("protected-paths"),
        acceptance: guardrail("acceptance"),
        paths: Vec::new(),
        gated_dirs: Vec::new(),
    };
    for gate in &gates {
        if let Some(paths) = gate.get("paths").and_then(|p| p.as_sequence()) {
            raw.paths
                .extend(paths.iter().filter_map(|p| p.as_str().map(str::to_string)));
        }
        if let Some(t) = gate.get("type").and_then(|t| t.as_str()) {
            if let Some(dir) = value
                .get("record_types")
                .and_then(|r| r.get(t))
                .and_then(|r| r.get("dir"))
                .and_then(|d| d.as_str())
            {
                raw.gated_dirs.push(dir.trim_end_matches('/').to_string());
            }
        }
    }
    Some(raw)
}

impl RawGates {
    /// Whether the edit is one the unreadable configuration cannot be trusted
    /// to allow: a gated path, the configuration itself, or a record in a gated
    /// type's directory.
    pub fn covers(&self, rel: &Path, config_rel: Option<&Path>) -> Option<&'static str> {
        if self.protected
            && (config_rel.is_some_and(|c| c == rel)
                || self.paths.iter().any(|e| rules::path_matches_entry(rel, e)))
        {
            return Some("protected-paths");
        }
        if self.acceptance {
            let parent = rel.parent().map(|p| p.to_string_lossy().to_string());
            if parent.is_some_and(|p| self.gated_dirs.contains(&p)) {
                return Some("acceptance");
            }
        }
        None
    }
}
