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

/// Which revision's table says who may decide (`authority.decider_table`,
/// `ADR-89` amendment of 2026-11-09). Ordered by strictness: `head` is the
/// least, `both` the most.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum DeciderTable {
    /// The pull request's own checkout (today's behaviour, and the default).
    #[serde(rename = "head")]
    Head,
    /// The base revision of the `--base` diff: a pull request cannot widen
    /// who may approve it.
    #[serde(rename = "base")]
    Base,
    /// Both: a gated change needs a decider under each table, and a gate in
    /// either applies.
    #[serde(rename = "both")]
    Both,
}

impl DeciderTable {
    pub fn as_str(self) -> &'static str {
        match self {
            DeciderTable::Head => "head",
            DeciderTable::Base => "base",
            DeciderTable::Both => "both",
        }
    }
}

/// The base revision's `authority` table, read by the caller from the config
/// file at the merge base and handed to the rule as data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseAuthority {
    /// The base config declares an `authority` section.
    Declared(AuthorityDecl),
    /// The base revision has no config file, or no `authority` section: it has
    /// no deciders and no gates.
    Absent,
    /// The base config could not be read or its `authority` section did not
    /// load; the caller said so in a notice and the head table is used.
    Unreadable,
}

impl BaseAuthority {
    /// Reads the base revision's config text (`None`: the file is not there).
    /// Only the `authority` section is read, so a base written by an older
    /// build does not have to load whole.
    pub fn from_config_text(text: Option<&str>) -> Result<BaseAuthority, String> {
        let Some(text) = text else {
            return Ok(BaseAuthority::Absent);
        };
        let value: yaml_serde::Value = yaml_serde::from_str(text)
            .map_err(|e| format!("the base revision's config does not parse as YAML: {e}"))?;
        match value.as_mapping().and_then(|m| m.get("authority")) {
            None => Ok(BaseAuthority::Absent),
            Some(v) => AuthorityDecl::from_value(v)
                .map(BaseAuthority::Declared)
                .map_err(|e| format!("the base revision's authority section does not load: {e}")),
        }
    }

    /// Whether the base revision declares no `authority` (no config file, or no
    /// such section): it names no decider and no gate.
    pub fn is_absent(&self) -> bool {
        matches!(self, BaseAuthority::Absent)
    }

    /// The base table, when one is declared.
    pub fn declared(&self) -> Option<&AuthorityDecl> {
        match self {
            BaseAuthority::Declared(a) => Some(a),
            _ => None,
        }
    }
}

/// What the base revision's config says about the decider gate: its
/// `authority` table and how its rule is set (`ADR-89` amendment of
/// 2026-11-09). All three are read from the same config text, so a pull request
/// that edits the head's rule setting or `fail_on` cannot loosen the gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseGate {
    pub authority: BaseAuthority,
    /// The base's `diff.decider-required` entry, when it has one.
    pub rule: Option<crate::config::RuleSetting>,
    /// The base's `fail_on`, when it declares one.
    pub fail_on: Option<crate::config::FailOn>,
}

impl BaseGate {
    /// A base revision with nothing to say (a run with no `--base`).
    pub fn absent() -> BaseGate {
        BaseGate {
            authority: BaseAuthority::Absent,
            rule: None,
            fail_on: None,
        }
    }

    /// Reads the base config text (`None`: the file is not there). Only the
    /// `authority`, `fail_on` and `rules.<rule>` keys are read.
    pub fn from_config_text(text: Option<&str>, rule: &str) -> Result<BaseGate, String> {
        let authority = BaseAuthority::from_config_text(text)?;
        let Some(text) = text else {
            return Ok(BaseGate {
                authority,
                ..BaseGate::absent()
            });
        };
        let value: yaml_serde::Value = yaml_serde::from_str(text)
            .map_err(|e| format!("the base revision's config does not parse as YAML: {e}"))?;
        let map = value.as_mapping();
        let rule_setting = map
            .and_then(|m| m.get("rules"))
            .and_then(|r| r.as_mapping())
            .and_then(|r| r.get(rule))
            .map(|v| {
                yaml_serde::from_value::<crate::config::RuleSetting>(v.clone()).map_err(|e| {
                    format!("the base revision's setting for {rule} does not load: {e}")
                })
            })
            .transpose()?;
        let fail_on = map
            .and_then(|m| m.get("fail_on"))
            .map(|v| {
                yaml_serde::from_value::<crate::config::FailOn>(v.clone())
                    .map_err(|e| format!("the base revision's fail_on does not load: {e}"))
            })
            .transpose()?;
        Ok(BaseGate {
            authority,
            rule: rule_setting,
            fail_on,
        })
    }
}

/// The mode in force for a run: the stricter of the head's declaration (if the
/// head has an `authority`) and the base revision's. Undeclared reads as `head`.
pub fn effective_mode(head: Option<&AuthorityDecl>, base: &BaseAuthority) -> DeciderTable {
    let head = head
        .and_then(|a| a.decider_table)
        .unwrap_or(DeciderTable::Head);
    let base = base
        .declared()
        .and_then(|a| a.decider_table)
        .unwrap_or(DeciderTable::Head);
    head.max(base)
}

/// The `authority` key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthorityDecl {
    /// Which revision's table decides; `None` is undeclared and reads as `head`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decider_table: Option<DeciderTable>,
    pub deciders: BTreeMap<String, DeciderGroup>,
    /// The kinds of host evidence that may name a decider; any one suffices.
    /// `pr-author` unless declared: `pr-approver` can never be satisfied by a
    /// single maintainer, because a host bars an author from approving their own
    /// pull request.
    pub evidence: Vec<EvidenceKind>,
    pub gates: Vec<Gate>,
}

const KEYS: &str = "deciders, evidence, gates, decider_table";

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
            // The same canonical form every other path option is compared in,
            // so `./.agent` gates what `.agent` does.
            paths: paths
                .iter()
                .map(|p| crate::config::normalize_published_path(p))
                .collect(),
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

fn decider_table_from_value(value: &yaml_serde::Value) -> Result<DeciderTable, String> {
    match value.as_str().map(str::trim) {
        Some("base") => Ok(DeciderTable::Base),
        Some("head") => Ok(DeciderTable::Head),
        Some("both") => Ok(DeciderTable::Both),
        _ => Err(problem(
            "authority.decider_table must be one of base, head, both",
        )),
    }
}

impl AuthorityDecl {
    /// A table with no group and no gate: what the base revision holds when it
    /// declares no `authority`.
    pub fn none() -> AuthorityDecl {
        AuthorityDecl {
            decider_table: None,
            deciders: BTreeMap::new(),
            evidence: vec![EvidenceKind::PrAuthor],
            gates: Vec::new(),
        }
    }

    /// The mode in force for a run: the stricter of what this (head) table and
    /// the base revision's table declare, so a pull request cannot loosen it
    /// by editing the key. Undeclared reads as `head`.
    pub fn effective_table(&self, base: &BaseAuthority) -> DeciderTable {
        effective_mode(Some(self), base)
    }

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
            decider_table: None,
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
                "decider_table" => decl.decider_table = Some(decider_table_from_value(v)?),
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
                let has_name = std::path::Path::new(p)
                    .components()
                    .any(|c| matches!(c, std::path::Component::Normal(_)));
                if let Some(why) = crate::config::published_path_problem(p) {
                    return Some(format!("{at} names the path '{p}', which {why}"));
                }
                if !has_name {
                    return Some(format!(
                        "{at} names the path '{p}', which names no path inside the repository, so it would gate nothing"
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
    fn decider_table_reads_three_values_and_refuses_the_rest() {
        assert_eq!(parse(OK).unwrap().decider_table, None);
        for (text, want) in [
            ("base", DeciderTable::Base),
            ("head", DeciderTable::Head),
            ("both", DeciderTable::Both),
        ] {
            let a = parse(&format!("decider_table: {text}\n{OK}")).unwrap();
            assert_eq!(a.decider_table, Some(want));
        }
        for bad in ["neither", "[base]", "1"] {
            let err = parse(&format!("decider_table: {bad}\n")).unwrap_err();
            assert!(err.contains("base, head, both"), "{bad}: {err}");
        }
    }

    #[test]
    fn the_mode_in_force_is_the_stricter_of_the_head_and_the_base() {
        let with = |mode: &str| parse(&format!("decider_table: {mode}\n")).unwrap();
        let none = parse(OK).unwrap();
        let base_of = |a: &AuthorityDecl| BaseAuthority::Declared(a.clone());
        assert_eq!(none.effective_table(&base_of(&none)), DeciderTable::Head);
        assert_eq!(
            none.effective_table(&BaseAuthority::Absent),
            DeciderTable::Head
        );
        assert_eq!(
            with("base").effective_table(&BaseAuthority::Absent),
            DeciderTable::Base
        );
        assert_eq!(
            with("head").effective_table(&base_of(&with("base"))),
            DeciderTable::Base
        );
        assert_eq!(
            none.effective_table(&base_of(&with("both"))),
            DeciderTable::Both
        );
        assert_eq!(
            with("both").effective_table(&base_of(&with("base"))),
            DeciderTable::Both
        );
        assert_eq!(
            with("base").effective_table(&BaseAuthority::Unreadable),
            DeciderTable::Base
        );
    }

    #[test]
    fn the_base_config_text_gives_declared_absent_or_an_error() {
        assert_eq!(
            BaseAuthority::from_config_text(None).unwrap(),
            BaseAuthority::Absent
        );
        assert_eq!(
            BaseAuthority::from_config_text(Some("schema_version: 2\n")).unwrap(),
            BaseAuthority::Absent
        );
        let text = format!(
            "schema_version: 2\nauthority:\n  decider_table: base\n  {}",
            OK.replace('\n', "\n  ")
        );
        let declared = BaseAuthority::from_config_text(Some(&text)).unwrap();
        assert_eq!(
            declared.declared().unwrap().decider_table,
            Some(DeciderTable::Base)
        );
        assert!(BaseAuthority::from_config_text(Some("authority: {colour: blue}\n")).is_err());
        assert!(BaseAuthority::from_config_text(Some("a: [\n")).is_err());
    }

    #[test]
    fn the_base_gate_reads_the_rule_setting_and_fail_on_from_the_same_text() {
        use crate::config::{FailOn, RuleLevel};
        let rule = "diff.decider-required";
        let g = BaseGate::from_config_text(
            Some("fail_on: warning\nrules:\n  diff.decider-required: warn\n"),
            rule,
        )
        .unwrap();
        assert_eq!(g.rule.unwrap().level, RuleLevel::Warn);
        assert_eq!(g.fail_on, Some(FailOn::Warning));
        assert!(g.authority.is_absent());
        let table = BaseGate::from_config_text(
            Some("rules:\n  diff.decider-required: {level: error, require_evidence: true}\n"),
            rule,
        )
        .unwrap();
        assert_eq!(table.rule.unwrap().level, RuleLevel::Error);
        assert_eq!(
            BaseGate::from_config_text(None, rule).unwrap(),
            BaseGate::absent()
        );
        assert!(BaseGate::from_config_text(Some("fail_on: loud\n"), rule).is_err());
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
