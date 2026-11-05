use super::*;
use crate::locator::realized_by_locators;

/// `embodiment.locator-on-back-pointer-path` (`ADR-78` item 4, `MILE-189`,
/// `RFC-7` section 5): prefer the lighter mechanism. A `Realized-by` locator
/// that names a file the back-pointer scan already reads
/// (`back_pointer_paths`) can be an `Implements:` or `Verifies:` comment in
/// that file instead, which is found automatically and cannot be left behind
/// when the code moves.
///
/// A nudge, not an error, and only where a comment says the same thing:
///
/// - `code:` on a file under `back_pointer_paths`: an `Implements:` comment is
///   code-tier evidence wherever it sits.
/// - `test:` on a file under `test_paths`: a `Verifies:` comment counts as
///   test-tier evidence only there.
/// - A `spec:` locator, a `test:` locator outside `test_paths`, and any locator
///   carrying a scope or a pin have no comment of the same tier or strength, so
///   `Realized-by` stays the only option and they are examined and not flagged.
///
/// The population counts the declared `Realized-by` slots. The suggested action
/// edits the field: set to the remaining locators, or remove it when this was
/// the only one.
pub fn embodiment_locator_on_back_pointer_path(
    records: &[Record],
    config: &Config,
    back_pointer_paths: &[String],
    test_paths: &[String],
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_EMBODIMENT_LOCATOR_ON_BACK_POINTER_PATH;
    let mut findings = Vec::new();
    let slots = declared_slots_for_roles(
        records,
        config,
        &[crate::config::RelationRole::EmbodimentLocator],
    );
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |record| *record,
        |record| {
            let field = relation_field_name(
                config,
                &record.record_type,
                crate::config::RelationRole::EmbodimentLocator,
            );
            let value = match declared_value(record, field) {
                Ok(v) => v,
                Err(outcome) => return outcome,
            };
            let id = record_id(record).unwrap_or_else(|| "<id>".to_string());
            let entries = realized_by_locators(value);
            for entry in &entries {
                let Ok(locator) = &entry.parsed else {
                    continue;
                };
                if locator.scope.is_some() || locator.anchor.is_some() {
                    continue;
                }
                let path = std::path::Path::new(&locator.path);
                let under_scan = back_pointer_paths
                    .iter()
                    .any(|e| path_is_published(path, e));
                let marker = match entry.category.as_str() {
                    "code" if under_scan => "Implements",
                    "test"
                        if under_scan && test_paths.iter().any(|e| path_matches_entry(path, e)) =>
                    {
                        "Verifies"
                    }
                    _ => continue,
                };
                let written = format!("{}:{}", entry.category, entry.raw);
                let remaining: Vec<String> = value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| {
                        !s.is_empty() && normalize_segment(s) != normalize_segment(&written)
                    })
                    .map(str::to_string)
                    .collect();
                let source = if remaining.is_empty() {
                    ActionSource::new("locator-on-back-pointer-path-sole")
                } else {
                    ActionSource::new("locator-on-back-pointer-path").text(remaining.join(", "))
                };
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Warning,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: source.field(field),
                    message: format!(
                        "{field} names '{}' ({}:), a file under back_pointer_paths: a `{marker}: {id}` comment in that file is read automatically and counts as the same tier, so it cannot go stale away from the code; keep the locator only where a comment cannot be written",
                        locator.path, entry.category
                    ),
                });
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// A `Realized-by` segment with the whitespace around its category separator
/// removed, so `code: a.rs` and `code:a.rs` compare equal.
fn normalize_segment(segment: &str) -> String {
    match segment.split_once(':') {
        Some((category, locator)) => format!("{}:{}", category.trim(), locator.trim()),
        None => segment.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_compare_without_the_space_after_the_category() {
        assert_eq!(
            normalize_segment("code: a.rs"),
            normalize_segment("code:a.rs")
        );
        assert_ne!(
            normalize_segment("code:a.rs"),
            normalize_segment("test:a.rs")
        );
    }
}
