//! Every rule, each with its own planted-violation test, and the table
//! (`RULE_TABLE`) and runner that register and execute them: one `RuleDef` per
//! rule is the only place a rule is named outside its own module.

use crate::config::Config;
use crate::field_state::{classify, classify_with};
use crate::record::Record;
use crate::report::{
    census, census_records, ActionKind, ActionOp, ActionSource, ActionTarget, Finding,
    FindingSeverity, Notice, NoticeSeverity, Outcome, Population, PopulationUnit, RuleExecution,
    RuleStatus,
};
use crate::FieldState;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

mod action;
mod back_pointer;
mod boundary;
mod catalog;
mod citation;
mod claim;
mod claim_field;
mod companion;
mod dates;
mod decider;
mod declaration;
mod delivery_items;
mod diff_gate;
mod embodiment;
mod field;
mod filename;
mod header;
mod identity;
mod index;
mod locator_mechanism;
mod narrative;
mod open_pr;
mod options;
mod override_log;
mod pointer;
mod relation;
mod revision_log;
mod roadmap;
mod runner;
mod schema_version;
mod section;
mod sequence;
mod shared;
mod status;
mod table;
mod type_scope;
mod waiver;

pub use action::*;
pub use back_pointer::*;
pub use boundary::*;
pub use catalog::*;
pub use citation::*;
pub use claim::*;
pub use claim_field::*;
pub use companion::*;
pub use dates::*;
pub use decider::*;
pub use declaration::*;
pub use delivery_items::*;
pub use diff_gate::*;
pub use embodiment::*;
pub use field::*;
pub use filename::*;
pub use header::*;
pub use identity::*;
pub use index::*;
pub use locator_mechanism::*;
pub use narrative::*;
pub use open_pr::*;
pub use options::*;
pub use override_log::*;
pub use pointer::*;
pub use relation::*;
pub use revision_log::*;
pub use roadmap::*;
pub use runner::*;
pub use schema_version::*;
pub use section::*;
pub use sequence::*;
pub use shared::*;
pub use status::*;
pub use table::*;
pub use type_scope::*;
pub use waiver::*;

/// Fixtures shared by every rule family's tests.
#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn record(path: &str, record_type: &str, content: &str) -> Record {
        Record::parse(PathBuf::from(path), record_type.to_string(), content)
    }

    pub(crate) fn record_with_prefix(
        path: &str,
        record_type: &str,
        prefix: &str,
        content: &str,
    ) -> Record {
        Record::parse_with_shape_and_prefix(
            PathBuf::from(path),
            record_type.to_string(),
            content,
            crate::header::HeaderShape::default(),
            prefix.to_string(),
        )
    }

    /// A minimal `Config` from `(type, RecordTypeConfig)` pairs -- for the
    /// capability-driven rules (`header_pointer_field_clean`,
    /// `narrative_field_stale`) that read `pointer_fields`/`narrative_fields`
    /// via `fields_with_capability` rather than a precomputed map (MILE-90).
    pub(crate) fn config_with_types(
        entries: Vec<(&str, crate::config::RecordTypeConfig)>,
    ) -> Config {
        Config {
            schema_version: 2,
            record_types: entries
                .into_iter()
                .map(|(type_name, type_config)| (type_name.to_string(), type_config))
                .collect(),
            ..Default::default()
        }
    }

    pub(crate) fn terminal_for_tests() -> Vec<String> {
        [
            "Fixed",
            "WontFix",
            "Accepted",
            "Rejected",
            "Superseded",
            "Done",
            "WontDo",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    pub(crate) fn type_config(spec: Option<&str>) -> crate::config::RecordTypeConfig {
        crate::config::RecordTypeConfig {
            dir: "docs/x".to_string(),
            spec: spec.map(|s| s.to_string()),
            ..Default::default()
        }
    }

    /// A `Config` declaring `required_fields` per type (`ADR-59`), for the
    /// rules that used to take a bare `required_by_type`/`declared_by_type`
    /// map directly.
    pub(crate) fn config_for_required(entries: Vec<(&str, Vec<&str>)>) -> Config {
        config_with_types(
            entries
                .into_iter()
                .map(|(name, fields)| {
                    (
                        name,
                        crate::config::RecordTypeConfig {
                            required_fields: fields.into_iter().map(String::from).collect(),
                            ..type_config(None)
                        },
                    )
                })
                .collect(),
        )
    }

    /// A `Config` declaring `known_fields` per type (`ADR-59`), for
    /// `header_field_set_consistency`'s `allowed_by_type`.
    pub(crate) fn config_for_known(entries: Vec<(&str, Vec<&str>)>) -> Config {
        config_with_types(
            entries
                .into_iter()
                .map(|(name, fields)| {
                    (
                        name,
                        crate::config::RecordTypeConfig {
                            known_fields: Some(fields.into_iter().map(String::from).collect()),
                            ..type_config(None)
                        },
                    )
                })
                .collect(),
        )
    }

    pub(crate) fn type_config_with_shape(
        shape: crate::header::HeaderShape,
    ) -> crate::config::RecordTypeConfig {
        crate::config::RecordTypeConfig {
            header_shape: shape,
            ..type_config(None)
        }
    }

    /// What the rule judged, per its own census. A planted test asserts the
    /// rule *looked* -- a finding's absence alone is the ADR-55 defect.
    pub(crate) fn examined(exec: &RuleExecution) -> usize {
        exec.population
            .as_ref()
            .expect("every rule carries a population")
            .examined()
    }

    pub(crate) fn unreadable(exec: &RuleExecution) -> usize {
        exec.population
            .as_ref()
            .expect("every rule carries a population")
            .unreadable()
    }

    pub(crate) fn embodiment_config() -> Config {
        config_with_types(vec![(
            "adr",
            type_config_pointer(&[], Some(&["Embodiment", "Realized-by"]), None, None),
        )])
    }

    pub(crate) fn type_config_pointer(
        required_fields: &[&str],
        known_fields: Option<&[&str]>,
        pointer_fields: Option<&[&str]>,
        narrative_fields: Option<&[&str]>,
    ) -> crate::config::RecordTypeConfig {
        crate::config::RecordTypeConfig {
            required_fields: required_fields.iter().map(|s| s.to_string()).collect(),
            known_fields: known_fields.map(|f| f.iter().map(|s| s.to_string()).collect()),
            pointer_fields: pointer_fields.map(|f| f.iter().map(|s| s.to_string()).collect()),
            narrative_fields: narrative_fields.map(|f| f.iter().map(|s| s.to_string()).collect()),
            ..type_config(None)
        }
    }

    pub(crate) fn exec_with(
        rule: &str,
        eligible: usize,
        examined: usize,
        out_of_scope: usize,
    ) -> RuleExecution {
        RuleExecution {
            rule: rule.to_string(),
            population: Some(Population::detailed(
                PopulationUnit::Record,
                eligible,
                examined,
                out_of_scope,
                0,
            )),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        }
    }

    /// A record with frontmatter, for the cases where the H1's own line
    /// number is what's under test.
    pub(crate) fn yaml_record(path: &str, content: &str) -> (Record, HashMap<PathBuf, String>) {
        let r = Record::parse_with_shape(
            PathBuf::from(path),
            "adr".to_string(),
            content,
            crate::header::HeaderShape::YamlFrontmatter,
        );
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), content.to_string());
        (r, full_text)
    }
}
