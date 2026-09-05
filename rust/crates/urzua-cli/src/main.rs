//! The `urzua` binary.
//!
//! Command surface per SPEC-0001. `check` is implemented (Phase A, SPEC-0002);
//! everything else still bails with "not implemented yet."
//!
//! Implements: SPEC-0001, SPEC-0002, SPEC-0003

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use urzua_core::config::Config;
use urzua_core::record::Record;
use urzua_core::report::{CheckReport, ReportStatus, ScopeInfo};
use urzua_core::rules;

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
    },

    /// Validate records. Non-zero exit on error.
    ///
    /// Reports files examined and rules executed: a check that found nothing
    /// must be distinguishable from a check that ran on nothing.
    Check {
        paths: Vec<PathBuf>,
        #[arg(long, value_enum, default_value = "human")]
        format: OutputFormat,
    },

    /// Cross-record reconciliation: supersession reciprocity, embodiment
    /// back-pointers, dangling references.
    ///
    /// Reports only. Never writes -- bulk cross-reference rewriting is a real
    /// data-loss risk without one, and `--fix`
    /// waits for a revision log to make repair recoverable.
    Audit {
        #[arg(long, value_enum, default_value = "human")]
        format: OutputFormat,
    },

    /// Backfill stable IDs into an existing sequentially-numbered corpus,
    /// retaining current numbers as display numbers. Dry-run by default.
    Migrate {
        #[arg(long)]
        apply: bool,
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

    /// Adopt an existing corpus: assign stable IDs, write `.urzua/`. Never
    /// clobbers, always idempotent, `--dry-run` byte-identical to the real run.
    Init {
        #[arg(long)]
        dry_run: bool,
    },

    /// Report on the tool's own configuration and invocation health: is
    /// there a config, does it parse, is it wired into a real gate. An
    /// unrecognized config key is an error here, not a warning.
    Doctor,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum ExportFormat {
    Agdr,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Check { paths, format } => run_check(cli.config, paths, format),
        Command::New { .. } => not_implemented("new"),
        Command::Audit { .. } => not_implemented("audit"),
        Command::Migrate { .. } => not_implemented("migrate"),
        Command::Export { .. } => not_implemented("export"),
        Command::Import { .. } => not_implemented("import"),
        Command::Init { dry_run } => run_init(dry_run),
        Command::Doctor => run_doctor(),
    }
}

/// Exit code 2: could not run at all -- distinct from "ran and found
/// nothing," per SPEC-0001's exit-code contract.
fn not_implemented(name: &str) -> ExitCode {
    eprintln!("`urzua {name}` is not implemented yet -- see docs/specs/0001-v0-cli.md");
    ExitCode::from(2)
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
fn run_doctor() -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => {
            eprintln!("urzua doctor: could not run: {e}");
            return ExitCode::from(2);
        }
    };

    let mut problems = Vec::new();
    let config_path = repo_root.join(".urzua/config.toml");

    if !config_path.exists() {
        println!(
            "[MISSING] {} does not exist -- run `urzua init` to adopt this corpus.",
            config_path.display()
        );
        return ExitCode::from(2);
    }
    println!("[OK] {} exists", config_path.display());

    let config = match load_config(&config_path) {
        Ok(c) => {
            println!("[OK] config parses (no unrecognized keys)");
            c
        }
        Err(e) => {
            println!("[ERROR] config does not parse: {e}");
            return ExitCode::from(1);
        }
    };

    if config.record_types.is_empty() {
        problems.push("no record_types declared -- check will never examine anything".to_string());
    }

    for (name, cfg) in &config.record_types {
        let dir_path = repo_root.join(&cfg.dir);
        if dir_path.is_dir() {
            println!("[OK] record type '{name}' -> {} exists", cfg.dir);
        } else {
            problems.push(format!(
                "record type '{name}' declares dir '{}', which does not exist",
                cfg.dir
            ));
        }
        if cfg.required_fields.is_empty() {
            println!("[WARN] record type '{name}' has no required_fields -- field-quality/header rules will never fire for it");
        }
    }

    let ci_path = repo_root.join(".github/workflows/ci.yml");
    let ci_wired = std::fs::read_to_string(&ci_path)
        .map(|content| content.contains("urzua check") || content.contains("make records"))
        .unwrap_or(false);
    if ci_wired {
        println!("[OK] CI workflow invokes `urzua check` or `make records`");
    } else {
        println!(
            "[WARN] {} does not invoke `urzua check` or `make records` -- a check that exists but is never run reports nothing to anyone",
            ci_path.display()
        );
    }

    for problem in &problems {
        println!("[ERROR] {problem}");
    }

    if problems.is_empty() {
        ExitCode::from(0)
    } else {
        ExitCode::from(1)
    }
}

fn run_check(config_path: Option<PathBuf>, paths: Vec<PathBuf>, format: OutputFormat) -> ExitCode {
    let repo_root = match find_repo_root(&paths) {
        Ok(root) => root,
        Err(e) => return report_could_not_run(&e, format),
    };

    let config_path = config_path.unwrap_or_else(|| repo_root.join(".urzua/config.toml"));
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => return report_could_not_run(&e, format),
    };

    let discovered = match urzua_io::discover_tracked_files(&repo_root) {
        Ok(d) => d,
        Err(e) => return report_could_not_run(&e.to_string(), format),
    };

    let (records, full_text) = load_records(&repo_root, &discovered.paths, &config);

    let mut required_by_type = HashMap::new();
    for (name, cfg) in &config.record_types {
        required_by_type.insert(name.clone(), cfg.required_fields.clone());
    }

    let (exec1, findings1) = rules::header_required_fields(&records, &required_by_type);
    let (exec2, findings2) = rules::pointer_resolution(&records);
    let (exec3, findings3) = rules::field_quality(&records, &required_by_type);
    let (exec4, findings4) = rules::filename_title_consistency(&records, &full_text);
    let (exec5, findings5) = rules::supersession_reciprocity(&records);

    let mut findings = findings1;
    findings.extend(findings2);
    findings.extend(findings3);
    findings.extend(findings4);
    findings.extend(findings5);

    // A waiver is a record (ADR-0011), never a config-level ignore list.
    // Waived findings stay listed -- only excluded from blocking/status.
    let waivers = urzua_core::waiver::load_waivers(&records);
    urzua_core::waiver::apply_waivers(&mut findings, &waivers, &urzua_io::today());

    let active_findings = || findings.iter().filter(|f| f.waived.is_none());
    let blocking = active_findings().any(|f| f.severity == urzua_core::report::Severity::Error);

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
        rules_executed: vec![exec1, exec2, exec3, exec4, exec5],
        scope: ScopeInfo {
            source: format!("{:?}", discovered.source),
            record_types: config.record_types.keys().cloned().collect(),
        },
        blocking,
        findings,
    };

    print_report(&report, format);
    ExitCode::from(report.exit_code() as u8)
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
                    records.push(Record::parse(rel_path.clone(), type_name.clone(), &content));
                    full_text.insert(rel_path.clone(), content);
                }
                Err(_) => continue,
            }
        }
    }
    (records, full_text)
}

fn report_could_not_run(message: &str, format: OutputFormat) -> ExitCode {
    let report = CheckReport {
        status: ReportStatus::NotRun,
        files_examined: 0,
        rules_executed: vec![],
        scope: ScopeInfo {
            source: "unavailable".to_string(),
            record_types: vec![],
        },
        blocking: false,
        findings: vec![],
    };
    eprintln!("urzua check: could not run: {message}");
    print_report(&report, format);
    ExitCode::from(2)
}

fn print_report(report: &CheckReport, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(report).unwrap());
        }
        OutputFormat::Human => {
            println!(
                "status: {:?}  files_examined: {}  blocking: {}",
                report.status, report.files_examined, report.blocking
            );
            for exec in &report.rules_executed {
                println!(
                    "  rule {} examined {} record(s)",
                    exec.rule, exec.records_examined
                );
            }
            for finding in &report.findings {
                println!(
                    "  [{:?}] {}:{} {} -- {}",
                    finding.severity,
                    finding.file.display(),
                    finding.line.map(|l| l.to_string()).unwrap_or_default(),
                    finding.rule,
                    finding.message
                );
            }
        }
    }
}
