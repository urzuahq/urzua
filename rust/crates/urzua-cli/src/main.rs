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
mod harness;
mod pins;
mod session;

/// `Notice.subject` constants, one per emitter, matching `Finding.rule`'s
/// existing `const RULE_ID` convention (ADR-0046).
pub(crate) const NOTICE_IDENTITY: &str = "identity";
/// `check` count of the permanent waivers in effect, by category (`ADR-86`).
pub(crate) const NOTICE_PERMANENT_WAIVERS: &str = "permanent-waivers";
/// `init` notice that a type's records disagree on, or parse under none of,
/// the header shapes (`BUG-151`).
/// `check` notice that tracked records are missing from the working tree.
pub(crate) const NOTICE_MISSING_RECORD: &str = "missing-record";
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

/// The evaluation date this run compared against, for a report's `as_of`.
pub(crate) fn as_of() -> urzua_core::report::AsOf {
    urzua_core::report::AsOf {
        date: urzua_io::today(),
        source: match urzua_io::as_of_source() {
            urzua_io::AsOfSource::Flag => urzua_core::report::AsOfSourceKind::Flag,
            urzua_io::AsOfSource::Clock => urzua_core::report::AsOfSourceKind::Clock,
        },
    }
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
    about = "The verifiable specification layer for agent-authored change: it checks the records a change traces back to and orders work from their declared dependencies. It never judges the change.",
    version
)]
struct Cli {
    /// Path to .urzua/config.yaml. Defaults to the nearest one at or above the cwd.
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// Evaluate every date comparison (waiver expiry, claim age, status
    /// expiry) as of this UTC date, `YYYY-MM-DD`, instead of today. Echoed in
    /// the report's `as_of`. Accepted by the read-only modes of `check`, `fix`
    /// and `reverify` (and `audit`); refused with `--apply`, where it would
    /// write a revision row with a false date, and by commands that compare
    /// no dates.
    #[arg(long, global = true, value_name = "YYYY-MM-DD")]
    as_of: Option<String>,

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

    /// Draft one pending waiver per rule and file for the current findings.
    ///
    /// Runs `check` in process and previews the waivers it would write; nothing is
    /// written yet. A draft carries a pending Reason and no Expires, so it suppresses
    /// nothing until a person fills it in. Exit 0 when it ran, 2 when it could not.
    Waive {
        /// Draft from the current findings (the only source today).
        #[arg(long, required = true)]
        from_report: bool,
        /// Only findings of these rules (comma-separated or repeated).
        #[arg(long, value_delimiter = ',')]
        rule: Vec<String>,
        /// Only findings in this file or under this directory (repeatable; a path, not a glob).
        #[arg(long)]
        path: Vec<PathBuf>,
        /// Author named in each draft's revision log; defaults to the git identity.
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
        /// A JSON file of the identities the host supplied for a `--base` run:
        /// `{"schema": 1, "pr_author": LOGIN, "approvers": [LOGIN, ...]}`, which
        /// the gate action writes from the GitHub API. Read by
        /// `diff.decider-required`; never fetched. Cannot be combined with
        /// --stdin-path.
        #[arg(long, value_name = "FILE")]
        actors: Option<PathBuf>,
        paths: Vec<PathBuf>,
    },

    /// Which records govern `path` -- every record whose `Realized-by`
    /// names it as evidence (ADR-0024). With `--rule <rule-id>`, what one rule
    /// is instead: its scope, the level this repository sets, its severity
    /// floor and waivable flag, its options with their current values, its
    /// action variants, its population unit and its spec (ADR-96).
    #[command(override_usage = "urzua explain <PATH>\n       urzua explain --rule <RULE_ID>")]
    Explain {
        /// A file, directory, glob or `path#scope`. Always a path, even when
        /// it spells a rule id; use `--rule` for a rule.
        #[arg(required_unless_present = "rule", conflicts_with = "rule")]
        path: Option<String>,
        /// A rule id, as `urzua rules` lists them.
        #[arg(long, value_name = "RULE_ID")]
        rule: Option<String>,
    },

    /// The full record-relationship graph: every `Implements`/
    /// `Derives-from`/`Supersedes` edge, as data (ADR-0024).
    Graph,

    /// Serve the corpus to an agent over the Model Context Protocol, read-only (`SPEC-26`).
    ///
    /// Speaks newline-delimited JSON-RPC on stdin and stdout until stdin closes; stdout
    /// carries protocol frames and nothing else. Tools: `urzua_check` and
    /// `urzua_list_records`. No tool writes. Register it in your agent's MCP
    /// configuration with `command: urzua`, `args: [mcp]`; `harness install` does not.
    Mcp {
        /// The repository to serve. Defaults to `CLAUDE_PROJECT_DIR`, else the current directory.
        #[arg(long)]
        root: Option<PathBuf>,
    },

    /// The order of work, computed from the records: waves, the ready set, the
    /// critical path, what each record blocks and a grouping (ADR-82).
    ///
    /// One graph over the sequencing pairs `roadmap.pairs` declares. Read-only;
    /// nothing is stored. A repository that declares none gets a note saying
    /// there is no roadmap, not an empty one.
    Roadmap,

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

    /// Emit records in another format. `--format json` is implemented; `agdr`
    /// is not. The format must be named: there is no default.
    Export {
        #[arg(long, value_enum, required = true)]
        format: ExportFormat,
    },

    /// Read records from another format. Not implemented yet. The format must
    /// be named: there is no default.
    Import {
        #[arg(long, value_enum, required = true)]
        format: ExportFormat,
        paths: Vec<PathBuf>,
    },

    /// Adopt an existing corpus: propose and write `.urzua/config.yaml` from
    /// what's already there. Never clobbers, always idempotent, `--dry-run`
    /// prints the config the real run would write. Stable-ID backfill is a separate
    /// step: `migrate ids`.
    Init {
        #[arg(long)]
        dry_run: bool,
    },

    /// Install and run the agent harness bundle: hook entries, a translator for
    /// the agent, and the guardrail cores they call (`ADR-89`).
    ///
    /// The bundle is embedded in this binary and versioned with it. `install`
    /// writes it and `.urzua/harness.lock`; `status` compares the lock with the
    /// files (exit 1 on drift); `update` rewrites files a person has not changed;
    /// `uninstall` removes what the lock says is unchanged. `run` is what the
    /// agent's hooks call: it reads a neutral request on stdin and prints a
    /// verdict. Nothing is enabled unless `harness.guardrails` names it.
    Harness {
        #[command(subcommand)]
        target: HarnessTarget,
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
    /// computes (ADR-0015). Read-only by default. Tier 1 (Embodiment) is the
    /// only drift tier implemented: any other `--tier` exits 2. `--rebaseline`
    /// instead re-pins stale evidence pins.
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
        /// Who ran the command: `agent` or `human`. Recorded in the revision
        /// row beside the identity source (`ADR-89`); a run without it records
        /// `unspecified`. The caller's claim, not proof.
        #[arg(long, value_enum)]
        actor: Option<ActorArg>,
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

    /// Renew an expired claim (`ADR-97`): re-resolve and re-hash every
    /// locator, refuse changed evidence, pin `code:` and `test:` locators that
    /// have no pin, and record who renewed it and why. Read only by default;
    /// `--apply` writes. A changed pin is `fix --rebaseline`'s; `--pin-only`
    /// pins the unpinned locators and renews nothing.
    Reverify {
        /// Review (and, with --apply, renew) these records: repo-relative
        /// paths. With none, the records whose claim has expired (or, with
        /// --pin-only, those with an unpinned locator to pin).
        #[arg(long, value_delimiter = ',')]
        ids: Vec<String>,
        /// Write the renewal. Requires --ids or --force, --reason, and an
        /// identity (--by, or resolved from gh/git config). At most
        /// `max_batch` records, every row carrying the same reason.
        #[arg(long)]
        apply: bool,
        /// Pin the unpinned `code:` and `test:` locators and set no date: the
        /// attested way to pin a corpus, or a drifted unpinned locator.
        #[arg(long)]
        pin_only: bool,
        #[arg(long)]
        by: Option<String>,
        /// Who ran the command: `agent` or `human`; refused as `agent` where
        /// the period's level is error. The caller's claim, not proof.
        #[arg(long, value_enum)]
        actor: Option<ActorArg>,
        /// Select what the mode finds instead of naming records. Refused when
        /// it selects more than one record: a batch is named, never swept up.
        #[arg(long)]
        force: bool,
        /// What you checked, recorded in every row. Required by --apply: one
        /// line, without `|`.
        #[arg(long)]
        reason: Option<String>,
    },
}

#[derive(Subcommand)]
enum HarnessTarget {
    /// Write the bundle's files for an adapter, merge its hook entries into the
    /// agent's project settings, write the marked block into the instructions
    /// file `harness.instructions_file` names, and write `.urzua/harness.lock`.
    /// Never overwrites a file that differs; refuses a settings file it cannot
    /// merge into (exit 2).
    Install {
        /// The adapter to install. Defaults to `harness.adapters`.
        #[arg(long)]
        adapter: Option<String>,
        /// List what would change and write nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Compare the lock with the files, the hook entries, the instructions
    /// block and the binary: missing, modified and out-of-date. Exit 1 on any.
    Status,
    /// Rewrite the files whose hash still matches the lock; report a file a
    /// person changed and leave it unless `--force`.
    Update {
        #[arg(long)]
        force: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove the files whose hash matches the lock, take the hook entries and
    /// the marked block out of the shared files, report what a person changed
    /// and leave it, and delete the lock last.
    Uninstall {
        #[arg(long)]
        dry_run: bool,
    },
    /// List the adapters this binary embeds.
    Adapters,
    /// Run a guardrail core: a neutral request on stdin, a verdict on stdout.
    Run {
        #[command(subcommand)]
        event: HarnessRun,
    },
}

#[derive(Subcommand)]
enum HarnessRun {
    /// Judge a proposed edit to a record (the editor hook).
    Edit,
    /// Check, at the start of a session, that the harness can do its job.
    SessionStart,
    /// After a shell command, run `check` on the records git shows as changed
    /// and report; never blocks.
    PostCommand,
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

    /// Rewrite the config from the previous schema version to the current one,
    /// in place, keeping its comments. Reports every key it added, changed,
    /// renamed, moved or dropped, and reports nothing to do when the config is
    /// already current. A version outside what this build reads is refused.
    Config,

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

/// The values `fix --actor` accepts; `unspecified` is what a run without the
/// flag records, not a value to pass.
#[derive(Clone, Copy, clap::ValueEnum)]
enum ActorArg {
    Agent,
    Human,
}

impl From<ActorArg> for urzua_core::fix::ActorKind {
    fn from(actor: ActorArg) -> Self {
        match actor {
            ActorArg::Agent => urzua_core::fix::ActorKind::Agent,
            ActorArg::Human => urzua_core::fix::ActorKind::Human,
        }
    }
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum ExportFormat {
    /// Not implemented: exits 2.
    Agdr,
    /// Every record as plain JSON data (SPEC-1).
    Json,
}

/// Turns a panic into the JSON `not-run` envelope and exit 2.
fn install_panic_hook() {
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
}

/// `--help`/`--version` surface as `Err(_)` too (`ErrorKind::DisplayHelp`/
/// `DisplayVersion`), not a separate success path -- print as clap already
/// rendered them and exit 0, the one stated plain-text exception (matching
/// `cargo`'s own convention: help/version are never gated behind a
/// machine-format flag). Every other parse error goes through the same
/// `emit()`/`CouldNotRun` path as any other fatal command error.
fn main() -> ExitCode {
    install_panic_hook();

    panic_if_requested();

    let cli = match parse_cli() {
        Ok(cli) => cli,
        Err(code) => return code,
    };

    match cli.command {
        Command::Check {
            base,
            fail_on,
            stdin_path,
            rules,
            actors,
            paths,
        } => commands::check::run(
            cli.config,
            commands::check::CheckArgs {
                paths,
                base,
                fail_on: fail_on.map(Into::into),
                stdin_path,
                rules,
                actors,
                ..Default::default()
            },
        ),
        Command::Explain { path, rule } => commands::explain::run(cli.config, path, rule),
        Command::Graph => commands::graph::run(cli.config),
        Command::Roadmap => commands::roadmap::run(cli.config),
        Command::Mcp { root } => commands::mcp::run(cli.config, root),
        Command::New {
            record_type,
            title,
            by,
        } => commands::new::run(cli.config, record_type, title, by),
        Command::Waive { rule, path, by, .. } => commands::waive::run(cli.config, rule, path, by),
        Command::Audit => commands::audit::run(cli.config),
        Command::Migrate { target } => dispatch_migrate(cli.config, target),
        Command::Export { format } => dispatch_export(cli.config, format),
        Command::Import { .. } => not_implemented("import"),
        Command::Init { dry_run } => commands::init::run(cli.config, dry_run),
        Command::Harness { target } => dispatch_harness(cli.config, target),
        Command::Doctor => commands::doctor::run(cli.config),
        Command::Rules => commands::rules::run(),
        Command::Index { apply } => commands::index::run(cli.config, apply),
        Command::Refs {
            target: RefsTarget::Refresh,
        } => commands::refs::run_refresh(cli.config),
        Command::Reverify {
            ids,
            apply,
            pin_only,
            by,
            actor,
            force,
            reason,
        } => commands::reverify::run(
            cli.config,
            commands::reverify::ReverifyArgs {
                ids,
                apply,
                pin_only,
                by,
                actor: actor.map_or(urzua_core::fix::ActorKind::Unspecified, Into::into),
                force,
                reason,
            },
        ),
        Command::Fix {
            paths,
            stdin_path,
            tier,
            apply,
            ids,
            by,
            actor,
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
                actor: actor.map_or(urzua_core::fix::ActorKind::Unspecified, Into::into),
                force,
                rebaseline,
                reason,
            },
        ),
    }
}

/// Debug builds only: lets the acceptance test prove the panic exit code.
#[cfg(debug_assertions)]
fn panic_if_requested() {
    if std::env::var_os("URZUA_TEST_PANIC").is_some() {
        panic!("URZUA_TEST_PANIC set");
    }
}

#[cfg(not(debug_assertions))]
fn panic_if_requested() {}

fn dispatch_export(config: Option<PathBuf>, format: ExportFormat) -> ExitCode {
    match format {
        ExportFormat::Json => commands::export::run(config),
        ExportFormat::Agdr => not_implemented("export"),
    }
}

fn dispatch_migrate(config: Option<PathBuf>, target: MigrateTarget) -> ExitCode {
    match target {
        MigrateTarget::Ids { apply } => commands::migrate::run_ids(config, apply),
        MigrateTarget::Config => commands::migrate::run_config(config),
        MigrateTarget::Schema {
            report: true,
            field: Some(field),
        } => commands::migrate::run_schema_report(config, field),
        MigrateTarget::Schema { .. } => emit(&CouldNotRun::from(
            "pass --report --field <Name> (--assist-waivers and --apply are not implemented yet)",
        )),
    }
}

fn dispatch_harness(config: Option<PathBuf>, target: HarnessTarget) -> ExitCode {
    match target {
        HarnessTarget::Install { adapter, dry_run } => {
            harness::run_install(config, adapter, dry_run)
        }
        HarnessTarget::Status => harness::run_status(config),
        HarnessTarget::Update { force, dry_run } => harness::run_update(config, force, dry_run),
        HarnessTarget::Uninstall { dry_run } => harness::run_uninstall(config, dry_run),
        HarnessTarget::Adapters => harness::run_adapters(),
        HarnessTarget::Run {
            event: HarnessRun::Edit,
        } => harness::run_edit(config),
        HarnessTarget::Run {
            event: HarnessRun::SessionStart,
        } => harness::run_session_start(config),
        HarnessTarget::Run {
            event: HarnessRun::PostCommand,
        } => harness::run_post_command(config),
    }
}

/// Exit code 2: could not run at all -- distinct from "ran and found
/// nothing," per SPEC-0001's exit-code contract.
fn not_implemented(name: &str) -> ExitCode {
    emit(&CouldNotRun::from(format!(
        "`urzua {name}` is not implemented yet -- see docs/specs/SPEC-1-cli.md"
    )))
}

/// Applies `--as-of` before any command reads a date. A date is refused (exit
/// 2) when it is malformed, when the command compares no dates, or when the
/// command writes: a revision row or an anchor dated other than the real day
/// would falsify the history it records.
fn fix_evaluation_date(cli: &Cli) -> Result<(), String> {
    let Some(value) = cli.as_of.as_deref() else {
        return Ok(());
    };
    let (name, writes) = match &cli.command {
        Command::Check { .. } => ("check", false),
        Command::Audit => ("audit", false),
        Command::Fix { apply, .. } => ("fix", *apply),
        Command::Reverify { apply, .. } => ("reverify", *apply),
        other => {
            return Err(format!(
                "--as-of is not accepted by `urzua {}`: it compares no dates against an evaluation date (it applies to check, audit, fix and reverify)",
                command_name(other)
            ))
        }
    };
    if writes {
        return Err(format!(
            "--as-of cannot be combined with --apply on `urzua {name}`: the revision row and anchor it writes carry the real date, and a different one would falsify the history. Preview with --as-of and no --apply, then apply without it"
        ));
    }
    urzua_io::set_as_of(value)
}

fn command_name(command: &Command) -> &'static str {
    match command {
        Command::New { .. } => "new",
        Command::Waive { .. } => "waive",
        Command::Check { .. } => "check",
        Command::Explain { .. } => "explain",
        Command::Graph => "graph",
        Command::Roadmap => "roadmap",
        Command::Mcp { .. } => "mcp",
        Command::Audit => "audit",
        Command::Migrate { .. } => "migrate",
        Command::Export { .. } => "export",
        Command::Import { .. } => "import",
        Command::Init { .. } => "init",
        Command::Harness { .. } => "harness",
        Command::Doctor => "doctor",
        Command::Rules => "rules",
        Command::Index { .. } => "index",
        Command::Refs { .. } => "refs",
        Command::Fix { .. } => "fix",
        Command::Reverify { .. } => "reverify",
    }
}

/// Parses the command line and fixes the evaluation date; the error is the
/// exit a refused invocation has already printed.
fn parse_cli() -> Result<Cli, ExitCode> {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) => {
            e.exit()
        }
        Err(e) => return Err(emit(&CouldNotRun::from(e.to_string()))),
    };
    fix_evaluation_date(&cli).map_err(|message| emit(&CouldNotRun::from(message)))?;
    Ok(cli)
}
