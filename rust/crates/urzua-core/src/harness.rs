//! The `harness` configuration key (`ADR-89` section 3, `MILE-198`): which
//! adapters a repository installs, the file its agent-instructions block lives
//! in, and which guardrails are on and what each does when it cannot judge.
//!
//! The engine names no agent vendor here or anywhere else: an adapter and a
//! guardrail are names the repository declares, checked against what the
//! build ships (`GUARDRAILS`) and, at install, against what the chosen adapter
//! provides. A key or guardrail this build does not know is not dropped: it is
//! kept in `problems`, which `urzua harness install` and `urzua doctor` report.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What a guardrail does when it cannot judge (`ADR-89` section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnUnjudged {
    /// Let the edit through with a loud message (`ADR-71`).
    Allow,
    /// Refuse an agent's edit. A person edits outside the agent harness.
    Deny,
}

impl OnUnjudged {
    pub fn as_str(self) -> &'static str {
        match self {
            OnUnjudged::Allow => "allow",
            OnUnjudged::Deny => "deny",
        }
    }
}

/// One guardrail this build knows: its name, whether it takes `on_unjudged`,
/// and what that defaults to. The acceptance and protected-path guardrails
/// default to `deny` because an unjudged gate is not a gate.
pub struct GuardrailInfo {
    pub name: &'static str,
    pub takes_on_unjudged: bool,
    pub default_on_unjudged: OnUnjudged,
}

/// Every guardrail name `harness.guardrails` accepts.
pub const GUARDRAILS: &[GuardrailInfo] = &[
    GuardrailInfo {
        name: "editor-hook",
        takes_on_unjudged: true,
        default_on_unjudged: OnUnjudged::Allow,
    },
    GuardrailInfo {
        name: "post-command",
        takes_on_unjudged: false,
        default_on_unjudged: OnUnjudged::Allow,
    },
    GuardrailInfo {
        name: "session-start",
        takes_on_unjudged: false,
        default_on_unjudged: OnUnjudged::Allow,
    },
    GuardrailInfo {
        name: "acceptance",
        takes_on_unjudged: true,
        default_on_unjudged: OnUnjudged::Deny,
    },
    GuardrailInfo {
        name: "protected-paths",
        takes_on_unjudged: true,
        default_on_unjudged: OnUnjudged::Deny,
    },
];

/// The guardrails that need the top-level `authority` table to enforce
/// anything: enabling one without it is a load error (`ADR-89` section 3).
pub const NEEDS_AUTHORITY: &[&str] = &["acceptance", "protected-paths"];

pub fn guardrail_info(name: &str) -> Option<&'static GuardrailInfo> {
    GUARDRAILS.iter().find(|g| g.name == name)
}

/// The load error for a `harness.guardrails` name this build does not ship,
/// suggesting the nearest valid name when one is close.
fn unknown_guardrail_message(name: &str) -> String {
    let hint = nearest_name(name, GUARDRAILS.iter().map(|g| g.name))
        .map(|valid| format!("; did you mean '{valid}'?"))
        .unwrap_or_default();
    format!(
        "harness.guardrails names '{name}', which this build does not know{hint} (it ships: {})",
        GUARDRAILS
            .iter()
            .map(|g| g.name)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// The candidate closest to `name` by edit distance, when one is close enough
/// to be a likely misspelling: within a third of the name's length, and at
/// least two edits. Ties go to the alphabetically first candidate.
pub fn nearest_name<'a>(name: &str, candidates: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    candidates
        .map(|c| (edit_distance(name, c), c))
        .min()
        .filter(|(distance, _)| *distance <= (name.chars().count() / 3).max(2))
        .map(|(_, c)| c)
}

/// Levenshtein distance between two short names.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (row[j + 1] + 1)
                .min(row[j] + 1)
                .min(diagonal + usize::from(ca != *cb));
            diagonal = above;
        }
    }
    row[b.len()]
}

/// One guardrail as the repository declared it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guardrail {
    pub enabled: bool,
    /// `None` is the guardrail's default (`GuardrailInfo::default_on_unjudged`).
    pub on_unjudged: Option<OnUnjudged>,
}

/// The `harness` key. An absent key is `None` in `Config`: it installs and
/// enables nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HarnessDecl {
    /// The adapters `urzua harness install` writes when `--adapter` is not
    /// given.
    pub adapters: Vec<String>,
    /// The repository's agent-instructions file, which carries the marked
    /// block (`ADR-89` section 1). The engine assumes no name.
    pub instructions_file: Option<String>,
    pub guardrails: BTreeMap<String, Guardrail>,
    /// Keys and guardrails this build does not know, and values a guardrail
    /// does not take. Reported at install and by `doctor`, never skipped.
    pub problems: Vec<String>,
    /// The unknown keys among `problems`, structured: where the key sits, what
    /// it is, and the nearest key the build does read. `urzua check` reports
    /// each as a warning notice (`ADR-89` amendment of 2026-11-07).
    pub unknown_keys: Vec<UnknownKey>,
}

/// A key the build does not read, found inside `harness` or a known guardrail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownKey {
    /// Where it sits: `harness` or `harness.guardrails.<name>`.
    pub subject: String,
    pub key: String,
    /// The valid key it is closest to, when one is close enough to be a likely
    /// misspelling.
    pub nearest: Option<String>,
    /// The valid keys at that place.
    pub valid: Vec<String>,
}

impl HarnessDecl {
    /// Whether the guardrail is named and on.
    pub fn enabled(&self, name: &str) -> bool {
        self.guardrails.get(name).is_some_and(|g| g.enabled)
    }

    /// The effective `on_unjudged` of a guardrail: the declared value, else its
    /// default. A guardrail that takes none always allows.
    pub fn on_unjudged(&self, name: &str) -> OnUnjudged {
        let Some(info) = guardrail_info(name) else {
            return OnUnjudged::Allow;
        };
        if !info.takes_on_unjudged {
            return OnUnjudged::Allow;
        }
        self.guardrails
            .get(name)
            .and_then(|g| g.on_unjudged)
            .unwrap_or(info.default_on_unjudged)
    }
}

impl<'de> Deserialize<'de> for HarnessDecl {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let value = yaml_serde::Value::deserialize(deserializer)?;
        let Some(map) = value.as_mapping() else {
            return Err(D::Error::custom(
                "harness must be a mapping (adapters, instructions_file, guardrails)",
            ));
        };
        let mut decl = HarnessDecl::default();
        for (key, v) in map {
            let key = key.as_str().unwrap_or("?");
            match key {
                "adapters" => {
                    let list = v.as_sequence().ok_or_else(|| {
                        D::Error::custom("harness.adapters must be a list of adapter names")
                    })?;
                    for entry in list {
                        match entry.as_str().map(str::trim) {
                            Some(name) if !name.is_empty() => decl.adapters.push(name.to_string()),
                            _ => {
                                return Err(D::Error::custom(
                                    "harness.adapters must be a list of non-blank adapter names",
                                ))
                            }
                        }
                    }
                }
                "instructions_file" => match v.as_str().map(str::trim) {
                    Some(path) if !path.is_empty() => {
                        decl.instructions_file = Some(path.to_string())
                    }
                    _ => {
                        return Err(D::Error::custom(
                            "harness.instructions_file must be a repository-relative file path",
                        ))
                    }
                },
                "guardrails" => {
                    let guardrails = v.as_mapping().ok_or_else(|| {
                        D::Error::custom("harness.guardrails must be a mapping of guardrail names")
                    })?;
                    for (name, body) in guardrails {
                        let name = name.as_str().unwrap_or("?").to_string();
                        // A misspelled name would otherwise leave the guardrail
                        // it meant off while reading as configured, so it stops
                        // the load for every command (`ADR-89` amendment).
                        let Some(info) = guardrail_info(&name) else {
                            return Err(D::Error::custom(unknown_guardrail_message(&name)));
                        };
                        let body = body.as_mapping().ok_or_else(|| {
                            D::Error::custom(format!(
                                "harness.guardrails.{name} must be a mapping ({{enabled: true}})"
                            ))
                        })?;
                        let mut guardrail = Guardrail {
                            enabled: false,
                            on_unjudged: None,
                        };
                        for (k, val) in body {
                            match k.as_str().unwrap_or("?") {
                                "enabled" => {
                                    guardrail.enabled = val.as_bool().ok_or_else(|| {
                                        D::Error::custom(format!(
                                            "harness.guardrails.{name}.enabled must be true or false"
                                        ))
                                    })?
                                }
                                "on_unjudged" if info.takes_on_unjudged => {
                                    guardrail.on_unjudged = Some(match val.as_str() {
                                        Some("allow") => OnUnjudged::Allow,
                                        Some("deny") => OnUnjudged::Deny,
                                        _ => {
                                            return Err(D::Error::custom(format!(
                                                "harness.guardrails.{name}.on_unjudged must be allow or deny"
                                            )))
                                        }
                                    })
                                }
                                other => {
                                    decl.problems.push(format!(
                                        "harness.guardrails.{name} has the key '{other}', which it does not take"
                                    ));
                                    let valid: Vec<&str> = if info.takes_on_unjudged {
                                        vec!["enabled", "on_unjudged"]
                                    } else {
                                        vec!["enabled"]
                                    };
                                    decl.unknown_keys.push(UnknownKey {
                                        subject: format!("harness.guardrails.{name}"),
                                        key: other.to_string(),
                                        nearest: nearest_name(other, valid.iter().copied())
                                            .map(str::to_string),
                                        valid: valid.iter().map(|v| v.to_string()).collect(),
                                    });
                                }
                            }
                        }
                        decl.guardrails.insert(name, guardrail);
                    }
                }
                other => {
                    decl.problems.push(format!(
                        "harness has the key '{other}', which this build does not know (it reads adapters, instructions_file, guardrails)"
                    ));
                    let valid = ["adapters", "instructions_file", "guardrails"];
                    decl.unknown_keys.push(UnknownKey {
                        subject: "harness".to_string(),
                        key: other.to_string(),
                        nearest: nearest_name(other, valid.iter().copied()).map(str::to_string),
                        valid: valid.iter().map(|v| v.to_string()).collect(),
                    });
                }
            }
        }
        Ok(decl)
    }
}

impl Serialize for HarnessDecl {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = s.serialize_map(None)?;
        if !self.adapters.is_empty() {
            map.serialize_entry("adapters", &self.adapters)?;
        }
        if let Some(file) = &self.instructions_file {
            map.serialize_entry("instructions_file", file)?;
        }
        if !self.guardrails.is_empty() {
            let guardrails: BTreeMap<&str, yaml_serde::Value> = self
                .guardrails
                .iter()
                .map(|(name, g)| {
                    let mut body = yaml_serde::Mapping::new();
                    body.insert("enabled".into(), g.enabled.into());
                    if let Some(on) = g.on_unjudged {
                        body.insert("on_unjudged".into(), on.as_str().into());
                    }
                    (name.as_str(), yaml_serde::Value::Mapping(body))
                })
                .collect();
            map.serialize_entry("guardrails", &guardrails)?;
        }
        map.end()
    }
}

/// The load-time check of `ADR-89` section 3: `acceptance` or
/// `protected-paths` enabled with no top-level `authority` is a gate with
/// nothing to enforce, so it is refused, naming the guardrail. So is one whose
/// table declares no gate of the kind it reads: `acceptance` reads the gates by
/// type and status, `protected-paths` the gates by path. Returns the message of
/// the first one found.
pub fn authority_problem(
    harness: &HarnessDecl,
    authority: Option<&crate::authority::AuthorityDecl>,
) -> Option<String> {
    for name in NEEDS_AUTHORITY {
        if !harness.enabled(name) {
            continue;
        }
        let Some(authority) = authority else {
            return Some(format!(
                "harness guardrail '{name}' is enabled but no top-level `authority` is declared, so it would enforce nothing and read as protection"
            ));
        };
        let (has, kind) = match *name {
            "acceptance" => (
                authority.has_status_gate(),
                "by type and status ({type, to, by})",
            ),
            _ => (authority.has_path_gate(), "by path ({paths, by})"),
        };
        if !has {
            return Some(format!(
                "harness guardrail '{name}' is enabled but authority.gates declares no gate {kind}, so it would enforce nothing and read as protection"
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> Result<HarnessDecl, yaml_serde::Error> {
        yaml_serde::from_str(yaml)
    }

    #[test]
    fn a_declared_harness_reads_back_its_adapters_file_and_guardrails() {
        let h = parse(
            "adapters: [example-agent]\ninstructions_file: AGENTS.md\nguardrails:\n  editor-hook: {enabled: true, on_unjudged: deny}\n  session-start: {enabled: true}\n",
        )
        .unwrap();
        assert_eq!(h.adapters, ["example-agent"]);
        assert_eq!(h.instructions_file.as_deref(), Some("AGENTS.md"));
        assert!(h.enabled("editor-hook") && h.enabled("session-start"));
        assert!(!h.enabled("post-command"));
        assert_eq!(h.on_unjudged("editor-hook"), OnUnjudged::Deny);
        assert!(h.problems.is_empty(), "{:?}", h.problems);
    }

    #[test]
    fn on_unjudged_defaults_by_guardrail() {
        let h = parse(
            "guardrails:\n  editor-hook: {enabled: true}\n  acceptance: {enabled: true}\n  protected-paths: {enabled: true}\n",
        )
        .unwrap();
        assert_eq!(h.on_unjudged("editor-hook"), OnUnjudged::Allow);
        assert_eq!(h.on_unjudged("acceptance"), OnUnjudged::Deny);
        assert_eq!(h.on_unjudged("protected-paths"), OnUnjudged::Deny);
        assert_eq!(h.on_unjudged("session-start"), OnUnjudged::Allow);
    }

    #[test]
    fn an_unknown_key_is_kept_as_a_problem_never_dropped() {
        let h = parse(
            "colour: blue\nguardrails:\n  session-start: {enabled: true, on_unjudged: deny}\n",
        )
        .unwrap();
        assert_eq!(h.problems.len(), 2, "{:?}", h.problems);
        assert!(h.problems.iter().any(|p| p.contains("'colour'")));
        assert!(h
            .problems
            .iter()
            .any(|p| p.contains("session-start") && p.contains("'on_unjudged'")));
    }

    #[test]
    fn an_unknown_guardrail_name_is_a_load_error_suggesting_the_nearest_name_observed_failing() {
        let e = parse("guardrails:\n  acceptence: {enabled: true}\n")
            .unwrap_err()
            .to_string();
        assert!(e.contains("'acceptence'"), "{e}");
        assert!(e.contains("did you mean 'acceptance'?"), "{e}");
        let e = parse("guardrails:\n  editor-hok: {enabled: true}\n")
            .unwrap_err()
            .to_string();
        assert!(e.contains("did you mean 'editor-hook'?"), "{e}");
        // Nothing near it: no guess, but the valid names are listed.
        let e = parse("guardrails:\n  teleport: {enabled: true}\n")
            .unwrap_err()
            .to_string();
        assert!(!e.contains("did you mean"), "{e}");
        assert!(e.contains("protected-paths"), "{e}");
    }

    #[test]
    fn a_malformed_value_is_a_load_error() {
        for bad in [
            "adapters: example-agent",
            "adapters: [\"\"]",
            "instructions_file: \"\"",
            "guardrails: [editor-hook]",
            "guardrails:\n  editor-hook: {enabled: yes-please}",
            "guardrails:\n  editor-hook: {enabled: true, on_unjudged: sometimes}",
            "guardrails:\n  editor-hook: on",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_gate_with_no_authority_names_the_guardrail() {
        let h = parse("guardrails:\n  acceptance: {enabled: true}\n").unwrap();
        let message = authority_problem(&h, None).expect("refused");
        assert!(message.contains("'acceptance'"), "{message}");
        let table = |gates: &str| {
            let v: yaml_serde::Value = yaml_serde::from_str(&format!(
                "deciders:\n  m: {{logins: [a]}}\ngates: {gates}\n"
            ))
            .unwrap();
            crate::authority::AuthorityDecl::from_value(&v).unwrap()
        };
        let by_status = table("[{type: adr, to: [Accepted], by: m}]");
        assert!(authority_problem(&h, Some(&by_status)).is_none());
        // A table with no gate of the kind the guardrail reads enforces nothing.
        let none = table("[]");
        let message = authority_problem(&h, Some(&none)).expect("refused");
        assert!(
            message.contains("'acceptance'") && message.contains("no gate"),
            "{message}"
        );
        let paths = parse("guardrails:\n  protected-paths: {enabled: true}\n").unwrap();
        assert!(authority_problem(&paths, Some(&by_status)).is_some());
        let off = parse("guardrails:\n  acceptance: {enabled: false}\n").unwrap();
        assert!(authority_problem(&off, None).is_none());
    }

    const BASE: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    statuses: {values: [Proposed, Accepted]}\nrules:\n  header.required-fields: error\n";

    #[test]
    fn a_config_with_no_harness_key_declares_none() {
        let config = crate::config::parse(BASE).unwrap();
        assert!(config.harness.is_none() && config.authority.is_none());
    }

    #[test]
    fn a_config_enabling_a_gate_without_authority_does_not_load() {
        for gate in ["acceptance", "protected-paths"] {
            let cfg = format!("{BASE}harness:\n  guardrails:\n    {gate}: {{enabled: true}}\n");
            let err = crate::config::parse(&cfg).unwrap_err().to_string();
            assert!(err.contains(&format!("'{gate}'")), "{err}");
            assert!(err.contains("authority"), "{err}");
            // With the table declared the config loads.
            let gate = if gate == "acceptance" {
                "{type: adr, to: [Accepted], by: m}"
            } else {
                "{paths: [.agent-config], by: m}"
            };
            let ok = format!(
                "{cfg}authority:\n  deciders:\n    m: {{logins: [a]}}\n  gates: [{gate}]\n"
            );
            assert!(crate::config::parse(&ok).is_ok(), "{gate}");
        }
        // An off gate needs no table.
        let off = format!("{BASE}harness:\n  guardrails:\n    acceptance: {{enabled: false}}\n");
        assert!(crate::config::parse(&off).is_ok());
    }

    #[test]
    fn the_harness_key_round_trips_through_the_config() {
        let cfg = format!(
            "{BASE}harness:\n  adapters: [example-agent]\n  instructions_file: AGENTS.md\n  guardrails:\n    editor-hook: {{enabled: true, on_unjudged: deny}}\n"
        );
        let config = crate::config::parse(&cfg).unwrap();
        let again: HarnessDecl =
            yaml_serde::from_str(&yaml_serde::to_string(config.harness.as_ref().unwrap()).unwrap())
                .unwrap();
        assert_eq!(&again, config.harness.as_ref().unwrap());
    }
}

#[cfg(test)]
mod unknown_key_tests {
    use super::*;

    #[test]
    fn an_unknown_key_is_kept_structured_with_its_nearest_valid_key() {
        let h: HarnessDecl = yaml_serde::from_str(
            "adaptors: [x]\nguardrails:\n  session-start: {enabeld: true}\n  editor-hook: {enabled: true, on_unjudged: deny, colour: red}\n",
        )
        .unwrap();
        let find = |k: &str| h.unknown_keys.iter().find(|u| u.key == k).unwrap().clone();
        let top = find("adaptors");
        assert_eq!(top.subject, "harness");
        assert_eq!(top.nearest.as_deref(), Some("adapters"));
        let typo = find("enabeld");
        assert_eq!(typo.subject, "harness.guardrails.session-start");
        assert_eq!(typo.nearest.as_deref(), Some("enabled"));
        assert_eq!(typo.valid, ["enabled"]);
        assert_eq!(find("colour").nearest, None);
        assert_eq!(h.unknown_keys.len(), h.problems.len());
    }
}
