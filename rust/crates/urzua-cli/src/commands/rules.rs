//! `urzua rules`: the complete, current rule set this build ships, with each
//! rule's adopter-facing description. Reads no config and no corpus -- this
//! is what the binary itself supports, not what any one repository enabled.
//! The one place `SPEC-2`'s own rule table is generated from
//! (`scripts/generate-rule-table.py`), so the two cannot drift.

use std::process::ExitCode;

use urzua_core::report::{ActionKind, ActionOp, Notice, PopulationUnit, Report};
use urzua_core::rules::{ActionDecl, RuleDef, VariantAction, RULE_SPEC, RULE_TABLE};

/// One finding variant: the action it offers (`kind`, and `op` where the kind
/// has them) or, for an exemption, the reason there is none.
#[derive(serde::Serialize)]
pub(crate) struct ActionEntry {
    pub(crate) variant: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<ActionKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    op: Option<ActionOp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exempt: Option<String>,
    /// The lowest severity a finding of this variant has, whatever level the
    /// repository sets for the rule (`ADR-96`); omitted when there is none.
    #[serde(skip_serializing_if = "Option::is_none")]
    severity_floor: Option<urzua_core::report::FindingSeverity>,
    /// `false` when no rule-level waiver matches this variant (`ADR-96`);
    /// omitted when one can.
    #[serde(skip_serializing_if = "is_true")]
    waivable: bool,
}

impl ActionEntry {
    pub(crate) fn of(a: &VariantAction) -> ActionEntry {
        let (kind, op, exempt) = match a.action {
            ActionDecl::Act { kind, op } => (Some(kind), op, None),
            ActionDecl::Exempt(reason) => (None, None, Some(reason.to_string())),
        };
        ActionEntry {
            variant: a.variant.to_string(),
            kind,
            op,
            exempt,
            severity_floor: a.severity_floor,
            waivable: a.waivable,
        }
    }
}

fn is_true(b: &bool) -> bool {
    *b
}

/// What one rule is, apart from its options: the part `urzua rules` and
/// `urzua explain --rule` print identically.
#[derive(serde::Serialize)]
pub(crate) struct RuleFacts {
    pub(crate) id: String,
    pub(crate) description: String,
    /// Where the rule looks (`records`, `config`, `files`, `diff`, `run`).
    pub(crate) scope: &'static str,
    /// What the rule's verdict on a record depends on (`ADR-90`): whether it
    /// may list the `scope` option.
    pub(crate) judges: &'static str,
    /// What one candidate in the rule's population is, as `check` reports it
    /// in `rules_executed`.
    pub(crate) population_unit: PopulationUnit,
    /// The unit of a second population the rule judges under the same id
    /// (`MILE-186`); omitted when it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) companion_population_unit: Option<PopulationUnit>,
    /// The spec that documents the rule.
    pub(crate) spec: &'static str,
    pub(crate) actions: Vec<ActionEntry>,
}

impl RuleFacts {
    pub(crate) fn of(d: &RuleDef) -> RuleFacts {
        RuleFacts {
            id: d.id.to_string(),
            description: d.description.to_string(),
            scope: d.scope.as_str(),
            judges: d.judges.as_str(),
            population_unit: d.population,
            companion_population_unit: d.companion.map(|(unit, _)| unit),
            spec: RULE_SPEC,
            actions: d.actions.iter().map(ActionEntry::of).collect(),
        }
    }
}

#[derive(serde::Serialize)]
struct RuleEntry {
    #[serde(flatten)]
    facts: RuleFacts,
    /// The options the rule accepts, by name; omitted when it takes none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    options: Vec<&'static str>,
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
            facts: RuleFacts::of(d),
            options: d.options.to_vec(),
        })
        .collect();
    crate::emit(&RulesReport {
        rules,
        notices: Vec::new(),
    })
}
