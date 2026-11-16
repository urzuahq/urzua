//! The missing inverse of a declared relation (`MILE-287`): when record A
//! claims a relation with B and B does not name A back, the other half is
//! computable, so `urzua fix` lists it and `--apply` writes it on B.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::{field_line, is_empty_yaml_value, InverseOf, Repair};
use crate::config::{Config, RuleLevel};
use crate::record::Record;
use crate::report::{Finding, Notice, NoticeSeverity};
use crate::rules::{
    build_index_reporting_collisions, extract_references, relation_pair_reciprocity,
    supersession_reciprocity, RULE_RELATION_PAIR_RECIPROCITY,
    RULE_RELATION_SUPERSESSION_RECIPROCITY,
};

/// The finding variants an inverse repair answers, and those whose target
/// does not exist.
pub const UNRECIPROCATED: [&str; 2] = ["supersession-unreciprocated", "pair-unreciprocated"];
const UNRESOLVED: [&str; 2] = ["supersession-unresolved", "pair-unresolved"];

fn enabled(config: &Config, rule: &str) -> bool {
    config
        .rules
        .get(rule)
        .is_some_and(|s| s.level != RuleLevel::Off)
}

/// Whether either reciprocity rule is on, so `fix` has inverses to repair.
pub fn any_enabled(config: &Config) -> bool {
    enabled(config, RULE_RELATION_PAIR_RECIPROCITY)
        || enabled(config, RULE_RELATION_SUPERSESSION_RECIPROCITY)
}

/// The text of the `Status` note every inverse repair carries.
pub fn status_note(status_field: &str) -> String {
    format!(
        "{status_field} is not changed: if this relation means the record's {status_field} should move (to Superseded, say), that is a separate step, and gated where the authority table names the status"
    )
}

/// One claimant asking a target field to name it.
struct Ask {
    claimant: PathBuf,
    claimant_field: String,
    id: String,
}

/// Inverse repairs for the claims in `scope` (a claimant or a target in it),
/// read from the reciprocity rules' own findings so `fix` repairs exactly what
/// `check` reports; plus a notice for each one that cannot be written.
pub fn detect(records: &[Record], scope: &[Record], config: &Config) -> (Vec<Repair>, Vec<Notice>) {
    let (index, _) = build_index_reporting_collisions(records);
    let untracked = Default::default();
    let mut findings: Vec<Finding> = Vec::new();
    if enabled(config, RULE_RELATION_SUPERSESSION_RECIPROCITY) {
        findings.extend(supersession_reciprocity(records, config, &index, &untracked).1);
    }
    if enabled(config, RULE_RELATION_PAIR_RECIPROCITY) && !config.relation_pairs.is_empty() {
        findings.extend(relation_pair_reciprocity(records, config, &index, &untracked).1);
    }
    let in_scope = |p: &PathBuf| scope.iter().any(|r| &r.path == p);
    let mut notices = Vec::new();
    let mut asks: BTreeMap<(PathBuf, String), Vec<Ask>> = BTreeMap::new();
    for f in &findings {
        let source = &f.action_source;
        if UNRESOLVED.contains(&source.variant) && in_scope(&f.file) {
            notices.push(warning(
                &f.file,
                format!(
                    "{}; no inverse can be written to a record that does not exist",
                    f.message
                ),
            ));
            continue;
        }
        if !UNRECIPROCATED.contains(&source.variant) {
            continue;
        }
        let (Some(target), Some(field), Some(crate::report::ActionValue::Text(id))) =
            (&source.target.file, &source.target.field, &source.value)
        else {
            continue;
        };
        if !in_scope(&f.file) && !in_scope(target) {
            continue;
        }
        let claimant_field = claimant_field(config, source.variant, field);
        asks.entry((target.clone(), field.clone()))
            .or_default()
            .push(Ask {
                claimant: f.file.clone(),
                claimant_field,
                id: id.clone(),
            });
    }
    let mut repairs = Vec::new();
    for ((target, field), asks) in asks {
        let Some(record) = records.iter().find(|r| r.path == target) else {
            continue;
        };
        match repair_for(record, &field, &asks, config) {
            Ok(repair) => repairs.push(repair),
            Err(why) => notices.push(warning(&target, why)),
        }
    }
    (repairs, notices)
}

fn warning(subject: &std::path::Path, message: String) -> Notice {
    Notice {
        severity: NoticeSeverity::Warning,
        subject: subject.display().to_string(),
        message,
        detail: None,
    }
}

/// The field the claim is written in: the same combined field for the
/// supersession rule, the other side of the pair for a declared pair.
fn claimant_field(config: &Config, variant: &str, echo: &str) -> String {
    if variant == "supersession-unreciprocated" {
        return echo.to_string();
    }
    config
        .relation_pairs
        .iter()
        .find_map(|p| {
            if p.inverse == echo {
                Some(p.forward.clone())
            } else if p.forward == echo {
                Some(p.inverse.clone())
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn repair_for(
    record: &Record,
    field: &str,
    asks: &[Ask],
    config: &Config,
) -> Result<Repair, String> {
    let mut ids: Vec<&str> = Vec::new();
    for a in asks {
        if !ids.contains(&a.id.as_str()) {
            ids.push(&a.id);
        }
    }
    let first = &asks[0];
    let claimants = asks
        .iter()
        .map(|a| format!("{} {}", a.claimant.display(), a.claimant_field))
        .collect::<Vec<_>>()
        .join("; ");
    let current = record.header.get(field);
    if current.is_none() {
        if let Some(t) = config.record_types.get(&record.record_type) {
            if !t
                .declared_fields()
                .contains(&crate::values::FieldName::new(field))
            {
                return Err(format!(
                    "{claimants} asks {field} to name {}, but type '{}' does not declare {field}; the inverse is not written (declare it, or correct the claim)",
                    ids.join(", "),
                    record.record_type
                ));
            }
        }
    }
    let stated = current.unwrap_or("").trim().to_string();
    let computed = if is_empty_yaml_value(&stated) || crate::values::is_no_value_sentinel(&stated) {
        ids.join(", ")
    } else if extract_references(&stated).is_empty() {
        return Err(format!(
            "{field} holds '{stated}', which names no record, and {claimants} asks it to name {}: the existing value is not overwritten; correct one of the two records by hand",
            ids.join(", ")
        ));
    } else if let Some(inner) = stated.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        format!("[{}, {}]", inner.trim(), ids.join(", "))
    } else {
        format!("{stated}, {}", ids.join(", "))
    };
    let status_field = crate::rules::relation_field_name(
        config,
        &record.record_type,
        crate::config::RelationRole::Status,
    );
    Ok(Repair {
        record: record.path.clone(),
        field: field.to_string(),
        line: field_line(record, field),
        current_value: if current.is_none() {
            String::new()
        } else {
            stated
        },
        computed_value: computed,
        tier: 2,
        evidence: claimants,
        inverse: Some(InverseOf {
            claimant: first.claimant.clone(),
            claimant_field: first.claimant_field.clone(),
            field_absent: current.is_none(),
            status_field: status_field.to_string(),
            status: status_note(status_field),
        }),
    })
}
