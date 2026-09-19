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
}

/// Group the tracked set by each record's own parent directory and propose one
/// record type per directory holding at least one record-shaped file. Adopt
/// mode: this only reads, it never writes or moves.
///
/// Derived from the corpus rather than from `docs/` (BUG-36). A hardcoded root
/// meant `init` could not adopt `npryce/adr-tools`, whose records live under
/// `doc/adr/`, and proposing `docs/<dir>` regardless would have been worse than
/// refusing: `check` would then examine zero files and report success.
pub fn detect_record_types(repo_root: &Path, discovered: &[PathBuf]) -> Vec<ProposedRecordType> {
    let mut counts: std::collections::BTreeMap<PathBuf, usize> = std::collections::BTreeMap::new();

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
    }

    // Discovery matches a type's `dir` by path prefix, so a directory proposed
    // alongside one that contains it is already covered by the outer type. The
    // outer one wins: dropping it instead adopts an `archive/` subdirectory and
    // leaves its parent's records ungoverned, with `check` reporting success
    // over them (BUG-43).
    let dirs: Vec<PathBuf> = counts.keys().cloned().collect();
    let nested: Vec<PathBuf> = dirs
        .iter()
        .filter(|inner| {
            dirs.iter()
                .any(|outer| outer != *inner && inner.starts_with(outer))
        })
        .cloned()
        .collect();
    // Fold only where the outer directory is the type's home, not where it
    // merely contains types. `docs/` holding `adr` and `rfc` is a container:
    // absorbing them produces one meaningless `doc` type and ingests whatever
    // else sits beside them (BUG-54). `doc/adr` holding `archive` is a home.
    let container = |outer: &PathBuf| {
        dirs.iter()
            .filter(|d| *d != outer && d.starts_with(outer))
            .filter_map(|d| d.strip_prefix(outer).ok())
            .filter_map(|r| r.components().next())
            .collect::<std::collections::HashSet<_>>()
            .len()
            > 1
    };

    for dir in &nested {
        let enclosing = counts
            .keys()
            .filter(|o| *o != dir && dir.starts_with(o))
            .max_by_key(|o| o.components().count())
            .cloned();
        if enclosing.as_ref().is_some_and(container) {
            continue;
        }
        let Some(n) = counts.remove(dir) else {
            continue;
        };
        // Folded into the nearest enclosing type rather than discarded, so the
        // count an adopter is shown matches what `check` will examine.
        if let Some(outer) = counts
            .keys()
            .filter(|o| dir.starts_with(o))
            .max_by_key(|o| o.components().count())
            .cloned()
        {
            *counts.entry(outer).or_insert(0) += n;
        }
    }

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
            ProposedRecordType {
                name,
                dir: dir.to_string_lossy().to_string(),
                record_count,
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

/// A leading underscore is a template (`SPEC-5`), never a record. Everything
/// else defers to `urzua_core`'s recogniser, so adopt mode and `urzua new`'s
/// numbering cannot drift apart again -- carrying one each is what let them
/// accept disjoint sets (BUG-37).
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
        types.insert(Value::from(rt.name.as_str()), Value::Mapping(entry));
    }

    // Every rule, proposed at `warn`. Adopt mode must not hand a corpus
    // blocking errors it never asked for -- MILE-51 measured exactly that:
    // nine of them, on a foreign corpus, from a config `init` itself wrote.
    // The diagnostics still appear; promoting one to `error` is the adopter's
    // decision to make once they have read it.
    let mut rules = Mapping::new();
    for id in urzua_core::rules::ALL_RULES {
        // A rule that requires options cannot be proposed bare: it would either
        // be rejected at load time or, worse, run inert (BUG-44). Adopt mode
        // has nothing to say about which statuses a corpus considers closed,
        // so it declines to guess and leaves the rule undeclared.
        if urzua_core::config::rule_requires_options(id) {
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
        }]);
        let parsed = urzua_core::config::parse(&rendered)
            .expect("init must never emit a config it cannot read back");
        assert_eq!(parsed.record_types.get("adr").unwrap().dir, hostile);
    }

    /// BUG-43: the containment filter dropped the *outer* directory, so a
    /// corpus with an `archive/` subdirectory adopted one record and left three
    /// ungoverned, with `check` then reporting success over them.
    #[test]
    fn a_nested_record_directory_does_not_displace_its_parent_observed_failing() {
        let discovered: Vec<PathBuf> = [
            "doc/adr/0001-a.md",
            "doc/adr/0002-b.md",
            "doc/adr/0003-c.md",
            "doc/adr/archive/0009-old.md",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();

        let proposed = detect_record_types(Path::new("/repo"), &discovered);
        assert_eq!(proposed.len(), 1, "{proposed:?}");
        assert_eq!(proposed[0].dir, "doc/adr");
        assert_eq!(proposed[0].name, "adr");
        // The nested record is folded in, so the count an adopter is shown
        // matches what `check` will examine.
        assert_eq!(proposed[0].record_count, 4);
    }

    /// BUG-54: one stray record-shaped file in a parent directory collapsed
    /// every sibling type into it, and pulled non-records into the corpus.
    #[test]
    fn a_container_directory_does_not_absorb_the_types_beneath_it_observed_failing() {
        let discovered: Vec<PathBuf> = [
            "docs/adr/0001-a.md",
            "docs/adr/0002-b.md",
            "docs/rfc/0001-r.md",
            "docs/0099-index.md",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();

        let proposed = detect_record_types(Path::new("/repo"), &discovered);
        let names: Vec<&str> = proposed.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"adr"), "{proposed:?}");
        assert!(names.contains(&"rfc"), "{proposed:?}");
        let adr = proposed.iter().find(|p| p.name == "adr").unwrap();
        assert_eq!(adr.dir, "docs/adr");
        assert_eq!(adr.record_count, 2);

        // BUG-43 must still hold: a genuine subdirectory folds into its home.
        let nested: Vec<PathBuf> = [
            "doc/adr/0001-a.md",
            "doc/adr/0002-b.md",
            "doc/adr/archive/0009-old.md",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();
        let proposed = detect_record_types(Path::new("/repo"), &nested);
        assert_eq!(proposed.len(), 1, "{proposed:?}");
        assert_eq!(proposed[0].dir, "doc/adr");
        assert_eq!(proposed[0].record_count, 3);
    }

    /// BUG-44: a rule that cannot be declared without options must not be
    /// proposed bare -- the generated config would not load.
    #[test]
    fn the_generated_config_omits_rules_that_require_options() {
        let rendered = render_config_yaml(&[ProposedRecordType {
            name: "adr".to_string(),
            dir: "docs/adr".to_string(),
            record_count: 1,
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
            })
            .collect();
        assert_eq!(render_config_yaml(&proposed), render_config_yaml(&proposed));
    }
}
