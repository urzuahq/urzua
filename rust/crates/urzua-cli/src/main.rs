//! The `urzua` binary.
//!
//! Command surface per SPEC-0001. `check`, `explain`, `graph`, `new`, `audit`,
//! `init` (adopt mode), `doctor`, `fix` (detect and apply), `migrate ids`,
//! `migrate schema --report`, and `export --format json` are implemented;
//! `migrate schema --assist-waivers`/`--apply`, `export --format agdr`, and
//! `import` still bail with "not implemented yet." Stdout is always JSON, on every implemented command --
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
mod diff_scope;
mod discovery;
mod file_texts;
mod gate;
mod pins;
mod session;

/// `Notice.subject` constants, one per emitter, matching `Finding.rule`'s
/// existing `const RULE_ID` convention (ADR-0046).
pub(crate) const NOTICE_IDENTITY: &str = "identity";
/// `init` notice that a type's records disagree on, or parse under none of,
/// the header shapes (`BUG-151`).
pub(crate) const NOTICE_HEADER_SHAPE: &str = "header-shape";
/// `init` warning that a proposed config leaves a command's rule set entirely
/// undeclared (`BUG-99`).
pub(crate) const NOTICE_RULE_APPLICABILITY: &str = "rule-applicability";

/// The one place every command prints (ADR-0046) -- printing is I/O, so this
/// lives here, not in `urzua-core::report` (which stays pure and defines
/// only the data shapes `Report` describes). Not a generic envelope around
/// the payload -- each `Report` stays its own real, fully-typed struct; this
/// just gives every one of them the same last step.
pub(crate) fn emit<T: Report>(report: &T) -> ExitCode {
    use std::io::Write;
    // writeln!, not println!: a closed stdout (`urzua check | head -1`) must
    // end the run quietly, not panic (MILE-176). The reader leaving is not an
    // error of ours, so the write result is dropped and the exit code stays
    // the report's.
    let _ = writeln!(
        std::io::stdout().lock(),
        "{}",
        serde_json::to_string_pretty(report).unwrap()
    );
    report.exit_code()
}

/// `--fail-on`'s values; mapped onto core's `FailOn`, which stays free of clap.
#[derive(Clone, Copy, clap::ValueEnum)]
enum FailOnArg {
    Error,
    Warning,
}

impl From<FailOnArg> for urzua_core::config::FailOn {
    fn from(arg: FailOnArg) -> Self {
        match arg {
            FailOnArg::Error => Self::Error,
            FailOnArg::Warning => Self::Warning,
        }
    }
}

#[derive(Parser)]
#[command(
    name = "urzua",
    about = "The decision layer for engineering orgs running AI agents",
    version
)]
struct Cli {
    /// Path to .urzua/config.yaml. Defaults to the nearest one at or above the cwd.
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a record with a stable ID assigned. Never asks you to pick a number.
    ///
    /// The number skips past any already used on a local branch or remote-tracking ref
    /// (named on stderr). Refs never fetched into this clone are invisible: `git fetch` first.
    New {
        /// Record type, as configured in .urzua/config.yaml (e.g. adr, rfc, spec).
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
    ///
    /// `--base <ref>` reports on only the records changed since the merge base of `<ref>` and
    /// HEAD (staged and unstaged edits included), and says so in `scope.source` and `scope.base`.
    /// Unchanged records that refer to a changed or deleted one are reported too, and references
    /// still resolve against the whole tracked corpus.
    Check {
        /// Report on records changed against this ref, plus unchanged records that refer to them.
        #[arg(long)]
        base: Option<String>,
        /// The lowest finding severity that makes the command exit 1,
        /// overriding the configuration's `fail_on`. The JSON is unchanged.
        #[arg(long, value_enum)]
        fail_on: Option<FailOnArg>,
        /// Read proposed content for this one record path from stdin and judge it
        /// against every enabled rule without writing; the path need not exist
        /// yet. Reports on that record only (`scope.source` is `stdin`). For an
        /// editor hook. Cannot be combined with PATHS or --base.
        #[arg(long, value_name = "PATH")]
        stdin_path: Option<PathBuf>,
        /// Run only these declared rules (comma-separated ids). Narrows what
        /// runs and never enables a rule; an id the configuration does not
        /// declare is an error (exit 2). The report names the filter in
        /// `rules_selected` and omits the other rules from `rules_executed`.
        #[arg(long, value_delimiter = ',', value_name = "ID")]
        rules: Vec<String>,
        paths: Vec<PathBuf>,
    },

    /// Which records govern `path` -- every record whose `Realized-by`
    /// names it as evidence (ADR-0024).
    Explain { path: String },

    /// The full record-relationship graph: every `Implements`/
    /// `Derives-from`/`Supersedes` edge, as data (ADR-0024).
    Graph,

    /// Deprecated alias of `check`: runs every rule the configuration
    /// declares and prints `check`'s report, plus a deprecation notice that
    /// never changes the exit code. Use `check` instead, or `check --rules
    /// <id>[,<id>...]` for a narrower run. Removed in the next release.
    ///
    /// Reports only. Never writes.
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

    /// Adopt an existing corpus: propose and write `.urzua/config.yaml` from
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

    /// The complete, current rule set this build ships, with each rule's
    /// description. Reads no config -- what the binary supports, not what
    /// any one repository enabled.
    Rules,

    /// Regenerate the declared index artifacts (`index.freshness`'s
    /// `indexes`) from the records. Detect mode by default: reports which
    /// artifacts differ and writes nothing; `--apply` writes them.
    Index {
        #[arg(long)]
        apply: bool,
    },

    /// Maintain the committed snapshot of issue and pull request numbers that
    /// `citation.issue-exists` checks citations against.
    Refs {
        #[command(subcommand)]
        target: RefsTarget,
    },

    /// Detect fields whose stated value disagrees with what the tool
    /// computes (ADR-0015). Read-only by default. Only Tier 1 (Embodiment)
    /// exists.
    Fix {
        /// Examine and repair only these records: a record file, or a
        /// directory holding records. A path that matches no record is
        /// reported as a failure. With none, the whole corpus.
        paths: Vec<PathBuf>,
        /// Read proposed content for this one record path from stdin and
        /// judge it without writing; the path need not exist yet. Drift is
        /// computed for that record only. For an editor hook. Cannot be
        /// combined with PATHS or any apply flag.
        #[arg(long, value_name = "PATH")]
        stdin_path: Option<PathBuf>,
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
        /// Re-pin stale `@sha256:` evidence pins instead of repairing
        /// Embodiment (`MILE-181`). Without --apply it lists each stale pin
        /// with its recorded and current hash; with --apply and --ids (or
        /// --force), --by and --reason it rewrites those records' pins and
        /// appends an attributed revision row.
        #[arg(long)]
        rebaseline: bool,
        /// Why the evidence still holds, recorded in the revision row.
        /// Required by --rebaseline --apply.
        #[arg(long)]
        reason: Option<String>,
    },
}

#[derive(Subcommand)]
enum RefsTarget {
    /// Fetch every issue and pull request number of the declared repository
    /// from the GitHub REST API and rewrite the snapshot file.
    ///
    /// The only command that uses the network (`check` never does). Reads a
    /// token from GITHUB_TOKEN or GH_TOKEN when one is set, and needs `curl`.
    /// A failed fetch writes nothing and exits 2. Run it when `check` reports
    /// a citation the snapshot predates, then commit the file.
    Refresh,
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
    /// Every record as plain JSON data (SPEC-1). Must be named explicitly.
    Json,
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
        // The documented code for "could not run" is 2; Rust's default for an
        // unwinding panic out of main is 101 (MILE-176).
        std::process::exit(2);
    }));

    // Debug builds only: lets the acceptance test prove the panic exit code.
    #[cfg(debug_assertions)]
    if std::env::var_os("URZUA_TEST_PANIC").is_some() {
        panic!("URZUA_TEST_PANIC set");
    }

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) => {
            e.exit()
        }
        Err(e) => return emit(&CouldNotRun::from(e.to_string())),
    };

    match cli.command {
        Command::Check {
            base,
            fail_on,
            stdin_path,
            rules,
            paths,
        } => commands::check::run(
            cli.config,
            commands::check::CheckArgs {
                paths,
                base,
                fail_on: fail_on.map(Into::into),
                stdin_path,
                rules,
                ..Default::default()
            },
        ),
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
        Command::Export {
            format: ExportFormat::Json,
        } => commands::export::run(cli.config),
        Command::Export {
            format: ExportFormat::Agdr,
        } => not_implemented("export"),
        Command::Import { .. } => not_implemented("import"),
        Command::Init { dry_run } => commands::init::run(dry_run),
        Command::Doctor => commands::doctor::run(),
        Command::Rules => commands::rules::run(),
        Command::Index { apply } => commands::index::run(cli.config, apply),
        Command::Refs {
            target: RefsTarget::Refresh,
        } => commands::refs::run_refresh(cli.config),
        Command::Fix {
            paths,
            stdin_path,
            tier,
            apply,
            ids,
            by,
            force,
            rebaseline,
            reason,
        } => commands::fix::run(
            cli.config,
            commands::fix::FixArgs {
                paths,
                stdin_path,
                tier,
                apply,
                ids,
                by,
                force,
                rebaseline,
                reason,
            },
        ),
    }
}

/// Exit code 2: could not run at all -- distinct from "ran and found
/// nothing," per SPEC-0001's exit-code contract.
fn not_implemented(name: &str) -> ExitCode {
    emit(&CouldNotRun::from(format!(
        "`urzua {name}` is not implemented yet -- see docs/specs/0001-v0-cli.md"
    )))
}
