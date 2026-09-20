//! `urzua init` adopt mode (SPEC-5). Adopt is the primary case: every
//! codebase this project's design was drawn from already had records before
//! it had tooling. Adopt never moves a file -- it reads the tree, proposes a
//! config, and writes only under `.urzua/`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::report::{CouldNotRun, Notice, Report, ReportStatus};

use crate::discovery::find_repo_root;
use crate::emit;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProposedRecordType {
    pub name: String,
    pub dir: String,
    pub record_count: usize,
    /// The type prefix the directory's filenames actually carry, when they all
    /// agree on one. `None` means the corpus uses the bare `NNNN-slug` shape,
    /// which carries no identity a rule can read (BUG-61).
    pub prefix: Option<String>,
}

/// Propose one record type per directory holding record-shaped files. Adopt
/// mode: this only reads, it never writes or moves.
pub fn detect_record_types(repo_root: &Path, discovered: &[PathBuf]) -> Vec<ProposedRecordType> {
    let mut counts: std::collections::BTreeMap<PathBuf, usize> = std::collections::BTreeMap::new();
    let mut prefixes: std::collections::BTreeMap<PathBuf, Vec<Option<String>>> =
        std::collections::BTreeMap::new();

    for rel_path in discovered {
        let Some(file_name) = rel_path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !is_record_shaped(file_name) {
            continue;
        }
        let Some(parent) = rel_path.parent() else {
            continue;
        };
        // A record at the repository root has no directory that identifies its
        // type, and proposing "" would claim every file in the repository.
        if parent.as_os_str().is_empty() {
            continue;
        }
        *counts.entry(parent.to_path_buf()).or_insert(0) += 1;
        let detected = urzua_core::new_record::parse_record_filename(file_name)
            .and_then(|(prefix, _)| prefix.map(|p| p.to_string()));
        prefixes
            .entry(parent.to_path_buf())
            .or_default()
            .push(detected);
    }

    // Discovery matches a type's `dir` by path prefix, so a directory proposed
    // alongside one that contains it is already covered by the outer type. The
    // outer one wins: dropping it instead adopts an `archive/` subdirectory and
    // leaves its parent's records ungoverned, with `check` reporting success
    // over them (BUG-43).
    // One type per directory. Inferring which directories belong together
    // loses records whenever the inference is wrong, and an adopter can merge
    // two proposed types far more easily than they can notice a missing one.

    let mut names: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for dir in counts.keys() {
        *names.entry(proposed_name(dir)).or_insert(0) += 1;
    }

    let _ = repo_root;
    counts
        .into_iter()
        .map(|(dir, record_count)| {
            let base = proposed_name(&dir);
            // Two directories can end in the same component (`doc/adr` and
            // `docs/adr`). A duplicated type name is a config that cannot load,
            // so qualify it rather than emit one.
            let name = if names.get(&base).copied().unwrap_or(0) > 1 {
                dir.components()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join("-")
            } else {
                base
            };
            // Unanimous or nothing. A directory whose filenames disagree has
            // no single prefix to declare, and guessing one silently ungoverns
            // every record that does not match it.
            let seen = prefixes.get(&dir).cloned().unwrap_or_default();
            let prefix = match seen.first() {
                Some(first) if seen.iter().all(|p| p == first) => first.clone(),
                _ => None,
            };
            ProposedRecordType {
                name,
                dir: dir.to_string_lossy().to_string(),
                record_count,
                prefix,
            }
        })
        .collect()
}

fn proposed_name(dir: &Path) -> String {
    let last = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    singular_type_name(&last)
}

/// A leading underscore is a template, never a record. Everything else defers
/// to the shared recogniser, so adopt mode and `new`'s numbering cannot accept
/// disjoint sets.
fn is_record_shaped(file_name: &str) -> bool {
    !file_name.starts_with('_')
        && urzua_core::new_record::parse_record_filename(file_name).is_some()
}

/// `adr` -> `adr`, `rfc` -> `rfc`, `specs` -> `spec` -- adopt proposes the
/// type name a config would actually declare, not the directory's own name.
fn singular_type_name(dir_name: &str) -> String {
    dir_name.strip_suffix('s').unwrap_or(dir_name).to_string()
}

/// Built through the real serializer rather than string-concatenated, so a
/// directory name containing YAML metacharacters cannot break out of its value
/// (BUG-36). The `Mapping` is ordered by `proposed`, so repeated runs over the
/// same corpus produce byte-identical output.
pub fn render_config_yaml(proposed: &[ProposedRecordType]) -> String {
    use yaml_serde::{Mapping, Value};

    let mut types = Mapping::new();
    for rt in proposed {
        let mut entry = Mapping::new();
        entry.insert(Value::from("dir"), Value::from(rt.dir.as_str()));
        entry.insert(Value::from("required_fields"), Value::Sequence(vec![]));
        // ADR-33: recommend the destination shape for any newly-adopted type
        // going forward, regardless of what shape the existing corpus happens
        // to use -- adopt mode proposes where to grow, not a preservation of
        // however the corpus already looks.
        entry.insert(Value::from("header_shape"), Value::from("yaml-frontmatter"));
        // Without this, the prefix defaults to the type name upper-cased --
        // which is the directory's name, not the filenames' -- and every rule
        // that reads a record's identity from its filename silently examines
        // nothing (BUG-61).
        if let Some(prefix) = &rt.prefix {
            entry.insert(Value::from("prefix"), Value::from(prefix.as_str()));
        }
        types.insert(Value::from(rt.name.as_str()), Value::Mapping(entry));
    }

    // Proposed at `warn`: an adopted corpus should be told what is irregular
    // without being blocked on it, and promoting a rule is the adopter's call.
    let mut rules = Mapping::new();
    for id in urzua_core::rules::ALL_RULES {
        // Adopt mode has nothing to say about a rule's options, and a rule
        // declared without the options it requires will not load.
        if urzua_core::config::rule_requires_options(id) {
            continue;
        }
        // A corpus whose filenames carry no type prefix has no identity for
        // these rules to read (ADR-36 declined the bare `NNNN-slug` shape), so
        // they would load, report `ran`, and examine zero records. Declining to
        // propose a rule that cannot fire is the same judgement already applied
        // to a rule whose required options adopt mode cannot supply.
        //
        // Every type, not any: these rules skip an individual record whose
        // filename yields no identity, so one prefix-less directory costs only
        // its own records -- dropping the rule outright would cost the rest of
        // the corpus its checks (BUG-70).
        if identity_dependent(id) && proposed.iter().all(|rt| rt.prefix.is_none()) {
            continue;
        }
        rules.insert(Value::from(*id), Value::from("warn"));
    }

    let mut root = Mapping::new();
    root.insert(
        Value::from("schema_version"),
        Value::from(urzua_core::config::CURRENT_SCHEMA_VERSION),
    );
    root.insert(Value::from("record_types"), Value::Mapping(types));
    root.insert(Value::from("rules"), Value::Mapping(rules));

    let body = yaml_serde::to_string(&Value::Mapping(root)).unwrap_or_default();
    format!("# Generated by `urzua init` (adopt mode). Nothing was moved.\n\n{body}")
}

/// Rules that derive a record's identity from its filename's type prefix, so
/// none of them can address a corpus that has none.
fn identity_dependent(rule: &str) -> bool {
    use urzua_core::rules as r;
    matches!(
        rule,
        r::RULE_POINTER_RESOLUTION
            | r::RULE_POINTER_TARGET_STATUS
            | r::RULE_FILENAME_TITLE_CONSISTENCY
            | r::RULE_RELATION_SUPERSESSION_RECIPROCITY
            | r::RULE_NARRATIVE_FIELD_STALE
    )
}

/// `urzua init`'s report (SPEC-5).
#[derive(serde::Serialize)]
struct InitReport {
    status: ReportStatus,
    dry_run: bool,
    config_path: PathBuf,
    proposed: Vec<ProposedRecordType>,
    written: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    config_yaml: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
}

impl Report for InitReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(0)
    }
}

/// Never clobbers an existing config; `--dry-run` produces byte-identical
/// output to the real run, differing only in whether the file is actually
/// written.
pub fn run(dry_run: bool) -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let config_path = crate::discovery::default_config_path(&repo_root);
    if config_path.exists() {
        return emit(&CouldNotRun::from(format!(
            "{} already exists -- refusing to overwrite. Edit it directly, or remove it to re-run adopt.",
            config_path.display()
        )));
    }

    let discovered = match urzua_io::discover_tracked_files(&repo_root) {
        Ok(d) => d,
        Err(e) => return emit(&CouldNotRun::from(e.to_string())),
    };

    let proposed = detect_record_types(&repo_root, &discovered.paths);
    if proposed.is_empty() {
        return emit(&CouldNotRun::from(
            "no record-shaped files found in the tracked set -- nothing to adopt",
        ));
    }

    let rendered = render_config_yaml(&proposed);
    let relative_config_path = config_path
        .strip_prefix(&repo_root)
        .unwrap_or(&config_path)
        .to_path_buf();

    if dry_run {
        return emit(&InitReport {
            status: ReportStatus::Ok,
            dry_run: true,
            config_path: relative_config_path,
            proposed,
            written: false,
            config_yaml: Some(rendered),
            notices: Vec::new(),
        });
    }

    if let Err(e) = std::fs::create_dir_all(config_path.parent().unwrap()) {
        return emit(&CouldNotRun::from(format!("could not create .urzua/: {e}")));
    }
    if let Err(e) = std::fs::write(&config_path, &rendered) {
        return emit(&CouldNotRun::from(format!(
            "could not write {}: {e}",
            config_path.display()
        )));
    }

    emit(&InitReport {
        status: ReportStatus::Ok,
        dry_run: false,
        config_path: relative_config_path,
        proposed,
        written: true,
        config_yaml: None,
        notices: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_one_type_per_populated_docs_subdirectory() {
        let discovered = vec![
            PathBuf::from("docs/adr/0001-x.md"),
            PathBuf::from("docs/adr/0002-y.md"),
            PathBuf::from("docs/rfc/0001-z.md"),
            PathBuf::from("README.md"),
            PathBuf::from("docs/adr/_template.md"),
        ];
        let proposed = detect_record_types(&PathBuf::from("."), &discovered);

        let adr = proposed.iter().find(|p| p.name == "adr").unwrap();
        assert_eq!(adr.record_count, 2);
        assert_eq!(adr.dir, "docs/adr");

        let rfc = proposed.iter().find(|p| p.name == "rfc").unwrap();
        assert_eq!(rfc.record_count, 1);

        assert_eq!(
            proposed.len(),
            2,
            "README.md and the template must not create a type"
        );
    }

    #[test]
    fn specs_directory_proposes_singular_type_name() {
        let discovered = vec![PathBuf::from("docs/specs/0001-x.md")];
        let proposed = detect_record_types(&PathBuf::from("."), &discovered);
        assert_eq!(proposed[0].name, "spec");
        assert_eq!(proposed[0].dir, "docs/specs");
    }

    /// BUG-36: the config was string-concatenated, so a directory name carrying
    /// the format's metacharacters escaped its value. Rendering through the real
    /// serializer is only a fix if the output parses back to the same string.
    #[test]
    fn a_directory_name_full_of_yaml_metacharacters_survives_render_and_reparse_observed_failing() {
        let hostile = "docs: [adr] #x\n  evil: true";
        let rendered = render_config_yaml(&[ProposedRecordType {
            name: "adr".to_string(),
            dir: hostile.to_string(),
            record_count: 1,
            prefix: None,
        }]);
        let parsed = urzua_core::config::parse(&rendered)
            .expect("init must never emit a config it cannot read back");
        assert_eq!(parsed.record_types.get("adr").unwrap().dir, hostile);
    }

    /// RFC-35: one type per directory holding records. No folding, no
    /// containers, no inference -- four heuristics guessed at the overlap
    /// prefix matching created, and each re-entered the last one's failure.
    /// The third silently dropped every record in a directory that had two
    /// record-bearing subdirectories (`BUG-43`'s own failure, re-entered).
    #[test]
    fn every_directory_holding_records_becomes_its_own_type() {
        let discovered: Vec<PathBuf> = [
            "docs/adr/0001-a.md",
            "docs/adr/0002-b.md",
            "docs/adr/0003-c.md",
            "docs/adr/archive/0009-old.md",
            "docs/adr/drafts/0010-new.md",
            "docs/rfc/0001-r.md",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();

        let proposed = detect_record_types(Path::new("/repo"), &discovered);
        let mut got: Vec<(String, usize)> = proposed
            .iter()
            .map(|p| (p.dir.clone(), p.record_count))
            .collect();
        got.sort();

        assert_eq!(
            got,
            vec![
                ("docs/adr".to_string(), 3),
                ("docs/adr/archive".to_string(), 1),
                ("docs/adr/drafts".to_string(), 1),
                ("docs/rfc".to_string(), 1),
            ]
        );

        // Every record is accounted for exactly once. The old fold lost three
        // of these entirely and `check` reported success over them.
        assert_eq!(got.iter().map(|(_, n)| n).sum::<usize>(), discovered.len());
    }

    /// BUG-44: a rule that cannot be declared without options must not be
    /// proposed bare -- the generated config would not load.
    #[test]
    fn the_generated_config_omits_rules_that_require_options() {
        let rendered = render_config_yaml(&[ProposedRecordType {
            name: "adr".to_string(),
            dir: "docs/adr".to_string(),
            record_count: 1,
            prefix: None,
        }]);
        assert!(!rendered.contains("claim.status-agreement"), "{rendered}");
        assert!(!rendered.contains("pointer.target-status"), "{rendered}");
        urzua_core::config::parse(&rendered).expect("init must emit a config that loads");
    }

    /// Two runs over the same corpus must produce the same bytes -- a
    /// `HashMap` in the render path would make this fail intermittently.
    #[test]
    fn rendering_is_deterministic_across_runs() {
        let proposed: Vec<_> = ["adr", "rfc", "spec", "bug", "milestone"]
            .iter()
            .map(|n| ProposedRecordType {
                name: n.to_string(),
                dir: format!("docs/{n}"),
                record_count: 1,
                prefix: None,
            })
            .collect();
        assert_eq!(render_config_yaml(&proposed), render_config_yaml(&proposed));
    }

    #[test]
    fn a_detected_prefix_is_written_so_the_proposed_rules_can_read_an_identity() {
        let proposed = vec![ProposedRecordType {
            name: "decision".to_string(),
            dir: "decisions".to_string(),
            record_count: 2,
            prefix: Some("ADR".to_string()),
        }];
        let yaml = render_config_yaml(&proposed);
        assert!(
            yaml.contains("prefix: ADR"),
            "without it the prefix defaults to DECISION and every identity rule goes inert: {yaml}"
        );
        assert!(yaml.contains("filename.title-consistency"));
    }

    #[test]
    fn one_prefix_less_type_does_not_disarm_the_types_that_have_a_prefix() {
        let proposed = vec![
            ProposedRecordType {
                name: "adr".to_string(),
                dir: "docs/adr".to_string(),
                record_count: 2,
                prefix: Some("ADR".to_string()),
            },
            ProposedRecordType {
                name: "note".to_string(),
                dir: "docs/notes".to_string(),
                record_count: 1,
                prefix: None,
            },
        ];
        let yaml = render_config_yaml(&proposed);
        assert!(
            yaml.contains("filename.title-consistency"),
            "adr carries a prefix, so the identity rules still address it: {yaml}"
        );
        assert!(yaml.contains("prefix: ADR"));
    }

    #[test]
    fn a_corpus_with_no_prefix_is_not_offered_rules_that_cannot_fire() {
        let proposed = vec![ProposedRecordType {
            name: "adr".to_string(),
            dir: "doc/adr".to_string(),
            record_count: 9,
            prefix: None,
        }];
        let yaml = render_config_yaml(&proposed);
        assert!(
            !yaml.contains("prefix:"),
            "there is none to declare: {yaml}"
        );
        for rule in [
            "filename.title-consistency",
            "relation.supersession-reciprocity",
            "pointer.resolution",
            "pointer.target-status",
            "narrative-field.stale",
        ] {
            assert!(
                !yaml.contains(rule),
                "{rule} would report `ran` over zero records: {yaml}"
            );
        }
        assert!(
            yaml.contains("header.required-fields"),
            "rules that do not read an identity are still proposed: {yaml}"
        );
    }
}
