use super::*;
use crate::config::RecordTypeConfig;

/// Why `rule` refuses the `types:` option, or `None` when it takes it. Decided
/// by what the rule judges, never per rule: a rule whose findings sit on a
/// loaded record takes it; one that judges the configuration, files that are
/// not records, a diff's paths, the run itself or a type's directory has no
/// record of a type to narrow to, so `types:` there would do nothing.
pub fn types_refusal(rule: &str) -> Option<&'static str> {
    let def = rule_def(rule)?;
    match (def.scope, def.anchor) {
        (Scope::Config, _) => Some("it judges the configuration, not records of a type"),
        (Scope::Files, _) => Some("it judges files that are not records"),
        (Scope::Diff, _) => Some("it judges a diff's changed paths, not records of a type"),
        (Scope::Run, _) => Some("it judges the run's other rules and findings, not records"),
        (Scope::Records, Anchor::Record | Anchor::RecordOrDeclaredFile(_)) => None,
        (Scope::Records, _) => {
            Some("its findings sit on a type's directory or a path, not on a record")
        }
    }
}

/// The notice subject of a `types:` entry naming a type the rule can never
/// judge.
pub const NOTICE_RULE_TYPES_NEVER_JUDGED: &str = "rule-types-never-judged";

/// Each enabled rule's `types:` entries that name a type the rule can never
/// judge however its records are edited, with why. Read from the type's own
/// declaration, so an entry whose type has no records yet is not reported: a
/// corpus that has not written one is not a configuration that cannot.
pub fn rule_types_never_judged(config: &Config) -> Vec<(&'static str, String, &'static str)> {
    let mut out = Vec::new();
    for (rule, setting) in &config.rules {
        let (Some(types), Some(def)) = (&setting.types, rule_def(rule)) else {
            continue;
        };
        if effective_level(config, def.id) == crate::config::RuleLevel::Off {
            continue;
        }
        for name in types {
            let Some(rt) = config.record_types.get(name) else {
                continue;
            };
            if let Some(why) = never_judges(def, name, rt, config) {
                out.push((def.id, name.clone(), why));
            }
        }
    }
    out
}

fn never_judges(
    def: &RuleDef,
    name: &str,
    rt: &RecordTypeConfig,
    config: &Config,
) -> Option<&'static str> {
    if rt.has_no_header() && def.header_none == HeaderNone::OutOfScope {
        return Some("it declares `header_shape: none` and the rule reads a header");
    }
    let sections = || {
        crate::document_model::declared_model(name, rt)
            .ok()
            .flatten()
            .and_then(|m| m.sections)
    };
    match def.reads_declaration? {
        Declaration::PartOf => (!may_be_a_parent(config, name))
            .then_some("no `part_of: true` pointer field names it as a target, so none of its records is a parent"),
        Declaration::SectionsRequired => sections()
            .is_none_or(|s| s.required.is_empty())
            .then_some("it declares no `sections.required`"),
        Declaration::UniqueHeadings => sections()
            .is_none_or(|s| !s.unique_headings)
            .then_some("it does not declare `sections.unique_headings`"),
        Declaration::NoSkippedLevels => sections()
            .is_none_or(|s| !s.no_skipped_levels)
            .then_some("it does not declare `sections.no_skipped_levels`"),
        Declaration::SectionOrder => sections()
            .is_none_or(|s| s.order.is_none())
            .then_some("it does not declare `sections.order`"),
        _ => None,
    }
}

/// Whether a record of `name` can be a parent: some `part_of` field may point
/// at it (it names `name` among its targets, or constrains no target type).
fn may_be_a_parent(config: &Config, name: &str) -> bool {
    config.part_of.iter().any(|(record_type, field)| {
        config
            .pointer_targets
            .get(record_type)
            .and_then(|fields| fields.get(field))
            .is_none_or(|targets| targets.is_empty() || targets.iter().any(|t| t == name))
    })
}

/// One warning naming every `types:` entry that can never be judged, so the
/// setting is never silently a no-op; nothing when there is none.
pub fn rule_types_notices(config: &Config) -> Vec<Notice> {
    let never = rule_types_never_judged(config);
    if never.is_empty() {
        return Vec::new();
    }
    let entries: Vec<String> = never
        .iter()
        .map(|(rule, record_type, why)| format!("{rule} lists '{record_type}', but {why}"))
        .collect();
    vec![Notice {
        severity: NoticeSeverity::Warning,
        subject: NOTICE_RULE_TYPES_NEVER_JUDGED.to_string(),
        message: format!(
            "a rule's `types:` names a record type the rule never judges, so that entry does nothing: {}",
            entries.join("; ")
        ),
        detail: None,
    }]
}
