//! Declared target types of reference fields (`MILE-259`): which record types
//! a pointer field, or the forward field of a relation pair, may name.
//!
//! A `pointer_fields` entry is either a field name or `{name, targets}`, and
//! may add `part_of: true`, which declares the field as containment: a record
//! naming another through it is part of that record. The typed parse keeps only
//! the names (so every reader of `pointer_fields` is unchanged) and `collect`
//! reads the `targets` and `part_of` from the raw document.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer};

use crate::config::{Config, ConfigError};

/// Declared target types: record type, then field, then the allowed types.
pub type PointerTargets = BTreeMap<String, BTreeMap<String, Vec<String>>>;

/// The field each record type declares as containment (`part_of: true`).
pub type PartOf = BTreeMap<String, String>;

/// One `pointer_fields` entry: a bare name, or `{name, targets}`. A bare
/// plain scalar reads as text, as the typed parse of a name always has, so a
/// numeric field name still loads.
struct Entry {
    name: String,
    targets: Option<Vec<String>>,
    part_of: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Constrained {
    name: String,
    #[serde(default)]
    targets: Option<Vec<String>>,
    #[serde(default)]
    part_of: bool,
}

impl<'de> Deserialize<'de> for Entry {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = Entry;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a field name or {name, targets}")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Entry, E> {
                Ok(Entry {
                    name: v.to_string(),
                    targets: None,
                    part_of: false,
                })
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Entry, E> {
                self.visit_str(&v.to_string())
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Entry, E> {
                self.visit_str(&v.to_string())
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Entry, E> {
                self.visit_str(&v.to_string())
            }
            fn visit_i128<E: serde::de::Error>(self, v: i128) -> Result<Entry, E> {
                self.visit_str(&v.to_string())
            }
            fn visit_u128<E: serde::de::Error>(self, v: u128) -> Result<Entry, E> {
                self.visit_str(&v.to_string())
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Entry, E> {
                self.visit_str(&format!("{v:?}"))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, map: A) -> Result<Entry, A::Error> {
                let c =
                    Constrained::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
                if c.targets.is_none() && !c.part_of {
                    return Err(serde::de::Error::missing_field("targets"));
                }
                Ok(Entry {
                    name: c.name,
                    targets: c.targets,
                    part_of: c.part_of,
                })
            }
        }
        d.deserialize_any(V)
    }
}

/// `pointer_fields`, read as names whichever form each entry takes.
pub(crate) fn names<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<String>>, D::Error> {
    let entries: Option<Vec<Entry>> = Option::deserialize(d)?;
    Ok(entries.map(|v| v.into_iter().map(|e| e.name).collect()))
}

/// The `targets` of every `{name, targets}` entry and the `part_of` field of
/// every type, read by a second, narrow parse that ignores every other key, so
/// it accepts whatever the typed parse did.
pub(crate) fn collect(content: &str) -> Result<(PointerTargets, PartOf), ConfigError> {
    #[derive(Deserialize)]
    struct Doc {
        #[serde(default)]
        record_types: BTreeMap<String, TypeDecl>,
    }
    #[derive(Deserialize)]
    struct TypeDecl {
        #[serde(default)]
        pointer_fields: Option<Vec<Entry>>,
    }
    let doc: Doc = yaml_serde::from_str(crate::config::strip_bom(content))?;
    let mut out = PointerTargets::new();
    let mut part_of = PartOf::new();
    for (type_name, decl) in doc.record_types {
        let entries = decl.pointer_fields.unwrap_or_default();
        read_part_of(&type_name, &entries, &mut part_of)?;
        if let Some(name) = repeated_constrained(&entries) {
            return Err(ConfigError::InvalidPointerTargets {
                reason: format!(
                    "record type '{type_name}' pointer_fields lists '{name}' more than once with targets; declare it once"
                ),
            });
        }
        for entry in entries {
            if let Some(targets) = entry.targets {
                out.entry(type_name.clone())
                    .or_default()
                    .insert(entry.name, targets);
            }
        }
    }
    Ok((out, part_of))
}

/// The one `part_of` field of a type. A containment field names the record a
/// record is part of, so it needs `targets` (a reference alone does not say
/// "is part of"), and a record has one parent, so a type declares one.
fn read_part_of(
    type_name: &str,
    entries: &[Entry],
    part_of: &mut PartOf,
) -> Result<(), ConfigError> {
    let marked: Vec<&Entry> = entries.iter().filter(|e| e.part_of).collect();
    if let Some(entry) = marked.iter().find(|e| e.targets.is_none()) {
        return bad(format!(
            "record type '{type_name}' pointer field '{}' declares part_of: true without targets; containment needs the types it may name",
            entry.name
        ));
    }
    if marked.len() > 1 {
        return bad(format!(
            "record type '{type_name}' declares part_of on more than one pointer field ('{}' and '{}'); a record has one parent",
            marked[0].name, marked[1].name
        ));
    }
    if let Some(entry) = marked.first() {
        part_of.insert(type_name.to_string(), entry.name.clone());
    }
    Ok(())
}

/// A field listed more than once where any listing carries `targets`: which
/// constraint was meant is ambiguous. Two bare names are left as they loaded before.
fn repeated_constrained(entries: &[Entry]) -> Option<&str> {
    entries.iter().find_map(|e| {
        let same: Vec<&Entry> = entries.iter().filter(|o| o.name == e.name).collect();
        (same.len() > 1 && same.iter().any(|o| o.targets.is_some())).then_some(e.name.as_str())
    })
}

fn bad(reason: String) -> Result<(), ConfigError> {
    Err(ConfigError::InvalidPointerTargets { reason })
}

/// A declared target list names configured record types, at least one, once each.
fn check_list(
    subject: &str,
    key: &str,
    targets: &[String],
    config: &Config,
) -> Result<(), ConfigError> {
    if targets.is_empty() {
        return bad(format!(
            "{subject} {key} must list at least one record type"
        ));
    }
    let mut seen = BTreeSet::new();
    for t in targets {
        if !seen.insert(t.as_str()) {
            return bad(format!("{subject} {key} lists '{t}' twice"));
        }
        if !config.record_types.contains_key(t) {
            return bad(format!(
                "{subject} {key} names '{t}', which is not a configured record type"
            ));
        }
    }
    Ok(())
}

/// The record types that carry a pair's forward field.
pub(crate) fn forward_carriers<'a>(
    config: &'a Config,
    pair: &'a crate::config::RelationPair,
) -> impl Iterator<Item = &'a str> + 'a {
    config
        .record_types
        .iter()
        .filter(move |(name, _)| {
            pair.types
                .as_ref()
                .is_none_or(|types| types.iter().any(|t| t == *name))
        })
        .filter(move |(_, rt)| {
            rt.declared_fields()
                .contains(&crate::values::FieldName::new(&pair.forward))
        })
        .map(|(name, _)| name.as_str())
}

/// Every declared target list is well formed, and agrees with `cross_type`.
pub(crate) fn validate(config: &Config) -> Result<(), ConfigError> {
    for (type_name, fields) in &config.pointer_targets {
        for (field, targets) in fields {
            let subject = format!("record type '{type_name}' pointer field '{field}'");
            check_list(&subject, "targets", targets, config)?;
        }
    }
    for pair in &config.relation_pairs {
        let Some(targets) = &pair.target_types else {
            continue;
        };
        let subject = format!("pair '{}'", pair.name);
        check_list(&subject, "target_types", targets, config)?;
        if pair.cross_type {
            continue;
        }
        // `cross_type: false` keeps a claim within the claimant's own type, so a
        // target type no claimant has could never be named without a finding.
        let carriers: BTreeSet<&str> = forward_carriers(config, pair).collect();
        if let Some(t) = targets.iter().find(|t| !carriers.contains(t.as_str())) {
            return bad(format!(
                "pair '{}' declares cross_type: false, but target_types names '{t}', which does not carry its forward field '{}'; a claim may only name its own type",
                pair.name, pair.forward
            ));
        }
    }
    Ok(())
}

/// The allowed target types of each constrained `(record type, field)`: a
/// `pointer_fields` entry's `targets` and a pair's `target_types` for its
/// forward field. Both declared for one field, a target must satisfy both.
pub fn constraints(config: &Config) -> PointerTargets {
    let mut out = config.pointer_targets.clone();
    for pair in &config.relation_pairs {
        let Some(targets) = &pair.target_types else {
            continue;
        };
        for carrier in forward_carriers(config, pair) {
            let slot = out.entry(carrier.to_string()).or_default();
            match slot.get_mut(&pair.forward) {
                Some(existing) => existing.retain(|t| targets.contains(t)),
                None => {
                    slot.insert(pair.forward.clone(), targets.clone());
                }
            }
        }
    }
    out
}

/// The allowed types for `field` on `record_type`, when constrained.
pub fn allowed<'a>(
    targets: &'a PointerTargets,
    record_type: &str,
    field: &str,
) -> Option<&'a [String]> {
    targets
        .get(record_type)
        .and_then(|f| f.get(field))
        .map(Vec::as_slice)
}

#[cfg(test)]
mod tests {
    use crate::config::{parse, ConfigError};

    const HEAD: &str = "schema_version: 2\nrecord_types:\n  a:\n    dir: docs/a\n    required_fields: [Status]\n    known_fields: [Parent, Up, Down]\n    narrative_fields: []\n";
    const B: &str = "  b:\n    dir: docs/b\n    required_fields: [Status]\n    known_fields: [Up, Down]\n    pointer_fields: []\n    narrative_fields: []\n";

    fn cfg(pointer: &str, tail: &str) -> String {
        format!("{HEAD}    pointer_fields: {pointer}\n{B}{tail}")
    }

    fn reason(text: &str) -> String {
        match parse(text) {
            Err(ConfigError::InvalidPointerTargets { reason }) => reason,
            other => panic!("expected InvalidPointerTargets, got {other:?}"),
        }
    }

    #[test]
    fn a_plain_and_a_constrained_entry_load_side_by_side() {
        let c = parse(&cfg("[Up, {name: Parent, targets: [b]}]", "")).unwrap();
        assert_eq!(
            c.record_types["a"].pointer_fields,
            Some(vec!["Up".to_string(), "Parent".to_string()])
        );
        assert_eq!(c.pointer_targets["a"]["Parent"], vec!["b".to_string()]);
        assert!(!c.pointer_targets["a"].contains_key("Up"));
    }

    #[test]
    fn part_of_names_a_types_containment_field_and_needs_targets() {
        let c = parse(&cfg("[Up, {name: Parent, targets: [b], part_of: true}]", "")).unwrap();
        assert_eq!(c.part_of["a"], "Parent");
        assert!(!c.part_of.contains_key("b"));
        assert_eq!(c.pointer_targets["a"]["Parent"], vec!["b".to_string()]);
        let c = parse(&cfg("[{name: Parent, targets: [b], part_of: false}]", "")).unwrap();
        assert!(c.part_of.is_empty(), "part_of: false declares no containment");
        let c = parse(&cfg("[{name: Parent, targets: [b]}]", "")).unwrap();
        assert!(c.part_of.is_empty(), "targets alone are not containment");

        assert!(reason(&cfg("[{name: Parent, part_of: true}]", "")).contains("without targets"));
        assert!(reason(&cfg(
            "[{name: Parent, targets: [b], part_of: true}, {name: Up, targets: [b], part_of: true}]",
            ""
        ))
        .contains("more than one pointer field"));
        assert!(parse(&cfg("[{name: Parent}]", "")).is_err(), "an entry with neither is still refused");
    }

    #[test]
    fn a_config_without_targets_has_no_constraints() {
        let c = parse(&cfg("[Parent]", "")).unwrap();
        assert!(c.pointer_targets.is_empty());
        assert!(super::constraints(&c).is_empty());
    }

    #[test]
    fn an_unknown_empty_or_repeated_target_is_a_load_error() {
        assert!(reason(&cfg("[{name: Parent, targets: [zz]}]", ""))
            .contains("'zz', which is not a configured record type"));
        assert!(
            reason(&cfg("[{name: Parent, targets: []}]", "")).contains("at least one record type")
        );
        assert!(reason(&cfg("[{name: Parent, targets: [b, b]}]", "")).contains("twice"));
    }

    #[test]
    fn a_field_listed_twice_with_targets_is_a_load_error_naming_it() {
        for list in [
            "[{name: Parent, targets: [b]}, {name: Parent, targets: [a]}]",
            "[Parent, {name: Parent, targets: [b]}]",
            "[{name: Parent, targets: [b]}, Parent]",
        ] {
            let r = reason(&cfg(list, ""));
            assert!(r.contains("'Parent' more than once"), "{list}: {r}");
        }
    }

    #[test]
    fn an_unknown_key_in_an_entry_is_a_load_error() {
        assert!(parse(&cfg("[{name: Parent, target: [b]}]", "")).is_err());
    }

    #[test]
    fn targets_on_a_field_not_declared_as_a_pointer_is_a_load_error() {
        let text = format!("{HEAD}    pointer_fields: []\n{B}").replace(
            "    narrative_fields: []\n    pointer_fields: []\n  b:",
            "    narrative_fields: [{name: Parent, targets: [b]}]\n    pointer_fields: []\n  b:",
        );
        assert!(text.contains("narrative_fields: [{name"));
        assert!(parse(&text).is_err());
    }

    #[test]
    fn pair_target_types_are_validated_and_merge_into_constraints() {
        let pair = |extra: &str| {
            format!("relation_pairs:\n  - {{name: link, forward: Up, inverse: Down, reciprocity: none{extra}}}\n")
        };
        let c = parse(&cfg("[]", &pair(", target_types: [b]"))).unwrap();
        let k = super::constraints(&c);
        assert_eq!(k["a"]["Up"], vec!["b".to_string()]);
        assert_eq!(k["b"]["Up"], vec!["b".to_string()]);
        assert!(reason(&cfg("[]", &pair(", target_types: []"))).contains("at least one"));
        assert!(reason(&cfg("[]", &pair(", target_types: [q]")))
            .contains("not a configured record type"));
    }

    #[test]
    fn cross_type_false_with_a_target_no_claimant_has_is_a_load_error() {
        let pair = "relation_pairs:\n  - {name: link, forward: Up, inverse: Down, reciprocity: none, cross_type: false, types: [a], target_types: [b]}\n";
        assert!(reason(&cfg("[]", pair)).contains("cross_type: false"));
        let ok = "relation_pairs:\n  - {name: link, forward: Up, inverse: Down, reciprocity: none, cross_type: false, types: [a], target_types: [a]}\n";
        assert!(parse(&cfg("[]", ok)).is_ok());
    }
}
