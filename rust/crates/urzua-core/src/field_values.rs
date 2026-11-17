//! A record type's declared value vocabulary for its ordinary header fields
//! (`MILE-248`, the enumerated-values part).
//!
//! ```yaml
//! record_types:
//!   work:
//!     known_fields: [Kind, Status]
//!     field_values:
//!       Kind: [feature, bug, chore]
//! ```
//!
//! Each entry maps a declared field to the whole values it may hold, compared
//! exactly (case and spelling). A value holding several comma-separated
//! elements (a YAML list reads as one) has each element checked, so a
//! vocabulary entry may not itself contain a comma. A missing or blank value
//! is not judged here: presence is `header.required-fields`' and quality is
//! `field.quality`'s. The status field keeps its own vocabulary in
//! `statuses.values`, and a pointer or narrative field holds references, not
//! a vocabulary. The value of a map entry is a list today; other value types
//! (a range, a pattern) can be added as a mapping beside it.

use crate::config::{ConfigError, RecordTypeConfig, RelationRole};
use std::collections::BTreeMap;

fn invalid(record_type: &str, reason: String) -> ConfigError {
    ConfigError::InvalidDocumentModel {
        record_type: record_type.to_string(),
        key: "field_values",
        reason,
    }
}

/// The elements of a header value, trimmed and non-blank.
pub fn elements(value: &str) -> impl Iterator<Item = &str> {
    value.split(',').map(str::trim).filter(|e| !e.is_empty())
}

/// The elements of `value` that are not in `allowed`, in order, each once.
pub fn undeclared<'a>(allowed: &[String], value: &'a str) -> Vec<&'a str> {
    let mut out: Vec<&str> = Vec::new();
    for e in elements(value) {
        if !allowed.iter().any(|a| a == e) && !out.contains(&e) {
            out.push(e);
        }
    }
    out
}

impl RecordTypeConfig {
    /// The values `field` may hold, when the type declares a vocabulary for it.
    pub fn field_vocabulary(&self, field: &str) -> Option<&[String]> {
        self.field_values
            .as_ref()
            .and_then(|m| m.get(field))
            .map(Vec::as_slice)
    }
}

pub(crate) fn validate(types: &BTreeMap<String, RecordTypeConfig>) -> Result<(), ConfigError> {
    for (name, rt) in types {
        if let Some(map) = &rt.field_values {
            validate_one(rt, map).map_err(|reason| invalid(name, reason))?;
        }
    }
    Ok(())
}

fn validate_one(rt: &RecordTypeConfig, map: &BTreeMap<String, Vec<String>>) -> Result<(), String> {
    if map.is_empty() {
        return Err(
            "is empty, which declares no vocabulary; name at least one field or remove the key"
                .into(),
        );
    }
    let declared = rt.declared_fields();
    let status_field = rt.relation_field(RelationRole::Status);
    for (field, values) in map {
        if field == status_field {
            return Err(format!(
                "names '{field}', the status field, whose values are declared under `statuses.values`"
            ));
        }
        if !declared.contains(&crate::values::FieldName::new(field.as_str())) {
            return Err(format!(
                "names '{field}', which is in neither `required_fields` nor `known_fields`"
            ));
        }
        for (key, list) in [
            ("pointer_fields", &rt.pointer_fields),
            ("narrative_fields", &rt.narrative_fields),
        ] {
            if list.iter().flatten().any(|f| f == field) {
                return Err(format!(
                    "names '{field}', which is declared in `{key}` and holds references, not a vocabulary"
                ));
            }
        }
        check_values(field, values)?;
    }
    Ok(())
}

fn check_values(field: &str, values: &[String]) -> Result<(), String> {
    if values.is_empty() {
        return Err(format!(
            "`{field}` lists no values, which no record could satisfy; list at least one or remove the entry"
        ));
    }
    for (i, v) in values.iter().enumerate() {
        if v.trim().is_empty() || v.trim() != v {
            return Err(format!(
                "`{field}` lists '{v}', which is blank or has surrounding whitespace"
            ));
        }
        if v.contains(',') {
            return Err(format!(
                "`{field}` lists '{v}', which contains a comma, the separator between the elements of a value"
            ));
        }
        if values[..i].contains(v) {
            return Err(format!("`{field}` lists '{v}' twice"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(extra: &str) -> Result<crate::config::Config, ConfigError> {
        crate::config::parse(&format!(
            "schema_version: 2\nrecord_types:\n  work:\n    dir: docs/work\n    required_fields: [Status]\n    known_fields: [Kind, Parent, Labels]\n    pointer_fields: [Parent]\n    narrative_fields: []\n{extra}"
        ))
    }

    fn err(extra: &str) -> String {
        load(extra).unwrap_err().to_string()
    }

    #[test]
    fn a_declared_vocabulary_loads() {
        let c = load("    field_values: {Kind: [feature, bug, chore]}\n").unwrap();
        assert_eq!(
            c.record_types["work"].field_vocabulary("Kind").unwrap(),
            ["feature", "bug", "chore"]
        );
    }

    #[test]
    fn bad_declarations_are_refused_at_load() {
        assert!(err("    field_values: {Kind: []}\n").contains("lists no values"));
        assert!(err("    field_values: {Kind: [bug, bug]}\n").contains("twice"));
        assert!(err("    field_values: {Kind: [\"a,b\"]}\n").contains("comma"));
        assert!(err("    field_values: {Status: [Open]}\n").contains("statuses.values"));
        assert!(err("    field_values: {Parent: [x]}\n").contains("pointer_fields"));
        assert!(err("    field_values: {Nope: [x]}\n").contains("neither"));
        assert!(err("    field_values: {}\n").contains("is empty"));
    }

    #[test]
    fn each_element_of_a_list_value_is_checked_exactly() {
        let allowed = vec!["bug".to_string(), "chore".to_string()];
        assert_eq!(
            undeclared(&allowed, "bug, Bug, feature, Bug"),
            ["Bug", "feature"]
        );
        assert!(undeclared(&allowed, "chore").is_empty());
    }
}
