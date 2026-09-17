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

/// Scan `docs/`'s direct subdirectories for record-shaped files
/// (`NNNN-slug.md`) and propose one record type per subdirectory that has
/// at least one. Adopt mode: this only reads, it never writes or moves.
pub fn detect_record_types(repo_root: &Path, discovered: &[PathBuf]) -> Vec<ProposedRecordType> {
    let docs_dir = PathBuf::from("docs");
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();

    for rel_path in discovered {
        let Ok(under_docs) = rel_path.strip_prefix(&docs_dir) else {
            continue;
        };
        let mut components = under_docs.components();
        let Some(subdir) = components.next() else {
            continue;
        };
        // Must be at least one more component (a file inside the subdir),
        // not a file directly under docs/.
        if components.next().is_none() {
            continue;
        }
        let Some(file_name) = rel_path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !is_record_shaped(file_name) {
            continue;
        }
        let subdir_name = subdir.as_os_str().to_string_lossy().to_string();
        *counts.entry(subdir_name).or_insert(0) += 1;
    }

    let _ = repo_root; // reserved: adopt mode may need repo_root for future classification passes
    counts
        .into_iter()
        .map(|(dir, record_count)| ProposedRecordType {
            name: singular_type_name(&dir),
            dir: format!("docs/{dir}"),
            record_count,
        })
        .collect()
}

/// `NNNN-slug.md`, never a leading underscore (a template, per SPEC-0005 --
/// though templates are expected to have already moved to `.urzua/templates/`
/// by the time adopt runs against a corpus this tool itself governs).
fn is_record_shaped(file_name: &str) -> bool {
    if !file_name.ends_with(".md") || file_name.starts_with('_') {
        return false;
    }
    let stem = &file_name[..file_name.len() - 3];
    let Some((number, _rest)) = stem.split_once('-') else {
        return false;
    };
    number.len() == 4 && number.chars().all(|c| c.is_ascii_digit())
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
            "no record-shaped files found under docs/ -- nothing to adopt",
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
