use super::*;
use crate::config::RelationRole;

/// The suggested action for one companion finding. With `known_fields`
/// declared, the missing field is appended. With none declared, appending
/// would create a one-item list and switch `header.field-set-consistency` on
/// for the type, so the suggestion sets the full set instead; for a pair of
/// which neither field is declared, it adds both.
fn companion_source(
    type_name: &str,
    type_config: &crate::config::RecordTypeConfig,
    records: &[Record],
    missing: Option<&str>,
    names: &[&str; 2],
) -> ActionSource {
    let key_path = ["record_types", type_name, "known_fields"];
    match missing {
        Some(field) => super::declaration::known_fields_source(
            "companion-field-missing",
            "companion-field-missing-declare-set",
            type_name,
            type_config,
            records,
            field,
        ),
        None if type_config.known_fields.is_none() => {
            let mut source = super::declaration::known_fields_source(
                "companion-pair-undeclared-declare-set",
                "companion-pair-undeclared-declare-set",
                type_name,
                type_config,
                records,
                names[0],
            );
            if let Some(crate::report::ActionValue::List(list)) = source.value.as_mut() {
                list.push(names[1].to_string());
                list.sort();
                list.dedup();
            }
            source
        }
        None => ActionSource::new("companion-pair-undeclared").key_path(key_path),
    }
}

/// Rule (`BUG-41`): a type declaring a companion pair declares both fields of
/// it. The pairs are the type's `companion_fields` (relation role keys),
/// resolved to field names through `relation_fields`; no pair is known to the
/// engine. A type declaring one field of a pair without the other lets records
/// claim a state that the rule reading both cannot judge, because that rule
/// skips a record lacking either field. A pair of which the type declares
/// neither field could never examine a record, and is reported too.
pub fn config_companion_declaration_missing(
    config: &Config,
    config_path: &std::path::Path,
    records: &[Record],
) -> (RuleExecution, Vec<Finding>) {
    schema_inventory_rule(
        config,
        RULE_CONFIG_COMPANION_DECLARATION_MISSING,
        |type_name, type_config, findings| {
            let declared = type_config.declared_fields();
            for pair in type_config.companion_fields.iter().flatten() {
                let names = pair.each_ref().map(|key| {
                    RelationRole::from_config_key(key)
                        .map_or(key.as_str(), |role| type_config.relation_field(role))
                });
                let has = names.map(|name| declared.contains(name));
                let mut missing: Option<&str> = None;
                let message = match has {
                    [true, true] => continue,
                    [true, false] => {
                        missing = Some(names[1]);
                        format!(
                        "record type '{type_name}' declares {} without its companion {} -- declare both, since a record can claim a state the rule reading both cannot check",
                        names[0], names[1]
                        )
                    }
                    [false, true] => {
                        missing = Some(names[0]);
                        format!(
                        "record type '{type_name}' declares {} without its companion {} -- declare both, since a record can claim a state the rule reading both cannot check",
                        names[1], names[0]
                        )
                    }
                    [false, false] => format!(
                        "record type '{type_name}' declares the companion pair {} / {} but neither field is in its required_fields or known_fields, so the pair could never examine a record",
                        names[0], names[1]
                    ),
                };
                findings.push(Finding {
                    rule: RULE_CONFIG_COMPANION_DECLARATION_MISSING.to_string(),
                    severity: FindingSeverity::Error,
                    file: config_path.to_path_buf(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: companion_source(
                        type_name,
                        type_config,
                        records,
                        missing,
                        &names,
                    ),
                    message,
                });
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::*;

    const EMBODIMENT: [&str; 2] = ["embodiment_state", "embodiment_locator"];

    fn run(
        known: &[&str],
        pairs: &[[&str; 2]],
        relation: Option<crate::config::RelationFields>,
    ) -> Vec<Finding> {
        let mut type_config = type_config_pointer(&[], Some(known), None, None);
        type_config.companion_fields = Some(
            pairs
                .iter()
                .map(|[a, b]| [a.to_string(), b.to_string()])
                .collect(),
        );
        type_config.relation_fields = relation;
        let config = config_with_types(vec![("spec", type_config)]);
        config_companion_declaration_missing(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
            &[],
        )
        .1
    }

    #[test]
    fn a_type_declaring_one_half_of_a_declared_pair_is_reported_observed_failing() {
        let findings = run(&["Embodiment"], &[EMBODIMENT], None);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Realized-by"));
        let other = run(&["Realized-by"], &[EMBODIMENT], None);
        assert_eq!(other.len(), 1);
        assert!(other[0].message.contains("Embodiment"));
    }

    #[test]
    fn both_halves_declared_is_clean() {
        assert!(run(&["Embodiment", "Realized-by"], &[EMBODIMENT], None).is_empty());
    }

    #[test]
    fn a_pair_with_neither_half_declared_is_reported_not_skipped() {
        let findings = run(&["Other"], &[EMBODIMENT], None);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("neither"));
    }

    #[test]
    fn the_pair_is_whatever_the_roles_name_not_embodiment_observed_failing() {
        let relation = crate::config::RelationFields {
            embodiment_state: Some("Verdict".to_string()),
            embodiment_locator: Some("Basis".to_string()),
            ..Default::default()
        };
        let findings = run(&["Verdict"], &[EMBODIMENT], Some(relation.clone()));
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Basis"));
        // The default literals mean nothing once the roles name other fields.
        assert_eq!(
            run(
                &["Embodiment", "Realized-by"],
                &[EMBODIMENT],
                Some(relation)
            )
            .len(),
            1
        );
    }

    #[test]
    fn a_type_declaring_no_pair_is_not_checked_and_every_pair_is() {
        assert!(run(&["Embodiment"], &[], None).is_empty());
        let findings = run(
            &["Embodiment", "Realized-by"],
            &[EMBODIMENT, ["status", "supersession"]],
            None,
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
    }
}
