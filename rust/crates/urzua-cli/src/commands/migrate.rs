//! `urzua migrate ids` / `urzua migrate schema --report` (SPEC-14): two
//! genuinely different operations under one verb -- backfilling stable
//! identifiers onto existing records, and previewing what a newly-required
//! field would break before it's ever added to config.

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::header::HeaderShape;
use urzua_core::report::{MigrateSchemaReport, Notice, Report, ReportStatus};

use crate::emit;
use crate::session::{emit_or_not_run, Session};

/// `urzua migrate ids`'s report (SPEC-14).
#[derive(serde::Serialize)]
struct MigrateIdsReport {
    status: ReportStatus,
    apply: bool,
    missing: Vec<PathBuf>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    results: Vec<MigrateIdsResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
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

fn run_ids_inner(config_path: Option<PathBuf>, apply: bool) -> Result<ExitCode, String> {
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
            status: ReportStatus::Ok,
            apply,
            missing: missing.into_iter().map(|r| r.path.clone()).collect(),
            results: Vec::new(),
            notices: Vec::new(),
        }));
    }

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
        match std::fs::write(&full_path, new_content) {
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

    Ok(emit(&MigrateIdsReport {
        status: ReportStatus::Ok,
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
) -> Result<ExitCode, String> {
    let Session {
        config, records, ..
    } = Session::open(config_path)?;
    if records.is_empty() {
        return Err("no records discovered -- nothing to check".to_string());
    }

    let (population, would_fail, notices) =
        urzua_core::migrate::schema_report(&records, &field, &config);

    Ok(emit(&MigrateSchemaReport {
        status: ReportStatus::Ok,
        field,
        population,
        would_fail,
        notices,
    }))
}
