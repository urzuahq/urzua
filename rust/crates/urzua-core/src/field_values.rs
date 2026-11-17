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

/// A `{Field: [value, ...]}` map whose values are scalars, read as their YAML
/// source text (`1`, `true`) the way a header value is; a null entry or a
/// non-scalar is refused naming `key`, never as a bare serde type error.
pub(crate) fn scalar_map(
    key: &str,
    raw: yaml_serde::Value,
) -> Result<BTreeMap<String, Vec<String>>, String> {
    use yaml_serde::Value;
    let Value::Mapping(map) = raw else {
        return Err(format!(
            "`{key}` is a mapping of a field to a list of values"
        ));
    };
    let mut out = BTreeMap::new();
    for (field, values) in map {
        let field = scalar_text(&field)
            .ok_or_else(|| format!("`{key}` has a field name that is not a plain value"))?;
        let Value::Sequence(items) = values else {
            return Err(format!("`{key}.{field}` is not a list of values"));
        };
        let mut list = Vec::new();
        for item in &items {
            list.push(scalar_text(item).ok_or_else(|| {
                format!("`{key}.{field}` has an entry that is null or not a plain value")
            })?);
        }
        out.insert(field, list);
    }
    Ok(out)
}

fn scalar_text(v: &yaml_serde::Value) -> Option<String> {
    use yaml_serde::Value;
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// `field_values` as a type declares it: present with no value is refused.
pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<BTreeMap<String, Vec<String>>>, D::Error> {
    use serde::Deserialize;
    match Option::<yaml_serde::Value>::deserialize(d)? {
        None => Err(serde::de::Error::custom(
            "`field_values` is present with no value; give it a mapping or remove the key",
        )),
        Some(v) => scalar_map("field_values", v)
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}

/// `when_field` inside a `statuses.require`/`forbid` entry.
pub(crate) fn deserialize_when_field<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<BTreeMap<String, Vec<String>>, D::Error> {
    use serde::Deserialize;
    match Option::<yaml_serde::Value>::deserialize(d)? {
        None => Err(serde::de::Error::custom(
            "`when_field` is present with no value; give it a mapping or remove the key",
        )),
        Some(v) => scalar_map("when_field", v).map_err(serde::de::Error::custom),
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

/// How a finding words a required field that holds nothing real: absent from
/// the header (`missing from the header`) or present with no value (`empty`),
/// the distinction `header.required-fields` already draws, then the values the
/// type declares for it in the ` (allowed: a, b)` form `field.value-not-declared`
/// uses.
pub fn no_value_text(absent: bool, allowed: Option<&[String]>) -> String {
    let allowed = allowed.map_or_else(String::new, |a| format!(" (allowed: {})", a.join(", ")));
    if absent {
        format!("missing from the header{allowed}")
    } else {
        format!("empty -- not a real, present value{allowed}")
    }
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
        assert!(err("    field_values: {Kind: [~]}\n")
            .contains("`field_values.Kind` has an entry that is null"));
        assert!(err("    field_values:\n").contains("`field_values` is present with no value"));
        let c = load("    field_values: {Kind: [1, true]}\n").unwrap();
        assert_eq!(
            c.record_types["work"].field_vocabulary("Kind").unwrap(),
            ["1", "true"]
        );
        let mapped = crate::config::parse(
            "schema_version: 2\nrecord_types:\n  work:\n    dir: docs/work\n    required_fields: [Status]\n    known_fields: [Parent]\n    pointer_fields: [{name: Parent, targets: [work]}]\n    narrative_fields: []\n    field_values: {Parent: [x]}\n",
        )
        .unwrap_err()
        .to_string();
        assert!(mapped.contains("pointer_fields"), "{mapped}");
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

    #[test]
    fn a_field_with_no_value_is_missing_or_empty_and_names_what_may_go_there() {
        let allowed = ["bug".to_string(), "feature".to_string()];
        assert_eq!(
            no_value_text(true, Some(&allowed)),
            "missing from the header (allowed: bug, feature)"
        );
        assert_eq!(
            no_value_text(false, Some(&allowed)),
            "empty -- not a real, present value (allowed: bug, feature)"
        );
        assert_eq!(no_value_text(true, None), "missing from the header");
        assert_eq!(
            no_value_text(false, None),
            "empty -- not a real, present value"
        );
    }
}
