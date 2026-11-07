//! `urzua migrate ids` / `urzua migrate schema --report` / `urzua migrate
//! config` (SPEC-14): three genuinely different operations under one verb --
//! backfilling stable identifiers onto existing records, previewing what a
//! newly-required field would break before it's ever added to config, and
//! rewriting a config from the previous schema version to the current one.

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::header::HeaderShape;
use urzua_core::report::{Failure, MigrateSchemaReport, Notice, Report, ReportStatus};

use crate::discovery::{default_config_path, find_repo_root};
use crate::emit;
use crate::session::{emit_or_not_run, Session};

/// `urzua migrate ids`'s report (SPEC-14).
#[derive(serde::Serialize)]
struct MigrateIdsReport {
    status: MigrateIdsStatus,
    apply: bool,
    missing: Vec<PathBuf>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    results: Vec<MigrateIdsResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
}

/// `partial-failure` when any record could not be written, so the report and
/// the exit code agree (as `fix --apply` and `index --apply` do).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum MigrateIdsStatus {
    Ok,
    PartialFailure,
}

#[derive(serde::Serialize)]
struct MigrateIdsResult {
    record: PathBuf,
    outcome: MigrateIdsOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum MigrateIdsOutcome {
    Applied,
    Skipped,
    Failed,
}

impl Report for MigrateIdsReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    /// Any real write failure exits 1, matching `fix --apply`'s
    /// `PartialFailure` signal for the same shape of outcome. `Skipped` (a
    /// record structurally can't take the field, e.g. no header-shaped
    /// region) is not a write failure and doesn't affect the exit code.
    fn exit_code(&self) -> ExitCode {
        if self
            .results
            .iter()
            .any(|r| r.outcome == MigrateIdsOutcome::Failed)
        {
            ExitCode::from(1)
        } else {
            ExitCode::from(0)
        }
    }
}

/// Backfills a `Stable-Id` header field (ADR-0003/0021) into every record
/// lacking one. Dry-run by default. Never touches the filename or any
/// cross-reference -- the display number stays exactly what it already is.
pub fn run_ids(config_path: Option<PathBuf>, apply: bool) -> ExitCode {
    emit_or_not_run(run_ids_inner(config_path, apply))
}

fn run_ids_inner(config_path: Option<PathBuf>, apply: bool) -> Result<ExitCode, Failure> {
    let Session {
        repo_root,
        config,
        records,
        full_text,
        ..
    } = Session::open(config_path)?;
    let missing: Vec<_> = records
        .iter()
        // Reserved (Header::get_reserved): a record spelling this
        // `stable-id` has made a typo, not declared a second field, and
        // reading it exactly would assign and write a conflicting second id.
        .filter(|r| r.header.get_reserved("Stable-Id").is_none())
        .collect();

    if missing.is_empty() || !apply {
        return Ok(emit(&MigrateIdsReport {
            status: MigrateIdsStatus::Ok,
            apply,
            missing: missing.into_iter().map(|r| r.path.clone()).collect(),
            results: Vec::new(),
            notices: Vec::new(),
        }));
    }

    // The lock `fix --apply` takes: two writers on one record must not
    // interleave.
    let _lock = urzua_io::FixLock::acquire(&repo_root)?;
    let missing_paths: Vec<PathBuf> = missing.iter().map(|r| r.path.clone()).collect();
    let mut results = Vec::with_capacity(missing.len());
    for r in &missing {
        let Some(region) = r.header.region else {
            results.push(MigrateIdsResult {
                record: r.path.clone(),
                outcome: MigrateIdsOutcome::Skipped,
                // A header written in a removed form carries its name here.
                error: Some(
                    r.header
                        .parse_error
                        .clone()
                        .unwrap_or_else(|| "no header-shaped region found".to_string()),
                ),
            });
            continue;
        };
        let Some(content) = full_text.get(&r.path) else {
            continue;
        };

        let id = urzua_id::StableId::generate();
        let header_shape = config
            .record_types
            .get(&r.record_type)
            .map(|cfg| cfg.header_shape)
            .unwrap_or_default();
        let new_line = match header_shape {
            HeaderShape::YamlFrontmatter => format!("Stable-Id: {}", id.as_str()),
            // `none` parses to `region: None` unconditionally (`ADR-50`), and
            // the `let Some(region) = ... else { Skipped }` guard above
            // already returned for exactly that case -- reaching here with
            // `region: Some(_)` and `header_shape: None` would mean that
            // invariant broke.
            HeaderShape::None => {
                unreachable!("a `none`-shaped record has no region to insert into")
            }
        };
        // region.0 is the 1-indexed first header line (the opening `---`
        // for yaml-frontmatter); inserting after it places the new field
        // right below, valid for the one shape with a header. Every other byte of the
        // file, line endings and trailing newline included, is left as is.
        let mut doc = urzua_core::line_edit::LineDoc::parse(content);
        doc.insert_after(region.0, new_line);
        let new_content = doc.to_string();

        let full_path = repo_root.join(&r.path);
        // Re-read right before writing (as `fix` does): the id was placed from
        // the text read at session start, and a record edited since would be
        // overwritten by it.
        match urzua_io::read_to_string(&full_path) {
            Ok(on_disk) if on_disk == *content => {}
            Ok(_) => {
                results.push(MigrateIdsResult {
                    record: r.path.clone(),
                    outcome: MigrateIdsOutcome::Failed,
                    error: Some(
                        "the record changed since it was read; nothing was written -- run `urzua migrate ids --apply` again"
                            .to_string(),
                    ),
                });
                continue;
            }
            Err(e) => {
                results.push(MigrateIdsResult {
                    record: r.path.clone(),
                    outcome: MigrateIdsOutcome::Failed,
                    error: Some(format!("could not re-read: {e}")),
                });
                continue;
            }
        }
        match urzua_io::write_atomic_in(&repo_root, &full_path, &new_content) {
            Ok(()) => results.push(MigrateIdsResult {
                record: r.path.clone(),
                outcome: MigrateIdsOutcome::Applied,
                error: None,
            }),
            Err(e) => results.push(MigrateIdsResult {
                record: r.path.clone(),
                outcome: MigrateIdsOutcome::Failed,
                error: Some(e.to_string()),
            }),
        }
    }

    let status = if results
        .iter()
        .any(|r| r.outcome == MigrateIdsOutcome::Failed)
    {
        MigrateIdsStatus::PartialFailure
    } else {
        MigrateIdsStatus::Ok
    };
    Ok(emit(&MigrateIdsReport {
        status,
        apply,
        missing: missing_paths,
        results,
        notices: Vec::new(),
    }))
}

/// A preview of which existing records would newly fail if `field` were
/// added to config's `required_fields` today. Read-only -- never writes,
/// never touches config. `--assist-waivers` and `--apply` are not
/// implemented (SPEC-0001): the former needs a human-authored `Reason` per
/// waiver, the latter needs `urzua fix` to know a field is tool-writable,
/// which almost none are.
pub fn run_schema_report(config_path: Option<PathBuf>, field: String) -> ExitCode {
    emit_or_not_run(run_schema_report_inner(config_path, field))
}

fn run_schema_report_inner(
    config_path: Option<PathBuf>,
    field: String,
) -> Result<ExitCode, Failure> {
    let Session {
        config,
        records,
        repo_root,
        ..
    } = Session::open(config_path)?;
    if records.is_empty() {
        return Err("no records discovered -- nothing to check".into());
    }

    let mut notices = Vec::new();
    let templates = super::check::load_templates(&repo_root, &config, &mut notices)?;
    let (population, would_fail, report_notices) =
        urzua_core::migrate::schema_report(&records, &field, &config, &templates);
    notices.extend(report_notices);

    Ok(emit(&MigrateSchemaReport {
        status: ReportStatus::Ok,
        field,
        population,
        would_fail,
        notices,
    }))
}

/// `urzua migrate config`'s report (SPEC-14).
#[derive(serde::Serialize)]
struct MigrateConfigReport {
    status: ReportStatus,
    /// The config file, repository-relative when it lies inside the repository.
    file: PathBuf,
    outcome: MigrateConfigOutcome,
    /// The `schema_version` the file declared before this run.
    from: u32,
    /// The `schema_version` it declares now.
    to: u32,
    /// Every key the rewrite added, changed, renamed, moved or dropped, in
    /// order. Empty when there was nothing to do.
    changes: Vec<urzua_core::schema_version::Change>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum MigrateConfigOutcome {
    Migrated,
    NothingToDo,
}

impl Report for MigrateConfigReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(0)
    }
}

/// Rewrites a config at the previous schema version to the current one, in
/// place, and reports every key it touched (`ADR-77`). A config already at the
/// current version is left alone and reported as nothing to do. A version
/// outside the window is never read further: it is the same structured error
/// `check` gives. The rewrite is checked to load before anything is written, so
/// a file that cannot be upgraded cleanly is left exactly as it was.
pub fn run_config(config_path: Option<PathBuf>) -> ExitCode {
    emit_or_not_run(run_config_inner(config_path))
}

fn run_config_inner(config_path: Option<PathBuf>) -> Result<ExitCode, Failure> {
    let repo_root = find_repo_root(&[])?;
    let path = config_path.unwrap_or_else(|| default_config_path(&repo_root));
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("could not read config at {}: {e}", path.display()))?;
    let doc: yaml_serde::Value =
        yaml_serde::from_str(&content).map_err(|e| format!("could not parse config: {e}"))?;
    let found = doc
        .as_mapping()
        .and_then(|m| m.get("schema_version"))
        .and_then(yaml_serde::Value::as_u64)
        .ok_or_else(|| "the config declares no integer `schema_version`".to_string())?;
    let file = path
        .strip_prefix(&repo_root)
        .map(PathBuf::from)
        .unwrap_or_else(|_| path.clone());

    let current = urzua_core::config::CURRENT_SCHEMA_VERSION;
    if found == u64::from(current) {
        return Ok(emit(&MigrateConfigReport {
            status: ReportStatus::Ok,
            file,
            outcome: MigrateConfigOutcome::NothingToDo,
            from: current,
            to: current,
            changes: Vec::new(),
            notices: Vec::new(),
        }));
    }
    if found != u64::from(urzua_core::schema_version::OLDEST_SUPPORTED_SCHEMA_VERSION) {
        let v = urzua_core::schema_version::SchemaVersionError::new(found);
        return Err(Failure {
            message: v.to_string(),
            schema_version: Some(Box::new(v)),
        });
    }

    let migrated = urzua_core::schema_version::migrate_text(&content)?;
    // The file is only replaced by text that loads: the same typed parse
    // `check` runs, so a config the upgrade cannot make valid is reported, not
    // written.
    urzua_core::config::parse(&migrated.text).map_err(|e| {
        format!(
            "the upgraded config would not load ({e}); {} is unchanged -- fix that and run `urzua migrate config` again",
            path.display()
        )
    })?;
    urzua_io::write_atomic(&path, &migrated.text)
        .map_err(|e| format!("could not write {}: {e}", path.display()))?;
    Ok(emit(&MigrateConfigReport {
        status: ReportStatus::Ok,
        file,
        outcome: MigrateConfigOutcome::Migrated,
        from: urzua_core::schema_version::OLDEST_SUPPORTED_SCHEMA_VERSION,
        to: current,
        changes: migrated.changes,
        notices: Vec::new(),
    }))
}
