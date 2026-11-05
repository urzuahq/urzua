//! `urzua rules`: the complete, current rule set this build ships, with each
//! rule's adopter-facing description. Reads no config and no corpus -- this
//! is what the binary itself supports, not what any one repository enabled.
//! The one place `SPEC-2`'s own rule table is generated from
//! (`scripts/generate-rule-table.py`), so the two cannot drift.

use std::process::ExitCode;

use urzua_core::report::{ActionKind, ActionOp, Notice, Report};
use urzua_core::rules::{ActionDecl, RULE_TABLE};

/// One finding variant: the action it offers (`kind`, and `op` where the kind
/// has them) or, for an exemption, the reason there is none.
#[derive(serde::Serialize)]
struct ActionEntry {
    variant: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<ActionKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    op: Option<ActionOp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exempt: Option<String>,
}

#[derive(serde::Serialize)]
struct RuleEntry {
    id: String,
    description: String,
    actions: Vec<ActionEntry>,
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
    let rules = RULE_TABLE
        .iter()
        .map(|d| RuleEntry {
            id: d.id.to_string(),
            description: d.description.to_string(),
            actions: d
                .actions
                .iter()
                .map(|a| match a.action {
                    ActionDecl::Act { kind, op } => ActionEntry {
                        variant: a.variant.to_string(),
                        kind: Some(kind),
                        op,
                        exempt: None,
                    },
                    ActionDecl::Exempt(reason) => ActionEntry {
                        variant: a.variant.to_string(),
                        kind: None,
                        op: None,
                        exempt: Some(reason.to_string()),
                    },
                })
                .collect(),
        })
        .collect();
    crate::emit(&RulesReport {
        rules,
        notices: Vec::new(),
    })
}
