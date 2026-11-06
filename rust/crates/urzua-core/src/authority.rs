//! The `authority` configuration key (`ADR-89` section 4, `MILE-200`): who may
//! move a record to an accepting status, and who may change a protected path.
//!
//! One table, read in two places. The editor hook (`urzua harness run edit`)
//! refuses an agent's edit-tool write that makes a gated move or writes a gated
//! path, whoever the agent is. The rule `diff.decider-required` reads the same
//! table in CI, against the identities the host supplied in an actors file, and
//! proves authorship (`pr-author`) or a current approval (`pr-approver`) and
//! nothing more: with a single maintainer it cannot tell an agent acting under
//! the maintainer's token from the maintainer.
//!
//! The engine names no status, type, login or host: every one is written in the
//! configuration, and a gate that names a type or status the configuration does
//! not declare is a load error that names the entry.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A pull-request fact the host supplies that can name a decider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EvidenceKind {
    /// The pull request's author is in the gate's group.
    #[serde(rename = "pr-author")]
    PrAuthor,
    /// A login whose approval is current for the head commit is in the group.
    #[serde(rename = "pr-approver")]
    PrApprover,
}

impl EvidenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceKind::PrAuthor => "pr-author",
            EvidenceKind::PrApprover => "pr-approver",
        }
    }
}

/// A named group of decider logins. The logins are the host's vocabulary (a
/// GitHub login here); the engine compares them without case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeciderGroup {
    pub logins: Vec<String>,
}

/// One gate: a move of a record of `record_type` to a status in `to`, or a
/// change under one of `paths`, that only the group named by `by` may make.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Gate {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub record_type: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub to: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
    pub by: String,
}

impl Gate {
    /// Whether this gate is by status (`type` and `to`), the acceptance gate.
    pub fn is_status_gate(&self) -> bool {
        self.record_type.is_some()
    }

    /// Whether this gate is by path, the protected-path gate.
    pub fn is_path_gate(&self) -> bool {
        !self.paths.is_empty()
    }

    /// A short name for a message: `type adr to Accepted, Rejected` or `paths ...`.
    pub fn describe(&self) -> String {
        match &self.record_type {
            Some(t) => format!("type {t} to {}", self.to.join(", ")),
            None => format!("paths {}", self.paths.join(", ")),
        }
    }
}

/// The `authority` key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthorityDecl {
    pub deciders: BTreeMap<String, DeciderGroup>,
    /// The kinds of host evidence that may name a decider; any one suffices.
    /// `pr-author` unless declared: `pr-approver` can never be satisfied by a
    /// single maintainer, because a host bars an author from approving their own
    /// pull request.
    pub evidence: Vec<EvidenceKind>,
    pub gates: Vec<Gate>,
}

const KEYS: &str = "deciders, evidence, gates";

fn problem(message: impl Into<String>) -> String {
    message.into()
}

fn string_list(value: &yaml_serde::Value, what: &str) -> Result<Vec<String>, String> {
    let Some(list) = value.as_sequence() else {
        return Err(problem(format!("{what} must be a list of strings")));
    };
    let mut out = Vec::new();
    for entry in list {
        match entry.as_str().map(str::trim) {
            Some(s) if !s.is_empty() => out.push(s.to_string()),
            _ => return Err(problem(format!("{what} must hold only non-blank strings"))),
        }
    }
    Ok(out)
}

fn group_from_value(name: &str, value: &yaml_serde::Value) -> Result<DeciderGroup, String> {
    let Some(map) = value.as_mapping() else {
        // A bare login, or a list of them, sits in no group.
        let logins = match value {
            yaml_serde::Value::String(s) => vec![s.clone()],
            yaml_serde::Value::Sequence(list) => list
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => Vec::new(),
        };
        return Err(problem(format!(
            "authority.deciders.{name} must be a mapping of the form {{logins: [...]}}; the login(s) {} are in no group",
            if logins.is_empty() {
                "it holds".to_string()
            } else {
                logins.join(", ")
            }
        )));
    };
    let mut logins = Vec::new();
    for (k, v) in map {
        match k.as_str().unwrap_or("?") {
            "logins" => {
                logins = string_list(v, &format!("authority.deciders.{name}.logins"))?;
            }
            other => {
                return Err(problem(format!(
                    "authority.deciders.{name} has the key '{other}', which it does not take (it reads logins)"
                )))
            }
        }
    }
    if logins.is_empty() {
        return Err(problem(format!(
            "authority.deciders.{name} names no login: a group nobody belongs to can decide nothing"
        )));
    }
    Ok(DeciderGroup { logins })
}

fn gate_from_value(index: usize, value: &yaml_serde::Value) -> Result<Gate, String> {
    let at = format!("authority.gates[{index}]");
    let Some(map) = value.as_mapping() else {
        return Err(problem(format!(
            "{at} must be a mapping ({{type, to, by}} or {{paths, by}})"
        )));
    };
    let (mut record_type, mut to, mut paths, mut by) = (None, None, None, None);
    for (k, v) in map {
        match k.as_str().unwrap_or("?") {
            "type" => match v.as_str().map(str::trim) {
                Some(t) if !t.is_empty() => record_type = Some(t.to_string()),
                _ => return Err(problem(format!("{at}.type must be a record type name"))),
            },
            "to" => to = Some(string_list(v, &format!("{at}.to"))?),
            "paths" => paths = Some(string_list(v, &format!("{at}.paths"))?),
            "by" => match v.as_str().map(str::trim) {
                Some(g) if !g.is_empty() => by = Some(g.to_string()),
                _ => return Err(problem(format!("{at}.by must name a decider group"))),
            },
            other => {
                return Err(problem(format!(
                    "{at} has the key '{other}', which a gate does not take (it reads type, to, paths, by)"
                )))
            }
        }
    }
    let Some(by) = by else {
        return Err(problem(format!("{at} names no group: add `by: <group>`")));
    };
    match (record_type, to, paths) {
        (Some(t), Some(to), None) if !to.is_empty() => Ok(Gate {
            record_type: Some(t),
            to,
            paths: Vec::new(),
            by,
        }),
        (Some(_), _, None) => Err(problem(format!(
            "{at} names a type but no `to` statuses: it would gate nothing"
        ))),
        (None, None, Some(paths)) if !paths.is_empty() => Ok(Gate {
            record_type: None,
            to: Vec::new(),
            paths,
            by,
        }),
        (None, None, Some(_)) => Err(problem(format!("{at}.paths is empty: it would gate nothing"))),
        (Some(_), _, Some(_)) | (None, Some(_), _) => Err(problem(format!(
            "{at} mixes the two forms: write {{type, to, by}} for a status or {{paths, by}} for a path, not both"
        ))),
        (None, None, None) => Err(problem(format!(
            "{at} gates nothing: write {{type, to, by}} for a status or {{paths, by}} for a path"
        ))),
    }
}

fn evidence_from_value(value: &yaml_serde::Value) -> Result<Vec<EvidenceKind>, String> {
    let names = string_list(value, "authority.evidence")?;
    if names.is_empty() {
        return Err(problem(
            "authority.evidence is empty: name pr-author, pr-approver or both, or omit it for pr-author",
        ));
    }
    let mut out = Vec::new();
    for name in names {
        let kind = match name.as_str() {
            "pr-author" => EvidenceKind::PrAuthor,
            "pr-approver" => EvidenceKind::PrApprover,
            other => {
                return Err(problem(format!(
                    "authority.evidence names '{other}', which is not one of pr-author, pr-approver"
                )))
            }
        };
        if !out.contains(&kind) {
            out.push(kind);
        }
    }
    Ok(out)
}

impl AuthorityDecl {
    /// Reads the key as written. Shape errors name the entry.
    pub fn from_value(value: &yaml_serde::Value) -> Result<AuthorityDecl, String> {
        if value.is_null() {
            return Err(problem(
                "authority is declared but empty: write deciders and gates, or remove the key",
            ));
        }
        let Some(map) = value.as_mapping() else {
            return Err(problem(format!("authority must be a mapping of {KEYS}")));
        };
        let mut decl = AuthorityDecl {
            deciders: BTreeMap::new(),
            evidence: vec![EvidenceKind::PrAuthor],
            gates: Vec::new(),
        };
        for (k, v) in map {
            match k.as_str().unwrap_or("?") {
                "deciders" => match v {
                    yaml_serde::Value::Mapping(groups) => {
                        for (name, body) in groups {
                            let name = name.as_str().unwrap_or("?").to_string();
                            let group = group_from_value(&name, body)?;
                            decl.deciders.insert(name, group);
                        }
                    }
                    yaml_serde::Value::Sequence(list) => {
                        let logins: Vec<&str> = list.iter().filter_map(|v| v.as_str()).collect();
                        return Err(problem(format!(
                            "authority.deciders must map a group name to {{logins: [...]}}; the login(s) {} are in no group",
                            logins.join(", ")
                        )));
                    }
                    _ => {
                        return Err(problem(
                            "authority.deciders must map a group name to {logins: [...]}",
                        ))
                    }
                },
                "evidence" => decl.evidence = evidence_from_value(v)?,
                "gates" => {
                    let Some(list) = v.as_sequence() else {
                        return Err(problem("authority.gates must be a list of gates"));
                    };
                    for (i, entry) in list.iter().enumerate() {
                        decl.gates.push(gate_from_value(i, entry)?);
                    }
                }
                other => {
                    return Err(problem(format!(
                        "authority has the key '{other}', which this build does not know (it reads {KEYS})"
                    )))
                }
            }
        }
        Ok(decl)
    }

    /// The load-time check against the rest of the configuration: every gate
    /// names a declared group, and every type and status it names is declared.
    /// Returns the first problem, naming the entry.
    pub fn problem_in(&self, config: &crate::config::Config) -> Option<String> {
        for (i, gate) in self.gates.iter().enumerate() {
            let at = format!("authority.gates[{i}] ({})", gate.describe());
            if !self.deciders.contains_key(&gate.by) {
                return Some(format!(
                    "{at} names the group '{}', which authority.deciders does not declare (declared: {})",
                    gate.by,
                    if self.deciders.is_empty() {
                        "none".to_string()
                    } else {
                        self.deciders.keys().cloned().collect::<Vec<_>>().join(", ")
                    }
                ));
            }
            if let Some(t) = &gate.record_type {
                let Some(type_config) = config.record_types.get(t) else {
                    return Some(format!(
                        "{at} names the type '{t}', which no record type declares (declared: {})",
                        config
                            .record_types
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                };
                let Some(statuses) = type_config.status_decl() else {
                    return Some(format!(
                        "{at}: the type '{t}' declares no statuses, so `to` can name none"
                    ));
                };
                if let Some(bad) = gate.to.iter().find(|s| !statuses.declares(s)) {
                    return Some(format!(
                        "{at} names the status '{bad}', which the type '{t}' does not declare (declared: {})",
                        statuses.values.join(", ")
                    ));
                }
            }
            for p in &gate.paths {
                let path = std::path::Path::new(p);
                if p.starts_with('/')
                    || path
                        .components()
                        .any(|c| matches!(c, std::path::Component::ParentDir))
                {
                    return Some(format!(
                        "{at} names the path '{p}', which is not a repository-relative path"
                    ));
                }
            }
        }
        None
    }

    /// The gates by status (the acceptance gates) that cover a move of a record
    /// of `record_type` to `status`.
    pub fn status_gates<'a>(
        &'a self,
        record_type: &'a str,
        status: &'a str,
    ) -> impl Iterator<Item = &'a Gate> + 'a {
        self.gates.iter().filter(move |g| {
            g.record_type.as_deref() == Some(record_type) && g.to.iter().any(|s| s == status)
        })
    }

    /// The record types any status gate names.
    pub fn gated_types(&self) -> BTreeSet<&str> {
        self.gates
            .iter()
            .filter_map(|g| g.record_type.as_deref())
            .collect()
    }

    /// Whether any gate is by status.
    pub fn has_status_gate(&self) -> bool {
        self.gates.iter().any(Gate::is_status_gate)
    }

    /// Whether any gate is by path.
    pub fn has_path_gate(&self) -> bool {
        self.gates.iter().any(Gate::is_path_gate)
    }

    /// The group a gate names. A validated table always has it.
    pub fn group(&self, gate: &Gate) -> Option<&DeciderGroup> {
        self.deciders.get(&gate.by)
    }

    /// Whether `login` belongs to the group `gate` names (case-insensitive).
    pub fn in_group(&self, gate: &Gate, login: &str) -> bool {
        self.group(gate).is_some_and(|g| {
            g.logins
                .iter()
                .any(|l| l.eq_ignore_ascii_case(login.trim()))
        })
    }

    /// Whether the actors the host supplied satisfy `gate`: some declared
    /// evidence kind names a login in the gate's group.
    pub fn satisfied(&self, gate: &Gate, actors: &Actors) -> bool {
        self.evidence.iter().any(|kind| match kind {
            EvidenceKind::PrAuthor => actors
                .pr_author
                .as_deref()
                .is_some_and(|a| self.in_group(gate, a)),
            EvidenceKind::PrApprover => actors.approvers.iter().any(|a| self.in_group(gate, a)),
        })
    }
}

impl<'de> Deserialize<'de> for AuthorityDecl {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let value = yaml_serde::Value::deserialize(deserializer)?;
        AuthorityDecl::from_value(&value).map_err(D::Error::custom)
    }
}

/// The identities the host supplied for one run, read by the caller from the
/// file `urzua check --actors` names and handed to the rule as data: the engine
/// fetches nothing (`check` does not use the network).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Actors {
    /// The pull request's author.
    pub pr_author: Option<String>,
    /// The logins whose approval is current for the head commit. An approval
    /// given before a later push is not listed by the gate action.
    pub approvers: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<AuthorityDecl, String> {
        let v: yaml_serde::Value = yaml_serde::from_str(text).unwrap();
        AuthorityDecl::from_value(&v)
    }

    const OK: &str = "deciders:\n  maintainers: {logins: [Alice, bob]}\ngates:\n  - {type: adr, to: [Accepted], by: maintainers}\n  - {paths: [.agent-config], by: maintainers}\n";

    #[test]
    fn a_table_reads_back_with_the_default_evidence() {
        let a = parse(OK).unwrap();
        assert_eq!(a.evidence, [EvidenceKind::PrAuthor]);
        assert_eq!(a.gates.len(), 2);
        assert!(a.has_status_gate() && a.has_path_gate());
        assert_eq!(a.gated_types().into_iter().collect::<Vec<_>>(), ["adr"]);
        assert_eq!(a.status_gates("adr", "Accepted").count(), 1);
        assert_eq!(a.status_gates("adr", "Rejected").count(), 0);
        assert_eq!(a.status_gates("rfc", "Accepted").count(), 0);
    }

    #[test]
    fn malformed_shapes_are_errors_naming_the_entry() {
        for (bad, needle) in [
            ("deciders:\n  m: {logins: []}\n", "names no login"),
            ("deciders:\n  m: [alice]\n", "in no group"),
            ("deciders: [alice]\n", "in no group"),
            ("deciders:\n  m: {logins: [a], x: 1}\n", "'x'"),
            ("evidence: []\n", "empty"),
            ("evidence: [friend]\n", "'friend'"),
            ("gates:\n  - {type: adr, by: m}\n", "authority.gates[0]"),
            ("gates:\n  - {type: adr, to: [A]}\n", "names no group"),
            (
                "gates:\n  - {type: adr, to: [A], paths: [x], by: m}\n",
                "mixes",
            ),
            ("gates:\n  - {by: m}\n", "gates nothing"),
            ("gates:\n  - {paths: [], by: m}\n", "empty"),
            ("colour: blue\n", "'colour'"),
        ] {
            let err = parse(bad).unwrap_err();
            assert!(err.contains(needle), "{bad}: {err}");
        }
        assert!(AuthorityDecl::from_value(&yaml_serde::Value::Null).is_err());
    }

    #[test]
    fn evidence_is_any_of_the_declared_kinds_and_logins_compare_without_case() {
        let mut a = parse(OK).unwrap();
        let gate = a.gates[0].clone();
        let author = Actors {
            pr_author: Some("ALICE".into()),
            approvers: vec![],
        };
        let approver = Actors {
            pr_author: Some("mallory".into()),
            approvers: vec!["bob".into()],
        };
        assert!(a.satisfied(&gate, &author));
        assert!(!a.satisfied(&gate, &approver));
        a.evidence = vec![EvidenceKind::PrApprover];
        assert!(!a.satisfied(&gate, &author));
        assert!(a.satisfied(&gate, &approver));
        a.evidence = vec![EvidenceKind::PrAuthor, EvidenceKind::PrApprover];
        assert!(a.satisfied(&gate, &author) && a.satisfied(&gate, &approver));
        assert!(!a.satisfied(&gate, &Actors::default()));
    }

    fn config(extra: &str) -> Result<crate::config::Config, crate::config::ConfigError> {
        crate::config::parse(&format!(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    statuses: {{values: [Proposed, Accepted]}}\n  note:\n    dir: docs/note\n{extra}"
        ))
    }

    #[test]
    fn a_gate_naming_what_the_configuration_does_not_declare_does_not_load() {
        let table = |gate: &str| {
            format!("authority:\n  deciders:\n    m: {{logins: [a]}}\n  gates:\n    - {gate}\n")
        };
        for (gate, needle) in [
            ("{type: adr, to: [Accepted], by: m}", ""),
            ("{type: adr, to: [Accepted], by: other}", "'other'"),
            ("{type: rfc, to: [Accepted], by: m}", "'rfc'"),
            ("{type: adr, to: [Approved], by: m}", "'Approved'"),
            (
                "{type: note, to: [Accepted], by: m}",
                "declares no statuses",
            ),
            ("{paths: [../x], by: m}", "'../x'"),
        ] {
            let result = config(&table(gate));
            if needle.is_empty() {
                assert!(result.is_ok(), "{gate}: {result:?}");
            } else {
                let err = result.unwrap_err().to_string();
                assert!(
                    err.contains(needle) && err.contains("authority.gates[0]"),
                    "{gate}: {err}"
                );
            }
        }
    }
}
