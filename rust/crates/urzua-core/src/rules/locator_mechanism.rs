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
/// The population counts the declared `Realized-by` slots. It offers no edit
/// action: the locator must stay until the comment exists, or the evidence is
/// gone before its replacement is, and writing the comment is an edit to a
/// source file, which no action kind addresses (`ADR-73`, exempt with a
/// reason). The message says to add the comment first, or, when the file
/// already carries the comment for this record, to drop the locator.
///
/// A type whose `statuses.delivery.back_pointer_evidence` is `false` counts no
/// back-pointer as evidence (`ADR-84`), so a comment cannot replace its locator
/// and its records are not nudged.
///
/// With `scope: changed` (`ADR-90`) the rule is a nudge for the moment someone
/// touches the record: it judges only the records the diff adds or changes, and
/// the rest are counted in the population's `deferred_by_scope`.
pub fn embodiment_locator_on_back_pointer_path(
    records: &[Record],
    config: &Config,
    back_pointer_paths: &[String],
    test_paths: &[String],
    scope: Option<&ChangedOnly>,
    read_text: &dyn Fn(&str) -> crate::locator::SharedText,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_EMBODIMENT_LOCATOR_ON_BACK_POINTER_PATH;
    let mut findings = Vec::new();
    let nudge = LighterMechanism {
        back_pointer_paths,
        test_paths,
        prefixes: declared_prefixes(config),
        read_text,
    };
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
            // `scope: changed` (`ADR-90`): a slot that is there but belongs to a
            // record the diff did not change is eligible and not judged. An
            // absent slot was counted absent above, so there is nothing to defer.
            if scope.is_some_and(|s| s.defers(record)) {
                return Outcome::DeferredByScope;
            }
            // A comment is no evidence for this type, so it cannot replace the locator.
            let comments_count = config
                .record_types
                .get(&record.record_type)
                .and_then(|t| t.status_decl())
                .and_then(|d| d.delivery)
                .is_none_or(|d| d.back_pointer_evidence);
            if !comments_count {
                return Outcome::Examined;
            }
            findings.extend(nudge.findings(record, field, value));
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

/// The declared inputs the lighter-mechanism nudge reads each locator against.
struct LighterMechanism<'a> {
    back_pointer_paths: &'a [String],
    test_paths: &'a [String],
    prefixes: Vec<String>,
    read_text: &'a dyn Fn(&str) -> crate::locator::SharedText,
}

impl LighterMechanism<'_> {
    /// One nudge per locator of the record's `Realized-by` that a comment could
    /// replace.
    fn findings(&self, record: &Record, field: &str, value: &str) -> Vec<Finding> {
        let id = record_id(record).unwrap_or_else(|| "<id>".to_string());
        let entries = realized_by_locators(value);
        entries
            .iter()
            .filter_map(|entry| self.nudge(record, field, &id, entries.len(), entry))
            .collect()
    }

    fn nudge(
        &self,
        record: &Record,
        field: &str,
        id: &str,
        entry_count: usize,
        entry: &crate::locator::LocatorEntry,
    ) -> Option<Finding> {
        let locator = entry.parsed.as_ref().ok()?;
        if locator.scope.is_some() || locator.anchor.is_some() {
            return None;
        }
        let path = std::path::Path::new(&locator.path);
        let under_scan = self
            .back_pointer_paths
            .iter()
            .any(|e| path_is_published(path, e));
        let marker = match entry.category.as_str() {
            "code" if under_scan => "Implements",
            "test" if under_scan && self.test_paths.iter().any(|e| path_matches_entry(path, e)) => {
                "Verifies"
            }
            _ => return None,
        };
        // Told apart through the shared `Realized-by` reader, never by
        // splitting the value: a path may itself contain a comma.
        let source = if entry_count == 1 {
            ActionSource::new("locator-on-back-pointer-path-sole")
        } else {
            ActionSource::new("locator-on-back-pointer-path")
        };
        let wanted = crate::values::RecordId::new(id);
        let comment_present = match &*(self.read_text)(&locator.path) {
            crate::locator::FileText::Text(text) => {
                let marker_word = format!("{marker}:");
                scan_marker(path, text, &self.prefixes, &marker_word)
                    .iter()
                    .any(|p| crate::values::RecordId::new(&p.id) == wanted)
            }
            _ => false,
        };
        let next_step = if comment_present {
            format!("that file already carries it, so drop this locator from {field}")
        } else {
            format!("add the comment first, then drop this locator from {field}; keep the locator only where a comment cannot be written")
        };
        Some(Finding {
            rule: RULE_EMBODIMENT_LOCATOR_ON_BACK_POINTER_PATH.to_string(),
            severity: FindingSeverity::Warning,
            file: record.path.clone(),
            line: None,
            waived: None,
            declared_severity: None,
            suggested_action: None,
            action_source: source.field(field),
            message: format!(
                "{field} names '{}' ({}:), a file under back_pointer_paths: a `{marker}: {id}` comment in that file is read automatically and counts as the same tier, so it cannot go stale away from the code; {next_step}",
                locator.path, entry.category
            ),
        })
    }
}
