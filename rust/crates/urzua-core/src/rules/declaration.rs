use super::*;

/// Rule (MILE-0090/ADR-0044): a type declaring either `pointer_fields` or
/// `narrative_fields` must declare both explicitly, even as an empty array --
/// omitting one is not the same as deliberately declaring zero fields of that
/// kind. Same config-level, schema-inventory shape as `type_no_declared_spec`.
pub fn config_pointer_declaration_missing(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    schema_inventory_rule(
        config,
        RULE_CONFIG_POINTER_DECLARATION_MISSING,
        |type_name, type_config, findings| {
            let declares_either =
                type_config.pointer_fields.is_some() || type_config.narrative_fields.is_some();
            if declares_either
                && (type_config.pointer_fields.is_none() || type_config.narrative_fields.is_none())
            {
                findings.push(Finding {
                    rule: RULE_CONFIG_POINTER_DECLARATION_MISSING.to_string(),
                    severity: FindingSeverity::Error,
                    file: config_path.to_path_buf(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new("pointer-declaration-missing").key_path(["record_types", type_name, if type_config.pointer_fields.is_none() { "pointer_fields" } else { "narrative_fields" }]).list(Vec::<String>::new()),
                    message: format!(
                        "record type '{type_name}' declares only one of pointer_fields/narrative_fields -- declare both explicitly, even as `[]`, since omitting one is not the same as declaring zero fields of that kind"
                    ),
                });
            }
        },
    )
}

/// Rule (`RFC-43`/`ADR-62`): a type declaring no `known_fields` at all gets no
/// field-set governance from `header.field-set-consistency` (`ADR-53`'s
/// "declared, not voted" default) -- opt-in, so a repository can require the
/// choice be made explicit rather than leaving it ambiguous by omission, the
/// same shape `config.pointer-declaration-missing` already is for a
/// different pair of fields.
pub fn config_known_fields_declaration_missing(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    schema_inventory_rule(
        config,
        RULE_CONFIG_KNOWN_FIELDS_DECLARATION_MISSING,
        |type_name, type_config, findings| {
            // A `none`-shaped type has nowhere for a header field to be
            // (`ADR-50`): `known_fields` governs which fields are permitted
            // beyond `required_fields`, which is meaningless when no field
            // can exist at all, the same reasoning
            // `config.header-none-has-no-required-fields` already applies to
            // `required_fields` itself.
            if !type_config.has_no_header() && type_config.known_fields.is_none() {
                findings.push(Finding {
                    rule: RULE_CONFIG_KNOWN_FIELDS_DECLARATION_MISSING.to_string(),
                    severity: FindingSeverity::Error,
                    file: config_path.to_path_buf(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new("known-fields-declaration-missing").key_path(["record_types", type_name, "known_fields"]),
                    message: format!(
                        "record type '{type_name}' does not declare known_fields -- declare it explicitly, even as `[]`, to make this type's field-set governance a conscious choice rather than an unchecked default"
                    ),
                });
            }
        },
    )
}

/// What a record type's `known_fields` should become once `field` joins it,
/// as the suggested action's source. With `known_fields` declared, `field` is
/// appended to it. With none declared, appending would create a one-item list,
/// which switches `header.field-set-consistency` on for the type and flags every
/// other optional field its records use; the suggestion instead sets the full
/// set: every field the type's records use beyond `required_fields`, plus
/// `field` (SPEC-3, `known_fields`).
fn known_fields_source(
    variant: &'static str,
    set_variant: &'static str,
    type_name: &str,
    type_config: &crate::config::RecordTypeConfig,
    records: &[Record],
    field: &str,
) -> ActionSource {
    let key_path = ["record_types", type_name, "known_fields"];
    if type_config.known_fields.is_some() {
        return ActionSource::new(variant).key_path(key_path).text(field);
    }
    let required = type_config.declared_fields();
    let mut full: std::collections::BTreeSet<String> = records
        .iter()
        .filter(|r| r.record_type == type_name)
        .flat_map(|r| r.header.fields.iter())
        .filter(|f| !required.contains(&f.key))
        .map(|f| f.key.as_str().to_string())
        .collect();
    full.insert(field.to_string());
    ActionSource::new(set_variant).key_path(key_path).list(full)
}

/// A declared alias not present in a type's own `required_fields`/
/// `known_fields`, shared by every "config declares a field name nothing
/// else knows about" rule (`config.pointer-field-not-known`,
/// `config.relation-field-not-known`).
fn undeclared_alias_finding(
    rule_id: &str,
    action_source: ActionSource,
    config_path: &std::path::Path,
    message: String,
) -> Finding {
    Finding {
        rule: rule_id.to_string(),
        severity: FindingSeverity::Error,
        file: config_path.to_path_buf(),
        line: None,
        waived: None,
        declared_severity: None,
        suggested_action: None,
        action_source,
        message,
    }
}

/// Rule (MILE-0090/ADR-0044): every field named in a type's `pointer_fields`
/// or `narrative_fields` must also appear in that type's own
/// `required_fields`/`known_fields` -- a relationship field the schema
/// inventory itself doesn't know about would silently never trip
/// `header.field-set-consistency`. Compared exactly (`ADR-57`).
pub fn config_pointer_field_not_known(
    config: &Config,
    config_path: &std::path::Path,
    records: &[Record],
) -> (RuleExecution, Vec<Finding>) {
    schema_inventory_rule(
        config,
        RULE_CONFIG_POINTER_FIELD_NOT_KNOWN,
        |type_name, type_config, findings| {
            let declared = type_config.declared_fields();

            let mut relation_fields: Vec<&String> = Vec::new();
            if let Some(pointer_fields) = &type_config.pointer_fields {
                relation_fields.extend(pointer_fields);
            }
            if let Some(narrative_fields) = &type_config.narrative_fields {
                relation_fields.extend(narrative_fields);
            }

            for field in relation_fields {
                if !declared.contains(field.as_str()) {
                    findings.push(undeclared_alias_finding(
                        RULE_CONFIG_POINTER_FIELD_NOT_KNOWN,
                        known_fields_source(
                            "pointer-field-not-known",
                            "pointer-field-not-known-declare-set",
                            type_name,
                            type_config,
                            records,
                            field,
                        ),
                        config_path,
                        format!(
                            "record type '{type_name}' declares '{field}' as a pointer/narrative field, but it isn't in required_fields or known_fields"
                        ),
                    ));
                }
            }
        },
    )
}

/// Rule (`RFC-42`/`ADR-61`, `MILE-176`): a declared `relation_fields` override
/// (a relation role, or the `date` and `author` fields `urzua new` fills) must also
/// appear in that type's own `required_fields`/`known_fields` -- otherwise
/// `ADR-60`'s declaration gate silently never examines it, the same trap
/// `BUG-109` closed for the unqualified `Status` case.
pub fn config_relation_field_not_known(
    config: &Config,
    config_path: &std::path::Path,
    records: &[Record],
) -> (RuleExecution, Vec<Finding>) {
    schema_inventory_rule(
        config,
        RULE_CONFIG_RELATION_FIELD_NOT_KNOWN,
        |type_name, type_config, findings| {
            let Some(relation_fields) = &type_config.relation_fields else {
                return;
            };
            let declared = type_config.declared_fields();
            for (key, field) in relation_fields.declared() {
                if !declared.contains(field) {
                    findings.push(undeclared_alias_finding(
                        RULE_CONFIG_RELATION_FIELD_NOT_KNOWN,
                        known_fields_source(
                            "relation-field-not-known",
                            "relation-field-not-known-declare-set",
                            type_name,
                            type_config,
                            records,
                            field,
                        ),
                        config_path,
                        format!(
                            "record type '{type_name}' declares '{field}' for relation_fields.{key}, but it isn't in required_fields or known_fields"
                        ),
                    ));
                }
            }
        },
    )
}

/// Rule (MILE-0090/ADR-0044): a field must be exactly one kind -- named in
/// both `pointer_fields` and `narrative_fields` for the same type is a
/// contradiction (clean-format enforcement and staleness-checking would both
/// apply, but the two kinds' rules disagree about whether prose is allowed).
pub fn config_pointer_narrative_overlap(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    schema_inventory_rule(
        config,
        RULE_CONFIG_POINTER_NARRATIVE_OVERLAP,
        |type_name, type_config, findings| {
            let (Some(pointer_fields), Some(narrative_fields)) =
                (&type_config.pointer_fields, &type_config.narrative_fields)
            else {
                return;
            };
            let declared_pointers: HashSet<&String> = pointer_fields.iter().collect();
            for field in narrative_fields {
                if declared_pointers.contains(field) {
                    findings.push(Finding {
                        rule: RULE_CONFIG_POINTER_NARRATIVE_OVERLAP.to_string(),
                        severity: FindingSeverity::Error,
                        file: config_path.to_path_buf(),
                        line: None,
                        waived: None,
                        declared_severity: None,
                        suggested_action: None,
                        action_source: ActionSource::new("pointer-narrative-overlap").key_path(["record_types", type_name, "narrative_fields"]).text(field.as_str()),
                        message: format!(
                            "record type '{type_name}' declares '{field}' in both pointer_fields and narrative_fields -- a field must be exactly one kind"
                        ),
                    });
                }
            }
        },
    )
}

/// Rule (`ADR-50`): a type declaring `header_shape: none` has nowhere for a
/// field to be, so a non-empty `required_fields` is a self-contradiction --
/// the same shape as `config.pointer-narrative-overlap`. `known_fields`,
/// `pointer_fields`, `narrative_fields`, and `relation_fields` are the exact
/// same contradiction, one level over: each names a field (or a field-name
/// override) that a `none`-shaped type has nowhere to hold. Without this, a
/// type declaring `header_shape: none` and `pointer_fields: ["Parent"]`
/// passes config validation, and every rule reading that slot
/// (`header.pointer-field-clean`, `pointer.resolution`, ...) reports every
/// one of that type's records as `Outcome::Unreadable` via
/// `Header::is_unreadable()` -- true in the narrowest sense (there is no
/// header) but a misleading diagnosis for a config mistake, not a defect in
/// any record.
pub fn config_header_none_has_no_required_fields(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    schema_inventory_rule(
        config,
        RULE_CONFIG_HEADER_NONE_HAS_NO_REQUIRED_FIELDS,
        |type_name, type_config, findings| {
            if !type_config.has_no_header() {
                return;
            }

            let contradiction = |findings: &mut Vec<Finding>,
                                 declared: bool,
                                 field: &str,
                                 detail: String| {
                if declared {
                    findings.push(Finding {
                            rule: RULE_CONFIG_HEADER_NONE_HAS_NO_REQUIRED_FIELDS.to_string(),
                            severity: FindingSeverity::Error,
                            file: config_path.to_path_buf(),
                            line: None,
                            waived: None,
                            declared_severity: None,
                            suggested_action: None,
                            action_source: ActionSource::new("header-none-field").key_path(["record_types", type_name, field]),
                            message: format!(
                                "record type '{type_name}' declares header_shape: none but {field} {detail} -- a type with no header has nowhere for a field to be"
                            ),
                        });
                }
            };

            contradiction(
                findings,
                !type_config.required_fields.is_empty(),
                "required_fields",
                format!("{:?}", type_config.required_fields),
            );
            contradiction(
                findings,
                type_config
                    .known_fields
                    .as_ref()
                    .is_some_and(|f| !f.is_empty()),
                "known_fields",
                format!("{:?}", type_config.known_fields),
            );
            contradiction(
                findings,
                type_config
                    .pointer_fields
                    .as_ref()
                    .is_some_and(|f| !f.is_empty()),
                "pointer_fields",
                format!("{:?}", type_config.pointer_fields),
            );
            contradiction(
                findings,
                type_config
                    .narrative_fields
                    .as_ref()
                    .is_some_and(|f| !f.is_empty()),
                "narrative_fields",
                format!("{:?}", type_config.narrative_fields),
            );
            contradiction(
                findings,
                type_config.relation_fields.as_ref().is_some_and(|r| {
                    crate::config::RelationRole::ALL
                        .iter()
                        .any(|role| r.get(*role).is_some())
                }),
                "relation_fields",
                format!("{:?}", type_config.relation_fields),
            );
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::*;

    #[test]
    fn a_type_declaring_only_pointer_fields_is_a_missing_declaration_observed_failing() {
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&["Implements"], None, Some(&["Implements"]), None),
        )]);
        let (exec, findings) =
            config_pointer_declaration_missing(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("spec"));
    }

    #[test]
    fn a_type_declaring_both_lists_including_an_explicit_empty_one_is_not_missing() {
        let config = config_with_types(vec![(
            "waiver",
            type_config_pointer(&[], None, Some(&[]), Some(&[])),
        )]);
        let (exec, findings) =
            config_pointer_declaration_missing(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_type_with_no_known_fields_declared_is_a_missing_declaration_observed_failing() {
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&["Status"], None, None, None),
        )]);
        let (exec, findings) = config_known_fields_declaration_missing(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("spec"));
    }

    #[test]
    fn a_header_none_type_with_no_known_fields_is_not_a_missing_declaration_observed_failing() {
        let config = config_with_types(vec![(
            "waiver",
            type_config_with_shape(crate::header::HeaderShape::None),
        )]);
        let (exec, findings) = config_known_fields_declaration_missing(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert_eq!(examined(&exec), 1);
        assert!(
            findings.is_empty(),
            "a type with no header has nowhere for known_fields to govern: {findings:?}"
        );
    }

    #[test]
    fn a_type_declaring_known_fields_as_an_explicit_empty_list_is_not_missing() {
        let config = config_with_types(vec![(
            "waiver",
            type_config_pointer(&[], Some(&[]), None, None),
        )]);
        let (exec, findings) = config_known_fields_declaration_missing(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// `ADR-50`: a type with no header has nowhere for a required field to be.
    #[test]
    fn a_header_none_type_with_required_fields_is_a_contradiction_observed_failing() {
        let config = config_with_types(vec![(
            "dec",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                header_shape: crate::header::HeaderShape::None,
                required_fields: vec!["Status".to_string()],
                ..type_config(None)
            },
        )]);
        let (exec, findings) = config_header_none_has_no_required_fields(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Status"));
    }

    #[test]
    fn a_header_none_type_with_no_required_fields_is_not_a_contradiction() {
        let config = config_with_types(vec![(
            "dec",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                header_shape: crate::header::HeaderShape::None,
                ..type_config(None)
            },
        )]);
        let (_, findings) = config_header_none_has_no_required_fields(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// A type with no header has nowhere for *any* field-shaped declaration
    /// to be, not only `required_fields` -- the same contradiction one level
    /// over. Without this, `header_shape: none` plus `pointer_fields:
    /// ["Parent"]` passes config validation, and every rule reading that
    /// slot then reports every one of that type's records as
    /// `Outcome::Unreadable`, a misleading diagnosis for a config mistake.
    #[test]
    fn a_header_none_type_with_known_fields_is_a_contradiction_observed_failing() {
        let config = config_with_types(vec![(
            "dec",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                header_shape: crate::header::HeaderShape::None,
                known_fields: Some(vec!["Status".to_string()]),
                ..type_config(None)
            },
        )]);
        let (_, findings) = config_header_none_has_no_required_fields(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("known_fields"));
    }

    #[test]
    fn a_header_none_type_with_pointer_fields_is_a_contradiction_observed_failing() {
        let config = config_with_types(vec![(
            "dec",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                header_shape: crate::header::HeaderShape::None,
                pointer_fields: Some(vec!["Parent".to_string()]),
                ..type_config(None)
            },
        )]);
        let (_, findings) = config_header_none_has_no_required_fields(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("pointer_fields"));
    }

    #[test]
    fn a_header_none_type_with_narrative_fields_is_a_contradiction_observed_failing() {
        let config = config_with_types(vec![(
            "dec",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                header_shape: crate::header::HeaderShape::None,
                narrative_fields: Some(vec!["Blocked-on".to_string()]),
                ..type_config(None)
            },
        )]);
        let (_, findings) = config_header_none_has_no_required_fields(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("narrative_fields"));
    }

    #[test]
    fn a_header_none_type_with_a_relation_field_override_is_a_contradiction_observed_failing() {
        let config = config_with_types(vec![(
            "dec",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                header_shape: crate::header::HeaderShape::None,
                relation_fields: Some(crate::config::RelationFields {
                    status: Some("Status".to_string()),
                    ..Default::default()
                }),
                ..type_config(None)
            },
        )]);
        let (_, findings) = config_header_none_has_no_required_fields(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("relation_fields"));
    }

    #[test]
    fn a_header_none_type_with_an_empty_pointer_fields_list_is_not_a_contradiction() {
        let config = config_with_types(vec![(
            "dec",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                header_shape: crate::header::HeaderShape::None,
                pointer_fields: Some(vec![]),
                narrative_fields: Some(vec![]),
                known_fields: Some(vec![]),
                ..type_config(None)
            },
        )]);
        let (_, findings) = config_header_none_has_no_required_fields(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert!(
            findings.is_empty(),
            "declaring an empty list is a conscious 'zero fields' choice, not a contradiction: {findings:?}"
        );
    }

    #[test]
    fn a_type_declaring_neither_list_is_not_missing() {
        let config = config_with_types(vec![("adr", type_config_pointer(&[], None, None, None))]);
        let (exec, findings) =
            config_pointer_declaration_missing(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn the_case_only_hint_names_the_same_declaration_every_run() {
        let allowed: std::collections::BTreeSet<crate::values::FieldName> =
            ["Blocked-on", "Blocked-On", "BLOCKED-on"]
                .iter()
                .map(|s| crate::values::FieldName::from(*s))
                .collect();
        assert_eq!(
            near_miss(&crate::values::FieldName::from("blocked-on"), &allowed).map(|f| f.as_str()),
            Some("BLOCKED-on"),
            "the hint must not depend on which spelling the set yields first"
        );
    }

    #[test]
    fn a_pointer_field_differing_only_in_case_is_absent_from_known_fields() {
        // Accepted here, the field is unreadable everywhere else: the record
        // writes `Derives-from` and `header.pointer-field-clean` looks up
        // `Derives-From`, which ADR-57 makes a different name.
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(
                &["Status"],
                Some(&["Derives-from"]),
                Some(&["Derives-From"]),
                Some(&[]),
            ),
        )]);
        let (exec, findings) = config_pointer_field_not_known(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
            &[],
        );
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Derives-From"));
    }

    #[test]
    fn a_relation_field_absent_from_known_fields_is_a_finding_observed_failing() {
        let config = config_with_types(vec![(
            "rfc",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                relation_fields: Some(crate::config::RelationFields {
                    status: Some("State".to_string()),
                    ..Default::default()
                }),
                ..type_config_pointer(&["Date"], None, None, None)
            },
        )]);
        let (exec, findings) = config_relation_field_not_known(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
            &[],
        );
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("State"));
    }

    #[test]
    fn an_undeclared_date_or_author_field_is_a_finding_observed_failing() {
        let config = config_with_types(vec![(
            "rfc",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                relation_fields: Some(crate::config::RelationFields {
                    date: Some("Created".to_string()),
                    author: Some("Owner".to_string()),
                    ..Default::default()
                }),
                ..type_config_pointer(&["Status"], None, None, None)
            },
        )]);
        let (_, findings) = config_relation_field_not_known(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
            &[],
        );
        assert_eq!(findings.len(), 2, "{findings:?}");
        assert!(findings[0].message.contains("relation_fields.date"));
        assert!(findings[0].message.contains("Created"));
        assert!(findings[1].message.contains("relation_fields.author"));
    }

    #[test]
    fn a_declared_date_and_author_field_is_not_a_finding() {
        let config = config_with_types(vec![(
            "rfc",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                relation_fields: Some(crate::config::RelationFields {
                    date: Some("Created".to_string()),
                    author: Some("Owner".to_string()),
                    ..Default::default()
                }),
                ..type_config_pointer(&["Created", "Owner"], None, None, None)
            },
        )]);
        let (_, findings) = config_relation_field_not_known(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
            &[],
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_relation_field_present_in_known_fields_is_not_a_finding() {
        let config = config_with_types(vec![(
            "rfc",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                relation_fields: Some(crate::config::RelationFields {
                    status: Some("State".to_string()),
                    ..Default::default()
                }),
                ..type_config_pointer(&["State"], None, None, None)
            },
        )]);
        let (exec, findings) = config_relation_field_not_known(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
            &[],
        );
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_type_with_no_declared_relation_fields_is_not_a_finding() {
        let config = config_with_types(vec![(
            "rfc",
            type_config_pointer(&["Status"], None, None, None),
        )]);
        let (exec, findings) = config_relation_field_not_known(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
            &[],
        );
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn pointer_and_narrative_names_differing_only_in_case_do_not_overlap() {
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(
                &[],
                Some(&["Blocked-on", "Blocked-On"]),
                Some(&["Blocked-on"]),
                Some(&["Blocked-On"]),
            ),
        )]);
        let (exec, findings) =
            config_pointer_narrative_overlap(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(
            findings.is_empty(),
            "two different names are two fields (ADR-57): {findings:?}"
        );
    }

    #[test]
    fn a_pointer_field_absent_from_known_fields_is_a_finding_observed_failing() {
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&["Status"], None, Some(&["Feeds-into"]), Some(&[])),
        )]);
        let (exec, findings) = config_pointer_field_not_known(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
            &[],
        );
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Feeds-into"));
    }

    #[test]
    fn a_pointer_field_present_in_known_fields_is_not_a_finding() {
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(
                &["Status"],
                Some(&["Feeds-into"]),
                Some(&["Feeds-into"]),
                Some(&[]),
            ),
        )]);
        let (exec, findings) = config_pointer_field_not_known(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
            &[],
        );
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_field_present_in_both_pointer_and_narrative_lists_is_an_overlap_observed_failing() {
        let config = config_with_types(vec![(
            "milestone",
            type_config_pointer(
                &["Blocked-on"],
                None,
                Some(&["Blocked-on"]),
                Some(&["Blocked-on"]),
            ),
        )]);
        let (exec, findings) =
            config_pointer_narrative_overlap(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Blocked-on"));
    }

    #[test]
    fn disjoint_pointer_and_narrative_lists_are_not_an_overlap() {
        let config = config_with_types(vec![(
            "milestone",
            type_config_pointer(
                &["Implements", "Blocked-on"],
                None,
                Some(&["Implements"]),
                Some(&["Blocked-on"]),
            ),
        )]);
        let (exec, findings) =
            config_pointer_narrative_overlap(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }
}
