//! `urzua harness hook <adapter> <event>`: the agent's hook command calls the
//! binary directly with the agent's hook event on stdin. The adapter module
//! turns that event into the neutral request of `urzua harness run`, runs the
//! core, and turns the verdict into the agent's hook output (SPEC-24).
//!
//! Nothing is installed in the repository to do this, so nothing there can be
//! edited to weaken it, and nothing needs a hash pinned (`ADR-89`, amendment of
//! 2026-11-17). Only the adapter modules name an agent.

mod claude_code;
mod once;
#[cfg(test)]
mod plugin_tests;
mod pyjson;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

/// (adapter, events it handles, translator, hook command of an event).
type Translator = fn(&str, &[u8], Option<&Path>) -> String;
type HookCommand = fn(&str) -> Option<String>;
const HOOKS: &[(&str, &[&str], Translator, HookCommand)] = &[(
    claude_code::NAME,
    claude_code::EVENTS,
    claude_code::translate,
    claude_code::hook_command,
)];

/// The command an agent's settings run for `event` of `adapter`.
pub(crate) fn command(adapter: &str, event: &str) -> Option<String> {
    HOOKS
        .iter()
        .find(|(a, ..)| *a == adapter)
        .and_then(|(_, _, _, command)| command(event))
}

pub fn run_hook(adapter: &str, event: &str) -> ExitCode {
    let Some((_, _, translate, _)) = HOOKS
        .iter()
        .find(|(a, events, ..)| *a == adapter && events.contains(&event))
    else {
        eprintln!(
            "urzua harness hook: this build has no translator for the adapter '{adapter}' and event '{event}' (it has: {})",
            HOOKS
                .iter()
                .map(|(a, e, ..)| format!("{a} {}", e.join("|")))
                .collect::<Vec<_>>()
                .join(", ")
        );
        return ExitCode::from(2);
    };
    let mut payload = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut payload);
    let project = project_dir();
    let out = once::run(project.as_deref(), adapter, event, &payload, || {
        translate(event, &payload, project.as_deref())
    });
    print!("{out}");
    ExitCode::SUCCESS
}

/// The project a hook judges first: the directory the agent names for the
/// session (the adapter's `PROJECT_ENV`) when it sets one, else the nearest git repository at or
/// above the current directory that holds a configuration, else the current
/// directory. It is the session's project only when it holds
/// `.urzua/config.yaml`.
fn project_dir_from(var: Option<String>) -> Option<PathBuf> {
    let governed = |d: &Path| d.join(".urzua/config.yaml").is_file();
    if let Some(d) = var.filter(|v| !v.is_empty()) {
        let d = lexical_absolute(Path::new(&d));
        return governed(&d).then_some(d);
    }
    let here = std::env::current_dir().ok()?;
    let mut d = here.clone();
    loop {
        let Some(top) = git_toplevel(&d) else { break };
        if governed(&top) {
            return Some(top);
        }
        match top.parent() {
            Some(p) if !p.as_os_str().is_empty() => d = p.to_path_buf(),
            _ => break,
        }
    }
    governed(&here).then_some(here)
}

fn project_dir() -> Option<PathBuf> {
    project_dir_from(std::env::var(claude_code::PROJECT_ENV).ok())
}

fn git_toplevel(dir: &Path) -> Option<PathBuf> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--show-toplevel"])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let top = String::from_utf8(out.stdout).ok()?;
    let top = top.trim_end_matches('\n');
    (!top.is_empty()).then(|| PathBuf::from(top))
}

/// `os.path.abspath`: absolute against the current directory, normalised
/// lexically, links not followed.
fn lexical_absolute(p: &Path) -> PathBuf {
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(p)
    };
    PathBuf::from(normpath(&joined.to_string_lossy()))
}

/// `posixpath.normpath`.
pub(super) fn normpath(path: &str) -> String {
    if path.is_empty() {
        return ".".to_string();
    }
    let lead = path.chars().take_while(|c| *c == '/').count();
    let prefix = match lead {
        0 => "",
        2 => "//",
        _ => "/",
    };
    let mut parts: Vec<&str> = Vec::new();
    for comp in path.split('/') {
        match comp {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|l| *l != "..") {
                    parts.pop();
                } else if prefix.is_empty() {
                    parts.push("..");
                }
            }
            c => parts.push(c),
        }
    }
    let joined = format!("{prefix}{}", parts.join("/"));
    if joined.is_empty() {
        ".".to_string()
    } else {
        joined
    }
}

/// The variable a debug build's tests name a stand-in engine with (a JSON list
/// of argv): an engine that fails, or one that logs the requests it is sent. A
/// release build never reads it, so nothing outside the binary can choose what
/// judges an edit.
const TEST_ENGINE: &str = "URZUA_HOOK_ENGINE_FOR_TESTS";

fn test_engine() -> Option<Vec<String>> {
    if !cfg!(debug_assertions) {
        return None;
    }
    serde_json::from_str(&std::env::var(TEST_ENGINE).ok()?).ok()
}

/// The engine command as messages name it.
pub(super) fn engine_display(event: &str) -> String {
    test_engine().map_or_else(|| format!("urzua harness run {event}"), |a| a.join(" "))
}

/// Runs `urzua harness run <event>` (this binary) with `request` on stdin, in
/// `cwd`; the verdict object, or why there is none. The core runs as its own
/// process, as it did under the script: it changes directory and keeps
/// per-process state while judging.
pub(super) fn engine(
    event: &str,
    request: &serde_json::Value,
    cwd: Option<&str>,
) -> Result<serde_json::Value, String> {
    let mut cmd = match test_engine() {
        Some(argv) if !argv.is_empty() => {
            let mut c = Command::new(&argv[0]);
            c.args(&argv[1..]);
            c
        }
        _ => {
            let mut c = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
            c.args(["harness", "run", event]);
            c
        }
    };
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(d) = cwd.filter(|d| !d.is_empty()) {
        cmd.current_dir(d);
    }
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(request.to_string().as_bytes());
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let verdict: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    if !verdict.as_object().is_some_and(|o| o.contains_key("verdict")) {
        return Err("the engine did not answer with a verdict".into());
    }
    Ok(verdict)
}

/// Python's truthiness of a JSON value.
pub(super) fn truthy(v: Option<&serde_json::Value>) -> bool {
    use serde_json::Value as V;
    match v {
        None | Some(V::Null) => false,
        Some(V::Bool(b)) => *b,
        Some(V::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
        Some(V::String(s)) => !s.is_empty(),
        Some(V::Array(a)) => !a.is_empty(),
        Some(V::Object(o)) => !o.is_empty(),
    }
}

/// A string field of a verdict, `""` when absent or not a string.
pub(super) fn text<'a>(v: &'a serde_json::Value, key: &str) -> &'a str {
    v.get(key).and_then(|m| m.as_str()).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normpath_matches_posixpath() {
        for (i, o) in [
            ("", "."),
            ("/", "/"),
            ("//", "//"),
            ("///a", "/a"),
            ("a/./b/../c", "a/c"),
            ("../a", "../a"),
            ("/../a", "/a"),
            ("a/..", "."),
            ("//a//b/", "//a/b"),
        ] {
            assert_eq!(normpath(i), o, "{i}");
        }
    }
}
