//! The `urzua` binary.
//!
//! Command surface per SPEC-0001. `check`, `explain`, `graph`, `init`
//! (adopt mode), `doctor`, `fix` (detect and apply), `migrate ids`, and
//! `migrate schema --report` are implemented; `new`, `audit`,
//! `migrate schema --assist-waivers`/`--apply`, `export`, and `import`
//! still bail with "not implemented yet." Stdout is always JSON, on every
//! implemented command -- no `--format` flag exists (ADR-0023).
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

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Check { paths } => run_check(cli.config, paths),
        Command::Explain { path } => run_explain(cli.config, path),
        Command::Graph => run_graph(cli.config),
        Command::New { .. } => not_implemented("new"),
        Command::Audit => not_implemented("audit"),
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
        } => {
            eprintln!("urzua migrate schema: pass --report --field <Name> (--assist-waivers and --apply are not implemented yet)");
            ExitCode::from(2)
        }
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

fn run_check(config_path: Option<PathBuf>, paths: Vec<PathBuf>) -> ExitCode {
    let repo_root = match find_repo_root(&paths) {
        Ok(root) => root,
        Err(e) => return report_could_not_run(&e),
    };

    let config_path = config_path.unwrap_or_else(|| repo_root.join(".urzua/config.toml"));
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => return report_could_not_run(&e),
    };

    let discovered = match urzua_io::discover_tracked_files(&repo_root) {
        Ok(d) => d,
        Err(e) => return report_could_not_run(&e.to_string()),
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
    let (exec6, findings6) = rules::revision_log_change_class(&records, &full_text);
    let (exec7, findings7) = rules::embodiment_consistency(&records);
    let (exec8, findings8) = rules::embodiment_locator_promotion_candidate(&records);

    let mut findings = findings1;
    findings.extend(findings2);
    findings.extend(findings3);
    findings.extend(findings4);
    findings.extend(findings5);
    findings.extend(findings6);
    findings.extend(findings7);
    findings.extend(findings8);

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
        rules_executed: vec![exec1, exec2, exec3, exec4, exec5, exec6, exec7, exec8],
        scope: ScopeInfo {
            source: format!("{:?}", discovered.source),
            record_types: config.record_types.keys().cloned().collect(),
        },
        blocking,
        findings,
    };

    print_report(&report);
    ExitCode::from(report.exit_code() as u8)
}

/// "Which decisions govern this file" (ADR-0024) -- every record whose
/// `Realized-by` names `path` as evidence. Stdout is always JSON (ADR-0023).
fn run_explain(config_path: Option<PathBuf>, path: String) -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => {
            eprintln!("urzua explain: could not run: {e}");
            return ExitCode::from(2);
        }
    };

    let config_path = config_path.unwrap_or_else(|| repo_root.join(".urzua/config.toml"));
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("urzua explain: could not run: {e}");
            return ExitCode::from(2);
        }
    };

    let discovered = match urzua_io::discover_tracked_files(&repo_root) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("urzua explain: could not run: {e}");
            return ExitCode::from(2);
        }
    };

    let (records, _full_text) = load_records(&repo_root, &discovered.paths, &config);
    let governing = urzua_core::graph::explain(&records, &path);

    let out = serde_json::json!({
        "path": path,
        "governing_records": governing,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());

    eprintln!(
        "urzua explain {path}: {} record(s) name this as evidence",
        governing.len()
    );
    for g in &governing {
        eprintln!("  [{}] {} ({})", g.via, g.record.display(), g.record_type);
    }
    if governing.is_empty() {
        eprintln!("  none -- no record's Realized-by names this path.");
    }

    ExitCode::from(0)
}

/// The full record-relationship graph, as data (ADR-0024). Stdout is always
/// JSON (ADR-0023).
fn run_graph(config_path: Option<PathBuf>) -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => {
            eprintln!("urzua graph: could not run: {e}");
            return ExitCode::from(2);
        }
    };

    let config_path = config_path.unwrap_or_else(|| repo_root.join(".urzua/config.toml"));
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("urzua graph: could not run: {e}");
            return ExitCode::from(2);
        }
    };

    let discovered = match urzua_io::discover_tracked_files(&repo_root) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("urzua graph: could not run: {e}");
            return ExitCode::from(2);
        }
    };

    let (records, _full_text) = load_records(&repo_root, &discovered.paths, &config);
    let edges = urzua_core::graph::graph(&records);

    let out = serde_json::json!({ "edges": edges });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());

    eprintln!("urzua graph: {} edge(s)", edges.len());
    for e in &edges {
        let marker = if e.dangling { " [DANGLING]" } else { "" };
        eprintln!("  {} --{}--> {}{}", e.from, e.relation, e.to, marker);
    }

    ExitCode::from(0)
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
        eprintln!("urzua fix: only tier 1 is implemented so far (ADR-0015 defines tiers 2 and 3, not yet built)");
        return ExitCode::from(2);
    }
    if apply && ids.is_empty() && !force {
        eprintln!("urzua fix --apply: pass --ids <record,...> or --force (never the default)");
        return ExitCode::from(2);
    }

    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => return report_fix_could_not_run(&e),
    };

    let config_path = config_path.unwrap_or_else(|| repo_root.join(".urzua/config.toml"));
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => return report_fix_could_not_run(&e),
    };

    let discovered = match urzua_io::discover_tracked_files(&repo_root) {
        Ok(d) => d,
        Err(e) => return report_fix_could_not_run(&e.to_string()),
    };

    let (records, _full_text) = load_records(&repo_root, &discovered.paths, &config);
    let (examined, repairs) = urzua_core::fix::detect_repairs(&records);

    if !apply {
        print_fix_report(examined, &repairs, &[]);
        return ExitCode::from(if examined == 0 { 2 } else { 0 });
    }

    let identity = match urzua_io::resolve_identity(by.as_deref(), &repo_root) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("urzua fix --apply: {e}");
            return ExitCode::from(2);
        }
    };
    let _lock = match urzua_io::FixLock::acquire(&repo_root) {
        Ok(lock) => lock,
        Err(e) => {
            eprintln!("urzua fix --apply: {e}");
            return ExitCode::from(2);
        }
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
        match urzua_core::fix::apply_repair(&fresh_content, &repair, &today, &identity) {
            Ok(new_content) => match std::fs::write(&full_path, new_content) {
                Ok(()) => applied.push(repair),
                Err(e) => failed.push((repair.record.clone(), format!("could not write: {e}"))),
            },
            Err(e) => failed.push((repair.record.clone(), e)),
        }
    }

    print_fix_report(examined, &applied, &failed);
    ExitCode::from(if failed.is_empty() { 0 } else { 1 })
}

/// ADR-0023: stdout is always JSON; stderr always carries the human
/// rendering too, so a terminal user still sees both.
fn print_fix_report(
    examined: usize,
    repairs: &[urzua_core::fix::Repair],
    failed: &[(PathBuf, String)],
) {
    let status = if examined == 0 {
        "not-run"
    } else if !failed.is_empty() {
        "partial-failure"
    } else if repairs.is_empty() {
        "ok"
    } else {
        "repairs-available"
    };

    let report = serde_json::json!({
        "status": status,
        "records_examined": examined,
        "repairs": repairs,
        "failed": failed.iter().map(|(p, e)| serde_json::json!({"record": p, "error": e})).collect::<Vec<_>>(),
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap());

    eprintln!(
        "status: {status}  records_examined: {examined}  repairs: {}  failed: {}",
        repairs.len(),
        failed.len()
    );
    for r in repairs {
        eprintln!(
            "  [tier {}] {}: {} '{}' -> '{}' ({})",
            r.tier,
            r.record.display(),
            r.field,
            r.current_value,
            r.computed_value,
            r.evidence
        );
    }
    for (path, err) in failed {
        eprintln!("  [FAILED] {}: {err}", path.display());
    }
}

fn report_fix_could_not_run(message: &str) -> ExitCode {
    eprintln!("urzua fix: could not run: {message}");
    let report = serde_json::json!({
        "status": "not-run",
        "records_examined": 0,
        "repairs": [],
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    ExitCode::from(2)
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
        Err(e) => {
            eprintln!("urzua migrate schema --report: could not run: {e}");
            return ExitCode::from(2);
        }
    };

    let config_path = config_path.unwrap_or_else(|| repo_root.join(".urzua/config.toml"));
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("urzua migrate schema --report: could not run: {e}");
            return ExitCode::from(2);
        }
    };

    let discovered = match urzua_io::discover_tracked_files(&repo_root) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("urzua migrate schema --report: could not run: {e}");
            return ExitCode::from(2);
        }
    };

    let (records, _full_text) = load_records(&repo_root, &discovered.paths, &config);
    if records.is_empty() {
        eprintln!("urzua migrate schema --report: no records discovered -- nothing to check");
        return ExitCode::from(2);
    }

    let report = urzua_core::migrate::schema_report(&records, &field);

    let out = serde_json::json!({
        "field": field,
        "records_examined": records.len(),
        "would_fail": report,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());

    eprintln!(
        "urzua migrate schema --report --field {field}: {} of {} record(s) would newly fail:",
        report.len(),
        records.len()
    );
    for entry in &report {
        eprintln!(
            "  [{:?}] {} ({})",
            entry.state,
            entry.record.display(),
            entry.record_type
        );
    }
    if report.is_empty() {
        eprintln!("  none -- every record already carries a real value for '{field}'.");
    }

    ExitCode::from(0)
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
                    records.push(Record::parse_with_shape(
                        rel_path.clone(),
                        type_name.clone(),
                        &content,
                        type_config.header_shape,
                    ));
                    full_text.insert(rel_path.clone(), content);
                }
                Err(_) => continue,
            }
        }
    }
    (records, full_text)
}

fn report_could_not_run(message: &str) -> ExitCode {
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
    print_report(&report);
    ExitCode::from(2)
}

/// ADR-0023: stdout is always the JSON report, unconditionally -- an agent
/// piping stdout never has to know to ask for it. The human-readable
/// rendering goes to stderr, always too, so a person running this directly
/// in a terminal still sees both (they share the same terminal by default);
/// only a consumer that captures stdout alone sees JSON only.
fn print_report(report: &CheckReport) {
    println!("{}", serde_json::to_string_pretty(report).unwrap());

    eprintln!(
        "status: {:?}  files_examined: {}  blocking: {}",
        report.status, report.files_examined, report.blocking
    );
    for exec in &report.rules_executed {
        eprintln!(
            "  rule {} examined {} record(s)",
            exec.rule, exec.records_examined
        );
    }
    for finding in &report.findings {
        eprintln!(
            "  [{:?}] {}:{} {} -- {}",
            finding.severity,
            finding.file.display(),
            finding.line.map(|l| l.to_string()).unwrap_or_default(),
            finding.rule,
            finding.message
        );
    }
}
