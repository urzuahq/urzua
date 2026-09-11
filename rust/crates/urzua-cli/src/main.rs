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

use std::path::PathBuf;
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{Parser, Subcommand};
use urzua_core::report::{CouldNotRun, Report};

mod commands;
mod discovery;

/// `Notice.subject` constants, one per emitter, matching `Finding.rule`'s
/// existing `const RULE_ID` convention (ADR-0046).
pub(crate) const NOTICE_IDENTITY: &str = "identity";

/// The one place every command prints (ADR-0046) -- printing is I/O, so this
/// lives here, not in `urzua-core::report` (which stays pure and defines
/// only the data shapes `Report` describes). Not a generic envelope around
/// the payload -- each `Report` stays its own real, fully-typed struct; this
/// just gives every one of them the same last step.
pub(crate) fn emit<T: Report>(report: &T) -> ExitCode {
    println!("{}", serde_json::to_string_pretty(report).unwrap());
    report.exit_code()
}

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
        Command::Check { paths } => commands::check::run(cli.config, paths),
        Command::Explain { path } => commands::explain::run(cli.config, path),
        Command::Graph => commands::graph::run(cli.config),
        Command::New {
            record_type,
            title,
            by,
        } => commands::new::run(cli.config, record_type, title, by),
        Command::Audit => commands::audit::run(cli.config),
        Command::Migrate {
            target: MigrateTarget::Ids { apply },
        } => commands::migrate::run_ids(cli.config, apply),
        Command::Migrate {
            target:
                MigrateTarget::Schema {
                    report: true,
                    field: Some(field),
                },
        } => commands::migrate::run_schema_report(cli.config, field),
        Command::Migrate {
            target: MigrateTarget::Schema { .. },
        } => emit(&CouldNotRun::from(
            "pass --report --field <Name> (--assist-waivers and --apply are not implemented yet)",
        )),
        Command::Export { .. } => not_implemented("export"),
        Command::Import { .. } => not_implemented("import"),
        Command::Init { dry_run } => commands::init::run(dry_run),
        Command::Doctor => commands::doctor::run(),
        Command::Fix {
            tier,
            apply,
            ids,
            by,
            force,
        } => commands::fix::run(cli.config, tier, apply, ids, by, force),
    }
}

/// Exit code 2: could not run at all -- distinct from "ran and found
/// nothing," per SPEC-0001's exit-code contract.
fn not_implemented(name: &str) -> ExitCode {
    emit(&CouldNotRun::from(format!(
        "`urzua {name}` is not implemented yet -- see docs/specs/0001-v0-cli.md"
    )))
}
