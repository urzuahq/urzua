//! The suggested action each finding variant declares (`ADR-73`).
//!
//! A rule's table entry lists, for every variant of finding the rule can emit,
//! either the kind and `op` of the action it offers or an exemption with a
//! reason. The rule supplies only what it computed (`ActionSource`: the
//! variant, the target and a value); `resolve_action` joins the two. A variant
//! with neither is a bug the coverage tests catch, not a silent default.

use super::*;
use crate::report::SuggestedAction;

/// What a finding variant offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionDecl {
    /// An action of this kind. `op` is `None` only for a kind that carries none.
    Act {
        kind: ActionKind,
        op: Option<ActionOp>,
    },
    /// No action by design, with the reason `urzua rules` shows.
    Exempt(&'static str),
}

impl ActionDecl {
    const fn act(kind: ActionKind, op: ActionOp) -> Self {
        ActionDecl::Act { kind, op: Some(op) }
    }

    pub const FIELD_SET: ActionDecl = Self::act(ActionKind::EditField, ActionOp::Set);
    pub const FIELD_REMOVE: ActionDecl = Self::act(ActionKind::EditField, ActionOp::Remove);
    pub const FIELD_APPEND: ActionDecl = Self::act(ActionKind::EditField, ActionOp::Append);
    pub const FIELD_RENAME: ActionDecl = Self::act(ActionKind::EditField, ActionOp::Rename);
    pub const SECTION_ADD: ActionDecl = Self::act(ActionKind::EditSection, ActionOp::Add);
    pub const SECTION_SET: ActionDecl = Self::act(ActionKind::EditSection, ActionOp::Set);
    pub const CONFIG_SET: ActionDecl = Self::act(ActionKind::EditConfig, ActionOp::Set);
    pub const CONFIG_REMOVE: ActionDecl = Self::act(ActionKind::EditConfig, ActionOp::Remove);
    pub const CONFIG_APPEND: ActionDecl = Self::act(ActionKind::EditConfig, ActionOp::Append);
    pub const ADD_RECORD: ActionDecl = ActionDecl::Act {
        kind: ActionKind::AddRecord,
        op: None,
    };
    pub const RUN_COMMAND: ActionDecl = ActionDecl::Act {
        kind: ActionKind::RunCommand,
        op: None,
    };
}

/// One finding variant of a rule and what it offers. Variant names are unique
/// across the whole table, so a variant names its rule.
#[derive(Clone, Copy, Debug)]
pub struct VariantAction {
    pub variant: &'static str,
    pub action: ActionDecl,
    /// The lowest severity a finding of this variant can have, whatever the
    /// repository's setting for the rule says (`ADR-96`: a floor).
    pub severity_floor: Option<FindingSeverity>,
    /// Whether a `WAIVER` record can excuse a finding of this variant. `false`
    /// means no rule-level waiver matches it, and the rule's `permanent`
    /// categories are not consulted for it (`ADR-96`).
    pub waivable: bool,
}

impl VariantAction {
    pub const fn new(variant: &'static str, action: ActionDecl) -> Self {
        VariantAction {
            variant,
            action,
            severity_floor: None,
            waivable: true,
        }
    }

    /// The variant's severity is at least `severity`, whatever the rule's
    /// declared level.
    pub const fn floored(mut self, severity: FindingSeverity) -> Self {
        self.severity_floor = Some(severity);
        self
    }

    /// No rule-level waiver matches a finding of this variant.
    pub const fn unwaivable(mut self) -> Self {
        self.waivable = false;
        self
    }
}

/// The declaration of the variant a finding is, when its rule's table entry
/// has one.
pub fn variant_declaration(finding: &Finding) -> Option<&'static VariantAction> {
    let variant = finding.action_source.variant;
    if variant.is_empty() {
        return None;
    }
    rule_def(&finding.rule)?
        .actions
        .iter()
        .find(|a| a.variant == variant)
}

/// Whether a rule-level waiver can match `finding`: `false` only for a variant
/// its table entry declares `unwaivable`.
pub fn finding_is_waivable(finding: &Finding) -> bool {
    variant_declaration(finding).is_none_or(|v| v.waivable)
}

/// The `target` keys a kind requires (`ADR-73`'s table). `file` is filled from
/// the finding's own when the rule does not name another.
fn missing_target_keys(kind: ActionKind, target: &ActionTarget) -> Vec<&'static str> {
    let mut missing = Vec::new();
    let mut need = |present: bool, key: &'static str| {
        if !present {
            missing.push(key);
        }
    };
    match kind {
        ActionKind::EditField => {
            need(target.file.is_some(), "file");
            need(target.field.is_some(), "field");
        }
        ActionKind::EditSection => {
            need(target.file.is_some(), "file");
            need(target.section.is_some(), "section");
        }
        ActionKind::EditConfig => {
            need(target.file.is_some(), "file");
            need(target.key_path.is_some(), "key_path");
        }
        ActionKind::AddRecord => {}
        ActionKind::RunCommand => need(target.argv.is_some(), "argv"),
    }
    missing
}

/// Joins what `finding` computed with what its rule declared for its variant,
/// and sets `suggested_action`. A finding no rule produced (an empty variant)
/// and an exempt variant get none.
///
/// A variant the rule did not declare, or a declared action missing a key its
/// kind requires, is a defect in the rule: it asserts in a debug build, where
/// every test that reaches it fails, and offers no action in a release build
/// rather than a malformed one.
pub fn resolve_action(def: &RuleDef, finding: &mut Finding) {
    let source = &finding.action_source;
    if source.variant.is_empty() {
        return;
    }
    let Some(declared) = def.actions.iter().find(|a| a.variant == source.variant) else {
        debug_assert!(
            false,
            "rule '{}' emitted the variant '{}', which its table entry does not declare",
            def.id, source.variant
        );
        return;
    };
    let ActionDecl::Act { kind, op } = declared.action else {
        return;
    };
    let mut target = source.target.clone();
    if kind != ActionKind::AddRecord && kind != ActionKind::RunCommand && target.file.is_none() {
        target.file = Some(finding.file.clone());
    }
    let missing = missing_target_keys(kind, &target);
    debug_assert!(
        missing.is_empty(),
        "rule '{}' variant '{}' lacks the target keys {missing:?} its kind requires",
        def.id,
        source.variant
    );
    if !missing.is_empty() {
        return;
    }
    finding.suggested_action = Some(SuggestedAction {
        kind,
        op,
        target,
        value: source.value.clone(),
    });
}

/// Resolves every finding of the rule `rule_id`. A rule not in the table (a
/// test double) leaves its findings as they are.
pub fn resolve_actions(rule_id: &str, findings: &mut [Finding]) {
    let Some(def) = rule_def(rule_id) else {
        return;
    };
    for finding in findings {
        resolve_action(def, finding);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::ActionValue;
    use std::collections::{BTreeMap, BTreeSet};

    fn finding_with(rule: &str, source: ActionSource) -> Finding {
        Finding {
            rule: rule.to_string(),
            severity: FindingSeverity::Error,
            file: PathBuf::from("docs/adr/ADR-1-x.md"),
            line: None,
            message: "m".to_string(),
            waived: None,
            declared_severity: None,
            suggested_action: None,
            action_source: source,
        }
    }

    /// Every variant of every rule, with the rule that declares it.
    fn declared() -> Vec<(&'static str, &'static VariantAction)> {
        RULE_TABLE
            .iter()
            .flat_map(|d| d.actions.iter().map(move |a| (d.id, a)))
            .collect()
    }

    #[test]
    fn every_rule_declares_at_least_one_variant() {
        let bare: Vec<&str> = RULE_TABLE
            .iter()
            .filter(|d| d.actions.is_empty())
            .map(|d| d.id)
            .collect();
        assert!(
            bare.is_empty(),
            "rules that declare no finding variant, so no action or exemption: {bare:?}"
        );
    }

    #[test]
    fn a_variant_names_one_rule_and_is_declared_once() {
        let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
        for (rule, a) in declared() {
            if let Some(other) = seen.insert(a.variant, rule) {
                panic!(
                    "variant '{}' is declared by '{other}' and by '{rule}'",
                    a.variant
                );
            }
            assert!(
                !a.variant.is_empty()
                    && a.variant
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c == '-'),
                "variant '{}' is not kebab-case",
                a.variant
            );
        }
    }

    #[test]
    fn an_exemption_states_a_reason_and_an_action_a_valid_op() {
        for (rule, a) in declared() {
            match a.action {
                ActionDecl::Exempt(reason) => assert!(
                    reason.trim().len() > 10,
                    "'{rule}' variant '{}' is exempt without a reason",
                    a.variant
                ),
                ActionDecl::Act { kind, op } => match op {
                    Some(op) => assert!(
                        kind.ops().contains(&op),
                        "'{rule}' variant '{}': {op:?} is not an op of {}",
                        a.variant,
                        kind.as_str()
                    ),
                    None => assert!(
                        kind.ops().is_empty(),
                        "'{rule}' variant '{}': {} needs an op",
                        a.variant,
                        kind.as_str()
                    ),
                },
            }
        }
    }

    /// The source of every rule module, test modules cut off: what a rule can
    /// construct, which is the set of variants the table must cover.
    fn rule_sources() -> Vec<(String, String)> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/rules");
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if !name.ends_with(".rs") || name == "table.rs" || name == "action.rs" {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let live = text
                .split("#[cfg(test)]")
                .next()
                .unwrap_or_default()
                .to_string();
            out.push((name, live));
        }
        out
    }

    #[test]
    fn a_variant_a_rule_constructs_is_declared_in_the_table() {
        let all: BTreeSet<&str> = declared().iter().map(|(_, a)| a.variant).collect();
        let mut missing = Vec::new();
        for (file, live) in rule_sources() {
            let mut rest = live.as_str();
            while let Some(at) = rest.find("ActionSource::new(\"") {
                let tail = &rest[at + "ActionSource::new(\"".len()..];
                let literal = &tail[..tail.find('"').unwrap()];
                if !all.contains(literal) {
                    missing.push(format!("{file}: '{literal}'"));
                }
                rest = tail;
            }
        }
        assert!(
            missing.is_empty(),
            "variants constructed with neither an action nor an exemption in RULE_TABLE: {missing:?}"
        );
    }

    #[test]
    fn a_declared_variant_is_constructed_by_some_rule() {
        let sources: Vec<String> = rule_sources().into_iter().map(|(_, s)| s).collect();
        let dead: Vec<String> = declared()
            .iter()
            .filter(|(_, a)| {
                let quoted = format!("\"{}\"", a.variant);
                !sources.iter().any(|s| s.contains(&quoted))
            })
            .map(|(rule, a)| format!("{rule}: '{}'", a.variant))
            .collect();
        assert!(dead.is_empty(), "declared but never constructed: {dead:?}");
    }

    /// The top-level arguments of every `Finding::new(` call in `live`, each
    /// trimmed. A scanner, not a parser: it balances brackets and skips string
    /// literals and line comments, which is all a rule module's calls need.
    fn finding_new_calls(live: &str) -> Vec<Vec<String>> {
        const CALL: &str = "Finding::new(";
        let mut calls = Vec::new();
        let mut from = 0;
        while let Some(at) = live[from..].find(CALL) {
            let start = from + at + CALL.len();
            let bytes = live.as_bytes();
            let (mut depth, mut i, mut arg_start) = (1usize, start, start);
            let mut args = Vec::new();
            while depth > 0 {
                match bytes[i] {
                    b'"' => {
                        i += 1;
                        while bytes[i] != b'"' {
                            i += if bytes[i] == b'\\' { 2 } else { 1 };
                        }
                    }
                    b'/' if bytes[i + 1] == b'/' => {
                        while bytes[i] != b'\n' {
                            i += 1;
                        }
                    }
                    b'(' | b'[' | b'{' => depth += 1,
                    b')' | b']' | b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            args.push(live[arg_start..i].trim().to_string());
                        }
                    }
                    b',' if depth == 1 => {
                        args.push(live[arg_start..i].trim().to_string());
                        arg_start = i + 1;
                    }
                    _ => {}
                }
                i += 1;
            }
            // A trailing comma leaves an empty last argument.
            if args.last().is_some_and(String::is_empty) {
                args.pop();
            }
            calls.push(args);
            from = i;
        }
        calls
    }

    /// Whether a `Finding::new` argument is an action source: built in place
    /// (`ActionSource::new(` anywhere in the expression), a call to a helper
    /// named `*_source`, a parameter declared as an `ActionSource`, or a local
    /// bound to one a few lines above.
    fn names_an_action_source(live: &str, arg: &str) -> bool {
        if arg.contains("ActionSource::new(") {
            return true;
        }
        let code: String = arg
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        let head: String = code
            .trim()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let bound_here = live.find(&format!("let {head} = ")).is_some_and(|at| {
            let rest = &live[at..];
            rest[..rest.len().min(400)].contains("ActionSource::new(")
        });
        !head.is_empty()
            && (head.ends_with("_source") && code.trim()[head.len()..].starts_with('(')
                || live.contains(&format!("{head}: ActionSource"))
                || bound_here)
    }

    /// A finding is built either by a struct literal, which must carry an
    /// `action_source` field, or by `Finding::new(rule, severity, file,
    /// action_source, message)`, whose fourth argument must be one. Both
    /// kinds are counted together, so a finding built either way without an
    /// action source is caught.
    #[test]
    fn every_finding_a_rule_constructs_names_a_variant() {
        for (file, live) in rule_sources() {
            let literals = live.matches("Finding {").count()
                - live.matches("-> Finding {").count()
                - live.matches("pub struct Finding {").count();
            let calls = finding_new_calls(&live);
            let constructed = literals + calls.len();
            let literal_sourced = live.matches("action_source: ").count()
                + live.matches("                action_source,").count()
                - live.matches("action_source: ActionSource,").count()
                - live.matches("action_source: ActionSource)").count();
            let call_sourced = calls
                .iter()
                .filter(|args| args.len() == 5 && names_an_action_source(&live, &args[3]))
                .count();
            assert_eq!(
                constructed,
                literal_sourced + call_sourced,
                "{file}: a Finding is built without an action_source"
            );
            assert!(
                !live.contains("ActionSource::default()"),
                "{file}: a rule builds a finding with the empty variant, which no rule can declare"
            );
        }
    }

    #[test]
    fn a_declared_action_is_joined_with_what_the_rule_computed() {
        let def = rule_def(RULE_FIELD_UNTRIMMED_VALUE).unwrap();
        let mut f = finding_with(
            def.id,
            ActionSource::new("untrimmed-value")
                .field("Status")
                .text("Accepted"),
        );
        resolve_action(def, &mut f);
        let action = f.suggested_action.expect("an action");
        assert_eq!(action.kind, ActionKind::EditField);
        assert_eq!(action.op, Some(ActionOp::Set));
        assert_eq!(action.target.field.as_deref(), Some("Status"));
        // `file` defaults to the finding's own.
        assert_eq!(
            action.target.file.as_deref(),
            Some(Path::new("docs/adr/ADR-1-x.md"))
        );
        assert_eq!(action.value, Some(ActionValue::Text("Accepted".into())));
    }

    #[test]
    fn an_exempt_variant_and_a_finding_no_rule_made_get_no_action() {
        let def = rule_def(RULE_IDENTITY_COLLISION).unwrap();
        let mut exempt = finding_with(def.id, ActionSource::new("identifier-collision"));
        resolve_action(def, &mut exempt);
        assert_eq!(exempt.suggested_action, None);
        let mut unmade = finding_with(def.id, ActionSource::default());
        resolve_action(def, &mut unmade);
        assert_eq!(unmade.suggested_action, None);
    }

    #[test]
    #[should_panic(expected = "does not declare")]
    #[cfg(debug_assertions)]
    fn a_variant_the_table_does_not_declare_is_a_defect() {
        let def = rule_def(RULE_FIELD_QUALITY).unwrap();
        let mut f = finding_with(def.id, ActionSource::new("not-a-variant"));
        resolve_action(def, &mut f);
    }

    #[test]
    #[should_panic(expected = "lacks the target keys")]
    #[cfg(debug_assertions)]
    fn a_declared_action_missing_a_required_key_is_a_defect() {
        // `edit-field` needs a field; the rule computed none.
        let def = rule_def(RULE_FIELD_QUALITY).unwrap();
        let mut f = finding_with(def.id, ActionSource::new("field-quality"));
        resolve_action(def, &mut f);
    }
}
