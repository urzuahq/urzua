//! `urzua rules`: the complete, current rule set this build ships, with each
//! rule's adopter-facing description. Reads no config and no corpus -- this
//! is what the binary itself supports, not what any one repository enabled.
//! The one place `SPEC-2`'s own rule table is generated from
//! (`scripts/generate-rule-table.py`), so the two cannot drift.

use std::process::ExitCode;

use urzua_core::report::{Notice, Report};
use urzua_core::rules::RULE_METADATA;

#[derive(serde::Serialize)]
struct RuleEntry {
    id: String,
    description: String,
}

#[derive(serde::Serialize)]
struct RulesReport {
    rules: Vec<RuleEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
}

impl Report for RulesReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::SUCCESS
    }
}

pub fn run() -> ExitCode {
    let rules = RULE_METADATA
        .iter()
        .map(|m| RuleEntry {
            id: m.id.to_string(),
            description: m.description.to_string(),
        })
        .collect();
    crate::emit(&RulesReport {
        rules,
        notices: Vec::new(),
    })
}
