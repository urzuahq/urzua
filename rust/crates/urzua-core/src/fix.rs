//! `urzua fix` detect mode (ADR-0015 §3): read-only, emits one entry per
//! repairable fact. Apply mode (`--ids`, `--by`, `--force`) is not built
//! yet -- it needs real file mutation, identity resolution, and a
//! revision-log write-path, each its own piece of work. Detect mode alone
//! is "a real Phase 1 feature" per ADR-0015, not a stub.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::config::{Config, RelationRole};
use crate::line_edit::{table_row_prefix, LineDoc};
use crate::record::Record;
use crate::report::{census, Outcome, Population, PopulationUnit};
use crate::rules::{
    compute_embodiment, find_revision_log_entries, parse_realized_by, relation_field_name,
    BackPointerEvidence,
};

/// One mechanically-detected disagreement between a record's stated value
/// and what the tool computes -- the same shape RFC-0008 §3 specifies for
/// `urzua fix`'s detect-mode output.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Repair {
    pub record: std::path::PathBuf,
    pub field: String,
    /// The 1-based line of the stated field in the content examined (the file,
    /// or the proposed content under `--stdin-path`), so an editor hook can
    /// place the repair without parsing the record (`MILE-159`).
    pub line: usize,
    pub current_value: String,
    pub computed_value: String,
    /// RFC-0008 §2's repair tiers. Only Tier 1 (recompute in place) exists
    /// in this MVP.
    pub tier: u8,
    /// The evidence the computation read, named in the record it produces
    /// (RFC-0008 §6) -- a reader can disagree with the derivation, not just
    /// trust it.
    pub evidence: String,
}

/// Tier 1 repairs for `Embodiment`: the only tool-writable field this MVP
/// knows about (ADR-0018). Every other field stays permanently
/// hand-authored (RFC-0008 §1's eligibility test).
///
/// The candidate is the *pair*: a repair needs both `Embodiment` and
/// `Realized-by`, so a record carrying only one of them is handed to this
/// function and reaches no verdict -- `Absent`, not a candidate that never
/// existed. Every loaded record is eligible; `fix` has no config-declared
/// field set to filter against the way `check`'s rules do, so unlike
/// `embodiment_consistency` this reads every record rather than a declared
/// slot.
///
/// Returns the population alongside the repairs found, so a caller can
/// distinguish "examined some, found nothing to repair" from "examined
/// nothing" -- the same no-silent-no-op discipline `check` follows.
pub fn detect_repairs(
    records: &[Record],
    config: &Config,
    drifted: &HashSet<PathBuf>,
    fresh_drift: &HashSet<PathBuf>,
    back_pointed: &BackPointerEvidence,
) -> (Population, Vec<Repair>) {
    let mut repairs = Vec::new();
    let sticky = crate::rules::sticky_states(config);

    let embodiment_outcome = |record: &Record, repairs: &mut Vec<Repair>| -> Outcome {
        let state_field =
            relation_field_name(config, &record.record_type, RelationRole::EmbodimentState);
        let locator_field =
            relation_field_name(config, &record.record_type, RelationRole::EmbodimentLocator);
        let Some(stated) = record.header.get(state_field) else {
            return Outcome::Absent;
        };
        // A sticky state is a human's; there is nothing to recompute (`MILE-148`).
        if sticky.contains(stated.trim()) {
            return Outcome::Examined;
        }
        let cited_by_code = back_pointed.code.contains(&record.path);
        let cited_by_test = back_pointed.test.contains(&record.path);
        // No `Realized-by` is repaired only when a back-pointer cites the
        // record, as `embodiment.consistency` judges it (`MILE-149`).
        let realized_by_value = record.header.get(locator_field);
        if realized_by_value.is_none() && !(cited_by_code || cited_by_test) {
            return Outcome::Absent;
        }

        // Drift (ADR-32) is computed by the caller from git history and
        // handed in as plain data, the same shape `embodiment_consistency`
        // takes, so this function stays pure (MILE-15).
        let computed = compute_embodiment(
            &config.embodiment,
            &realized_by_value.map(parse_realized_by).unwrap_or_default(),
            drifted.contains(&record.path),
            cited_by_code,
            cited_by_test,
        );
        let stated = stated.trim();
        // Fresh drift (`drift_grace_days`) never flips a stated `Drift detected`.
        let keeps_stated_drift =
            fresh_drift.contains(&record.path) && config.embodiment.is_drift(stated);
        if stated != computed && !keeps_stated_drift {
            repairs.push(Repair {
                record: record.path.clone(),
                field: state_field.to_string(),
                line: record
                    .header
                    .fields
                    .iter()
                    .find(|f| f.key.as_str() == state_field)
                    .map_or(0, |f| f.line),
                current_value: stated.to_string(),
                computed_value: computed.to_string(),
                tier: 1,
                evidence: match realized_by_value {
                    Some(value) => format!("{locator_field}: {value}"),
                    None => format!(
                        "no {locator_field} field; cited by {} back-pointers",
                        if cited_by_test { "test" } else { "source" }
                    ),
                },
            });
        }
        Outcome::Examined
    };

    let delivery_types = crate::rules::statuses_by_type(config);
    let population = census(PopulationUnit::Record, records.iter().collect(), |record| {
        let embodiment = embodiment_outcome(record, &mut repairs);
        // `Status` against delivery evidence (`MILE-183`): the one case with a
        // single correct value is repaired; the reverse is `unevidenced_successes`.
        let status = match delivery_types.get(&record.record_type) {
            Some(decl) if decl.delivery.is_some() => {
                match crate::rules::judge_delivery(record, config, decl, back_pointed) {
                    Ok(judged) => {
                        if let Some(crate::rules::DeliveryDisagreement::DeliveredButNotDone {
                            set_status,
                        }) = judged.disagreement
                        {
                            repairs.push(Repair {
                                record: record.path.clone(),
                                field: judged.status_field.clone(),
                                line: record
                                    .header
                                    .fields
                                    .iter()
                                    .find(|f| f.key.as_str() == judged.status_field)
                                    .map_or(0, |f| f.line),
                                current_value: judged.status.clone(),
                                computed_value: set_status,
                                tier: 1,
                                evidence: format!(
                                    "{} computes to {}",
                                    judged.evidence, judged.tier
                                ),
                            });
                        }
                        Outcome::Examined
                    }
                    Err(outcome) => outcome,
                }
            }
            _ => Outcome::Absent,
        };
        if embodiment == Outcome::Examined || status == Outcome::Examined {
            Outcome::Examined
        } else if embodiment != Outcome::Absent {
            embodiment
        } else {
            status
        }
    });

    (population, repairs)
}

/// Records whose success `Status` has no delivery evidence (`MILE-183`). `fix`
/// does not downgrade them -- the repair is adding evidence, which only a person
/// or the delivering change can supply -- so they are reported, never written.
pub fn unevidenced_successes(
    records: &[Record],
    config: &Config,
    back_pointed: &BackPointerEvidence,
) -> Vec<crate::report::Notice> {
    let declared = crate::rules::statuses_by_type(config);
    records
        .iter()
        .filter_map(|record| {
            let decl = declared.get(&record.record_type)?;
            let judged = crate::rules::judge_delivery(record, config, decl, back_pointed).ok()?;
            matches!(
                judged.disagreement,
                Some(crate::rules::DeliveryDisagreement::SuccessWithoutEvidence)
            )
            .then(|| crate::report::Notice {
                severity: crate::report::NoticeSeverity::Warning,
                subject: record.path.display().to_string(),
                message: format!(
                    "{} is '{}' with no delivery evidence (computes to '{}'); not downgraded -- add the evidence that delivered it",
                    judged.status_field, judged.status, judged.tier
                ),
            })
        })
        .collect()
}

/// How a stated value reads in prose: the value in code spans, or "no value"
/// when the field was empty, `null` or `~`.
fn describe_stated(value: &str) -> String {
    if value.is_empty() {
        "no value".to_string()
    } else {
        format!("`{value}`")
    }
}

/// A YAML value that states nothing: empty, `~` or `null` in any case.
fn is_empty_yaml_value(value: &str) -> bool {
    let v = value.trim();
    v.is_empty() || v == "~" || v.eq_ignore_ascii_case("null")
}

/// Replaces the value after `field:` on `line`, keeping everything before the
/// value (a `> ` prefix, the key and the spacing) byte for byte. `None` when the
/// line is not that field's own line or its value is not the stated one. A
/// stated value of `""` matches an empty, `~` or `null` value.
fn replace_field_value(line: &str, field: &str, stated: &str, computed: &str) -> Option<String> {
    let body = line.trim_start_matches(['>', ' ', '\t']);
    let rest = body.strip_prefix(field)?.strip_prefix(':')?;
    let matches = if stated.is_empty() {
        is_empty_yaml_value(rest)
    } else {
        rest.trim() == stated
    };
    if !matches {
        return None;
    }
    let value_start = line.len() - rest.len();
    let gap: &str = &rest[..rest.len() - rest.trim_start().len()];
    let gap = if gap.is_empty() { " " } else { gap };
    Some(format!("{}{gap}{computed}", &line[..value_start]))
}

/// Computes the new file content for applying one Tier-1 repair. Pure --
/// returns the new text; the caller performs the actual write, the identity
/// resolution, and the re-verification-before-write RFC-0008 §4 requires.
///
/// When the type requires a revision log (the default), refuses rather than
/// silently skipping the requirement if the record has no revision-log section
/// (its type's declared identity) to append to: ADR-0015's reversibility clause
/// depends on that entry existing. A type declared `revision_log: {required:
/// false}` (`MILE-159`) takes the field edit alone, with no row.
pub fn apply_repair(
    content: &str,
    repair: &Repair,
    today: &str,
    by: &str,
    log_id: &crate::config::RevisionLogId,
    log_required: bool,
) -> Result<String, String> {
    let mut doc = LineDoc::parse(content);

    // Edit by field position: the value after `Field:` on the field's own line,
    // never a substring match (an empty or null stated value is a substring of
    // everything and would be inserted at column 0).
    let (field_line, edited_line) = (1..)
        .map_while(|n| doc.line(n).map(|l| (n, l)))
        .find_map(|(n, line)| {
            replace_field_value(line, &repair.field, &repair.current_value, &repair.computed_value)
                .map(|edited| (n, edited))
        })
        .ok_or_else(|| {
            format!(
                "could not find a '{}:' line stating {} to edit -- the record may have changed since detect ran",
                repair.field,
                describe_stated(&repair.current_value)
            )
        })?;
    doc.replace_line(field_line, edited_line);

    if !log_required {
        return Ok(doc.to_string());
    }
    let edited_content = doc.to_string();
    let entries = find_revision_log_entries(&edited_content, log_id).ok_or_else(|| {
        format!(
            "record has no Revision log section (looked for {}) -- refusing to apply without one to record the change in (ADR-0015's reversibility requirement); declare revision_log: {{required: false}} on the type to apply without a log",
            log_id.describe()
        )
    })?;
    let Some(last_entry) = entries.last() else {
        return Err(
            "record's Revision log section has no existing rows to append after".to_string(),
        );
    };

    // The new row repeats the existing last row's own prefix, so it lands
    // inside the table whether the log is a blockquote table or a plain one.
    let prefix = doc
        .line(last_entry.line)
        .and_then(table_row_prefix)
        .unwrap_or("> ")
        .to_string();
    let new_row = format!(
        "{prefix}| {today} | Tool-authored (by {by}): recomputed `{}` from `{}` -- stated {} disagreed with the computed value `{}`. | **structural** |",
        repair.field,
        repair.evidence,
        describe_stated(&repair.current_value),
        repair.computed_value
    );
    doc.insert_after(last_entry.line, new_row);

    Ok(doc.to_string())
}

/// One pinned locator whose current content no longer matches its pin
/// (`MILE-181`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PinDrift {
    pub locator: String,
    pub recorded_hash: String,
    pub current_hash: String,
}

/// A record whose `Realized-by` carries one or more stale `@sha256:` pins: the
/// unit `urzua fix --rebaseline` lists and, on review, re-pins (`MILE-181`).
/// A record is the selector, as in `--ids`; all its stale pins move together
/// and one attributed revision row records them.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Rebaseline {
    pub record: PathBuf,
    pub field: String,
    pub line: usize,
    pub pins: Vec<PinDrift>,
}

// Implements: ADR-67 (attributed re-baselining of a pin)
/// Lists the records whose pinned locators changed. Pure: `current_hash` is
/// the CLI's content-hash lookup for a repo-relative locator path. A locator
/// whose file cannot be read is not a stale pin (`embodiment.locator-exists`
/// reports it), and an unpinned locator has nothing to re-pin.
pub fn detect_rebaselines(
    records: &[Record],
    config: &Config,
    current_hash: &dyn Fn(&str) -> Option<String>,
) -> (Population, Vec<Rebaseline>) {
    let mut found = Vec::new();
    let population = census(PopulationUnit::Record, records.iter().collect(), |record| {
        let locator_field =
            relation_field_name(config, &record.record_type, RelationRole::EmbodimentLocator);
        let Some(value) = record.header.get(locator_field) else {
            return Outcome::Absent;
        };
        let mut pins: Vec<PinDrift> = Vec::new();
        for (locator, pin) in crate::rules::realized_by_pinned_locators(value) {
            let Some(recorded) = pin else { continue };
            let Some(current) = current_hash(&locator) else {
                continue;
            };
            if current != recorded
                && !pins
                    .iter()
                    .any(|p| p.locator == locator && p.recorded_hash == recorded)
            {
                pins.push(PinDrift {
                    locator,
                    recorded_hash: recorded,
                    current_hash: current,
                });
            }
        }
        if !pins.is_empty() {
            found.push(Rebaseline {
                record: record.path.clone(),
                field: locator_field.to_string(),
                line: record
                    .header
                    .fields
                    .iter()
                    .find(|f| f.key.as_str() == locator_field)
                    .map_or(0, |f| f.line),
                pins,
            });
        }
        Outcome::Examined
    });
    (population, found)
}

// Implements: ADR-67 (attributed re-baselining of a pin)
/// New file content for re-pinning one record: each stale pin on the
/// `Realized-by` line becomes the current hash and nothing else on the line
/// moves, then one revision-log row names who attested and why. Pure. Fails
/// when a pin is no longer on the line (a concurrent edit since detect) or the
/// record has no revision log to record the attestation in.
pub fn apply_rebaseline(
    content: &str,
    rebaseline: &Rebaseline,
    today: &str,
    by: &str,
    reason: &str,
    log_id: &crate::config::RevisionLogId,
) -> Result<String, String> {
    let mut doc = LineDoc::parse(content);
    let prefix_key = format!("{}:", rebaseline.field);
    let field_line = doc
        .find_line(|l| l.trim_start().starts_with(&prefix_key))
        .ok_or_else(|| {
            format!(
                "could not find the '{}' line to edit -- the record may have changed since detect ran",
                rebaseline.field
            )
        })?;
    let mut line = doc.line(field_line).unwrap_or_default().to_string();
    for pin in &rebaseline.pins {
        let old = format!("{}@sha256:{}", pin.locator, pin.recorded_hash);
        if !line.contains(&old) {
            return Err(format!(
                "the pin on '{}' is no longer on the '{}' line -- the record may have changed since detect ran",
                pin.locator, rebaseline.field
            ));
        }
        line = line.replace(
            &old,
            &format!("{}@sha256:{}", pin.locator, pin.current_hash),
        );
    }
    doc.replace_line(field_line, line);

    let edited = doc.to_string();
    let entries = find_revision_log_entries(&edited, log_id).ok_or_else(|| {
        "record has no Revision log section -- refusing to re-baseline without one to record the attestation in".to_string()
    })?;
    let Some(last_entry) = entries.last() else {
        return Err(
            "record's Revision log section has no existing rows to append after".to_string(),
        );
    };
    let row_prefix = doc
        .line(last_entry.line)
        .and_then(table_row_prefix)
        .unwrap_or("> ")
        .to_string();
    let moved = rebaseline
        .pins
        .iter()
        .map(|p| {
            format!(
                "`{}` {} -> {}",
                p.locator,
                short(&p.recorded_hash),
                short(&p.current_hash)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    doc.insert_after(
        last_entry.line,
        format!(
            "{row_prefix}| {today} | Re-baselined (attested by {by}): re-pinned {moved}. Reason: {reason} **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |"
        ),
    );
    Ok(doc.to_string())
}

fn short(hash: &str) -> &str {
    hash.get(..12).unwrap_or(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn record(path: &str, content: &str) -> Record {
        Record::parse(PathBuf::from(path), "adr".to_string(), content)
    }

    fn no_relation_field_overrides() -> Config {
        Config {
            schema_version: crate::config::CURRENT_SCHEMA_VERSION,
            relation_pairs: Vec::new(),
            fail_on: Default::default(),
            embodiment: Default::default(),
            rules: std::collections::BTreeMap::new(),
            record_types: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn a_sticky_state_is_never_repaired_and_a_neighbour_still_is() {
        let mut config = no_relation_field_overrides();
        config.rules.insert(
            crate::rules::RULE_EMBODIMENT_CONSISTENCY.to_string(),
            yaml_serde::from_str("{level: warn, sticky_states: [Deprecated]}").unwrap(),
        );
        let sticky = record(
            "docs/adr/0001-x.md",
            "---\nEmbodiment: Deprecated\nRealized-by: spec:docs/x.md\n---\n",
        );
        let neighbour = record(
            "docs/adr/0002-y.md",
            "---\nEmbodiment: Verified\nRealized-by: spec:docs/x.md\n---\n",
        );
        let (population, repairs) = detect_repairs(
            &[sticky, neighbour],
            &config,
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
        );
        assert_eq!(population.examined(), 2);
        assert_eq!(repairs.len(), 1);
        assert!(repairs[0].record.ends_with("0002-y.md"));
    }

    #[test]
    fn a_back_pointed_record_repairs_to_implemented_not_specified_observed_failing() {
        let r = record(
            "docs/adr/0001-x.md",
            "---\nEmbodiment: Specified\nRealized-by: spec:docs/x.md\n---\n",
        );
        let cited: HashSet<PathBuf> = [PathBuf::from("docs/adr/0001-x.md")].into();
        let (_, repairs) = detect_repairs(
            &[r],
            &no_relation_field_overrides(),
            &HashSet::new(),
            &HashSet::new(),
            &BackPointerEvidence {
                code: cited.clone(),
                ..Default::default()
            },
        );
        assert_eq!(repairs.len(), 1);
        assert_eq!(repairs[0].computed_value, "Implemented");
    }

    #[test]
    fn a_mismatched_embodiment_is_a_tier_1_repair_observed_failing() {
        let r = record(
            "docs/adr/0001-x.md",
            "---\nEmbodiment: Not started\nRealized-by: code:src/lib.rs\n---\n",
        );
        let (population, repairs) = detect_repairs(
            &[r],
            &no_relation_field_overrides(),
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
        );
        assert_eq!((population.eligible(), population.examined()), (1, 1));
        assert_eq!(repairs.len(), 1);
        assert_eq!(repairs[0].field, "Embodiment");
        assert_eq!(repairs[0].current_value, "Not started");
        assert_eq!(repairs[0].computed_value, "Implemented");
        assert_eq!(repairs[0].tier, 1);
    }

    #[test]
    fn a_drifted_record_is_repaired_to_drift_detected_observed_failing() {
        let r = record(
            "docs/adr/0001-x.md",
            "---\nEmbodiment: Implemented\nRealized-by: code:src/lib.rs\n---\n",
        );
        let drifted: HashSet<PathBuf> = [PathBuf::from("docs/adr/0001-x.md")].into();
        let (_, repairs) = detect_repairs(
            &[r],
            &no_relation_field_overrides(),
            &drifted,
            &HashSet::new(),
            &Default::default(),
        );
        assert_eq!(repairs.len(), 1);
        assert_eq!(repairs[0].computed_value, "Drift detected");
    }

    #[test]
    fn a_matching_embodiment_is_not_a_repair() {
        let r = record(
            "docs/adr/0001-x.md",
            "---\nEmbodiment: Implemented\nRealized-by: code:src/lib.rs\n---\n",
        );
        let (population, repairs) = detect_repairs(
            &[r],
            &no_relation_field_overrides(),
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
        );
        assert_eq!((population.eligible(), population.examined()), (1, 1));
        assert!(repairs.is_empty());
    }

    #[test]
    fn a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing()
    {
        let r = record(
            "docs/adr/0001-x.md",
            "---\nPhase: Not started\nEvidence: code:src/lib.rs\n---\n",
        );
        let config = Config {
            schema_version: crate::config::CURRENT_SCHEMA_VERSION,
            relation_pairs: Vec::new(),
            fail_on: Default::default(),
            embodiment: Default::default(),
            rules: std::collections::BTreeMap::new(),
            record_types: [(
                "adr".to_string(),
                crate::config::RecordTypeConfig {
                    exclude_files: Vec::new(),
                    dir: "docs/adr".to_string(),
                    extra_dirs: Vec::new(),
                    identity: None,
                    fields: None,
                    sections: None,
                    statuses: None,
                    required_fields: vec![],
                    header_shape: crate::header::HeaderShape::default(),
                    prefix: None,
                    known_fields: Some(vec!["Phase".to_string(), "Evidence".to_string()]),
                    pointer_fields: None,
                    narrative_fields: None,
                    spec: None,
                    companion_fields: None,
                    relation_fields: Some(crate::config::RelationFields {
                        embodiment_state: Some("Phase".to_string()),
                        embodiment_locator: Some("Evidence".to_string()),
                        ..Default::default()
                    }),
                    revision_log: None,
                },
            )]
            .into_iter()
            .collect(),
        };
        let (population, repairs) = detect_repairs(
            &[r],
            &config,
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
        );
        assert_eq!((population.eligible(), population.examined()), (1, 1));
        assert_eq!(repairs.len(), 1, "{repairs:?}");
        assert_eq!(repairs[0].field, "Phase");
        assert_eq!(repairs[0].computed_value, "Implemented");
    }

    #[test]
    fn no_realized_by_leaves_the_record_eligible_but_unexamined() {
        // Absent, not out of population: the record was handed to this
        // function and reached no verdict, which is what `eligible() > 0`
        // says and a bare examined-count could not.
        let r = record("docs/adr/0001-x.md", "---\nEmbodiment: Not started\n---\n");
        let (population, repairs) = detect_repairs(
            &[r],
            &no_relation_field_overrides(),
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
        );
        assert_eq!((population.eligible(), population.examined()), (1, 0));
        assert!(repairs.is_empty());
    }

    fn sample_repair() -> Repair {
        Repair {
            record: PathBuf::from("docs/adr/0001-x.md"),
            field: "Embodiment".to_string(),
            line: 2,
            current_value: "Not started".to_string(),
            computed_value: "Verified".to_string(),
            tier: 1,
            evidence: "Realized-by: code:src/lib.rs, test:tests/it.rs".to_string(),
        }
    }

    #[test]
    fn apply_repair_edits_the_field_and_appends_a_revision_log_row_observed_failing() {
        let content = "---\nEmbodiment: Not started\n---\n# 0001 — X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Initial. | **structural** |\n";
        let result = apply_repair(
            content,
            &sample_repair(),
            "2026-09-05",
            "beau",
            &Default::default(),
            true,
        )
        .unwrap();
        assert!(result.contains("Embodiment: Verified"));
        assert!(!result.contains("Embodiment: Not started"));
        assert!(result.contains("Tool-authored (by beau)"));
        assert!(result.contains("**structural**"));
        // the original row survives untouched
        assert!(result.contains("| 2026-08-20 | Initial. | **structural** |"));
    }

    fn empty_value_repair() -> Repair {
        Repair {
            current_value: String::new(),
            computed_value: "Implemented".to_string(),
            ..sample_repair()
        }
    }

    #[test]
    fn apply_repair_fills_a_null_tilde_or_empty_value_by_position_observed_failing() {
        for raw in ["null", "~", "", "Null", "NULL"] {
            let content = format!(
                "---\nEmbodiment: {raw}\nRealized-by: code:src/lib.rs\n---\n# 0001 — X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Initial. | **structural** |\n"
            );
            let result = apply_repair(
                &content,
                &empty_value_repair(),
                "2026-09-05",
                "beau",
                &Default::default(),
                true,
            )
            .unwrap();
            assert!(
                result.contains("\nEmbodiment: Implemented\n"),
                "{raw:?}: {result}"
            );
            assert!(!result.contains("ImplementedEmbodiment"), "{raw:?}");
            assert!(result.contains("stated no value disagreed"), "{raw:?}");
            assert!(!result.contains("stated `` disagreed"), "{raw:?}");
        }
    }

    #[test]
    fn apply_repair_does_not_touch_a_substring_of_the_field_name_or_prose_observed_failing() {
        // The stated value "Done" also occurs inside "Done-when:" and prose; only the
        // value after `Status:` may change.
        let repair = Repair {
            field: "Status".to_string(),
            current_value: "Done".to_string(),
            computed_value: "Planned".to_string(),
            ..sample_repair()
        };
        let content = "---\nDone-when: Done\nStatus: Done\n---\n# Done\n";
        let result = apply_repair(
            content,
            &repair,
            "2026-09-05",
            "beau",
            &Default::default(),
            false,
        )
        .unwrap();
        assert_eq!(
            result,
            "---\nDone-when: Done\nStatus: Planned\n---\n# Done\n"
        );
    }

    #[test]
    fn apply_repair_refuses_an_empty_stated_value_when_the_field_now_has_one() {
        let content = "---\nEmbodiment: Verified\n---\n# 0001 — X\n";
        let err = apply_repair(
            content,
            &empty_value_repair(),
            "2026-09-05",
            "beau",
            &Default::default(),
            false,
        )
        .unwrap_err();
        assert!(err.contains("could not find"), "{err}");
    }

    #[test]
    fn apply_repair_refuses_without_a_revision_log_section() {
        let content = "---\nEmbodiment: Not started\n---\n# 0001 — X\n";
        let err = apply_repair(
            content,
            &sample_repair(),
            "2026-09-05",
            "beau",
            &Default::default(),
            true,
        )
        .unwrap_err();
        assert!(err.contains("Revision log"));
    }

    #[test]
    fn apply_repair_without_a_required_log_edits_only_the_field() {
        let content = "---\nEmbodiment: Not started\n---\n# 0001 — X\n";
        let result = apply_repair(
            content,
            &sample_repair(),
            "2026-09-05",
            "beau",
            &Default::default(),
            false,
        )
        .unwrap();
        assert_eq!(result, "---\nEmbodiment: Verified\n---\n# 0001 — X\n");
    }

    #[test]
    fn apply_repair_refuses_when_the_stated_value_has_already_changed() {
        // The record changed since detect ran (e.g. a concurrent edit) --
        // RFC-0008 §4's re-verify-before-write requirement, exercised at the
        // level this pure function can enforce: it can't find what it was
        // told to replace.
        let content = "---\nEmbodiment: Implemented\n---\n# 0001 — X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Initial. | **structural** |\n";
        let err = apply_repair(
            content,
            &sample_repair(),
            "2026-09-05",
            "beau",
            &Default::default(),
            true,
        )
        .unwrap_err();
        assert!(err.contains("could not find"));
    }

    #[test]
    fn apply_repair_appends_a_plain_table_row_inside_the_plain_table_observed_failing() {
        let content = "---\nEmbodiment: Not started\n---\n# 0001 — X\n\n**Revision log**\n\n| Date | Change | Class |\n|---|---|---|\n| 2026-08-20 | Initial. | **structural** |\n";
        let result = apply_repair(
            content,
            &sample_repair(),
            "2026-09-05",
            "beau",
            &Default::default(),
            true,
        )
        .unwrap();
        let last = result.lines().last().unwrap();
        assert!(last.starts_with("| 2026-09-05 |"), "{last}");
        assert!(!result.contains("> | 2026-09-05"));
    }

    #[test]
    fn apply_repair_on_a_crlf_file_changes_only_the_intended_lines_observed_failing() {
        let content = "# 0001 — X\r\n\r\n> Embodiment: Not started\r\n\r\n> **Revision log**\r\n>\r\n> | Date | Change | Class |\r\n> |---|---|---|\r\n> | 2026-08-20 | Initial. | **structural** |";
        let result = apply_repair(
            content,
            &sample_repair(),
            "2026-09-05",
            "beau",
            &Default::default(),
            true,
        )
        .unwrap();
        assert!(result.starts_with(
            "# 0001 — X\r\n\r\n> Embodiment: Verified\r\n\r\n> **Revision log**\r\n>\r\n"
        ));
        assert!(!result.replace("\r\n", "").contains('\n'), "bare LF");
        assert!(!result.ends_with('\n'), "trailing newline was added");
        assert_eq!(result.lines().count(), content.lines().count() + 1);
    }

    #[test]
    fn fresh_drift_leaves_a_stated_drift_detected_alone_observed_failing() {
        let r = record(
            "docs/adr/0001-x.md",
            "---\nEmbodiment: Drift detected\nRealized-by: code:src/lib.rs\n---\n",
        );
        let fresh: HashSet<PathBuf> = [PathBuf::from("docs/adr/0001-x.md")].into();
        let (_, repairs) = detect_repairs(
            &[r],
            &no_relation_field_overrides(),
            &HashSet::new(),
            &fresh,
            &Default::default(),
        );
        assert!(repairs.is_empty(), "{repairs:?}");
    }

    const H1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const H2: &str = "2222222222222222222222222222222222222222222222222222222222222222";

    fn pinned(path: &str, hash: &str) -> Record {
        record(
            path,
            &format!(
                "---\nEmbodiment: Implemented\nRealized-by: 'spec:docs/s.md, code:src/a.rs@sha256:{hash}, test:t.rs'\n---\n# 1\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
            ),
        )
    }

    #[test]
    fn a_stale_pin_is_listed_with_both_hashes_and_a_current_one_is_not_observed_failing() {
        let stale = pinned("docs/adr/0001-x.md", H1);
        let current = pinned("docs/adr/0002-y.md", H2);
        let lookup = |l: &str| (l == "src/a.rs").then(|| H2.to_string());
        let (population, found) =
            detect_rebaselines(&[stale, current], &no_relation_field_overrides(), &lookup);
        assert_eq!(population.examined(), 2);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].pins[0].recorded_hash, H1);
        assert_eq!(found[0].pins[0].current_hash, H2);
    }

    #[test]
    fn re_pinning_moves_only_the_hash_and_logs_who_and_why_observed_failing() {
        let r = pinned("docs/adr/0001-x.md", H1);
        let lookup = |_: &str| Some(H2.to_string());
        let (_, found) = detect_rebaselines(&[r], &no_relation_field_overrides(), &lookup);
        let content = format!(
            "---\nEmbodiment: Implemented\nRealized-by: 'spec:docs/s.md, code:src/a.rs@sha256:{H1}, test:t.rs'\n---\n# 1\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
        );
        let out = apply_rebaseline(
            &content,
            &found[0],
            "2026-11-04",
            "ana",
            "Read the diff; the claim holds.",
            &Default::default(),
        )
        .unwrap();
        assert!(out.contains(&format!(
            "Realized-by: 'spec:docs/s.md, code:src/a.rs@sha256:{H2}, test:t.rs'"
        )));
        assert!(out.contains("attested by ana"));
        assert!(out.contains("Read the diff; the claim holds."));
        // Applying the same rebaseline again finds no pin to move.
        assert!(apply_rebaseline(&out, &found[0], "d", "ana", "r", &Default::default()).is_err());
    }

    #[test]
    fn a_status_repair_edits_the_status_line_not_a_title_that_mentions_both_words() {
        let content = "---\nStatus: Planned\n---\n# Planned work whose Status is open\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Filed. | **structural** |\n";
        let repair = Repair {
            record: PathBuf::from("docs/milestones/MILE-1-x.md"),
            field: "Status".to_string(),
            line: 2,
            current_value: "Planned".to_string(),
            computed_value: "Done".to_string(),
            tier: 1,
            evidence: "Realized-by computes to Implemented".to_string(),
        };
        let result = apply_repair(
            content,
            &repair,
            "2026-11-04",
            "beau",
            &Default::default(),
            true,
        )
        .unwrap();
        assert!(result.contains("\nStatus: Done\n"), "{result}");
        assert!(
            result.contains("# Planned work whose Status is open"),
            "{result}"
        );
        assert!(
            result.contains("disagreed with the computed value `Done`"),
            "{result}"
        );
    }
}
