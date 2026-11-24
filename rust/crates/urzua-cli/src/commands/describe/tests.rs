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

#[test]
fn the_globals_are_declared_once_at_the_top() {
    let d = describe(urzua()).unwrap();
    let top: Vec<&str> = d.args.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(top, ["--config", "--as-of"]);
    let as_of = &d.args[1];
    assert_eq!(as_of.format, Some("date"));
    assert_eq!(d.args[0].value_type, Some("path"));
    for c in &d.commands {
        assert!(
            c.args.iter().all(|a| !top.contains(&a.name.as_str())),
            "`{}` repeats a global",
            c.name
        );
        assert!(c.args.iter().all(|a| a.name != "--help"));
    }
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

/// Every `URZUA_`-prefixed variable named in the source is declared in `env`
/// or is one only a test build reads.
#[test]
fn env_drift() {
    const TEST_ONLY: [&str; 9] = [
        "URZUA_TEST_PANIC",
        "URZUA_TODAY_CHILD",
        "URZUA_UPDATE_GOLDEN",
        "URZUA_WRITE_PLUGIN",
        "URZUA_HOOK_DEADLINE_FOR_TESTS",
        "URZUA_HOOK_ENGINE_FOR_TESTS",
        "URZUA_HOOK_PANIC_FOR_TESTS",
        "URZUA_BOARD_TEST_BUILD_DELAY_MS",
        "URZUA_BOARD_TEST_CHECK_DELAY_MS",
    ];
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let literal = regex::Regex::new(r#""(URZUA_[A-Z0-9_]+)""#).unwrap();
    let mut undeclared = std::collections::BTreeSet::new();
    let mut stack = vec![crates];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path
                    .file_name()
                    .is_some_and(|n| n != "tests" && n != "target")
                {
                    stack.push(path);
                }
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                for name in literal.captures_iter(&text).map(|c| c[1].to_string()) {
                    if !ENV.iter().any(|(n, _)| *n == name) && !TEST_ONLY.contains(&name.as_str()) {
                        undeclared.insert(name);
                    }
                }
            }
        }
    }
    assert!(
        undeclared.is_empty(),
        "read but not declared in env: {undeclared:?}"
    );
}
