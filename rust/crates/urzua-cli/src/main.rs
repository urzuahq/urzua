//! The `urzua` binary.
//!
//! Command surface per SPEC-0001. `check`, `explain`, `graph`, `new`, `audit`,
//! `init` (adopt mode), `doctor`, `fix` (detect and apply), `migrate ids`, and
//! `migrate schema --report` are implemented; `migrate schema
//! --assist-waivers`/`--apply`, `export`, and `import` still bail with "not
//! implemented yet." Stdout is always JSON, on every implemented command --
//! no `--format` flag exists (ADR-0023).
//!
//! Implements: SPEC-0001, SPEC-0002, SPEC-0003

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{Parser, Subcommand};
use urzua_core::config::Config;
use urzua_core::record::Record;
use urzua_core::report::{
    CheckReport, CouldNotRun, ExplainReport, Finding, FixFailure, FixReport, FixStatus,
    GraphReport, MigrateSchemaReport, NewReport, Notice, NoticeSeverity, Report, ReportStatus,
    RuleExecution, ScopeInfo,
};
use urzua_core::rules;

/// `Notice.subject` constants, one per emitter, matching `Finding.rule`'s
/// existing `const RULE_ID` convention (ADR-0046).
const NOTICE_IDENTITY: &str = "identity";

/// The one place every command prints (ADR-0046) -- printing is I/O, so this
/// lives here, not in `urzua-core::report` (which stays pure and defines
/// only the data shapes `Report` describes). Not a generic envelope around
/// the payload -- each `Report` stays its own real, fully-typed struct; this
/// just gives every one of them the same last step.
fn emit<T: Report>(report: &T) -> ExitCode {
    println!("{}", serde_json::to_string_pretty(report).unwrap());
    report.exit_code()
}

mod init;

#[derive(Parser)]
#[command(
    name = "urzua",
    about = "The decision layer for engineering orgs running AI agents",
    version
)]
struct Cli {
    /// Path to .urzua/config.toml. Defaults to the nearest one at or above the cwd.
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a record with a stable ID assigned. Never asks you to pick a number.
    New {
        /// Record type, as configured in .urzua/config.toml (e.g. adr, rfc, spec).
        record_type: String,
        title: Option<String>,
        #[arg(long)]
        by: Option<String>,
    },

    /// Validate records. Non-zero exit on error.
    ///
    /// Reports files examined and rules executed: a check that found nothing
    /// must be distinguishable from a check that ran on nothing. Stdout is
    /// always the JSON report (ADR-0023) -- an agent piping it never has to
    /// know to ask.
    Check { paths: Vec<PathBuf> },

    /// Which records govern `path` -- every record whose `Realized-by`
    /// names it as evidence (ADR-0024).
    Explain { path: String },

    /// The full record-relationship graph: every `Implements`/
    /// `Derives-from`/`Supersedes` edge, as data (ADR-0024).
    Graph,

    /// Cross-record reconciliation: supersession reciprocity, dangling
    /// references. Embodiment consistency lives in `check` and `fix`
    /// instead -- it turned out to be per-record, not cross-record
    /// (ADR-0018/0019).
    ///
    /// Reports only. Never writes -- bulk cross-reference rewriting is a
    /// real data-loss risk without one.
    Audit,

    /// Migrate the corpus itself: identifiers, or field-schema rollout.
    Migrate {
        #[command(subcommand)]
        target: MigrateTarget,
    },

    /// Emit records in another format. Warns rather than silently dropping fields.
    Export {
        #[arg(long, value_enum, default_value = "agdr")]
        format: ExportFormat,
    },

    /// Read records from another format.
    Import {
        #[arg(long, value_enum, default_value = "agdr")]
        format: ExportFormat,
        paths: Vec<PathBuf>,
    },

    /// Adopt an existing corpus: propose and write `.urzua/config.toml` from
    /// what's already there. Never clobbers, always idempotent, `--dry-run`
    /// byte-identical to the real run. Stable-ID backfill is a separate
    /// step: `migrate ids`.
    Init {
        #[arg(long)]
        dry_run: bool,
    },

    /// Report on the tool's own configuration and invocation health: is
    /// there a config, does it parse, is it wired into a real gate. An
    /// unrecognized config key is an error here, not a warning.
    Doctor,

    /// Detect fields whose stated value disagrees with what the tool
    /// computes (ADR-0015). Read-only by default. Only Tier 1 (Embodiment)
    /// exists.
    Fix {
        #[arg(long, default_value_t = 1)]
        tier: u8,
        /// Write the computed values back. Requires --ids or --force, and
        /// an identity (--by, or resolved from gh/git config). Every write
        /// appends a structural revision-log entry (ADR-0014) -- a record
        /// with no Revision log section is refused, never silently skipped.
        #[arg(long)]
        apply: bool,
        #[arg(long, value_delimiter = ',')]
        ids: Vec<String>,
        #[arg(long)]
        by: Option<String>,
        /// Apply to every detected repair, bypassing --ids. Never the
        /// default.
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum MigrateTarget {
    /// Backfill stable IDs (ADR-0003/0021) into every record lacking one.
    /// Retains the filename's current number as the display number
    /// unchanged -- cross-references keep resolving by filename exactly as
    /// they do today. Dry-run by default.
    Ids {
        #[arg(long)]
        apply: bool,
    },

    /// Field-rollout assistant for a newly-required field. Only --report
    /// exists so far: a read-only preview of which existing records would
    /// newly fail, before the field is ever added to config.
    /// --assist-waivers and --apply are not implemented yet.
    Schema {
        /// Preview which records lack a real value for --field, without
        /// requiring it in config first.
        #[arg(long)]
        report: bool,
        #[arg(long)]
        field: Option<String>,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum ExportFormat {
    Agdr,
}

/// `--help`/`--version` surface as `Err(_)` too (`ErrorKind::DisplayHelp`/
/// `DisplayVersion`), not a separate success path -- print as clap already
/// rendered them and exit 0, the one stated plain-text exception (matching
/// `cargo`'s own convention: help/version are never gated behind a
/// machine-format flag). Every other parse error goes through the same
/// `emit()`/`CouldNotRun` path as any other fatal command error.
fn main() -> ExitCode {
    std::panic::set_hook(Box::new(|info| {
        use std::io::Write;
        let out = serde_json::json!({ "status": "not-run", "panic": info.to_string() });
        // write_all + let-ignore, never println!/a panicking macro here -- a
        // broken pipe during the hook itself would double-panic and abort
        // with no output at all, defeating the hook's entire purpose.
        let _ = std::io::stdout().write_all(out.to_string().as_bytes());
        let _ = std::io::stdout().write_all(b"\n");
    }));

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) => {
            e.exit()
        }
        Err(e) => return emit(&CouldNotRun::from(e.to_string())),
    };

    match cli.command {
        Command::Check { paths } => run_check(cli.config, paths),
        Command::Explain { path } => run_explain(cli.config, path),
        Command::Graph => run_graph(cli.config),
        Command::New {
            record_type,
            title,
            by,
        } => run_new(cli.config, record_type, title, by),
        Command::Audit => run_audit(cli.config),
        Command::Migrate {
            target: MigrateTarget::Ids { apply },
        } => run_migrate_ids(cli.config, apply),
        Command::Migrate {
            target:
                MigrateTarget::Schema {
                    report: true,
                    field: Some(field),
                },
        } => run_migrate_schema_report(cli.config, field),
        Command::Migrate {
            target: MigrateTarget::Schema { .. },
        } => emit(&CouldNotRun::from(
            "pass --report --field <Name> (--assist-waivers and --apply are not implemented yet)",
        )),
        Command::Export { .. } => not_implemented("export"),
        Command::Import { .. } => not_implemented("import"),
        Command::Init { dry_run } => run_init(dry_run),
        Command::Doctor => run_doctor(),
        Command::Fix {
            tier,
            apply,
            ids,
            by,
            force,
        } => run_fix(cli.config, tier, apply, ids, by, force),
    }
}

/// Exit code 2: could not run at all -- distinct from "ran and found
/// nothing," per SPEC-0001's exit-code contract.
fn not_implemented(name: &str) -> ExitCode {
    emit(&CouldNotRun::from(format!(
        "`urzua {name}` is not implemented yet -- see docs/specs/0001-v0-cli.md"
    )))
}

/// Adopt mode only (SPEC-0005). Never clobbers an existing config; `--dry-run`
/// produces byte-identical output to the real run, differing only in whether
/// the file is actually written.
fn run_init(dry_run: bool) -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => {
            eprintln!("urzua init: could not run: {e}");
            return ExitCode::from(2);
        }
    };

    let config_path = repo_root.join(".urzua/config.toml");
    if config_path.exists() {
        println!(
            "urzua init: {} already exists -- refusing to overwrite. \
             Edit it directly, or remove it to re-run adopt.",
            config_path.display()
        );
        return ExitCode::from(2);
    }

    let discovered = match urzua_io::discover_tracked_files(&repo_root) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("urzua init: could not run: {e}");
            return ExitCode::from(2);
        }
    };

    let proposed = init::detect_record_types(&repo_root, &discovered.paths);
    if proposed.is_empty() {
        println!("urzua init: no record-shaped files found under docs/ -- nothing to adopt.");
        return ExitCode::from(2);
    }

    let rendered = init::render_config_toml(&proposed);

    println!(
        "urzua init: adopt mode -- proposing {} record type(s):",
        proposed.len()
    );
    for rt in &proposed {
        println!(
            "  {} ({}): {} record(s) found",
            rt.name, rt.dir, rt.record_count
        );
    }

    if dry_run {
        println!(
            "\n--dry-run: would write {}:\n\n{rendered}",
            config_path.display()
        );
        return ExitCode::from(0);
    }

    if let Err(e) = std::fs::create_dir_all(config_path.parent().unwrap()) {
        eprintln!("urzua init: could not create .urzua/: {e}");
        return ExitCode::from(2);
    }
    if let Err(e) = std::fs::write(&config_path, &rendered) {
        eprintln!("urzua init: could not write {}: {e}", config_path.display());
        return ExitCode::from(2);
    }

    println!(
        "\nWrote {}. Nothing else was moved or modified.",
        config_path.display()
    );
    ExitCode::from(0)
}

/// Reports on the tool's own configuration and invocation health, not on
/// record content -- `check` validates records; whether *itself* is
/// correctly invoked is a different question with a different failure mode
/// (SPEC-0001's own open question). An unrecognized config key is an error
/// here, not a warning: the alternative is a typo silently disabling a rule.
#[derive(serde::Serialize)]
struct DoctorCheck {
    check: String,
    status: DoctorStatus,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum DoctorStatus {
    Ok,
    Warn,
    Error,
}

#[derive(serde::Serialize)]
struct DoctorReport {
    status: DoctorStatus,
    checks: Vec<DoctorCheck>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
    /// Not serialized -- `run_doctor` has three real exit codes (2 = config
    /// missing, 1 = config parse error or any check `Error`, 0 = otherwise),
    /// but `status` alone can't distinguish the first two (both are
    /// `DoctorStatus::Error`). Set explicitly by whichever branch builds the
    /// report, so `exit_code()` never has to (mis)derive it from `status`.
    #[serde(skip)]
    exit_code: u8,
}

impl Report for DoctorReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.exit_code)
    }
}

/// Emits one report, JSON, unconditionally (ADR-23) -- BUG-4 was `doctor`
/// printing `[OK]`/`[WARN]`/`[ERROR]` lines instead, the one command that
/// hadn't caught up with every other command's output contract.
fn run_doctor() -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let mut checks = Vec::new();
    let config_path = repo_root.join(".urzua/config.toml");

    if !config_path.exists() {
        checks.push(DoctorCheck {
            check: "config-exists".to_string(),
            status: DoctorStatus::Error,
            message: format!(
                "{} does not exist -- run `urzua init` to adopt this corpus",
                config_path.display()
            ),
        });
        return emit(&DoctorReport {
            status: DoctorStatus::Error,
            checks,
            notices: Vec::new(),
            exit_code: 2,
        });
    }
    checks.push(DoctorCheck {
        check: "config-exists".to_string(),
        status: DoctorStatus::Ok,
        message: format!("{} exists", config_path.display()),
    });

    let config = match load_config(&config_path) {
        Ok(c) => {
            checks.push(DoctorCheck {
                check: "config-parses".to_string(),
                status: DoctorStatus::Ok,
                message: "config parses (no unrecognized keys)".to_string(),
            });
            c
        }
        Err(e) => {
            checks.push(DoctorCheck {
                check: "config-parses".to_string(),
                status: DoctorStatus::Error,
                message: format!("config does not parse: {e}"),
            });
            return emit(&DoctorReport {
                status: DoctorStatus::Error,
                checks,
                notices: Vec::new(),
                exit_code: 1,
            });
        }
    };

    if config.record_types.is_empty() {
        checks.push(DoctorCheck {
            check: "record-types-declared".to_string(),
            status: DoctorStatus::Error,
            message: "no record_types declared -- check will never examine anything".to_string(),
        });
    }

    for (name, cfg) in &config.record_types {
        let dir_path = repo_root.join(&cfg.dir);
        if dir_path.is_dir() {
            checks.push(DoctorCheck {
                check: "record-type-dir".to_string(),
                status: DoctorStatus::Ok,
                message: format!("record type '{name}' -> {} exists", cfg.dir),
            });
        } else {
            checks.push(DoctorCheck {
                check: "record-type-dir".to_string(),
                status: DoctorStatus::Error,
                message: format!(
                    "record type '{name}' declares dir '{}', which does not exist",
                    cfg.dir
                ),
            });
        }
        if cfg.required_fields.is_empty() {
            checks.push(DoctorCheck {
                check: "record-type-required-fields".to_string(),
                status: DoctorStatus::Warn,
                message: format!(
                    "record type '{name}' has no required_fields -- field-quality/header rules will never fire for it"
                ),
            });
        }
    }

    let ci_path = repo_root.join(".github/workflows/ci.yml");
    let ci_wired = std::fs::read_to_string(&ci_path)
        .map(|content| content.contains("urzua check") || content.contains("make records"))
        .unwrap_or(false);
    if ci_wired {
        checks.push(DoctorCheck {
            check: "ci-wired".to_string(),
            status: DoctorStatus::Ok,
            message: "CI workflow invokes `urzua check` or `make records`".to_string(),
        });
    } else {
        checks.push(DoctorCheck {
            check: "ci-wired".to_string(),
            status: DoctorStatus::Warn,
            message: format!(
                "{} does not invoke `urzua check` or `make records` -- a check that exists but is never run reports nothing to anyone",
                ci_path.display()
            ),
        });
    }

    let has_error = checks.iter().any(|c| c.status == DoctorStatus::Error);
    let has_warn = checks.iter().any(|c| c.status == DoctorStatus::Warn);
    let status = if has_error {
        DoctorStatus::Error
    } else if has_warn {
        DoctorStatus::Warn
    } else {
        DoctorStatus::Ok
    };

    emit(&DoctorReport {
        status,
        checks,
        notices: Vec::new(),
        exit_code: if has_error { 1 } else { 0 },
    })
}

fn run_check(config_path: Option<PathBuf>, paths: Vec<PathBuf>) -> ExitCode {
    let repo_root = match find_repo_root(&paths) {
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

    let scoped = match scope_to_requested_paths(&repo_root, &discovered.paths, &paths) {
        Ok(p) => p,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let (records, full_text) = load_records(&repo_root, &scoped, &config);

    let mut required_by_type = HashMap::new();
    let mut header_layout_by_type = HashMap::new();
    let mut known_fields_by_type = HashMap::new();
    let mut pointer_fields_by_type = HashMap::new();
    let mut narrative_fields_by_type = HashMap::new();
    for (name, cfg) in &config.record_types {
        required_by_type.insert(name.clone(), cfg.required_fields.clone());
        if let Some(layout) = cfg.header_layout {
            header_layout_by_type.insert(name.clone(), layout);
        }
        if let Some(known) = &cfg.known_fields {
            let allowed: std::collections::HashSet<String> = cfg
                .required_fields
                .iter()
                .chain(known.iter())
                .map(|f| f.to_ascii_lowercase())
                .collect();
            known_fields_by_type.insert(name.clone(), allowed);
        }
        if let Some(pointer_fields) = &cfg.pointer_fields {
            pointer_fields_by_type.insert(name.clone(), pointer_fields.clone());
        }
        if let Some(narrative_fields) = &cfg.narrative_fields {
            narrative_fields_by_type.insert(name.clone(), narrative_fields.clone());
        }
    }

    let drifted = compute_drifted_records(&repo_root, &records);

    // Each rule's (RuleExecution, Vec<Finding>) collected into one list --
    // rules_executed/findings both derive from it below, so there's no
    // second hand-written list that has to be kept in sync by hand.
    let rule_results: Vec<(RuleExecution, Vec<Finding>)> = vec![
        rules::header_required_fields(&records, &required_by_type),
        rules::pointer_resolution(&records, &pointer_fields_by_type, &narrative_fields_by_type),
        rules::field_quality(&records, &required_by_type),
        rules::filename_title_consistency(&records, &full_text),
        rules::supersession_reciprocity(&records),
        rules::revision_log_change_class(&records, &full_text),
        rules::embodiment_consistency(&records, &drifted),
        rules::embodiment_locator_promotion_candidate(&records),
        rules::header_layout_consistency(&records, &header_layout_by_type),
        rules::header_field_set_consistency(&records, &known_fields_by_type),
        rules::narrative_field_stale(&records, &config),
        rules::type_no_declared_spec(&config, &config_path),
        rules::header_deprecated_shape(&config, &config_path),
        rules::header_pointer_field_clean(&records, &config),
        rules::config_pointer_declaration_missing(&config, &config_path),
        rules::config_pointer_field_not_known(&config, &config_path),
        rules::config_pointer_narrative_overlap(&config, &config_path),
    ];

    let mut rules_executed = Vec::with_capacity(rule_results.len());
    let mut findings = Vec::new();
    for (exec, rule_findings) in rule_results {
        rules_executed.push(exec);
        findings.extend(rule_findings);
    }

    // A waiver is a record (ADR-0011), never a config-level ignore list.
    // Waived findings stay listed -- only excluded from blocking/status.
    let waivers = urzua_core::waiver::load_waivers(&records);
    urzua_core::waiver::apply_waivers(&mut findings, &waivers, &urzua_io::today());

    let active_findings = || findings.iter().filter(|f| f.waived.is_none());
    let blocking =
        active_findings().any(|f| f.severity == urzua_core::report::FindingSeverity::Error);

    let status = if records.is_empty() {
        ReportStatus::NotRun
    } else if active_findings().count() == 0 {
        ReportStatus::Ok
    } else {
        ReportStatus::FindingsPresent
    };

    let report = CheckReport {
        status,
        files_examined: records.len(),
        rules_executed,
        scope: ScopeInfo {
            source: format!("{:?}", discovered.source),
            record_types: config.record_types.keys().cloned().collect(),
        },
        blocking,
        findings,
        notices: Vec::new(),
    };

    emit(&report)
}

/// Cross-record reconciliation: supersession reciprocity and dangling
/// cross-references, reusing the same rule functions `check` calls rather
/// than a second implementation (ADR-0030). Never writes -- a bulk
/// cross-reference rewrite is a real data-loss risk without a review step
/// this command doesn't have.
fn run_audit(config_path: Option<PathBuf>) -> ExitCode {
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

    let mut pointer_fields_by_type = HashMap::new();
    let mut narrative_fields_by_type = HashMap::new();
    for (name, cfg) in &config.record_types {
        if let Some(pointer_fields) = &cfg.pointer_fields {
            pointer_fields_by_type.insert(name.clone(), pointer_fields.clone());
        }
        if let Some(narrative_fields) = &cfg.narrative_fields {
            narrative_fields_by_type.insert(name.clone(), narrative_fields.clone());
        }
    }

    let (exec1, findings1) =
        rules::pointer_resolution(&records, &pointer_fields_by_type, &narrative_fields_by_type);
    let (exec2, findings2) = rules::supersession_reciprocity(&records);
    let mut findings = findings1;
    findings.extend(findings2);

    let waivers = urzua_core::waiver::load_waivers(&records);
    urzua_core::waiver::apply_waivers(&mut findings, &waivers, &urzua_io::today());

    let active_findings = || findings.iter().filter(|f| f.waived.is_none());
    let blocking =
        active_findings().any(|f| f.severity == urzua_core::report::FindingSeverity::Error);

    let status = if records.is_empty() {
        ReportStatus::NotRun
    } else if active_findings().count() == 0 {
        ReportStatus::Ok
    } else {
        ReportStatus::FindingsPresent
    };

    let report = CheckReport {
        status,
        files_examined: records.len(),
        rules_executed: vec![exec1, exec2],
        scope: ScopeInfo {
            source: format!("{:?}", discovered.source),
            record_types: config.record_types.keys().cloned().collect(),
        },
        blocking,
        findings,
        notices: Vec::new(),
    };

    emit(&report)
}

/// "Which decisions govern this file" (ADR-0024) -- every record whose
/// `Realized-by` names `path` as evidence. Stdout is always JSON (ADR-0023).
fn run_explain(config_path: Option<PathBuf>, path: String) -> ExitCode {
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
    let governing_records = urzua_core::graph::explain(&records, &path);

    emit(&ExplainReport {
        status: ReportStatus::Ok,
        path,
        governing_records,
        notices: Vec::new(),
    })
}

/// The full record-relationship graph, as data (ADR-0024). Stdout is always
/// JSON (ADR-0023).
fn run_graph(config_path: Option<PathBuf>) -> ExitCode {
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

    let mut pointer_fields_by_type = HashMap::new();
    let mut narrative_fields_by_type = HashMap::new();
    for (name, cfg) in &config.record_types {
        if let Some(pointer_fields) = &cfg.pointer_fields {
            pointer_fields_by_type.insert(name.clone(), pointer_fields.clone());
        }
        if let Some(narrative_fields) = &cfg.narrative_fields {
            narrative_fields_by_type.insert(name.clone(), narrative_fields.clone());
        }
    }

    let edges =
        urzua_core::graph::graph(&records, &pointer_fields_by_type, &narrative_fields_by_type);

    emit(&GraphReport {
        status: ReportStatus::Ok,
        edges,
        notices: Vec::new(),
    })
}

/// Creates a record from the configured template with a stable ID assigned
/// (ADR-0003). Never asks the author to pick a number -- scans the type's
/// directory for the highest existing prefix and takes the next one.
fn run_new(
    config_path: Option<PathBuf>,
    record_type: String,
    title: Option<String>,
    by: Option<String>,
) -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let config_path = config_path.unwrap_or_else(|| repo_root.join(".urzua/config.toml"));
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let Some(type_config) = config.record_types.get(&record_type) else {
        return emit(&CouldNotRun::from(format!(
            "unrecognized record type '{record_type}' -- see [record_types] in {}",
            config_path.display()
        )));
    };

    let dir = repo_root.join(&type_config.dir);
    let filenames: Vec<String> = match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect(),
        Err(e) => {
            return emit(&CouldNotRun::from(format!(
                "could not read {}: {e}",
                dir.display()
            )))
        }
    };
    let display_number = urzua_core::new_record::next_display_number(&filenames);

    let title = title.unwrap_or_else(|| "Title".to_string());
    let stable_id = urzua_id::StableId::generate();
    let today = urzua_io::today();
    let identity = match urzua_io::resolve_identity(by.as_deref(), &repo_root) {
        Ok(id) => id,
        Err(e) => {
            return emit(&CouldNotRun::from(format!(
                "could not resolve an identity: {e}"
            )))
        }
    };

    let params = urzua_core::new_record::NewRecordParams {
        display_number,
        title: &title,
        stable_id: stable_id.as_str(),
        author: &identity.name,
        today: &today,
    };

    // BUG-3: the configured header_shape wins unconditionally -- a template
    // file's own (possibly stale, possibly blockquote) header shape is never
    // allowed to override what the config actually declares. A yaml-
    // frontmatter type still gets a template's body scaffolding (`## What`,
    // the revision log) when one exists; it just doesn't get the template's
    // header.
    let template_path = repo_root
        .join(".urzua/templates")
        .join(format!("{record_type}.md"));
    let content = if type_config.header_shape == urzua_core::header::HeaderShape::YamlFrontmatter {
        let mut out =
            urzua_core::new_record::render_synthetic_yaml(&params, &type_config.required_fields);
        if let Ok(template) = urzua_io::read_to_string(&template_path) {
            if let Some(body) = urzua_core::new_record::template_body(&template) {
                out.push('\n');
                out.push_str(body);
            }
        }
        out
    } else {
        match urzua_io::read_to_string(&template_path) {
            Ok(template) => urzua_core::new_record::render_from_template(&template, &params),
            Err(e) => {
                return emit(&CouldNotRun::from(format!(
                    "no template at {} and no synthesizable shape declared: {e}",
                    template_path.display()
                )))
            }
        }
    };

    // Type prefix explicit in the filename (ADR-36) -- matches how a record
    // is referenced in prose everywhere else (e.g. "Implements: ADR-36").
    // Never zero-padded (ADR-36 amendment): padding doesn't solve
    // lexicographic sort order permanently, only defers the break. The
    // prefix is the type's configured `prefix` when it declares one (e.g.
    // `MILE` for `milestone`), otherwise the type name itself, upper-cased.
    let type_prefix = type_config
        .prefix
        .clone()
        .unwrap_or_else(|| record_type.to_ascii_uppercase());
    let filename = format!(
        "{}-{}-{}.md",
        type_prefix,
        display_number,
        urzua_core::new_record::slugify(&title)
    );
    let file_path = dir.join(&filename);
    if file_path.exists() {
        return emit(&CouldNotRun::from(format!(
            "{} already exists -- refusing to overwrite",
            file_path.display()
        )));
    }

    if let Err(e) = std::fs::write(&file_path, content) {
        return emit(&CouldNotRun::from(format!(
            "could not write {}: {e}",
            file_path.display()
        )));
    }

    let notices = identity
        .warning
        .into_iter()
        .map(|message| Notice {
            severity: NoticeSeverity::Warning,
            subject: NOTICE_IDENTITY.to_string(),
            message,
        })
        .collect();

    emit(&NewReport {
        status: ReportStatus::Ok,
        path: file_path
            .strip_prefix(&repo_root)
            .unwrap_or(&file_path)
            .to_path_buf(),
        display_number,
        stable_id: stable_id.as_str().to_string(),
        notices,
    })
}

/// Detect mode only (ADR-0015 §3): read-only, safe in CI. Apply mode
/// (`--ids`, `--by`, `--force`) is not built yet -- it needs real file
/// mutation, identity resolution, and a revision-log write-path, none of
/// which this command touches.
fn run_fix(
    config_path: Option<PathBuf>,
    tier: u8,
    apply: bool,
    ids: Vec<String>,
    by: Option<String>,
    force: bool,
) -> ExitCode {
    if tier != 1 {
        return emit(&CouldNotRun::from(
            "only tier 1 is implemented so far -- tiers 2 and 3 are not yet built",
        ));
    }
    if apply && ids.is_empty() && !force {
        return emit(&CouldNotRun::from(
            "urzua fix --apply: pass --ids <record,...> or --force (never the default)",
        ));
    }

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
    let (examined, repairs) = urzua_core::fix::detect_repairs(&records);

    if !apply {
        return emit(&build_fix_report(examined, repairs, Vec::new(), None));
    }

    let identity = match urzua_io::resolve_identity(by.as_deref(), &repo_root) {
        Ok(id) => id,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };
    let _lock = match urzua_io::FixLock::acquire(&repo_root) {
        Ok(lock) => lock,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let today = urzua_io::today();
    let selected: Vec<_> = repairs
        .into_iter()
        .filter(|r| force || ids.contains(&r.record.display().to_string()))
        .collect();

    let mut applied = Vec::new();
    let mut failed = Vec::new();
    for repair in selected {
        let full_path = repo_root.join(&repair.record);
        // Re-read from disk right before writing (RFC-0008 §4's
        // re-verify-before-write): apply_repair itself fails if the current
        // value it was told to replace is no longer there, which is exactly
        // the case a concurrent edit since detect ran would produce.
        let fresh_content = match urzua_io::read_to_string(&full_path) {
            Ok(c) => c,
            Err(e) => {
                failed.push((repair.record.clone(), format!("could not re-read: {e}")));
                continue;
            }
        };
        match urzua_core::fix::apply_repair(&fresh_content, &repair, &today, &identity.name) {
            Ok(new_content) => match std::fs::write(&full_path, new_content) {
                Ok(()) => applied.push(repair),
                Err(e) => failed.push((repair.record.clone(), format!("could not write: {e}"))),
            },
            Err(e) => failed.push((repair.record.clone(), e)),
        }
    }

    emit(&build_fix_report(
        examined,
        applied,
        failed,
        identity.warning,
    ))
}

fn build_fix_report(
    examined: usize,
    repairs: Vec<urzua_core::fix::Repair>,
    failed: Vec<(PathBuf, String)>,
    warning: Option<String>,
) -> FixReport {
    let status = if examined == 0 {
        FixStatus::NotRun
    } else if !failed.is_empty() {
        FixStatus::PartialFailure
    } else if repairs.is_empty() {
        FixStatus::Ok
    } else {
        FixStatus::RepairsAvailable
    };

    FixReport {
        status,
        records_examined: examined,
        repairs,
        failed: failed
            .into_iter()
            .map(|(record, error)| FixFailure { record, error })
            .collect(),
        notices: warning
            .into_iter()
            .map(|message| Notice {
                severity: NoticeSeverity::Warning,
                subject: NOTICE_IDENTITY.to_string(),
                message,
            })
            .collect(),
    }
}

/// Backfills a `Stable-Id` header field (ADR-0003/0021) into every record
/// lacking one. Dry-run by default. Never touches the filename or any
/// cross-reference -- the display number stays exactly what it already is.
fn run_migrate_ids(config_path: Option<PathBuf>, apply: bool) -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => {
            eprintln!("urzua migrate ids: could not run: {e}");
            return ExitCode::from(2);
        }
    };

    let config_path = config_path.unwrap_or_else(|| repo_root.join(".urzua/config.toml"));
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("urzua migrate ids: could not run: {e}");
            return ExitCode::from(2);
        }
    };

    let discovered = match urzua_io::discover_tracked_files(&repo_root) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("urzua migrate ids: could not run: {e}");
            return ExitCode::from(2);
        }
    };

    let (records, full_text) = load_records(&repo_root, &discovered.paths, &config);
    let missing: Vec<_> = records
        .iter()
        .filter(|r| r.header.get("Stable-Id").is_none())
        .collect();

    if missing.is_empty() {
        println!("urzua migrate ids: every record already has a Stable-Id. Nothing to backfill.");
        return ExitCode::from(0);
    }

    println!(
        "urzua migrate ids: {} record(s) missing a Stable-Id:",
        missing.len()
    );
    for r in &missing {
        println!("  {}", r.path.display());
    }

    if !apply {
        println!("\n--dry-run (default): pass --apply to write.");
        return ExitCode::from(0);
    }

    println!();
    for r in &missing {
        let Some(region) = r.header.region else {
            println!(
                "  [SKIPPED] {}: no header-shaped region found",
                r.path.display()
            );
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
            Ok(()) => println!("  [OK] {}: Stable-Id: {}", r.path.display(), id.as_str()),
            Err(e) => println!("  [FAILED] {}: {e}", r.path.display()),
        }
    }

    ExitCode::from(0)
}

/// A preview of which existing records would newly fail if `field` were
/// added to config's `required_fields` today. Read-only -- never writes,
/// never touches config. `--assist-waivers` and `--apply` are not
/// implemented (SPEC-0001): the former needs a human-authored `Reason` per
/// waiver, the latter needs `urzua fix` to know a field is tool-writable,
/// which almost none are.
fn run_migrate_schema_report(config_path: Option<PathBuf>, field: String) -> ExitCode {
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

/// Restrict discovered files to those under any of the requested paths,
/// relative to `repo_root`. Empty `requested` means no restriction -- the
/// existing "check everything" behavior when no path argument is given.
///
/// BUG-0001: `check`'s `paths` argument was previously used only to locate
/// the repo root, never to filter which files were actually examined --
/// `check docs/adr/` and `check docs/` returned identical results. Caught
/// by hand while exercising a genuinely narrower path for the first time;
/// no existing test used anything but `check docs/`, so nothing exercised
/// the narrower case at all.
fn scope_to_requested_paths(
    repo_root: &std::path::Path,
    discovered: &[PathBuf],
    requested: &[PathBuf],
) -> Result<Vec<PathBuf>, String> {
    if requested.is_empty() {
        return Ok(discovered.to_vec());
    }

    let mut relative_scopes = Vec::new();
    for p in requested {
        let canonical = p
            .canonicalize()
            .map_err(|e| format!("could not resolve {}: {e}", p.display()))?;
        let relative = canonical
            .strip_prefix(repo_root)
            .map_err(|_| format!("{} is outside the repository", p.display()))?
            .to_path_buf();
        relative_scopes.push(relative);
    }

    Ok(discovered
        .iter()
        .filter(|d| relative_scopes.iter().any(|s| d.starts_with(s)))
        .cloned()
        .collect())
}

fn find_repo_root(paths: &[PathBuf]) -> Result<PathBuf, String> {
    let start = paths.first().cloned().unwrap_or_else(|| PathBuf::from("."));
    let mut dir = start
        .canonicalize()
        .map_err(|e| format!("could not resolve {}: {e}", start.display()))?;
    if dir.is_file() {
        dir = dir
            .parent()
            .ok_or_else(|| "path has no parent directory".to_string())?
            .to_path_buf();
    }
    loop {
        if dir.join(".git").exists() {
            return Ok(dir);
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return Err("no .git directory found above the given path".to_string()),
        }
    }
}

fn load_config(path: &PathBuf) -> Result<Config, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read config at {}: {e}", path.display()))?;
    urzua_core::config::parse(&content).map_err(|e| e.to_string())
}

fn load_records(
    repo_root: &std::path::Path,
    discovered: &[PathBuf],
    config: &Config,
) -> (Vec<Record>, HashMap<PathBuf, String>) {
    let mut records = Vec::new();
    let mut full_text = HashMap::new();
    for (type_name, type_config) in &config.record_types {
        let type_dir = PathBuf::from(&type_config.dir);
        for rel_path in discovered {
            if !rel_path.starts_with(&type_dir) {
                continue;
            }
            let Some(file_name) = rel_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if file_name.starts_with('_') || !file_name.ends_with(".md") {
                continue;
            }
            let full_path = repo_root.join(rel_path);
            match urzua_io::read_to_string(&full_path) {
                Ok(content) => {
                    let type_prefix = type_config
                        .prefix
                        .clone()
                        .unwrap_or_else(|| type_name.to_ascii_uppercase());
                    records.push(Record::parse_with_shape_and_prefix(
                        rel_path.clone(),
                        type_name.clone(),
                        &content,
                        type_config.header_shape,
                        type_prefix,
                    ));
                    full_text.insert(rel_path.clone(), content);
                }
                Err(_) => continue,
            }
        }
    }
    (records, full_text)
}

/// Which records have at least one `Realized-by` locator that changed, per
/// git history, since the `Realized-by` line was last touched (ADR-0032).
/// The one piece of I/O `embodiment_consistency` needs but can't do itself
/// -- computed here and handed in as plain data, same shape as `full_text`.
fn compute_drifted_records(
    repo_root: &std::path::Path,
    records: &[Record],
) -> std::collections::HashSet<PathBuf> {
    let mut drifted = std::collections::HashSet::new();

    for record in records {
        let Some(realized_by_value) = record.header.get("Realized-by") else {
            continue;
        };
        let Some(field) = record
            .header
            .fields
            .iter()
            .find(|f| f.key.eq_ignore_ascii_case("Realized-by"))
        else {
            continue;
        };
        let Ok(Some(reference_commit)) =
            urzua_io::commit_for_line(repo_root, &record.path, field.line)
        else {
            continue;
        };

        for locator in rules::realized_by_locator_paths(realized_by_value) {
            let locator_path = PathBuf::from(&locator);
            let Ok(Some(locator_commit)) = urzua_io::last_commit_for_path(repo_root, &locator_path)
            else {
                continue;
            };
            if urzua_io::commit_strictly_before(repo_root, &reference_commit, &locator_commit) {
                drifted.insert(record.path.clone());
                break;
            }
        }
    }

    drifted
}
