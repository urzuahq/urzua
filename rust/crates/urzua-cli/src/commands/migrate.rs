//! `urzua migrate ids` / `urzua migrate schema --report` (SPEC-14): two
//! genuinely different operations under one verb -- backfilling stable
//! identifiers onto existing records, and previewing what a newly-required
//! field would break before it's ever added to config.

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{CouldNotRun, MigrateSchemaReport, Notice, Report, ReportStatus};

use crate::discovery::{find_repo_root, load_config, load_records};
use crate::emit;

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
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let config_path = config_path.unwrap_or_else(|| repo_root.join(".urzua/config.toml"));
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let discovered = match urzua_io::discover_tracked_files(&repo_root) {
        Ok(d) => d,
        Err(e) => return emit(&CouldNotRun::from(e.to_string())),
    };

    let (records, full_text) = load_records(&repo_root, &discovered.paths, &config);
    let missing: Vec<_> = records
        .iter()
        .filter(|r| r.header.get("Stable-Id").is_none())
        .collect();

    if missing.is_empty() || !apply {
        return emit(&MigrateIdsReport {
            status: ReportStatus::Ok,
            apply,
            missing: missing.into_iter().map(|r| r.path.clone()).collect(),
            results: Vec::new(),
            notices: Vec::new(),
        });
    }

    let missing_paths: Vec<PathBuf> = missing.iter().map(|r| r.path.clone()).collect();
    let mut results = Vec::with_capacity(missing.len());
    for r in &missing {
        let Some(region) = r.header.region else {
            results.push(MigrateIdsResult {
                record: r.path.clone(),
                outcome: MigrateIdsOutcome::Skipped,
                error: Some("no header-shaped region found".to_string()),
            });
            continue;
        };
        let Some(content) = full_text.get(&r.path) else {
            continue;
        };

        let id = urzua_id::StableId::generate();
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        // region.0 is the 1-indexed first header line; inserting at this
        // 0-indexed Vec position places the new field right after it.
        lines.insert(region.0, format!("> Stable-Id: {}", id.as_str()));
        let new_content = lines.join("\n") + "\n";

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

    emit(&MigrateIdsReport {
        status: ReportStatus::Ok,
        apply,
        missing: missing_paths,
        results,
        notices: Vec::new(),
    })
}

/// A preview of which existing records would newly fail if `field` were
/// added to config's `required_fields` today. Read-only -- never writes,
/// never touches config. `--assist-waivers` and `--apply` are not
/// implemented (SPEC-0001): the former needs a human-authored `Reason` per
/// waiver, the latter needs `urzua fix` to know a field is tool-writable,
/// which almost none are.
pub fn run_schema_report(config_path: Option<PathBuf>, field: String) -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let config_path = config_path.unwrap_or_else(|| repo_root.join(".urzua/config.toml"));
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let discovered = match urzua_io::discover_tracked_files(&repo_root) {
        Ok(d) => d,
        Err(e) => return emit(&CouldNotRun::from(e.to_string())),
    };

    let (records, _full_text) = load_records(&repo_root, &discovered.paths, &config);
    if records.is_empty() {
        return emit(&CouldNotRun::from(
            "no records discovered -- nothing to check",
        ));
    }

    let would_fail = urzua_core::migrate::schema_report(&records, &field);

    emit(&MigrateSchemaReport {
        status: ReportStatus::Ok,
        records_examined: records.len(),
        field,
        would_fail,
        notices: Vec::new(),
    })
}
