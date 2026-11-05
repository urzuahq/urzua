use super::*;

/// Reports a config that was read through the schema upgrade (`ADR-77`). The
/// loader leaves `Config::upgraded_from` set when it upgraded the file in
/// memory, so the rule never sees the old shape and still can say it was there.
pub fn config_schema_version_outdated(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_CONFIG_SCHEMA_VERSION_OUTDATED;
    let mut findings = Vec::new();
    let population = census(PopulationUnit::ConfigFile, vec![config_path], |_| {
        if let Some(upgrade) = &config.upgraded_from {
            let dropped: Vec<&str> = upgrade
                .changes
                .iter()
                .filter(|c| c.action == crate::schema_version::ChangeAction::Dropped)
                .map(|c| c.key.as_str())
                .collect();
            let dropped = if dropped.is_empty() {
                String::new()
            } else {
                format!(
                    "; the upgrade dropped {}, which `{}` lists with the reason",
                    dropped.join(", "),
                    crate::schema_version::MIGRATE_COMMAND
                )
            };
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Warning,
                file: config_path.to_path_buf(),
                line: None,
                waived: None,
                declared_severity: None,
                suggested_action: None,
                action_source: ActionSource::new("schema-version-outdated")
                    .argv(["urzua", "migrate", "config"]),
                message: format!(
                    "the config is at schema_version {}, and this build writes {}; it was read by upgrading it in memory{dropped} -- run `{}` to rewrite the file",
                    upgrade.from,
                    crate::config::CURRENT_SCHEMA_VERSION,
                    crate::schema_version::MIGRATE_COMMAND
                ),
            });
        }
        Outcome::Examined
    });
    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::*;

    #[test]
    fn a_config_at_the_current_version_is_examined_and_clean() {
        let config = config_with_types(vec![]);
        let (exec, findings) =
            config_schema_version_outdated(&config, std::path::Path::new("c.yaml"));
        assert!(findings.is_empty());
        assert_eq!(exec.population.unwrap().examined(), 1);
    }

    #[test]
    fn an_upgraded_config_is_one_warning_on_the_config_file_with_the_command() {
        let config = crate::config::parse(
            "schema_version: 1\nrecord_types:\n  adr:\n    dir: docs/adr\n    header_shape: yaml-frontmatter\n    header_layout: one-per-line\n",
        )
        .unwrap();
        let (_, mut findings) =
            config_schema_version_outdated(&config, std::path::Path::new("c.yaml"));
        assert_eq!(findings.len(), 1);
        resolve_actions(RULE_CONFIG_SCHEMA_VERSION_OUTDATED, &mut findings);
        let f = &findings[0];
        assert_eq!(f.file, std::path::Path::new("c.yaml"));
        assert!(f.message.contains("schema_version 1"), "{}", f.message);
        assert!(f.message.contains("urzua migrate config"), "{}", f.message);
        assert!(
            f.message.contains("record_types.adr.header_layout"),
            "{}",
            f.message
        );
        let action = f.suggested_action.as_ref().unwrap();
        assert_eq!(
            action.target.argv.as_deref(),
            Some(
                &[
                    "urzua".to_string(),
                    "migrate".to_string(),
                    "config".to_string()
                ][..]
            )
        );
    }
}
