//! `urzua harness hook <adapter> <event>`: the agent's hook command calls the
//! binary directly with the agent's hook event on stdin. The adapter module
//! turns that event into the neutral request of `urzua harness run`, runs the
//! core, and turns the verdict into the agent's hook output (SPEC-24).
//!
//! Nothing is installed in the repository to do this, so nothing there can be
//! edited to weaken it, and nothing needs a hash pinned. The registration
//! table below and the adapter modules it lists are the only code that names
//! an agent.

mod claude_code;
mod hook_json;
mod once;
#[cfg(test)]
mod plugin_tests;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// What an adapter provides: the events it handles, the translator, the hook
/// command of an event, and the output that refuses an edit (or, for an event
/// that cannot refuse, says it was not judged) when the translator could not
/// finish.
struct Adapter {
    name: &'static str,
    events: &'static [&'static str],
    translate: fn(&str, &[u8], Option<&Path>) -> String,
    command: fn(&str) -> Option<String>,
    fail_closed: fn(&str, &str) -> String,
}

const HOOKS: &[Adapter] = &[Adapter {
    name: claude_code::NAME,
    events: claude_code::EVENTS,
    translate: claude_code::translate,
    command: claude_code::hook_command,
    fail_closed: claude_code::fail_closed,
}];

/// The command an agent's settings run for `event` of `adapter`.
pub(crate) fn command(adapter: &str, event: &str) -> Option<String> {
    HOOKS
        .iter()
        .find(|a| a.name == adapter)
        .and_then(|a| (a.command)(event))
}

pub fn run_hook(adapter: &str, event: &str) -> ExitCode {
    let Some(a) = HOOKS
        .iter()
        .find(|a| a.name == adapter && a.events.contains(&event))
    else {
        eprintln!(
            "urzua harness hook: this build has no translator for the adapter '{adapter}' and event '{event}' (it has: {})",
            HOOKS
                .iter()
                .map(|a| format!("{} {}", a.name, a.events.join("|")))
                .collect::<Vec<_>>()
                .join(", ")
        );
        return ExitCode::from(2);
    };
    // A panic here must end in the adapter's refusal, as one document on
    // stdout, never in an exit before anything is printed.
    std::panic::set_hook(Box::new(|info| {
        eprintln!("urzua harness hook: {info}");
    }));
    let caught = std::panic::catch_unwind(|| judge_once(a, event));
    let out = caught.unwrap_or_else(|e| {
        (a.fail_closed)(
            event,
            &format!("the hook failed ({})", panic_text(e.as_ref())),
        )
    });
    use std::io::Write;
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(out.as_bytes());
    let _ = stdout.flush();
    ExitCode::SUCCESS
}

fn panic_text(e: &(dyn std::any::Any + Send)) -> String {
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|t| t.to_string()))
        .unwrap_or_default()
}

fn judge_once(a: &Adapter, event: &str) -> String {
    let mut payload = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut payload);
    let project = project_dir();
    start_deadline(project.as_deref());
    test_panic("outer");
    once::run(project.as_deref(), a.name, event, &payload, || {
        let out = (a.translate)(event, &payload, project.as_deref());
        if TIMED_OUT.load(Ordering::SeqCst) {
            return (a.fail_closed)(
                event,
                "the engine did not answer before the hook's deadline",
            );
        }
        out
    })
}

/// When the hook gives up on the engine: before the agent's own timeout, which
/// lets the call through, so the hook still answers (with a refusal).
static DEADLINE: OnceLock<Instant> = OnceLock::new();
static TIMED_OUT: AtomicBool = AtomicBool::new(false);

/// The margin left before `harness.limits.hook_timeout_seconds` for the hook to
/// print its refusal after it stops waiting.
const DEADLINE_MARGIN: Duration = Duration::from_secs(15);

fn start_deadline(project: Option<&Path>) {
    let timeout =
        super::limits::of(None, project).get(urzua_core::harness::limits::HOOK_TIMEOUT_SECONDS);
    let budget = test_seam("URZUA_HOOK_DEADLINE_FOR_TESTS")
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(timeout).saturating_sub(DEADLINE_MARGIN));
    let _ = DEADLINE.set(Instant::now() + budget);
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

/// `git` in `dir` with only the repository on disk deciding the answer: the
/// variables and config sources that would point it at another repository or
/// work tree are removed, so the session's environment cannot steer which
/// repository a guard reads.
pub(super) fn pinned_git(dir: &Path) -> urzua_io::git::GitRun {
    let mut run = urzua_io::git::GitRun::new()
        .args(["-c", "safe.directory=*"])
        .cwd(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .discard_stderr();
    for v in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_NAMESPACE",
        "GIT_CEILING_DIRECTORIES",
        "GIT_DISCOVERY_ACROSS_FILESYSTEM",
        "GIT_CONFIG",
        "GIT_CONFIG_PARAMETERS",
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_SYSTEM",
    ] {
        run = run.env_remove(v);
    }
    run
}

pub(super) fn git_toplevel(dir: &Path) -> Option<PathBuf> {
    let out = pinned_git(dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let top = String::from_utf8(out.stdout).ok()?;
    let top = top.trim_end_matches('\n');
    (!top.is_empty()).then(|| PathBuf::from(top))
}

/// Absolute against the current directory, normalised
/// lexically, links not followed.
fn lexical_absolute(p: &Path) -> PathBuf {
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(p)
    };
    PathBuf::from(normpath(&joined.to_string_lossy()))
}

/// A path normalised lexically: `.` and repeated slashes dropped, `..` taken
/// back where it can be (never above an absolute root), exactly two leading
/// slashes kept.
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

/// A variable the tests use to steer the hook (a stand-in engine, a shorter
/// deadline, a forced panic). Read only in a build with the `test-seams`
/// feature, which only `cargo test` turns on: an installed binary ignores them,
/// so nothing outside the binary can choose what judges an edit.
fn test_seam(name: &str) -> Option<String> {
    if !cfg!(feature = "test-seams") {
        return None;
    }
    std::env::var(name).ok()
}

fn test_engine() -> Option<Vec<String>> {
    serde_json::from_str(&test_seam("URZUA_HOOK_ENGINE_FOR_TESTS")?).ok()
}

#[cfg(feature = "test-seams")]
fn test_engine_command() -> Option<Command> {
    let argv = test_engine().filter(|a| !a.is_empty())?;
    let mut c = Command::new(&argv[0]);
    c.args(&argv[1..]);
    Some(c)
}

#[cfg(not(feature = "test-seams"))]
fn test_engine_command() -> Option<Command> {
    None
}

/// Panics where the tests ask, so the panic path is tested with a real panic.
pub(super) fn test_panic(at: &str) {
    if test_seam("URZUA_HOOK_PANIC_FOR_TESTS").as_deref() == Some(at) {
        panic!("a panic the tests asked for at {at}");
    }
}

/// The engine command as messages name it.
pub(super) fn engine_display(event: &str) -> String {
    test_engine().map_or_else(|| format!("urzua harness run {event}"), |a| a.join(" "))
}

/// Runs `urzua harness run <event>` (this binary) with `request` on stdin, in
/// `cwd`; the verdict object, or why there is none. The core runs as its own
/// process because it changes directory and keeps per-process state while
/// judging. It is stopped at the hook's deadline.
pub(super) fn engine(
    event: &str,
    request: &serde_json::Value,
    cwd: Option<&str>,
) -> Result<serde_json::Value, String> {
    let mut cmd = match test_engine_command() {
        Some(c) => c,
        None => {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let mut c = Command::new(exe);
            c.args(["harness", "run", event]);
            c
        }
    };
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(d) = cwd.filter(|d| !d.is_empty()) {
        cmd.current_dir(d);
    }
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(request.to_string().as_bytes());
    }
    let mut stdout = child.stdout.take().ok_or("no stdout")?;
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            break;
        }
        if DEADLINE.get().is_some_and(|d| Instant::now() >= *d) {
            TIMED_OUT.store(true, Ordering::SeqCst);
            let _ = child.kill();
            let _ = child.wait();
            return Err("the engine did not answer before the hook's deadline".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let out = reader
        .join()
        .map_err(|_| "the engine's output could not be read")?;
    let verdict: serde_json::Value = serde_json::from_slice(&out).map_err(|e| e.to_string())?;
    if !verdict
        .as_object()
        .is_some_and(|o| o.contains_key("verdict"))
    {
        return Err("the engine did not answer with a verdict".into());
    }
    Ok(verdict)
}

/// The truthiness the adapter gives a JSON value: empty and zero are false.
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
    fn a_path_is_normalised_lexically() {
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
