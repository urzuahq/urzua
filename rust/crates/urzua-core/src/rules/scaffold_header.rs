//! `header.scaffold-mismatch` (`MILE-103`): a record a `--base` diff adds whose
//! header differs from the header `urzua new` would have written for it.
//!
//! The comparison re-renders the header through the shared, pure
//! `new_record::scaffold_record` from the configuration the run reads (the
//! creation commit's, when the run is at that commit), with the values only the
//! author can have chosen taken from the record itself: its number, title,
//! `Stable-Id`, author and date. What is left to compare is what `urzua new`
//! decides: which fields exist, in which order, and the value each starts at. A
//! field the scaffold leaves null may be filled; no field may be added, removed
//! or reordered, and a value `new` wrote (the initial status, the starting
//! version) may not be changed. A field the scaffold leaves null may also be
//! left out: a record that drops an optional placeholder it did not fill is
//! still the record `new` wrote. `urzua-record.sh finish` applies a stricter
//! form of this in shell, to what an agent may do; this rule measures what was
//! merged, and is kept here so a pull request sees it before merge.
//!
//! Opt-in, a warning, and silent without `--base`: a whole-repository run has no
//! notion of "newly added".

use super::*;
use crate::header::Header;
use crate::report::{Outcome, Population};

/// What stands for a value `urzua new` resolves (the id, the author, the date)
/// when the record has none, so the message names what the record lacks.
const RESOLVED_BY_NEW: &str = "<resolved by urzua new>";

/// One way a record's header departs from the scaffold, rendered for the
/// finding's message.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Difference {
    /// A field `urzua new` writes that the record does not have.
    Missing(String),
    /// A field the record has that `urzua new` does not write (a repeated key
    /// counts once per extra occurrence).
    Extra(String),
    /// The same fields, written in another order.
    Order {
        scaffold: Vec<String>,
        found: Vec<String>,
    },
    /// A value `urzua new` wrote, changed.
    Value {
        key: String,
        scaffold: String,
        found: String,
    },
}

impl Difference {
    fn describe(&self) -> String {
        match self {
            Difference::Missing(k) => format!("'{k}' is missing (urzua new writes it)"),
            Difference::Extra(k) => format!("'{k}' is not a field urzua new writes"),
            Difference::Order { scaffold, found } => format!(
                "the fields are in another order (urzua new: {}; here: {})",
                scaffold.join(", "),
                found.join(", ")
            ),
            Difference::Value {
                key,
                scaffold,
                found,
            } => format!("'{key}' is '{found}' where urzua new writes '{scaffold}'"),
        }
    }
}

/// How `found` departs from `scaffold`, in a fixed order: missing fields, extra
/// fields, field order, then changed values. Empty means conformant: the same
/// keys in the same order, with scaffold-null values filled in or the field left
/// out.
pub(crate) fn header_differences(scaffold: &Header, found: &Header) -> Vec<Difference> {
    // A null field the record leaves out is not a difference: nothing `new`
    // decided is missing from it.
    let scaffold_keys: Vec<&str> = scaffold
        .fields
        .iter()
        .filter(|f| !f.value.trim().is_empty() || found.fields.iter().any(|g| g.key == f.key))
        .map(|f| f.key.as_str())
        .collect();
    let found_keys: Vec<&str> = found.fields.iter().map(|f| f.key.as_str()).collect();
    let mut out = Vec::new();

    let mut remaining = found_keys.clone();
    for key in &scaffold_keys {
        match remaining.iter().position(|k| k == key) {
            Some(at) => {
                remaining.remove(at);
            }
            None => out.push(Difference::Missing((*key).to_string())),
        }
    }
    for key in &remaining {
        out.push(Difference::Extra((*key).to_string()));
    }

    // The order is only a separate question once the same fields are present.
    let same_fields = !out
        .iter()
        .any(|d| matches!(d, Difference::Missing(_) | Difference::Extra(_)));
    if same_fields && scaffold_keys != found_keys {
        out.push(Difference::Order {
            scaffold: scaffold_keys.iter().map(|k| (*k).to_string()).collect(),
            found: found_keys.iter().map(|k| (*k).to_string()).collect(),
        });
    }

    for field in &scaffold.fields {
        // Null in the scaffold: a decision or content left for the author.
        if field.value.trim().is_empty() {
            continue;
        }
        if let Some(value) = found.get(field.key.as_str()) {
            if value.trim() != field.value.trim() {
                out.push(Difference::Value {
                    key: field.key.as_str().to_string(),
                    scaffold: field.value.trim().to_string(),
                    found: value.trim().to_string(),
                });
            }
        }
    }
    out
}

/// The header `urzua new` would have written for `record`, or `None` when the
/// record's own file name carries no number to write it for.
pub(crate) fn scaffold_header_for(
    record: &Record,
    config: &Config,
    template: Option<&str>,
) -> Option<Header> {
    let type_config = config.record_types.get(&record.record_type)?;
    let file_name = record.path.file_name()?.to_str()?;
    let compiled = crate::document_model::declared_model(&record.record_type, type_config)
        .ok()
        .flatten()
        .and_then(|m| m.identity)
        .and_then(|decl| crate::document_model::CompiledIdentity::compile(&decl).ok());
    let number = match &compiled {
        Some(c) => c.identify(file_name).number?.parse::<u32>().ok()?,
        None => crate::new_record::parse_record_filename(file_name)?.1,
    };
    // A value only `new`'s own resolution can produce (the id, the author, the
    // date, and the title where the type declares one) is taken from the record.
    // A record that lacks it, or leaves it blank, gets a stand-in, so the
    // scaffold's field is not null and its absence is a difference.
    let own = |value: Option<&str>| match value.map(str::trim) {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => RESOLVED_BY_NEW.to_string(),
    };
    let title = type_config
        .title_field()
        .map(|f| own(record.header.get(f)))
        .unwrap_or_default();
    let stable_id = own(record.header.get_reserved("Stable-Id"));
    let author = own(record.header.get(type_config.author_field()));
    let today = own(record.header.get(type_config.date_field()));
    let text = crate::new_record::scaffold_record(
        &crate::new_record::ScaffoldSpec::plain(
            type_config,
            compiled.as_ref(),
            &record.type_prefix,
            file_name,
            number,
            &title,
            &stable_id,
            &author,
            &today,
        ),
        template,
    );
    Some(crate::header::parse(&text))
}

pub fn header_scaffold_mismatch(
    records: &[Record],
    config: &Config,
    added: Option<&[PathBuf]>,
    renamed: &HashSet<PathBuf>,
    template_texts: &HashMap<String, String>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_HEADER_SCAFFOLD_MISMATCH;
    let Some(added) = added else {
        // A diff rule: without `--base` nothing is "newly added".
        return not_applicable(RULE_ID);
    };
    let added: HashSet<&PathBuf> = added.iter().collect();
    let mut candidates: Vec<&Record> = records.iter().filter(|r| added.contains(&r.path)).collect();
    candidates.sort_by(|a, b| a.path.cmp(&b.path));
    let mut findings = Vec::new();
    let (population, examined): (Population, Vec<PathBuf>) = scoped_census_records(
        config,
        PopulationUnit::Record,
        candidates,
        |record| *record,
        |record| {
            if record.header.is_unreadable() {
                return Outcome::Unreadable;
            }
            // A file moved under a new name is not a creation: its header was
            // written, once, by whatever made it then.
            if renamed.contains(&record.path) {
                return Outcome::Absent;
            }
            let template = template_texts.get(&record.record_type).map(String::as_str);
            let Some(scaffold) = scaffold_header_for(record, config, template) else {
                // No number in the file name: `filename.identity-pattern`'s to say.
                return Outcome::OutOfScope;
            };
            let differences = header_differences(&scaffold, &record.header);
            if !differences.is_empty() {
                let listed = differences
                    .iter()
                    .map(Difference::describe)
                    .collect::<Vec<_>>()
                    .join("; ");
                findings.push(Finding::new(
                    RULE_ID,
                    FindingSeverity::Warning,
                    record.path.clone(),
                    ActionSource::new("scaffold-mismatch"),
                    format!(
                        "this record's header is not the one `urzua new {}` writes: {listed}. Create a record with `urzua new` and fill only its null fields and body; if the difference is deliberate, waive this finding",
                        record.record_type
                    ),
                ));
            }
            Outcome::Examined
        },
    );
    (RuleExecution::ran(RULE_ID, population, examined), findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::*;

    fn adr_config() -> Config {
        config_with_types(vec![(
            "adr",
            crate::config::RecordTypeConfig {
                dir: "docs/adr".to_string(),
                prefix: Some("ADR".to_string()),
                required_fields: ["Status", "Date", "Author", "Stable-Id", "Supersedes"]
                    .map(String::from)
                    .to_vec(),
                statuses: Some(
                    yaml_serde::from_str("values: [Draft, Accepted]\ninitial: [Draft]").unwrap(),
                ),
                ..Default::default()
            },
        )])
    }

    fn adr(path: &str, header: &str) -> Record {
        record_with_prefix(
            path,
            "adr",
            "ADR",
            &format!("---\n{header}---\n# ADR-4 — T\n"),
        )
    }

    fn run(records: &[Record], added: Option<&[&str]>) -> (RuleExecution, Vec<Finding>) {
        let paths: Option<Vec<PathBuf>> = added.map(|a| a.iter().map(PathBuf::from).collect());
        header_scaffold_mismatch(
            records,
            &adr_config(),
            paths.as_deref(),
            &HashSet::new(),
            &HashMap::new(),
        )
    }

    const WHAT_NEW_WRITES: &str =
        "Stable-Id: 01ABC\nStatus: Draft\nDate: 2026-11-14\nAuthor: ann\nSupersedes: null\n";

    #[test]
    fn a_header_that_is_exactly_the_scaffold_is_conformant() {
        let r = adr("docs/adr/ADR-4-t.md", WHAT_NEW_WRITES);
        let (exec, findings) = run(&[r], Some(&["docs/adr/ADR-4-t.md"]));
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exec.population.as_ref().map(|p| p.examined()), Some(1));
    }

    #[test]
    fn filling_a_null_field_is_conformant() {
        let header = WHAT_NEW_WRITES.replace("Supersedes: null", "Supersedes: ADR-2");
        let r = adr("docs/adr/ADR-4-t.md", &header);
        let (_, findings) = run(&[r], Some(&["docs/adr/ADR-4-t.md"]));
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn leaving_a_null_field_out_is_conformant() {
        let header = WHAT_NEW_WRITES.replace("Supersedes: null\n", "");
        let r = adr("docs/adr/ADR-4-t.md", &header);
        let (exec, findings) = run(&[r], Some(&["docs/adr/ADR-4-t.md"]));
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exec.population.as_ref().map(|p| p.examined()), Some(1));
    }

    #[test]
    fn a_missing_field_the_scaffold_fills_is_reported() {
        let header = WHAT_NEW_WRITES.replace("Author: ann\n", "");
        let r = adr("docs/adr/ADR-4-t.md", &header);
        let (_, findings) = run(&[r], Some(&["docs/adr/ADR-4-t.md"]));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("'Author' is missing"));
        assert_eq!(findings[0].severity, FindingSeverity::Warning);
        assert_eq!(findings[0].rule, RULE_HEADER_SCAFFOLD_MISMATCH);
    }

    #[test]
    fn a_missing_stable_id_is_reported() {
        let header = WHAT_NEW_WRITES.replace("Stable-Id: 01ABC\n", "");
        let r = adr("docs/adr/ADR-4-t.md", &header);
        let (_, findings) = run(&[r], Some(&["docs/adr/ADR-4-t.md"]));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("'Stable-Id' is missing"));
    }

    #[test]
    fn a_blank_author_is_a_changed_value() {
        let header = WHAT_NEW_WRITES.replace("Author: ann", "Author:");
        let r = adr("docs/adr/ADR-4-t.md", &header);
        let (_, findings) = run(&[r], Some(&["docs/adr/ADR-4-t.md"]));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("'Author' is ''"));
    }

    #[test]
    fn leaving_null_fields_out_keeps_the_order_of_the_rest() {
        // Status, Date and Author are in the scaffold's order with the null
        // fields gone; swapping Date and Author is an order difference.
        let kept = "Stable-Id: 01ABC\nStatus: Draft\nDate: 2026-11-14\nAuthor: ann\n";
        let (_, findings) = run(
            &[adr("docs/adr/ADR-4-t.md", kept)],
            Some(&["docs/adr/ADR-4-t.md"]),
        );
        assert!(findings.is_empty(), "{findings:?}");
        let swapped = "Stable-Id: 01ABC\nStatus: Draft\nAuthor: ann\nDate: 2026-11-14\n";
        let (_, findings) = run(
            &[adr("docs/adr/ADR-4-t.md", swapped)],
            Some(&["docs/adr/ADR-4-t.md"]),
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("another order"));
    }

    #[test]
    fn an_added_field_is_reported() {
        let header = format!("{WHAT_NEW_WRITES}Extra: x\n");
        let r = adr("docs/adr/ADR-4-t.md", &header);
        let (_, findings) = run(&[r], Some(&["docs/adr/ADR-4-t.md"]));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("'Extra' is not a field"));
    }

    #[test]
    fn a_reordered_header_is_reported_as_an_order_difference() {
        let header =
            "Status: Draft\nStable-Id: 01ABC\nDate: 2026-11-14\nAuthor: ann\nSupersedes: null\n";
        let r = adr("docs/adr/ADR-4-t.md", header);
        let (_, findings) = run(&[r], Some(&["docs/adr/ADR-4-t.md"]));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].message.contains("another order"),
            "{}",
            findings[0].message
        );
    }

    #[test]
    fn a_changed_scaffold_value_is_reported() {
        // Both `Status` and the id are scaffolded values: a hand-set state is a
        // decision `urzua new` never makes.
        let header = WHAT_NEW_WRITES.replace("Status: Draft", "Status: Accepted");
        let r = adr("docs/adr/ADR-4-t.md", &header);
        let (_, findings) = run(&[r], Some(&["docs/adr/ADR-4-t.md"]));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("'Status' is 'Accepted'"));
    }

    #[test]
    fn it_judges_only_added_records_and_only_with_a_diff() {
        let hand = adr("docs/adr/ADR-4-t.md", "Status: Draft\n");
        let (exec, findings) = run(std::slice::from_ref(&hand), None);
        assert!(findings.is_empty());
        assert_eq!(exec.status, RuleStatus::NotApplicable);
        let (_, findings) = run(&[hand], Some(&["docs/adr/ADR-9-other.md"]));
        assert!(findings.is_empty(), "an unchanged record is not examined");
    }

    #[test]
    fn a_renamed_record_is_not_a_creation() {
        let hand = adr("docs/adr/ADR-4-t.md", "Status: Draft\n");
        let renamed: HashSet<PathBuf> = [PathBuf::from("docs/adr/ADR-4-t.md")].into();
        let (exec, findings) = header_scaffold_mismatch(
            &[hand],
            &adr_config(),
            Some(&[PathBuf::from("docs/adr/ADR-4-t.md")]),
            &renamed,
            &HashMap::new(),
        );
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exec.population.as_ref().map(|p| p.examined()), Some(0));
    }

    #[test]
    fn a_template_field_the_type_declares_is_part_of_the_scaffold() {
        let mut config = adr_config();
        config.record_types.get_mut("adr").unwrap().known_fields = Some(vec!["Note".to_string()]);
        let template = "---\nNote: null\n---\n# 0 -- T\n";
        let texts: HashMap<String, String> = [("adr".to_string(), template.to_string())].into();
        let with_note = format!("{WHAT_NEW_WRITES}Note: null\n");
        let r = adr("docs/adr/ADR-4-t.md", &with_note);
        let (_, findings) = header_scaffold_mismatch(
            &[r],
            &config,
            Some(&[PathBuf::from("docs/adr/ADR-4-t.md")]),
            &HashSet::new(),
            &texts,
        );
        assert!(findings.is_empty(), "{findings:?}");
        // The template's field is null, so leaving it out is still conformant; a
        // field it does not list, written by hand, is not.
        let r = adr("docs/adr/ADR-4-t.md", WHAT_NEW_WRITES);
        let (_, findings) = header_scaffold_mismatch(
            &[r],
            &config,
            Some(&[PathBuf::from("docs/adr/ADR-4-t.md")]),
            &HashSet::new(),
            &texts,
        );
        assert!(findings.is_empty(), "{findings:?}");
        let r = adr(
            "docs/adr/ADR-4-t.md",
            &format!("{WHAT_NEW_WRITES}Other: x\n"),
        );
        let (_, findings) = header_scaffold_mismatch(
            &[r],
            &config,
            Some(&[PathBuf::from("docs/adr/ADR-4-t.md")]),
            &HashSet::new(),
            &texts,
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
    }
}
