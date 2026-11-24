use super::*;
use clap::CommandFactory;

fn urzua() -> Command {
    crate::Cli::command()
}

fn find<'a>(d: &'a Describe, name: &str) -> &'a CommandDecl {
    d.commands
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("`{name}` is not described"))
}

#[test]
fn every_command_is_described_and_every_entry_is_a_command() {
    let d = describe(urzua()).unwrap();
    assert_eq!(d.commands.len(), COMMANDS.len());
    assert!(d.commands.iter().any(|c| c.name == "migrate ids"));
    assert!(d
        .commands
        .iter()
        .all(|c| c.name != "describe" && c.name != "harness"));
}

#[test]
fn a_command_added_without_an_entry_is_refused() {
    let planted = urzua().subcommand(Command::new("planted").about("x"));
    let error = describe(planted)
        .err()
        .expect("an undescribed command is refused");
    assert!(error.contains("`planted`"), "{error}");

    let nested = urzua().mut_subcommand("migrate", |m| m.subcommand(Command::new("planted")));
    assert!(describe(nested)
        .err()
        .unwrap()
        .contains("`migrate planted`"));
}

#[test]
fn an_argument_added_to_a_command_is_declared_from_its_definition() {
    let planted = urzua().mut_subcommand("graph", |g| {
        g.arg(
            Arg::new("planted")
                .long("planted")
                .value_parser(clap::value_parser!(u16))
                .action(ArgAction::Append)
                .requires("other")
                .help("A planted option."),
        )
        .arg(
            Arg::new("other")
                .long("other")
                .action(ArgAction::SetTrue)
                .help("Another."),
        )
    });
    let d = describe(planted).unwrap();
    let arg = find(&d, "graph")
        .args
        .iter()
        .find(|a| a.name == "--planted")
        .expect("the planted argument is declared");
    assert_eq!(arg.kind, "option");
    assert_eq!(arg.value_type, Some("integer"));
    assert!(arg.repeatable);
    assert_eq!(arg.means, "A planted option.");
    assert_eq!(arg.requires, vec!["--other".to_string()]);
}

#[test]
fn every_argument_says_what_it_means() {
    let d = describe(urzua()).unwrap();
    let mut silent: Vec<String> = d
        .args
        .iter()
        .filter(|a| a.means.is_empty())
        .map(|a| a.name.clone())
        .collect();
    for c in &d.commands {
        silent.extend(
            c.args
                .iter()
                .filter(|a| a.means.is_empty())
                .map(|a| format!("{} {}", c.name, a.name)),
        );
    }
    assert!(silent.is_empty(), "arguments with no help text: {silent:?}");
}

/// No option is accepted by every command, so the top level declares none;
/// each global is declared on the commands that accept and honour it.
#[test]
fn a_global_is_declared_only_on_the_commands_that_take_it() {
    let d = describe(urzua()).unwrap();
    assert!(d.args.is_empty());
    let with = |name: &str| -> Vec<&str> {
        d.commands
            .iter()
            .filter(|c| c.args.iter().any(|a| a.name == name))
            .map(|c| c.name.as_str())
            .collect()
    };
    assert_eq!(
        with("--as-of"),
        ["waive", "check", "audit", "fix", "reverify"]
    );
    let as_of = find(&d, "check")
        .args
        .iter()
        .find(|a| a.name == "--as-of")
        .unwrap();
    assert_eq!(as_of.format, Some("date"));
    let config = with("--config");
    assert_eq!(config.len(), d.commands.len() - CONFIG_IGNORED.len());
    assert!(CONFIG_IGNORED.iter().all(|c| !config.contains(c)));
    for c in &d.commands {
        assert!(c.args.iter().all(|a| a.name != "--help"), "{}", c.name);
    }
}

#[test]
fn relations_refused_in_code_are_declared_on_both_arguments() {
    let d = describe(urzua()).unwrap();
    let check = find(&d, "check");
    let arg = |c: &CommandDecl, n: &str| {
        c.args
            .iter()
            .find(|a| a.name == n)
            .unwrap()
            .conflicts_with
            .clone()
    };
    assert_eq!(arg(check, "--stdin-path"), ["paths", "--base", "--actors"]);
    assert!(arg(check, "--base").contains(&"--stdin-path".to_string()));
    let fix = find(&d, "fix");
    let apply = fix.args.iter().find(|a| a.name == "--apply").unwrap();
    assert!(
        apply.requires.is_empty(),
        "needs --ids or --force: only in means"
    );
    assert!(apply.conflicts_with.contains(&"--as-of".to_string()));
    assert_eq!(
        fix.args
            .iter()
            .find(|a| a.name == "--replace")
            .unwrap()
            .requires,
        ["--pin"]
    );
}

#[test]
fn a_code_rule_naming_no_argument_is_refused() {
    let mut args = Vec::new();
    assert!(apply_code_rules("check", &mut args).is_err());
}

#[test]
fn relations_between_arguments_are_read_from_clap() {
    let d = describe(urzua()).unwrap();
    let board = find(&d, "board");
    let port = board.args.iter().find(|a| a.name == "--port").unwrap();
    assert_eq!(port.requires, vec!["--serve".to_string()]);
    assert_eq!(port.conflicts_with, vec!["--prompt".to_string()]);
    let explain = find(&d, "explain");
    let path = explain.args.iter().find(|a| a.name == "path").unwrap();
    assert_eq!(path.kind, "positional");
    assert!(!path.required, "required only unless another is given");
    let export = find(&d, "export");
    let format = export.args.iter().find(|a| a.name == "--format").unwrap();
    assert!(format.required);
    assert_eq!(format.values, vec!["agdr".to_string(), "json".to_string()]);
}

#[test]
fn the_protocol_commands_are_exempt_and_name_their_protocol() {
    let d = describe(urzua()).unwrap();
    for (name, protocol) in [
        ("mcp", "mcp"),
        ("harness hook", "harness-hook"),
        ("harness run edit", "harness-run"),
        ("hooks run", "git-hook"),
    ] {
        let c = find(&d, name);
        assert_eq!(
            (c.profile, c.protocol),
            ("exempt", Some(protocol)),
            "{name}"
        );
    }
    for c in &d.commands {
        assert_eq!(c.profile == "exempt", c.protocol.is_some(), "{}", c.name);
    }
}

/// The variables `env` leaves out on purpose, each with the reason.
const NOT_DECLARED: &[(&str, &str)] = &[
    (
        "URZUA_BOARD_TEST_BUILD_DELAY_MS",
        "read by a test-seams build only",
    ),
    (
        "URZUA_BOARD_TEST_CHECK_DELAY_MS",
        "read by a test-seams build only",
    ),
    ("PATH", "only used to find programs"),
    ("HOME", "only used to find the home directory"),
    ("TZ", "read by a test of the clock, never by urzua"),
    ("URZUA_TODAY_CHILD", "read by a test of the clock"),
    ("URZUA_UPDATE_GOLDEN", "read by a golden-file test"),
    ("URZUA_WRITE_PLUGIN", "read by a plugin test"),
    (
        "URZUA_TEST_PANIC",
        "read by a debug build only, for the panic test",
    ),
    (
        "URZUA_HOOK_DEADLINE_FOR_TESTS",
        "read by a test-seams build only",
    ),
    (
        "URZUA_HOOK_ENGINE_FOR_TESTS",
        "read by a test-seams build only",
    ),
    (
        "URZUA_HOOK_PANIC_FOR_TESTS",
        "read by a test-seams build only",
    ),
];

/// A read whose name is a variable, and the names it can hold. `None`: names
/// a test-seams build alone reads.
const DYNAMIC_READS: [(&str, &str, Option<&[&str]>); 5] = [
    ("harness/record.rs", "v", Some(&["URZUA_BIN", "URZUA"])),
    ("commands/refs.rs", "k", Some(&["GITHUB_TOKEN", "GH_TOKEN"])),
    // The variables an embedded adapter's manifest requires: its host's.
    ("harness/run.rs", "var", Some(&["CLAUDE_PROJECT_DIR"])),
    ("harness/hook/mod.rs", "name", None),
    ("commands/board_server.rs", "variable", None),
];

fn rust_sources() -> Vec<(std::path::PathBuf, String)> {
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut out = Vec::new();
    let mut stack: Vec<_> = std::fs::read_dir(&crates)
        .unwrap()
        .map(|e| e.unwrap().path().join("src"))
        .filter(|p| p.is_dir())
        .collect();
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs")
                && !path.ends_with("describe/tests.rs")
            {
                let text = std::fs::read_to_string(&path).unwrap();
                out.push((path, text));
            }
        }
    }
    out
}

/// The names the source reads with `env::var`/`var_os` that `env` does not
/// declare and [`NOT_DECLARED`] does not excuse, and each read whose name
/// cannot be resolved. An `URZUA_` literal anywhere counts as a read too, for
/// the names passed to a helper (`test_seam("...")`).
fn undeclared_reads(sources: &[(std::path::PathBuf, String)]) -> Vec<String> {
    let call = regex::Regex::new(r"env::var(?:_os)?\(\s*([^)]*?)\s*\)").unwrap();
    let constant = regex::Regex::new(r#"const ([A-Z_]+): &str = "([^"]+)";"#).unwrap();
    let urzua = regex::Regex::new(r#""(URZUA_[A-Z0-9_]+)""#).unwrap();
    let consts: std::collections::HashMap<String, String> = sources
        .iter()
        .flat_map(|(_, t)| {
            constant
                .captures_iter(t)
                .map(|c| (c[1].to_string(), c[2].to_string()))
        })
        .collect();
    let declared = |n: &str| {
        ENV.iter()
            .chain(crate::harness::ADAPTER_ENV)
            .any(|(d, _)| *d == n)
            || NOT_DECLARED.iter().any(|(d, _)| *d == n)
    };
    let mut problems = std::collections::BTreeSet::new();
    for (path, text) in sources {
        let file = path.to_string_lossy().replace('\\', "/");
        let mut names: Vec<String> = urzua
            .captures_iter(text)
            .map(|c| c[1].to_string())
            .collect();
        for c in call.captures_iter(text) {
            let arg = c[1].trim();
            let ident = arg.rsplit("::").next().unwrap();
            if let Some(name) = arg.strip_prefix('"').and_then(|a| a.strip_suffix('"')) {
                names.push(name.to_string());
            } else if let Some(name) = consts.get(ident) {
                names.push(name.clone());
            } else if let Some((_, _, held)) = DYNAMIC_READS
                .iter()
                .find(|(f, a, _)| file.ends_with(f) && *a == arg)
            {
                names.extend(held.iter().flat_map(|h| h.iter()).map(|n| n.to_string()));
            } else {
                problems.insert(format!(
                    "{file}: env::var({arg}) names no variable this test can read"
                ));
            }
        }
        for n in names.into_iter().filter(|n| !declared(n)) {
            problems.insert(format!("{file}: {n} is read but not declared in env"));
        }
    }
    problems.into_iter().collect()
}

#[test]
fn env_drift() {
    let problems = undeclared_reads(&rust_sources());
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn env_drift_catches_a_planted_read() {
    let mut sources = rust_sources();
    let planted =
        "fn f() { let _ = std::env::var(\"PLANTED_TOKEN\"); let _ = std::env::var_os(who); }";
    sources.push(("crates/planted/src/lib.rs".into(), planted.to_string()));
    let problems = undeclared_reads(&sources);
    assert_eq!(problems.len(), 2, "{problems:#?}");
    assert!(problems.iter().any(|p| p.contains("PLANTED_TOKEN")));
    assert!(problems.iter().any(|p| p.contains("env::var(who)")));
}
