//! `urzua describe`: what a caller needs to invoke urzua without reading
//! `--help` (`MILE-359`; the published agent CLI contract's RFC-4 section 6).
//! Every argument is read from the clap definitions in `main.rs`; only what clap
//! cannot know (a command's profile, whether it is safe, what it reads on
//! stdin, an example) is stated here, in [`COMMANDS`], once per command.

use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};
use serde::Serialize;

/// The contract version urzua conforms to. `MILE-370` moves it to `1.1` once
/// every 1.1 piece is delivered; the inputs below are printed already.
const CONTRACT_VERSION: &str = "1.0";

/// What clap cannot say about one command, keyed by its full path.
struct Entry {
    name: &'static str,
    profile: &'static str,
    safe: bool,
    kind: Option<&'static str>,
    scopes: &'static [&'static str],
    metrics: &'static [&'static str],
    stdin: Option<(&'static str, &'static str)>,
    example: Option<&'static [&'static str]>,
    protocol: Option<&'static str>,
}

const fn base(name: &'static str, safe: bool) -> Entry {
    Entry {
        name,
        profile: "base",
        safe,
        kind: None,
        scopes: &[],
        metrics: &[],
        stdin: None,
        example: None,
        protocol: None,
    }
}

const fn exempt(name: &'static str, protocol: &'static str, stdin: &'static str) -> Entry {
    Entry {
        profile: "exempt",
        stdin: Some(("json", stdin)),
        protocol: Some(protocol),
        ..base(name, false)
    }
}

const RECORD_STDIN: Option<(&str, &str)> = Some((
    "text",
    "With --stdin-path only: the proposed content of that record.",
));

/// `safe` is true only for a command that writes nothing whatever its
/// arguments and finishes quickly; one whose `--apply` writes is not safe.
/// `describe` itself is not listed: its answer is this object, not a report.
const COMMANDS: &[Entry] = &[
    base("new", false),
    base("waive", false),
    Entry {
        profile: "gate",
        kind: Some("records"),
        scopes: &["paths", "base-ref"],
        metrics: &["files"],
        stdin: RECORD_STDIN,
        example: Some(&["--rules", "header.required-fields"]),
        ..base("check", true)
    },
    Entry {
        scopes: &["paths"],
        example: Some(&["--types"]),
        ..base("explain", true)
    },
    base("graph", true),
    exempt(
        "mcp",
        "mcp",
        "Newline-delimited JSON-RPC requests, until stdin closes.",
    ),
    base("board", false),
    base("roadmap", true),
    // Runs every declared rule with no way to narrow it, so it is not quick.
    Entry {
        profile: "gate",
        kind: Some("records"),
        metrics: &["files"],
        ..base("audit", false)
    },
    base("migrate ids", false),
    base("migrate filenames", false),
    base("migrate config", false),
    Entry {
        example: Some(&["--report", "--field", "Status"]),
        ..base("migrate schema", true)
    },
    Entry {
        example: Some(&["--format", "json", "--no-text", "--no-checklist-items"]),
        ..base("export", true)
    },
    Entry {
        scopes: &["paths"],
        ..base("import", true)
    },
    Entry {
        stdin: Some((
            "text",
            "On a terminal in a repository with no record directory: which record types to govern.",
        )),
        ..base("init", false)
    },
    base("harness install", false),
    base("harness status", true),
    base("harness update", false),
    base("harness uninstall", false),
    base("harness adapters", true),
    base("harness limits", true),
    exempt(
        "harness hook",
        "harness-hook",
        "The hook event the agent sends, in that agent's format.",
    ),
    base("harness record", false),
    exempt(
        "harness run edit",
        "harness-run",
        "A neutral request naming the proposed edit.",
    ),
    exempt(
        "harness run session-start",
        "harness-run",
        "A neutral request naming the session.",
    ),
    exempt(
        "harness run post-command",
        "harness-run",
        "A neutral request naming the shell command that ran.",
    ),
    exempt(
        "harness run question",
        "harness-run",
        "A neutral request carrying the question the agent is about to ask.",
    ),
    exempt(
        "harness run answer",
        "harness-run",
        "A neutral request carrying the person's answer.",
    ),
    base("hooks install", false),
    base("hooks status", true),
    base("hooks uninstall", false),
    base("hooks verify", false),
    Entry {
        profile: "exempt",
        protocol: Some("git-hook"),
        ..base("hooks run", false)
    },
    base("doctor", true),
    base("rules", true),
    base("index", false),
    base("refs refresh", false),
    Entry {
        scopes: &["paths"],
        stdin: RECORD_STDIN,
        ..base("fix", false)
    },
    base("reverify", false),
];

/// Every environment variable that changes what urzua does. `PATH` and `HOME`
/// are only used to find programs and the home directory, and variables read
/// only by a test build are left out (`env_drift` in the tests names them).
/// The adapters' own variables are in [`crate::harness::ADAPTER_ENV`].
const ENV: &[(&str, &str)] = &[
    (
        "URZUA_HOOK_OVERRIDE",
        "The named person overriding a blocked edit; every override is logged for review.",
    ),
    (
        "URZUA_HOOK_REASON",
        "The reason recorded with URZUA_HOOK_OVERRIDE.",
    ),
    (
        "URZUA_BY",
        "The identity `harness record` records, instead of git's user.name.",
    ),
    (
        "URZUA_BIN",
        "The urzua binary `harness record` runs, instead of itself.",
    ),
    ("URZUA", "Read as URZUA_BIN when URZUA_BIN is unset."),
    (
        "URZUA_HOOKS_ON_NOT_RUN",
        "`block` makes `hooks run` block when the configuration cannot be read.",
    ),
    (
        "URZUA_HOOKS_PROBE",
        "Set by `hooks verify --run`: the directory in which a run that a hook reached writes a marker.",
    ),
    (
        "URZUA_GITHUB_API",
        "The GitHub REST API base URL `refs refresh` fetches from.",
    ),
    (
        "GITHUB_TOKEN",
        "The token `refs refresh` sends to the GitHub API.",
    ),
    (
        "GH_TOKEN",
        "Read as GITHUB_TOKEN when GITHUB_TOKEN is unset.",
    ),
];

#[derive(Serialize)]
struct Describe {
    name: &'static str,
    version: &'static str,
    contract_version: &'static str,
    system: u8,
    args: Vec<ArgDecl>,
    env: Vec<EnvDecl>,
    config: Vec<ConfigDecl>,
    log: Option<()>,
    commands: Vec<CommandDecl>,
}

#[derive(Serialize)]
struct EnvDecl {
    name: &'static str,
    means: &'static str,
}

#[derive(Serialize)]
struct ConfigDecl {
    path: &'static str,
    format: &'static str,
}

#[derive(Serialize)]
struct StdinDecl {
    format: &'static str,
    means: &'static str,
}

#[derive(Serialize)]
struct CommandDecl {
    name: String,
    profile: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<&'static str>,
    safe: bool,
    scopes: &'static [&'static str],
    metrics: &'static [&'static str],
    args: Vec<ArgDecl>,
    stdin: Option<StdinDecl>,
    #[serde(skip_serializing_if = "Option::is_none")]
    example: Option<&'static [&'static str]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    protocol: Option<&'static str>,
}

#[derive(Serialize, Debug, PartialEq)]
pub(crate) struct ArgDecl {
    pub(crate) name: String,
    kind: &'static str,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    value_type: Option<&'static str>,
    required: bool,
    repeatable: bool,
    pub(crate) means: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<&'static str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) conflicts_with: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) requires: Vec<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    after_separator: bool,
}

pub fn run(cli: Command) -> ExitCode {
    use std::io::Write;
    let text =
        describe(cli).and_then(|d| serde_json::to_string_pretty(&d).map_err(|e| e.to_string()));
    match text {
        Ok(text) => {
            let _ = writeln!(std::io::stdout().lock(), "{text}");
            ExitCode::SUCCESS
        }
        Err(e) => crate::emit(&urzua_core::report::CouldNotRun::from(format!(
            "urzua describe could not describe the command line: {e}"
        ))),
    }
}

/// The answer for this command line, or why it cannot be given: a command
/// without an entry in [`COMMANDS`], or an entry naming no command.
fn describe(mut cli: Command) -> Result<Describe, String> {
    cli.build();
    let mut leaves = Vec::new();
    collect_leaves(&cli, &mut Vec::new(), &mut leaves);
    for entry in COMMANDS {
        if !leaves.iter().any(|(path, _)| path == entry.name) {
            return Err(format!(
                "`{}` is described but is not a command",
                entry.name
            ));
        }
    }
    let commands = leaves
        .iter()
        .filter(|(path, _)| path != "describe")
        .map(|(path, command)| command_decl(path, command))
        .collect::<Result<Vec<_>, _>>()?;
    let globals: Vec<&Arg> = cli.get_arguments().filter(|a| a.is_global_set()).collect();
    Ok(Describe {
        name: "urzua",
        version: env!("CARGO_PKG_VERSION"),
        contract_version: CONTRACT_VERSION,
        system: 0,
        args: args_of(&cli, &globals),
        env: ENV
            .iter()
            .chain(crate::harness::ADAPTER_ENV)
            .map(|&(name, means)| EnvDecl { name, means })
            .collect(),
        config: vec![ConfigDecl {
            path: ".urzua/config.yaml",
            format: "yaml",
        }],
        log: None,
        commands,
    })
}

/// Every command that runs (one with no subcommands), by its full path.
fn collect_leaves<'a>(
    command: &'a Command,
    path: &mut Vec<&'a str>,
    out: &mut Vec<(String, &'a Command)>,
) {
    for sub in command.get_subcommands().filter(|s| s.get_name() != "help") {
        path.push(sub.get_name());
        if sub.has_subcommands() {
            collect_leaves(sub, path, out);
        } else {
            out.push((path.join(" "), sub));
        }
        path.pop();
    }
}

fn command_decl(path: &str, command: &Command) -> Result<CommandDecl, String> {
    let entry = COMMANDS
        .iter()
        .find(|e| e.name == path)
        .ok_or_else(|| format!("`{path}` has no entry in describe's command table"))?;
    let own: Vec<&Arg> = command
        .get_arguments()
        .filter(|a| !a.is_global_set() && !is_builtin(a))
        .collect();
    Ok(CommandDecl {
        name: path.to_string(),
        profile: entry.profile,
        kind: entry.kind,
        safe: entry.safe,
        scopes: entry.scopes,
        metrics: entry.metrics,
        args: args_of(command, &own),
        stdin: entry
            .stdin
            .map(|(format, means)| StdinDecl { format, means }),
        example: entry.example,
        protocol: entry.protocol,
    })
}

fn is_builtin(arg: &Arg) -> bool {
    matches!(
        arg.get_action(),
        ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
    )
}

/// The declaration of each of `args`, all of them arguments of `command`.
fn args_of(command: &Command, args: &[&Arg]) -> Vec<ArgDecl> {
    args.iter()
        .filter(|a| !is_builtin(a))
        .map(|arg| ArgDecl {
            name: name_of(arg),
            kind: kind_of(arg),
            value_type: (kind_of(arg) != "flag").then(|| type_of(arg)),
            required: arg.is_required_set(),
            repeatable: repeatable(arg),
            means: arg
                .get_help()
                .map(|h| h.to_string().trim().to_string())
                .unwrap_or_default(),
            values: arg
                .get_possible_values()
                .iter()
                .filter(|v| !v.is_hide_set())
                .map(|v| v.get_name().to_string())
                .collect(),
            format: date_format(arg),
            conflicts_with: command
                .get_arg_conflicts_with(arg)
                .into_iter()
                .filter(|other| !is_builtin(other))
                .map(name_of)
                .collect(),
            requires: requires_of(command, arg, args),
            after_separator: arg.is_last_set(),
        })
        .collect()
}

pub(crate) fn name_of(arg: &Arg) -> String {
    match (arg.get_long(), arg.get_short()) {
        (Some(long), _) => format!("--{long}"),
        (None, Some(short)) => format!("-{short}"),
        (None, None) => arg.get_id().to_string(),
    }
}

fn kind_of(arg: &Arg) -> &'static str {
    if arg.is_positional() {
        "positional"
    } else if arg.get_action().takes_values() {
        "option"
    } else {
        "flag"
    }
}

fn type_of(arg: &Arg) -> &'static str {
    use std::any::TypeId;
    let id = arg.get_value_parser().type_id();
    let integers = [
        TypeId::of::<u8>(),
        TypeId::of::<u16>(),
        TypeId::of::<u32>(),
        TypeId::of::<u64>(),
        TypeId::of::<usize>(),
        TypeId::of::<i64>(),
    ];
    if integers.iter().any(|t| id == *t) {
        "integer"
    } else if id == TypeId::of::<std::path::PathBuf>() {
        "path"
    } else {
        "string"
    }
}

fn repeatable(arg: &Arg) -> bool {
    matches!(arg.get_action(), ArgAction::Append | ArgAction::Count)
        || arg.get_num_args().is_some_and(|n| n.max_values() > 1)
}

/// `date` for a value written `YYYY-MM-DD`, the one narrower string form urzua takes.
fn date_format(arg: &Arg) -> Option<&'static str> {
    arg.get_value_names()
        .is_some_and(|names| names.iter().any(|n| n.as_str() == "YYYY-MM-DD"))
        .then_some("date")
}

/// The arguments `arg` requires. clap exposes no getter for `requires`, so the
/// command is parsed with `arg` alone (and every unconditionally required
/// argument) and the arguments clap then reports missing are the answer.
fn requires_of(command: &Command, arg: &Arg, siblings: &[&Arg]) -> Vec<String> {
    let mut probe = command.clone().no_binary_name(true);
    let mut argv: Vec<String> = Vec::new();
    for other in siblings
        .iter()
        .filter(|o| o.is_required_set() && o.get_id() != arg.get_id())
    {
        argv.extend(tokens_for(other));
    }
    argv.extend(tokens_for(arg));
    let Err(error) = probe.try_get_matches_from_mut(&argv) else {
        return Vec::new();
    };
    if error.kind() != clap::error::ErrorKind::MissingRequiredArgument {
        return Vec::new();
    }
    let Some(clap::error::ContextValue::Strings(missing)) =
        error.get(clap::error::ContextKind::InvalidArg)
    else {
        return Vec::new();
    };
    siblings
        .iter()
        .filter(|s| missing.iter().any(|m| rendered_as(s, m)))
        .map(|s| name_of(s))
        .collect()
}

/// The words that give `arg` on a command line, with a value it accepts.
fn tokens_for(arg: &Arg) -> Vec<String> {
    let value = arg
        .get_possible_values()
        .first()
        .map(|v| v.get_name().to_string())
        .unwrap_or_else(|| {
            if type_of(arg) == "integer" {
                "0".into()
            } else {
                "x".into()
            }
        });
    match kind_of(arg) {
        "positional" => vec![value],
        "flag" => vec![name_of(arg)],
        _ => vec![name_of(arg), value],
    }
}

/// Whether clap's rendering of a missing argument (`--serve`, `--format
/// <FORMAT>`, `<RECORD_TYPE>`) is `arg`.
fn rendered_as(arg: &Arg, rendered: &str) -> bool {
    let first = rendered.split_whitespace().next().unwrap_or("");
    if arg.is_positional() {
        let names: Vec<String> = arg
            .get_value_names()
            .map(|n| n.iter().map(|s| s.to_string()).collect())
            .unwrap_or_else(|| vec![arg.get_id().to_string().to_uppercase()]);
        names
            .iter()
            .any(|n| first.trim_matches(|c| "<>[].".contains(c)) == n)
    } else {
        first == name_of(arg)
    }
}

#[cfg(test)]
mod tests;
