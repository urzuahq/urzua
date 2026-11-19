//! The guardrail cores (`ADR-89` section 1): `urzua harness run edit` and
//! `urzua harness run session-start`.
//!
//! They speak a neutral request on stdin and a neutral verdict on stdout, and
//! know no agent. An adapter (`urzua harness hook`) turns an agent's hook payload into the
//! request and the verdict back into the agent's output; everything that used to
//! fail open in a `jq`, `bash` and `python3` chain (no `jq`, no binary, an
//! unreadable payload) is a verdict stated here, which `on_unjudged` can turn
//! from allow into deny for an agent.
//!
//! Request: `{event, tool, paths, content, actor, cwd, override?}`.
//! Verdict: `{verdict, path?, message, findings, override?, content?}` with
//! `verdict` one of `allow`, `allow-with-warnings`, `block`, `allow-unjudged`,
//! `allow-override`, `not-governed` and `ask` (the adapter asks the person
//! through the host and, on approval, writes `content`). Exit 0 for every
//! verdict but `block` (exit 1).

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::{Deserialize, Serialize};
use urzua_core::config::Config;
use urzua_core::harness::OnUnjudged;
use urzua_core::report::{
    ActionKind, CheckReport, Finding, FindingSeverity, Notice, Report, ReportStatus, RuleStatus,
};

use super::gate;
use super::lock;
use super::settings;
use crate::commands::check::judge_proposed;
use crate::emit;
use crate::session::Workspace;

mod ask;
mod outside;
pub(super) mod unjudged;

/// Who is acting. An absent actor is treated as an agent: the safer reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Actor {
    Agent,
    Human,
}

#[derive(Deserialize, Default, Clone)]
struct OverrideRequest {
    name: Option<String>,
    reason: Option<String>,
}

#[derive(Deserialize, Default, Clone)]
struct Request {
    // The adapter also sends `event` and `tool`; the engine takes the event
    // from its command line, so serde ignores them (SPEC-24).
    #[serde(default)]
    paths: Vec<String>,
    content: Option<String>,
    /// For a `gates_only` request naming several paths: the proposed content
    /// of each, parallel to `paths`. A path without an entry (or with a null
    /// one) is judged with `content`.
    #[serde(default)]
    contents: Vec<Option<String>>,
    actor: Option<String>,
    cwd: Option<String>,
    /// The directory the session was started in (the translator's project
    /// directory), when the adapter knows it: the repository the session
    /// belongs to is judged first, whatever the file's own place suggests.
    #[serde(default)]
    project_dir: Option<String>,
    #[serde(rename = "override")]
    override_request: Option<OverrideRequest>,
    /// Set by an adapter whose translator could not build a request (an
    /// unreadable payload, an edit it could not place): the engine decides what
    /// the guardrails do about a call nobody judged.
    #[serde(default)]
    adapter_error: Option<String>,
    /// Judge only the acceptance and protected-path gates, which answer in
    /// milliseconds, and say whether the full editor-hook judgement of this
    /// path would run a check (`record` in the verdict). An adapter that has
    /// several files to judge asks this of each first, so a denial returns
    /// before any record is checked.
    #[serde(default)]
    gates_only: bool,
    /// What the adapter knows about asking the person at the keyboard about
    /// this call (`ADR-102`); absent from an adapter that cannot ask.
    #[serde(default)]
    ask: Option<ask::AskRequest>,
    /// Set while the session's own repository judges a file it holds: there an
    /// editor hook its configuration turned off says nothing. Anywhere else (a
    /// linked worktree, an outer repository) the unjudged edit stays loud.
    #[serde(skip)]
    off_is_quiet: bool,
}

impl Request {
    /// The proposed content of the path at `index`: its own entry in
    /// `contents`, else the request's `content`.
    fn content_at(&self, index: usize) -> Option<&str> {
        self.contents
            .get(index)
            .and_then(|c| c.as_deref())
            .or(self.content.as_deref())
    }

    fn actor(&self) -> Actor {
        match self.actor.as_deref() {
            Some("human") => Actor::Human,
            _ => Actor::Agent,
        }
    }
}

#[derive(Serialize)]
struct Verdict {
    verdict: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    message: String,
    findings: Vec<serde_json::Value>,
    #[serde(rename = "override", skip_serializing_if = "Option::is_none")]
    override_name: Option<String>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    extra: Option<Box<Extra>>,
}

/// What a verdict carries beyond its message, boxed to keep `Verdict` small.
#[derive(Serialize)]
#[serde(untagged)]
enum Extra {
    Gates(GatesAnswer),
    /// Text for the agent alone: the adapter passes it on an `allow` session
    /// start, which shows the person nothing.
    Agent {
        agent: &'static str,
    },
    /// For `ask`: the file as the approved edit writes it, receipt included.
    Ask {
        content: String,
    },
    /// The move a refusal is about, when a person may answer it in the session.
    Gated {
        #[serde(skip)]
        gated_move: gate::GatedMove,
    },
}

/// The answer to a `gates_only` request that no gate denied: whether the full
/// judgement of the path checks a record (the costly part), and, for a request
/// that names several paths, the same per path in the order of the request.
#[derive(Serialize)]
struct GatesAnswer {
    record: bool,
    records: Vec<bool>,
}

impl Verdict {
    fn new(verdict: &'static str, path: Option<&str>, message: String) -> Self {
        Verdict {
            verdict,
            path: path.map(str::to_string),
            message,
            findings: Vec::new(),
            override_name: None,
            extra: None,
        }
    }
}

impl Report for Verdict {
    fn notices(&self) -> &[Notice] {
        &[]
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(if self.verdict == "block" { 1 } else { 0 })
    }
}

fn read_request() -> Result<Request, String> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| format!("could not read the request from stdin: {e}"))?;
    serde_json::from_str(&input).map_err(|e| format!("the request is not valid JSON: {e}"))
}

/// What a guardrail does when it cannot judge, read leniently from the raw
/// configuration for the case where the configuration itself does not load:
/// a broken file must not silently turn a denying guardrail into an allowing one.
fn raw_policy(config_path: Option<&Path>, root_hint: &Path, guardrail: &str) -> (bool, OnUnjudged) {
    let path = config_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root_hint.join(".urzua/config.yaml"));
    let declared = std::fs::read_to_string(path)
        .ok()
        .and_then(|t| {
            yaml_serde::from_str::<yaml_serde::Value>(urzua_core::config::strip_bom(&t)).ok()
        })
        .and_then(|v| v.get("harness")?.get("guardrails")?.get(guardrail).cloned());
    let Some(g) = declared else {
        return (false, OnUnjudged::Allow);
    };
    let enabled = g.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
    let default = urzua_core::harness::guardrail_info(guardrail)
        .map(|i| i.default_on_unjudged)
        .unwrap_or(OnUnjudged::Allow);
    let on = match g.get("on_unjudged").and_then(|o| o.as_str()) {
        Some("deny") => OnUnjudged::Deny,
        Some("allow") => OnUnjudged::Allow,
        _ => default,
    };
    (enabled, on)
}

/// An agent's write inside a git directory outside the working tree (a
/// separate git directory, a symlinked `.git`, a submodule's, a bare
/// repository's), refused before anything else is tried, so no route that
/// cannot place or judge the edit lets it through. A git directory inside the
/// working tree is the engine-owned `.git` entry, refused by the gates.
fn git_dir_first(config: Option<&Path>, request: &Request) -> Option<Verdict> {
    let cwd = std::env::current_dir().ok()?;
    let root = request
        .project_dir
        .as_deref()
        .map(PathBuf::from)
        .filter(|d| d.join(".urzua/config.yaml").is_file())
        .or_else(|| nearest_governed_root(&cwd))?;
    if !raw_policy(config, &root, "protected-paths").0 {
        return None;
    }
    request.paths.iter().find_map(|p| {
        let in_tree =
            crate::discovery::relative_to_root(&root, &absolute(&cwd, p)).is_ok_and(|rel| {
                rel.components()
                    .any(|c| gate::fold_path(&c.as_os_str().to_string_lossy()) == ".git")
            });
        if in_tree {
            return None;
        }
        gate::git_dir_refusal(Path::new(p)).map(|d| denied(&d, p, request))
    })
}

/// What the editor hook does with an edit it cannot read a request for, or
/// whose request names a directory that cannot be entered: the declaration is
/// read from the process's own directory, so a denying guardrail stays denying.
fn on_unreadable_request(config: Option<&Path>) -> OnUnjudged {
    let here = std::env::current_dir().unwrap_or_default();
    match raw_policy(config, &here, "editor-hook") {
        (true, on) => on,
        (false, _) => OnUnjudged::Allow,
    }
}

const HATCH: &str = "To repair it, a person edits outside the agent harness (a plain editor or a terminal), where no hook runs; the override of ADR-78 is not accepted for this verdict. Or set on_unjudged: allow for the guardrail in .urzua/config.yaml, and the loud message remains.";

/// The verdict for an edit the engine could not judge: allowed with a loud
/// message, or, under `on_unjudged: deny`, refused for an agent.
fn unjudged(on: OnUnjudged, actor: Actor, rel: Option<&str>, why: &str) -> Verdict {
    unjudged_as("editor-hook", on, actor, rel, why)
}

/// `unjudged` for the named guardrail (`editor-hook`, `acceptance`,
/// `protected-paths`).
fn unjudged_as(
    guardrail: &str,
    on: OnUnjudged,
    actor: Actor,
    rel: Option<&str>,
    why: &str,
) -> Verdict {
    let what = rel.map_or("this edit".to_string(), |r| format!("this edit to {r}"));
    match (on, actor) {
        (OnUnjudged::Deny, Actor::Agent) => Verdict::new(
            "block",
            rel,
            format!(
                "urzua harness ({guardrail}): urzua did NOT judge {what} ({why}), and the guardrail is set to on_unjudged: deny, so an agent's edit is refused. {HATCH}"
            ),
        ),
        (OnUnjudged::Deny, Actor::Human) => Verdict::new(
            "allow-unjudged",
            rel,
            format!(
                "urzua harness ({guardrail}): urzua did NOT judge {what} ({why}). The actor is a person, so the edit is allowed even though the guardrail is set to on_unjudged: deny; run 'urzua check' before committing."
            ),
        ),
        (OnUnjudged::Allow, _) => Verdict::new(
            "allow-unjudged",
            rel,
            format!(
                "urzua harness ({guardrail}): urzua did NOT judge {what} ({why}). The edit is allowed; run 'urzua check' before committing."
            ),
        ),
    }
}

/// The verdict for an agent's edit the authority table refuses (`ADR-89`
/// section 4). Final: the override of `ADR-78` is a name in an environment
/// variable the agent controls, so it is not accepted here, and the output says
/// so; nothing is written to the override log.
fn denied(denial: &gate::Denial, rel: &str, request: &Request) -> Verdict {
    let named = request
        .override_request
        .as_ref()
        .and_then(|o| o.name.clone())
        .or_else(|| std::env::var("URZUA_HOOK_OVERRIDE").ok())
        .filter(|n| !n.trim().is_empty());
    let override_note = match named {
        Some(name) => format!(
            " The override '{name}' you named was NOT applied and nothing was written to the override log."
        ),
        None => String::new(),
    };
    let mut v = Verdict::new(
        "block",
        Some(rel),
        format!(
            "urzua harness ({}): refused. {}. A person makes this change by editing outside the agent harness (a plain editor or a terminal), where no hook runs. The override of ADR-78 is not accepted for this verdict.{override_note}",
            denial.guardrail, denial.reason
        ),
    );
    if let Some(mv) = denial.gated_move.clone() {
        v.extra = Some(Box::new(Extra::Gated { gated_move: mv }));
    }
    v
}

/// When the configuration does not load, the raw file still says what the
/// denying gates protect: an agent's edit to the configuration or a gated path
/// is refused rather than allowed because the gate could not be read.
fn raw_gate_denial(config: Option<&Path>, request: &Request, why: &str) -> Option<Verdict> {
    if request.actor() != Actor::Agent {
        return None;
    }
    let root = crate::discovery::find_repo_root(&[PathBuf::from(".")]).ok()?;
    let config_path = config
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::discovery::default_config_path(&root));
    let raw = gate::raw_gates(&std::fs::read_to_string(&config_path).ok()?)?;
    let rel = crate::discovery::relative_to_root(&root, Path::new(request.paths.first()?)).ok()?;
    let config_rel = crate::discovery::relative_to_root(&root, &config_path).ok();
    let guardrail = raw.covers(&rel, config_rel.as_deref())?;
    Some(unjudged_as(
        guardrail,
        OnUnjudged::Deny,
        Actor::Agent,
        Some(&rel.to_string_lossy()),
        &format!("{why}; the {guardrail} guardrail cannot read what it protects"),
    ))
}

// ------------------------------------------------------------------- edit

pub fn run_edit(config: Option<PathBuf>) -> ExitCode {
    let verdict = match read_request() {
        Ok(request) => judge_edit(config, request),
        Err(why) => cannot_place(config.as_deref(), Actor::Agent, &why),
    };
    emit(&verdict)
}

/// The verdict for an edit whose request cannot be placed in a repository (it
/// cannot be read, or the directory it names cannot be entered), so nothing
/// about the file is known: an agent's edit is refused where a denying
/// `acceptance` or `protected-paths` guardrail covers what it cannot judge, as
/// `adapter_failure` does for an adapter that could not build a request;
/// otherwise the editor hook's own `on_unjudged` decides.
fn cannot_place(config: Option<&Path>, actor: Actor, why: &str) -> Verdict {
    if actor == Actor::Agent {
        if let Some(verdict) = targeted_denial(config, &Request::default(), why) {
            return verdict;
        }
    }
    unjudged(on_unreadable_request(config), actor, None, why)
}

/// The verdict for a call the adapter could not turn into a request: refused
/// for an agent under `on_unjudged: deny`, else allowed with the adapter's loud
/// message. An adapter's own failure never blocks when the configuration says
/// allow. With no file named (an unreadable payload) any denying `acceptance` or
/// `protected-paths` guardrail refuses. With a file named (an edit the
/// translator could not place) the call is refused only when that file is a
/// protected path or a record of a gated type: the edit tool's own failure
/// handles an absent `old_string` anywhere else, and the loud message stays.
fn adapter_failure(config: Option<&Path>, request: &Request, why: &str) -> Verdict {
    let path = request.paths.first().map(String::as_str);
    if let Some(verdict) = targeted_denial(config, request, why) {
        return verdict;
    }
    Verdict::new(
        "allow-unjudged",
        path,
        format!("urzua harness: urzua did NOT judge this call ({why}). It is allowed; run 'urzua harness status' and 'urzua check' before committing."),
    )
}

/// An agent's refusal of a request urzua cannot judge, when a guardrail set to
/// `on_unjudged: deny` covers a path it names (or, with no path, a gate that
/// denies what it cannot judge).
fn targeted_denial(config: Option<&Path>, request: &Request, why: &str) -> Option<Verdict> {
    let here = std::env::current_dir().unwrap_or_default();
    let root = crate::discovery::find_repo_root(&[PathBuf::from(".")]).unwrap_or(here);
    let path = request.paths.first().map(String::as_str);
    let guardrail = match path {
        None => ["protected-paths", "acceptance"]
            .into_iter()
            .find(|g| matches!(raw_policy(config, &root, g), (true, OnUnjudged::Deny))),
        Some(_) => {
            // The raw configuration is read once, however many paths the
            // request names.
            let covers = TargetedGates::read(config, &root)?;
            request
                .paths
                .iter()
                .find_map(|p| covers.guardrail(Path::new(p)))
        }
    };
    guardrail.map(|g| unjudged_as(g, OnUnjudged::Deny, Actor::Agent, path, why))
}

/// The raw gates of the configuration, for naming the denying guardrail that
/// covers a path: a protected path, the configuration itself, or a record
/// directory of a gated type.
struct TargetedGates {
    root: PathBuf,
    raw: gate::RawGates,
    config_rel: Option<PathBuf>,
    entries: Vec<String>,
}

impl TargetedGates {
    fn read(config: Option<&Path>, root: &Path) -> Option<Self> {
        let config_path = config
            .map(Path::to_path_buf)
            .unwrap_or_else(|| crate::discovery::default_config_path(root));
        let raw = gate::raw_gates(&std::fs::read_to_string(&config_path).ok()?)?;
        let config_rel = crate::discovery::relative_to_root(root, &config_path).ok();
        let mut entries = raw.paths.clone();
        entries.extend(raw.owned.iter().cloned());
        Some(TargetedGates {
            root: root.to_path_buf(),
            raw,
            config_rel,
            entries,
        })
    }

    /// The denying guardrail that covers `path`, judging the path as written
    /// and as it resolves.
    fn guardrail(&self, path: &Path) -> Option<&'static str> {
        let lexical = crate::discovery::relative_to_root(&self.root, path).ok();
        let resolved = resolved_rel(&self.root, path);
        let linked = gate::hardlink_aliases(&self.root, path, &self.entries, &self.raw.gated_dirs);
        lexical
            .iter()
            .chain(resolved.iter())
            .chain(linked.iter())
            .find_map(|rel| self.raw.covers(rel, self.config_rel.as_deref()))
    }
}

/// A relative path made absolute against `cwd`: the directory it is relative
/// to is left when a repository is entered.
fn absolute(cwd: &Path, p: &str) -> PathBuf {
    let p = Path::new(p);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    }
}

/// The repositories an edit is judged against, in the order they are tried
/// (`ADR-89`, amendment of 2026-11-08): the session's own repository (the one
/// that holds the directory the session started in, else the one that holds the
/// current directory), then every governed repository that holds the file the
/// edit writes, inner or outer, with symlinks followed (a link can land
/// somewhere its own place does not suggest). A deny from any of them is the
/// verdict. A repository the session did not start in can therefore add
/// restrictions and never remove them: two unjudged writes outside the
/// session's repository (an empty `.git`, a `.urzua/config.yaml`) mint an outer
/// "repository" that holds nothing the session's own gates do not still judge.
fn judging_roots(request: &Request, cwd: &Path) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    let mut push = |r: PathBuf| {
        if !roots.contains(&r) {
            roots.push(r);
        }
    };
    if let Some(r) = request
        .project_dir
        .as_deref()
        .and_then(|d| nearest_governed_root(Path::new(d)))
    {
        push(r);
    }
    if let Some(r) = nearest_governed_root(cwd) {
        push(r);
    }
    for p in &request.paths {
        let abs = absolute(cwd, p);
        for start in [resolve_links(&abs), Some(abs)].into_iter().flatten() {
            for r in governed_roots(&start) {
                push(r);
            }
        }
    }
    roots
}

/// Whether no `.urzua/config.yaml` exists at or above the current directory,
/// `project_dir`, or any of `paths` (relative to the current directory), with
/// symlinks followed: the files are in no repository that uses urzua, so an
/// edit or a command there is not judged and says nothing.
pub(super) fn ungoverned(project_dir: Option<&str>, paths: &[String]) -> bool {
    let Ok(cwd) = std::env::current_dir() else {
        return false;
    };
    let mut starts = vec![cwd.clone()];
    starts.extend(project_dir.map(PathBuf::from));
    for p in paths {
        let abs = absolute(&cwd, p);
        starts.extend(resolve_links(&abs));
        starts.push(abs);
    }
    !starts.iter().any(|s| {
        s.ancestors()
            .any(|a| crate::discovery::default_config_path(a).exists())
    })
}

/// Whether the file the edit writes is inside `root`, as written or once its
/// links are followed.
fn holds(root: &Path, cwd: &Path, request: &Request) -> bool {
    let Ok(root) = root.canonicalize() else {
        return false;
    };
    request.paths.iter().any(|p| {
        let abs = absolute(cwd, p);
        resolve_links(&abs).is_some_and(|r| r.starts_with(&root)) || abs.starts_with(&root)
    })
}

/// The directories at or above `start` that are governed repositories,
/// innermost first. `start` may be a file, or a path that does not exist yet
/// (its nearest existing directory is used).
pub(super) fn governed_roots(start: &Path) -> Vec<PathBuf> {
    let mut dir = start;
    while !dir.is_dir() {
        match dir.parent() {
            Some(p) => dir = p,
            None => return Vec::new(),
        }
    }
    let Ok(dir) = dir.canonicalize() else {
        return Vec::new();
    };
    dir.ancestors()
        .filter(|d| crate::discovery::default_config_path(d).is_file() && is_git_root(d))
        .map(Path::to_path_buf)
        .collect()
}

/// The nearest governed repository at or above `start`.
pub(super) fn nearest_governed_root(start: &Path) -> Option<PathBuf> {
    governed_roots(start).into_iter().next()
}

thread_local! {
    /// What `is_git_root` found for each directory this process asked about: a
    /// request names many paths under a few directories, and each answer costs
    /// a git process.
    static GIT_ROOTS: std::cell::RefCell<std::collections::HashMap<PathBuf, bool>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    /// How many answers were worked out rather than remembered (tests only).
    #[cfg(test)]
    static GIT_ROOT_SPAWNS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Starts the memory of `is_git_root` afresh.
fn forget_git_roots() {
    GIT_ROOTS.with(|m| m.borrow_mut().clear());
}

/// Whether `dir` is the top of a real git working tree: `.git` exists and git
/// itself accepts it. An empty file or directory named `.git` is not one (git
/// rejects it, or walks past it to a repository above), so a file an agent can
/// write cannot make a directory a repository. Remembered per directory for the
/// life of one request (`forget_git_roots`).
fn is_git_root(dir: &Path) -> bool {
    if let Some(known) = GIT_ROOTS.with(|m| m.borrow().get(dir).copied()) {
        return known;
    }
    #[cfg(test)]
    GIT_ROOT_SPAWNS.with(|c| c.set(c.get() + 1));
    let answer = git_accepts(dir);
    GIT_ROOTS.with(|m| m.borrow_mut().insert(dir.to_path_buf(), answer));
    answer
}

fn git_accepts(dir: &Path) -> bool {
    if !dir.join(".git").exists() {
        return false;
    }
    let Ok(out) = urzua_io::git::GitRun::new()
        .args(["-c", "safe.directory=*", "rev-parse", "--show-toplevel"])
        .cwd(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
    else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    let top = String::from_utf8_lossy(&out.stdout);
    match (Path::new(top.trim()).canonicalize(), dir.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// The repository that holds the file when none of the governed ones does: the
/// edit is judged there (it has no gates, and says so).
fn plain_root(request: &Request, cwd: &Path) -> Option<PathBuf> {
    let first = request.paths.first()?;
    if !Path::new(first).is_absolute() {
        return None;
    }
    let first = absolute(cwd, first);
    let dir = first.ancestors().skip(1).find(|d| d.is_dir())?;
    crate::discovery::find_repo_root(&[dir.to_path_buf()]).ok()
}

/// One step of walking a path.
enum Step {
    /// A root (`/`, or a drive prefix and root): start again from it.
    Reset(PathBuf),
    Parent,
    Name(std::ffi::OsString),
}

fn steps(path: &Path) -> Vec<Step> {
    use std::path::Component;
    let mut head = PathBuf::new();
    let mut rest = Vec::new();
    for c in path.components() {
        match c {
            Component::Prefix(_) | Component::RootDir => head.push(c.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => rest.push(Step::Parent),
            Component::Normal(n) => rest.push(Step::Name(n.to_owned())),
        }
    }
    if !head.as_os_str().is_empty() {
        rest.insert(0, Step::Reset(head));
    }
    rest
}

/// `abs` with every symbolic link followed, including a link whose target does
/// not exist yet: a write through it creates the target, so the target is the
/// file written. `canonicalize` cannot do that, because it fails on a dangling
/// link and the fallback of resolving the parent only then keeps the link's own
/// name. `None` after more than `harness.limits.symlink_hops` links (a loop).
fn resolve_links(abs: &Path) -> Option<PathBuf> {
    let mut queue: std::collections::VecDeque<Step> = steps(abs).into();
    let mut cur = PathBuf::new();
    let mut followed = 0;
    let hops = super::limits::get(urzua_core::harness::limits::SYMLINK_HOPS);
    while let Some(step) = queue.pop_front() {
        match step {
            Step::Reset(root) => cur = root,
            Step::Parent => {
                cur.pop();
            }
            Step::Name(name) => {
                let next = cur.join(&name);
                if std::fs::symlink_metadata(&next).is_ok_and(|m| m.file_type().is_symlink()) {
                    followed += 1;
                    if followed > hops {
                        return None;
                    }
                    let target = std::fs::read_link(&next).ok()?;
                    for step in steps(&target).into_iter().rev() {
                        queue.push_front(step);
                    }
                } else {
                    cur = next;
                }
            }
        }
    }
    Some(cur)
}

/// `path` relative to the repository root with symlinks resolved, so an edit
/// through a link is judged as the file it writes, a link to a file that does
/// not exist yet included. `None` when the path cannot be resolved or resolves
/// outside the repository.
fn resolved_rel(repo_root: &Path, path: &Path) -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let root = repo_root.canonicalize().ok()?;
    let full = resolve_links(&abs)?;
    full.strip_prefix(&root).ok().map(PathBuf::from)
}

/// The most paths of one request that are judged
/// (`harness.limits.paths_per_call`). Each costs a search for the governed
/// repositories that hold it, so a request with more is an edit that was not
/// judged, as the translator makes a call with more files: the first are judged
/// against the gates and the rest are refused under `on_unjudged: deny` where a
/// guardrail covers one of them.
fn max_paths() -> usize {
    super::limits::get(urzua_core::harness::limits::PATHS_PER_CALL)
}

fn judge_edit(config: Option<PathBuf>, mut request: Request) -> Verdict {
    let actor = request.actor();
    if let Some(cwd) = &request.cwd {
        // A directory that cannot be entered (gone, not a directory, not
        // readable) is replaced by the session's own project directory, where
        // the adapter names one; with neither, nothing can be placed.
        let entered = std::env::set_current_dir(cwd).is_ok()
            || request
                .project_dir
                .as_deref()
                .is_some_and(|d| std::env::set_current_dir(d).is_ok());
        if !entered {
            return cannot_place(
                config.as_deref(),
                actor,
                &format!("the request's cwd '{cwd}' cannot be entered"),
            );
        }
    }
    // The session's own configuration bounds how much of the request is read,
    // whichever repository later judges each file.
    super::limits::activate(&super::limits::of(
        config.clone(),
        request.project_dir.as_deref().map(Path::new),
    ));
    if actor == Actor::Agent {
        if let Some(v) = git_dir_first(config.as_deref(), &request) {
            return v;
        }
    }
    let Ok(cwd) = std::env::current_dir() else {
        return judge_edit_in(config, request);
    };
    if config.is_none() && ungoverned(request.project_dir.as_deref(), &request.paths) {
        return Verdict::new(
            "allow",
            request.paths.first().map(String::as_str),
            String::new(),
        );
    }
    // The directories of this process are looked at once each.
    forget_git_roots();
    let total = request.paths.len();
    let overflow = take_overflow(&mut request, &cwd);
    let verdict = judge_in_roots(config.clone(), request.clone(), &cwd);
    if overflow.is_empty() || verdict.verdict == "block" {
        return verdict;
    }
    let _ = std::env::set_current_dir(&cwd);
    let rest = Request {
        paths: overflow,
        actor: request.actor.clone(),
        ..Request::default()
    };
    let max = max_paths();
    let why = format!(
        "the request names {total} paths, more than the {max} it judges in one call; the first {max} were judged against the gates"
    );
    if actor == Actor::Agent {
        adapter_failure(config.as_deref(), &rest, &why)
    } else {
        unjudged(OnUnjudged::Allow, actor, None, &why)
    }
}

/// The paths of `request` beyond `max_paths()`, made absolute against `cwd` and
/// removed from the request (with their proposed contents, when it has one for
/// each path).
fn take_overflow(request: &mut Request, cwd: &Path) -> Vec<String> {
    let max = max_paths();
    if request.paths.len() <= max {
        return Vec::new();
    }
    if request.contents.len() == request.paths.len() {
        request.contents.truncate(max);
    }
    request
        .paths
        .split_off(max)
        .iter()
        .map(|p| absolute(cwd, p).to_string_lossy().to_string())
        .collect()
}

/// Judges `request` in every repository that governs it, the process being in `cwd`.
fn judge_in_roots(config: Option<PathBuf>, mut request: Request, cwd: &Path) -> Verdict {
    // An explicit configuration path names the repository's file itself.
    let roots = if config.is_some() {
        Vec::new()
    } else {
        judging_roots(&request, cwd)
    };
    let root = roots.first().cloned().or_else(|| plain_root(&request, cwd));
    let Some(first_root) = root else {
        return judge_edit_in(config, request);
    };
    request.paths = request
        .paths
        .iter()
        .map(|p| absolute(cwd, p).to_string_lossy().to_string())
        .collect();
    if roots.is_empty() {
        let _ = std::env::set_current_dir(first_root);
        return judge_edit_in(config, request);
    }
    // The session's repository, when the policy says so, also judges a write
    // that lands outside it (`harness.outside_writes`).
    if let Some(verdict) = outside::denial(&roots[0], cwd, &request) {
        return verdict;
    }
    // Every repository judges; a deny from any wins. Otherwise the verdict is
    // the first repository's that holds the file (the session's own first).
    let mut verdicts: Vec<(bool, Verdict)> = Vec::new();
    for (i, root) in roots.iter().enumerate() {
        let _ = std::env::set_current_dir(root);
        let mut one = request.clone();
        one.off_is_quiet = i == 0 && holds(root, cwd, &request);
        let verdict = judge_edit_in(config.clone(), one);
        if verdict.verdict == "block" {
            return verdict;
        }
        verdicts.push((holds(root, cwd, &request), verdict));
    }
    let at = verdicts.iter().position(|(held, _)| *held).unwrap_or(0);
    verdicts.swap_remove(at).1
}

/// The protected-path or acceptance gate's denial of an agent's write of
/// `content` to `path`, judging the path as written, as it resolves and through
/// hard links.
fn gate_denial(
    repo_root: &Path,
    loaded: &Config,
    path: &str,
    content: &str,
    request: &Request,
) -> Option<Verdict> {
    let lexical = crate::discovery::relative_to_root(repo_root, Path::new(path)).ok();
    let resolved = resolved_rel(repo_root, Path::new(path));
    // A hard link is the file it shares an inode with (Unix).
    let mut entries = gate::protected_entries(loaded);
    entries.extend(gate::engine_owned(loaded.override_log.as_deref()));
    let linked = gate::hardlink_aliases(
        repo_root,
        Path::new(path),
        &entries,
        &gate::gated_record_dirs(loaded),
    );
    lexical
        .iter()
        .chain(resolved.iter().filter(|r| Some(*r) != lexical.as_ref()))
        .chain(linked.iter())
        .find_map(|rel| {
            gate::judge(repo_root, loaded, rel, content)
                .map(|denial| denied(&denial, &rel.to_string_lossy(), request))
        })
}

/// Judges the edit in the repository the process is in.
fn judge_edit_in(config: Option<PathBuf>, mut request: Request) -> Verdict {
    let actor = request.actor();
    if request.adapter_error.is_none()
        && !request.contents.is_empty()
        && request.contents.len() != request.paths.len()
    {
        request.adapter_error = Some(format!(
            "the request names {} path(s) but {} proposed content(s)",
            request.paths.len(),
            request.contents.len()
        ));
    }
    if let Some(why) = &request.adapter_error {
        return adapter_failure(config.as_deref(), &request, why);
    }
    let ws = match Workspace::open(&[PathBuf::from(".")], config.clone()) {
        Ok(ws) => ws,
        Err(e) => return config_unloadable(config.as_deref(), &request, actor, &e.message),
    };
    let Workspace {
        repo_root,
        config_path,
        config: loaded,
    } = ws;
    // The acceptance and protected-path gates come first and do not depend on the
    // editor hook being on: an agent's move to a gated status, or write to a gated
    // path, is refused outright (`ADR-89` section 4).
    if actor == Actor::Agent {
        if let Some(denial) = agent_gate_denial(&repo_root, &loaded, &request) {
            let ws = (repo_root.as_path(), config_path.as_path(), &loaded);
            return answer_denial(ws, config.clone(), &request, denial);
        }
        if request.gates_only {
            return gates_only_verdict(&repo_root, &loaded, &request);
        }
    }
    if let Some(quiet) = quiet_outside_write(&repo_root, &loaded, &request, actor) {
        return quiet;
    }
    // An editor hook the session's configuration turned off says nothing about
    // the edits it does not judge: the choice is the configuration's, and
    // `harness install` and `doctor` name it.
    let Some(harness) = loaded.harness.as_ref().filter(|h| h.enabled("editor-hook")) else {
        if request.off_is_quiet {
            return Verdict::new(
                "allow",
                request.paths.first().map(String::as_str),
                String::new(),
            );
        }
        return unjudged(
            OnUnjudged::Allow,
            actor,
            request.paths.first().map(String::as_str),
            "the editor-hook guardrail is not enabled under harness.guardrails in .urzua/config.yaml",
        );
    };
    let on = harness.on_unjudged("editor-hook");
    let [path] = request.paths.as_slice() else {
        let why = "the request must name exactly one path";
        // Only an agent's request naming several paths is gated: a guardrail set
        // to deny that covers one of the paths refuses it. A person's request, a
        // request naming none, and an agent's that no guardrail covers are
        // judged as a request urzua cannot judge.
        if actor == Actor::Agent && on != OnUnjudged::Deny && request.paths.len() > 1 {
            if let Some(verdict) = targeted_denial(config.as_deref(), &request, why) {
                return verdict;
            }
        }
        return unjudged(on, actor, None, why);
    };
    let Some(content) = request.content.clone() else {
        return unjudged(
            on,
            actor,
            Some(path),
            "the request carries no proposed content",
        );
    };
    let rel = match crate::discovery::relative_to_root(&repo_root, Path::new(path)) {
        // An edit through a link is the file the link resolves to.
        Ok(r) => resolved_rel(&repo_root, Path::new(path)).unwrap_or(r),
        Err(e) => return unjudged(on, actor, Some(path), &e),
    };
    let rel_text = rel.to_string_lossy().to_string();
    judge_record(
        &Judging {
            repo_root: &repo_root,
            config_path: &config_path,
            config: &loaded,
            config_arg: config,
            on,
            actor,
            rel,
            rel_text: &rel_text,
        },
        content,
        &request,
    )
}

/// A gate's refusal of an agent's edit, or the person asked instead (`run/ask.rs`).
fn answer_denial(
    (repo_root, config_path, loaded): (&Path, &Path, &Config),
    config_arg: Option<PathBuf>,
    request: &Request,
    denial: Verdict,
) -> Verdict {
    ask::ask_or_refuse(
        &Judging {
            repo_root,
            config_path,
            config: loaded,
            config_arg,
            on: OnUnjudged::Allow,
            actor: Actor::Agent,
            rel: PathBuf::new(),
            rel_text: "",
        },
        request,
        denial,
    )
}

/// An allowed write whose every path, as written and as its links resolve, lies
/// outside this repository says nothing unless `harness.report_outside_writes`
/// is on (`MILE-278`). The editor hook would refuse it for an agent under
/// `on_unjudged: deny`, so that case is left to the loud path below.
fn quiet_outside_write(
    repo_root: &Path,
    loaded: &Config,
    request: &Request,
    actor: Actor,
) -> Option<Verdict> {
    let harness = loaded.harness.as_ref();
    if harness.is_some_and(|h| h.report_outside_writes) || request.paths.is_empty() {
        return None;
    }
    let editor = harness.filter(|h| h.enabled("editor-hook"));
    if actor == Actor::Agent
        && editor.is_some_and(|h| h.on_unjudged("editor-hook") == OnUnjudged::Deny)
    {
        return None;
    }
    let root = repo_root.canonicalize().ok()?;
    let cwd = std::env::current_dir().ok()?;
    let outside = |p: &String| {
        let abs = absolute(&cwd, p);
        crate::discovery::relative_to_root(repo_root, &abs).is_err()
            && resolve_links(&abs).is_some_and(|full| !full.starts_with(&root))
    };
    request.paths.iter().all(outside).then(|| {
        Verdict::new(
            "allow",
            request.paths.first().map(String::as_str),
            String::new(),
        )
    })
}

/// The verdict for an edit when the configuration does not load.
fn config_unloadable(
    config: Option<&Path>,
    request: &Request,
    actor: Actor,
    message: &str,
) -> Verdict {
    let cwd_here = std::env::current_dir().unwrap_or_default();
    // The configuration does not load, so only the raw file can say what
    // the guardrail was told to do about an edit it cannot judge.
    let (enabled, on) = raw_policy(config, &cwd_here, "editor-hook");
    let why = format!("the configuration does not load: {message}");
    if let Some(verdict) = raw_gate_denial(config, request, &why) {
        return verdict;
    }
    if enabled {
        unjudged(on, actor, request.paths.first().map(String::as_str), &why)
    } else {
        unjudged(
            OnUnjudged::Allow,
            actor,
            request.paths.first().map(String::as_str),
            &why,
        )
    }
}

/// An agent's write refused by the acceptance or protected-path gate. A request
/// that names several paths is malformed (SPEC-24), but a gated path among them
/// still blocks: every path is judged and any denial wins.
fn agent_gate_denial(repo_root: &Path, loaded: &Config, request: &Request) -> Option<Verdict> {
    request.paths.iter().enumerate().find_map(|(i, path)| {
        // A path without content is still judged by its path (a protected
        // path blocks); the acceptance gate, which reads content, finds no status.
        let content = request.content_at(i).unwrap_or("");
        gate_denial(repo_root, loaded, path, content, request).or_else(|| {
            gate::git_dir_denial(loaded, Path::new(path)).map(|d| denied(&d, path, request))
        })
    })
}

/// The answer to a `gates_only` request no gate denied: allowed so far, with
/// whether the editor hook would go on to check a record at that path. Anything
/// it cannot place, or an editor hook that is off (whose notice the full
/// judgement carries), counts as a record so the full judgement runs.
fn gates_only_verdict(repo_root: &Path, loaded: &Config, request: &Request) -> Verdict {
    let editor_on = loaded
        .harness
        .as_ref()
        .is_some_and(|h| h.enabled("editor-hook"));
    let governed = |path: &String| {
        let Ok(lexical) = crate::discovery::relative_to_root(repo_root, Path::new(path)) else {
            return true;
        };
        let resolved = resolved_rel(repo_root, Path::new(path)).unwrap_or_else(|| lexical.clone());
        [lexical, resolved].iter().any(|rel| {
            crate::discovery::is_governed(loaded, rel)
                || crate::discovery::is_governed(loaded, &gate::fold_to_declared_dir(loaded, rel))
        })
    };
    let mut verdict = Verdict::new(
        "allow",
        request.paths.first().map(String::as_str),
        String::new(),
    );
    let records: Vec<bool> = request
        .paths
        .iter()
        .map(|p| !editor_on || governed(p))
        .collect();
    verdict.extra = Some(Box::new(Extra::Gates(GatesAnswer {
        record: records.iter().any(|r| *r) || (records.is_empty() && !editor_on),
        records,
    })));
    verdict
}

struct Judging<'a> {
    repo_root: &'a Path,
    config_path: &'a Path,
    config: &'a Config,
    config_arg: Option<PathBuf>,
    on: OnUnjudged,
    actor: Actor,
    rel: PathBuf,
    rel_text: &'a str,
}

pub(super) fn live(report: &CheckReport) -> Vec<&Finding> {
    report
        .findings
        .iter()
        .filter(|f| f.waived.is_none())
        .collect()
}

pub(super) fn key(f: &Finding) -> String {
    format!("{}|{}", f.rule, f.message)
}

/// One finding per line, as the old hook listed them; `hint` adds the
/// `urzua` command that repairs a record already on disk.
fn describe(findings: &[&Finding], hint: bool) -> String {
    findings
        .iter()
        .map(|f| {
            let run = f
                .suggested_action
                .as_ref()
                .filter(|a| hint && a.kind == ActionKind::RunCommand)
                .and_then(|a| a.target.argv.as_ref())
                .map(|argv| format!(" (run: {})", argv.join(" ")))
                .unwrap_or_default();
            format!(
                "  {}:{}: [{}] {}{}",
                f.file.display(),
                f.line.map_or("-".to_string(), |l| l.to_string()),
                f.rule,
                f.message,
                run
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn to_json(findings: &[&Finding]) -> Vec<serde_json::Value> {
    findings
        .iter()
        .map(|f| serde_json::to_value(f).expect("a finding serializes"))
        .collect()
}

fn ran_summary(report: &CheckReport) -> String {
    let ran = report
        .rules_executed
        .iter()
        .filter(|e| e.status == RuleStatus::Ran)
        .count();
    let na: Vec<&str> = report
        .rules_executed
        .iter()
        .filter(|e| e.status == RuleStatus::NotApplicable)
        .map(|e| e.rule.as_str())
        .collect();
    let mut out = format!("{ran} rules ran on this record");
    if !na.is_empty() {
        out.push_str(&format!(
            "; {} enabled rules cannot judge a single-record edit and were NOT run here (they run at commit and in CI): {}",
            na.len(),
            na.join(", ")
        ));
    }
    out.push('.');
    out
}

const EDIT_P: &str = "urzua harness (editor-hook)";

fn judge_record(j: &Judging<'_>, content: String, request: &Request) -> Verdict {
    let rel = j.rel_text;
    let kept = super::parent_state::enabled(j.config).then(|| content.clone());
    let report = match proposed_report(j, content) {
        Ok(r) => r,
        Err(verdict) => return *verdict,
    };
    let parent_note = kept.map_or_else(String::new, |c| parent_note(j, &c));

    // Missing history costs only the rules that read it (embodiment drift).
    // Every other finding is still judged and still blocks; the message says
    // what was skipped.
    let hist = report
        .notices
        .iter()
        .filter(|n| n.subject == "embodiment-drift-git")
        .map(|n| n.message.as_str())
        .collect::<Vec<_>>()
        .join("; ");
    let histnote = if hist.is_empty() {
        String::new()
    } else {
        format!("\nNOT judged here: embodiment drift (the age of the evidence), because git history is unreadable: {hist}")
    };

    let all = live(&report);
    let errors: Vec<&Finding> = all
        .iter()
        .copied()
        .filter(|f| f.severity == FindingSeverity::Error)
        .collect();
    let others: Vec<&Finding> = all
        .iter()
        .copied()
        .filter(|f| f.severity != FindingSeverity::Error)
        .collect();
    let (introduced, existing) = split_introduced(j, errors);
    let (pending, introduced): (Vec<&Finding>, Vec<&Finding>) = introduced
        .into_iter()
        .partition(|f| pending_half(f, &j.rel));
    // The other half going into a protected path is not one edit the agent
    // can make, so that half is refused for a decider instead.
    let (protected, pending): (Vec<&Finding>, Vec<&Finding>) = pending.into_iter().partition(|f| {
        urzua_core::fix::inverse::UNRECIPROCATED_PROTECTED.contains(&f.action_source.variant)
    });
    let protected_note = protected_note(&protected);
    let introduced: Vec<&Finding> = introduced.into_iter().chain(protected).collect();
    let pending_allowed = urzua_core::harness::PendingPair::Allow
        == j.config
            .harness
            .as_ref()
            .map(|h| h.pending_pair())
            .unwrap_or_default();
    let (introduced, pending_note) = if pending.is_empty() {
        (introduced, String::new())
    } else if pending_allowed && introduced.is_empty() {
        (introduced, follow_up(&pending, true))
    } else {
        let mut all = introduced;
        all.extend(&pending);
        (all, follow_up(&pending, false))
    };

    let mut note = standing_note(&histnote, &existing, &others);

    if introduced.is_empty() {
        let shown: Vec<&Finding> = existing
            .iter()
            .chain(&others)
            .chain(&pending)
            .copied()
            .collect();
        let (kind, mut message) = allow_message(rel, &report, &shown, &histnote, &note);
        let (kind, message) = if pending.is_empty() {
            with_parent_note(kind, message, &parent_note)
        } else {
            message = format!("{EDIT_P}: urzua judged {rel}; the edit is allowed as the first half of a relation pair.{pending_note}\n{}{note}", ran_summary(&report));
            with_parent_note(kind, message, &parent_note)
        };
        let mut v = Verdict::new(kind, Some(rel), message);
        v.findings = to_json(&shown);
        return v;
    }
    note.push_str(&pending_note);
    note.push_str(&protected_note);
    note.push_str(&parent_note);

    // A "run:" hint repairs the record on disk, so it is shown only for a
    // finding the disk already has; an introduced finding is listed without it.
    let list = describe(&introduced, false);
    let mut v = Verdict::new("block", Some(rel), String::new());
    v.findings = to_json(&introduced);
    override_verdict(j, request, v, &introduced, &list, &note)
}

/// What the edit leaves standing: the history note, the errors the record on
/// disk already has, and the warnings.
fn standing_note(histnote: &str, existing: &[&Finding], others: &[&Finding]) -> String {
    let mut note = histnote.to_string();
    if !existing.is_empty() {
        note.push_str(&format!(
            "\nAlready failing on disk before this edit (not blocked):\n{}",
            describe(existing, true)
        ));
    }
    if !others.is_empty() {
        note.push_str(&format!(
            "\nWarnings (shown, not blocking):\n{}",
            describe(others, true)
        ));
    }
    note
}

/// An allowed edit's verdict with the parent note added; the note makes an
/// otherwise plain allow one with a warning.
fn with_parent_note(
    kind: &'static str,
    mut message: String,
    parent_note: &str,
) -> (&'static str, String) {
    if parent_note.is_empty() {
        return (kind, message);
    }
    message.push_str(parent_note);
    ("allow-with-warnings", message)
}

/// What the proposed record does to the state of the record it is part of, as a
/// note under the verdict; empty when `status.children-agreement` is off or the
/// parent agrees. Never blocks: the finding is on the parent's file.
fn parent_note(j: &Judging<'_>, content: &str) -> String {
    if !super::parent_state::enabled(j.config) {
        return String::new();
    }
    let Ok(mut session) = crate::session::Session::open(j.config_arg.clone()) else {
        return String::new();
    };
    let Ok(record) = crate::discovery::parse_prospective_record(&session.config, &j.rel, content)
    else {
        return String::new();
    };
    session.records.retain(|r| r.path != j.rel);
    session.records.push(record);
    let changed = HashSet::from([j.rel.clone()]);
    let lines = super::parent_state::lines(&session, &changed, &HashSet::new());
    if lines.is_empty() {
        return String::new();
    }
    format!(
        "\nParent state (shown, not blocking):\n{}",
        lines.join("\n")
    )
}

/// Whether a finding is only the missing other half of a declared relation,
/// which one edit of another record (or `urzua fix`) supplies (`MILE-287`).
fn pending_half(f: &Finding, rel: &Path) -> bool {
    let source = &f.action_source;
    (urzua_core::fix::inverse::UNRECIPROCATED.contains(&source.variant)
        || urzua_core::fix::inverse::UNRECIPROCATED_PROTECTED.contains(&source.variant))
        && source.target.file.as_deref().is_some_and(|t| t != rel)
        && source.target.field.is_some()
        && source.value.is_some()
}

fn protected_note(protected: &[&Finding]) -> String {
    if protected.is_empty() {
        return String::new();
    }
    let records = protected
        .iter()
        .filter_map(|f| f.action_source.target.file.as_deref())
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!("\nThe other record of the pair is a protected path ({records}): an agent cannot write it, through an edit or `urzua fix`, so a decider must make the paired change.")
}

/// What the agent must do next for each pending half: the record, the field
/// and value, and the `urzua fix` command that writes it.
fn follow_up(pending: &[&Finding], allowed: bool) -> String {
    let steps = pending
        .iter()
        .map(|f| {
            let s = &f.action_source;
            let target = s.target.file.as_deref().unwrap_or(Path::new("")).display();
            let field = s.target.field.as_deref().unwrap_or_default();
            let id = match &s.value {
                Some(urzua_core::report::ActionValue::Text(t)) => t.clone(),
                _ => String::new(),
            };
            format!(
                "  - add {id} to {field} in {target}: edit that record, or run `urzua fix --apply --ids {target} --actor agent` (it writes only that field and a revision-log row, never a Status)"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if allowed {
        format!(
            "\nNow required, the other half of the pair:\n{steps}\nUntil it is written the pair is unmatched: the post-command hook reports it, and pre-commit and CI block it. A Status move the relation implies (to Superseded, say) is a separate step, gated where the authority table names it."
        )
    } else {
        format!(
            "\nThe other half of a relation pair is missing; write it without the shell:\n{steps}"
        )
    }
}

/// The check report of the proposed content, or the verdict when it cannot
/// be had.
fn proposed_report(j: &Judging<'_>, content: String) -> Result<CheckReport, Box<Verdict>> {
    let rel = j.rel_text;
    match judge_proposed(j.config_arg.clone(), j.rel.clone(), content) {
        Err(e) if e.message.starts_with("no record type governs ") => Err(Box::new(Verdict::new(
            "not-governed",
            Some(rel),
            format!("{rel} is not a governed record; nothing to judge"),
        ))),
        Err(e) => Err(Box::new(unjudged(
            j.on,
            j.actor,
            Some(rel),
            &format!("urzua reported not-run: {}", e.message),
        ))),
        Ok(r) if r.status == ReportStatus::NotRun => Err(Box::new(unjudged(
            j.on,
            j.actor,
            Some(rel),
            "urzua reported not-run: no enabled rule examined the record",
        ))),
        Ok(r) => Ok(r),
    }
}

/// Splits the errors into those this edit introduced and those the record on
/// disk already has.
///
/// Errors the record on disk already has are not this edit's (`block_on:
/// introduced`, the default). A baseline run that cannot judge leaves every
/// error introduced. `block_on: all` blocks any unwaived error and skips it.
fn split_introduced<'a>(
    j: &Judging<'_>,
    errors: Vec<&'a Finding>,
) -> (Vec<&'a Finding>, Vec<&'a Finding>) {
    let block_all = j.config.block_on.as_deref() == Some("all");
    let (mut introduced, mut existing) = (errors.clone(), Vec::<&Finding>::new());
    if !block_all && !errors.is_empty() {
        if let Ok(on_disk) = std::fs::read_to_string(j.repo_root.join(&j.rel)) {
            if let Ok(baseline) = judge_proposed(j.config_arg.clone(), j.rel.clone(), on_disk) {
                if matches!(
                    baseline.status,
                    ReportStatus::Ok | ReportStatus::FindingsPresent
                ) {
                    let keys: HashSet<String> = live(&baseline)
                        .into_iter()
                        .filter(|f| f.severity == FindingSeverity::Error)
                        .map(key)
                        .collect();
                    (existing, introduced) =
                        errors.iter().copied().partition(|f| keys.contains(&key(f)));
                }
            }
        }
    }
    (introduced, existing)
}

/// The verdict kind and message of an edit that introduced no error.
fn allow_message(
    rel: &str,
    report: &CheckReport,
    shown: &[&Finding],
    histnote: &str,
    note: &str,
) -> (&'static str, String) {
    if !shown.is_empty() {
        return (
            "allow-with-warnings",
            format!(
                "{EDIT_P}: urzua judged {rel}; nothing this edit introduced blocks it. {}{note}",
                ran_summary(report)
            ),
        );
    }
    if histnote.is_empty() {
        (
            "allow",
            format!(
                "{EDIT_P}: urzua judged {rel}; no findings. {}",
                ran_summary(report)
            ),
        )
    } else {
        (
            "allow-with-warnings",
            format!(
                "{EDIT_P}: urzua judged {rel} with what it can see; no findings. {}{histnote}",
                ran_summary(report)
            ),
        )
    }
}

/// The verdict of an edit that introduced errors: blocked, unless a named
/// override is given (and recorded, when an `override_log` is declared).
fn override_verdict(
    j: &Judging<'_>,
    request: &Request,
    mut v: Verdict,
    introduced: &[&Finding],
    list: &str,
    note: &str,
) -> Verdict {
    const P: &str = EDIT_P;
    let rel = j.rel_text;
    let env_name = std::env::var("URZUA_HOOK_OVERRIDE")
        .ok()
        .filter(|n| !n.is_empty());
    let (given, name, reason) = match &request.override_request {
        Some(o) => (
            true,
            o.name.clone().unwrap_or_default(),
            o.reason.clone().unwrap_or_default(),
        ),
        None => (
            env_name.is_some(),
            env_name.unwrap_or_default(),
            std::env::var("URZUA_HOOK_REASON").unwrap_or_default(),
        ),
    };
    if !given {
        v.message = format!(
            "{P}: blocked. urzua reports errors this edit introduced:\n{list}\nFix the record as the findings say. The rest of the edit is fine. To proceed anyway, a named override is required: URZUA_HOOK_OVERRIDE=<name>.{note}"
        );
        return v;
    }
    if name.trim().is_empty() {
        v.message = format!(
            "{P}: blocked. An override needs a name (URZUA_HOOK_OVERRIDE=NAME); an empty one is not accepted.\n{list}"
        );
        return v;
    }
    let recorded = match j.config.override_log.as_deref() {
        None => format!("{P}: this override is NOT recorded: no override_log is declared in .urzua/config.yaml."),
        Some(log) => {
            if reason.trim().is_empty() {
                v.message = format!(
                    "{P}: blocked. An override is recorded in {log} and needs a reason (URZUA_HOOK_REASON=TEXT).\n{list}"
                );
                return v;
            }
            if let Err(e) = persist_override(j.repo_root, log, &name, rel, introduced, &reason) {
                v.message = format!(
                    "{P}: blocked. The override could not be recorded in {log} ({e}), and an override that is not recorded is not allowed.\n{list}"
                );
                return v;
            }
            format!("Recorded in {log}; a person reviews it by setting Reviewed-by on its row.")
        }
    };
    v.verdict = "allow-override";
    v.override_name = Some(name.clone());
    v.message = format!(
        "{P}: OVERRIDE '{name}' let this edit to {rel} through despite:\n{list}\n{recorded}{note}"
    );
    let _ = j.config_path;
    v
}

/// One cell of a Markdown table row: no pipe, no newline.
fn cell(text: &str) -> String {
    text.replace(['\n', '|'], " ").trim().to_string()
}

/// Appends one row per overridden finding to the committed override log. An
/// override that cannot be written is refused by the caller.
fn persist_override(
    root: &Path,
    log: &str,
    name: &str,
    record: &str,
    findings: &[&Finding],
    reason: &str,
) -> Result<(), String> {
    use std::io::Write;
    let path = root.join(log);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let needs_header = std::fs::metadata(&path)
        .map(|m| m.len() == 0)
        .unwrap_or(true);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    if needs_header {
        file.write_all(
            b"# Override log\n\nEach row is an editor-hook override that let a blocked record edit through\n(written by `urzua harness run edit`). A person sets Reviewed-by once they\nhave looked at it; `override.unreviewed` reports every row where it is blank.\n\n| Date | Name | Record | Field | Rule | Reason | Reviewed-by |\n|---|---|---|---|---|---|---|\n",
        )
        .map_err(|e| e.to_string())?;
    }
    let today = urzua_io::today();
    for f in findings {
        let field = f
            .suggested_action
            .as_ref()
            .and_then(|a| a.target.field.clone())
            .unwrap_or_else(|| "-".to_string());
        writeln!(
            file,
            "| {today} | {} | {} | {} | {} | {} | |",
            cell(name),
            cell(record),
            cell(&field),
            cell(&f.rule),
            cell(reason)
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ----------------------------------------------------------- session-start

#[derive(Serialize)]
struct Problem {
    check: &'static str,
    message: String,
}

/// Whether `name` resolves to a file on `PATH`.
fn on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| {
            let p = dir.join(name);
            p.is_file()
        })
    })
}

/// Moves into the session's own governed repository: the one that holds the
/// directory the session started in (`project_dir`), else the nearest above the
/// current directory. A session whose directory is inside a nested repository
/// with no configuration still belongs to the governed repository around it;
/// a repository the session did not start in is never chosen over its own.
pub(super) fn enter_governed_cwd(project_dir: Option<&str>) {
    let root = project_dir
        .and_then(|d| nearest_governed_root(Path::new(d)))
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .as_deref()
                .and_then(nearest_governed_root)
        });
    if let Some(root) = root {
        let _ = std::env::set_current_dir(root);
    }
}

pub fn run_session_start(config: Option<PathBuf>) -> ExitCode {
    let request = read_request().unwrap_or_default();
    if let Some(cwd) = &request.cwd {
        let _ = std::env::set_current_dir(cwd);
    }
    if config.is_none() && ungoverned(request.project_dir.as_deref(), &[]) {
        return emit(&Verdict::new("allow", None, String::new()));
    }
    if config.is_none() {
        enter_governed_cwd(request.project_dir.as_deref());
    }
    emit(&session_start(config))
}

/// The settings files read so far, `None` when absent or unreadable.
type SettingsDocs = std::collections::BTreeMap<String, Option<super::json::J>>;

fn cached_doc<'a>(
    docs: &'a mut SettingsDocs,
    root: &Path,
    file: &str,
) -> &'a Option<super::json::J> {
    docs.entry(file.to_string()).or_insert_with(|| {
        super::load_settings(root, file)
            .ok()
            .and_then(|s| s.text.map(|_| s.doc))
    })
}

/// The lock, or `None` after saying why there is none.
fn installed_lock_problems(root: &Path, problems: &mut Vec<Problem>) -> Option<lock::Lock> {
    let lock = match super::installed_lock(root) {
        Ok(l) => l,
        Err(m) => {
            problems.push(Problem {
                check: "lock",
                message: m,
            });
            None
        }
    };
    if lock.is_none() {
        problems.push(Problem {
            check: "lock",
            message: format!(
                "{} is missing: the harness is not installed (run `urzua harness install`)",
                lock::LOCK_PATH
            ),
        });
    }
    lock
}

fn version_problem(lock: &lock::Lock, problems: &mut Vec<Problem>) {
    let version = super::engine_version();
    if lock.bundle_version != version {
        let older = super::version_is_older(&lock.bundle_version, version);
        problems.push(Problem {
            check: "version",
            message: if older {
                format!("the installed bundle ({}) is older than this binary ({version}): run `urzua harness update`", lock.bundle_version)
            } else {
                format!("the installed bundle ({}) is newer than this binary ({version}): update urzua", lock.bundle_version)
            },
        });
    }
}

/// Each locked adapter must be embedded, with its interpreters and
/// environment present. Returns the directories of the commands it installs.
fn adapter_problems(lock: &lock::Lock, problems: &mut Vec<Problem>) -> Vec<String> {
    let mut command_dirs: Vec<String> = Vec::new();
    for name in &lock.adapters {
        let Some(adapter) = super::bundle::adapter(name) else {
            problems.push(Problem {
                check: "adapter",
                message: format!(
                    "the lock names the adapter '{name}', which this binary does not embed"
                ),
            });
            continue;
        };
        let manifest = adapter.manifest();
        if let Some(dir) = &manifest.commands_dir {
            command_dirs.push(format!("{}/", dir.trim_end_matches('/')));
        }
        for interpreter in &manifest.requires.interpreters {
            if !on_path(interpreter) {
                problems.push(Problem {
                    check: "interpreter",
                    message: format!("'{interpreter}', which the {name} adapter's hook commands need, is not on PATH: its hook commands exit 127, which the agent treats as non-blocking and tells you nothing, so edits go unjudged"),
                });
            }
        }
        for var in &manifest.requires.env {
            if std::env::var_os(var).is_none_or(|v| v.is_empty()) {
                problems.push(Problem {
                    check: "environment",
                    message: format!("{var} is not set in this session. The {name} adapter's hook commands find the project from it; without it they fall back to the nearest git repository at or above the current directory that holds .urzua/config.yaml, which is the same project only while the session was started inside it"),
                });
            }
        }
    }
    command_dirs
}

/// The hook entries are wired in the agent's project settings.
fn hook_entry_problems(
    root: &Path,
    lock: &lock::Lock,
    wanted: &std::collections::BTreeMap<(String, String), String>,
    docs: &mut SettingsDocs,
    problems: &mut Vec<Problem>,
) {
    for e in &lock.entries {
        let doc = cached_doc(docs, root, &e.file);
        let found = doc
            .as_ref()
            .and_then(|d| settings::find(d, &e.event, &e.matcher, &e.handler));
        match found {
            None => problems.push(Problem {
                check: "hook-entry",
                message: format!("the hook entry '{}' is not wired in {}: the guardrail does not run (run `urzua harness status`, then `urzua harness install`)", e.key(), e.file),
            }),
            Some(hook) if lock::hash(&hook.compact()) != e.hash => problems.push(Problem {
                check: "hook-entry",
                message: format!("the hook entry '{}' in {} is not the one the install wrote (its hash differs from {}): it may no longer run the guardrail (run `urzua harness status`, then `urzua harness update --force`)", e.key(), e.file, lock::LOCK_PATH),
            }),
            Some(_) if wanted.get(&(e.file.clone(), e.key())).is_some_and(|h| *h != e.hash) => problems.push(Problem {
                check: "hook-entry",
                message: format!("the hook entry '{}' in {} is out of date: the bundle or the configuration (such as harness.limits.hook_timeout_seconds) now renders it differently (run `urzua harness update`)", e.key(), e.file),
            }),
            Some(_) => {}
        }
    }
}

/// The files the install owns are the ones it wrote: a translator that was
/// neutered or deleted passes every other check here and judges nothing.
fn owned_file_problems(
    root: &Path,
    lock: &lock::Lock,
    command_dirs: &[String],
    problems: &mut Vec<Problem>,
) {
    for (path, locked) in &lock.files {
        let customisable = command_dirs.iter().any(|d| path.starts_with(d.as_str()));
        match super::read(root, path) {
            Ok(None) => problems.push(Problem {
                check: "file",
                message: format!("{path} is missing: it is part of the installed harness and the guardrail that uses it cannot run (run `urzua harness status`, then `urzua harness update`)"),
            }),
            Ok(Some(text)) if !customisable && !lock::text_matches(locked, &text) => {
                problems.push(Problem {
                    check: "file",
                    message: format!("{path} is not the file the install wrote (its hash differs from {}): it may judge nothing (run `urzua harness status`, then `urzua harness update --force` to restore it)", lock::LOCK_PATH),
                })
            }
            Err(m) => problems.push(Problem {
                check: "file",
                message: format!("{m}, so the installed harness file cannot be checked"),
            }),
            Ok(Some(_)) => {}
        }
    }
}

/// The kill-switch problems of the installed adapters, for `status`.
pub(super) fn kill_switch_messages(root: &Path, lock: &lock::Lock) -> Vec<String> {
    let mut problems = Vec::new();
    kill_switch_problems(root, lock, &mut SettingsDocs::new(), &mut problems);
    problems.into_iter().map(|p| p.message).collect()
}

/// A setting that switches every hook off, which no hook entry can show.
fn kill_switch_problems(
    root: &Path,
    lock: &lock::Lock,
    docs: &mut SettingsDocs,
    problems: &mut Vec<Problem>,
) {
    for name in &lock.adapters {
        let Some(adapter) = super::bundle::adapter(name) else {
            continue;
        };
        let manifest = adapter.manifest();
        let Some(doc) = cached_doc(docs, root, &manifest.settings_file).as_ref() else {
            continue;
        };
        for key in &manifest.kill_switches {
            let on = match doc.get(key) {
                Some(super::json::J::Bool(b)) => *b,
                Some(super::json::J::Str(s)) => s.trim().eq_ignore_ascii_case("true"),
                _ => false,
            };
            if on {
                problems.push(Problem {
                    check: "kill-switch",
                    message: format!("{} sets \"{key}\" to true: every hook of the {name} adapter is switched off, so no guardrail runs in this session (remove the setting)", manifest.settings_file),
                });
            }
        }
    }
}

fn session_start(config: Option<PathBuf>) -> Verdict {
    const P: &str = "urzua harness (session-start)";
    let ws = match Workspace::open(&[PathBuf::from(".")], config.clone()) {
        Ok(ws) => ws,
        Err(e) => {
            return Verdict::new(
                "allow-with-warnings",
                None,
                format!(
                    "{P}: the configuration does not load, so the harness cannot be checked: {}",
                    e.message
                ),
            )
        }
    };
    let harness = ws.config.harness.clone();
    if let Some(h) = &harness {
        super::limits::activate(&h.limits);
    }
    if !harness.as_ref().is_some_and(|h| h.enabled("session-start")) {
        return Verdict::new(
            "allow",
            None,
            format!("{P}: the session-start guardrail is not enabled under harness.guardrails"),
        );
    }
    let harness = harness.expect("checked above");
    let mut shrunk = None;
    if harness.enabled("post-command") {
        // The commit the first post-command check of the session starts from.
        super::post_command::seed_head(&ws.repo_root);
        // Before the baseline is retaken from the configuration as it is now.
        shrunk = super::post_command::protected_set_shrank(&ws.repo_root, &ws.config);
        super::post_command::seed_protected(&ws.repo_root, &ws.config_path, &ws.config);
    }
    let mut problems: Vec<Problem> = Vec::new();
    if let Some(message) = shrunk {
        problems.push(Problem {
            check: "protected-paths",
            message,
        });
    }
    for p in &harness.problems {
        problems.push(Problem {
            check: "config",
            message: p.clone(),
        });
    }

    // The binary: found on PATH, and the same version as the installed bundle.
    if !on_path("urzua") {
        problems.push(Problem {
            check: "binary",
            message: "urzua is not on PATH in this session, so the hook commands cannot find it (a build directory needs URZUA_BIN set)".to_string(),
        });
    }
    let Some(lock) = installed_lock_problems(&ws.repo_root, &mut problems) else {
        return finish(problems, &harness);
    };
    version_problem(&lock, &mut problems);
    let command_dirs = adapter_problems(&lock, &mut problems);
    let mut docs = SettingsDocs::new();
    let wanted = super::wanted_entry_hashes(&ws.repo_root, &ws.config, &lock.adapters);
    hook_entry_problems(&ws.repo_root, &lock, &wanted, &mut docs, &mut problems);
    owned_file_problems(&ws.repo_root, &lock, &command_dirs, &mut problems);
    kill_switch_problems(&ws.repo_root, &lock, &mut docs, &mut problems);
    for message in super::presence::leftovers(&ws.repo_root, &lock.adapters) {
        problems.push(Problem {
            check: "leftover",
            message,
        });
    }

    for message in crate::commands::doctor::git_hook_problems(&ws.repo_root) {
        problems.push(Problem {
            check: "git-hook",
            message,
        });
    }
    finish(problems, &harness)
}

/// One line naming the command that prints the repository's record types.
const TYPES_LINE: &str = "Record types: `urzua explain --types`";

fn finish(problems: Vec<Problem>, harness: &urzua_core::harness::HarnessDecl) -> Verdict {
    const P: &str = "urzua harness (session-start)";
    if problems.is_empty() {
        let mut v = Verdict::new(
            "allow",
            None,
            format!("{P}: the harness is installed and wired"),
        );
        v.extra = Some(Box::new(Extra::Agent { agent: TYPES_LINE }));
        return v;
    }
    let mut message = format!(
        "{P}: {} thing(s) would make a guardrail fail open or not run:",
        problems.len()
    );
    for p in &problems {
        message.push_str(&format!("\n  - [{}] {}", p.check, p.message));
    }
    // A git hook that does not run cannot make a harness guardrail refuse an edit.
    if problems.iter().any(|p| p.check != "git-hook")
        && urzua_core::harness::GUARDRAILS
            .iter()
            .any(|g| harness.enabled(g.name) && harness.on_unjudged(g.name) == OnUnjudged::Deny)
    {
        message.push_str(&format!(
            "\nA guardrail is set to on_unjudged: deny, so an agent's edit is refused while it cannot judge. {HATCH}"
        ));
    }
    message.push_str(&format!("\n{TYPES_LINE}"));
    let mut v = Verdict::new("allow-with-warnings", None, message);
    v.findings = problems
        .iter()
        .map(|p| serde_json::to_value(p).expect("a problem serializes"))
        .collect();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory under the system temporary directory, removed when dropped.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "urzua-run-unit-{name}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Scratch(dir.canonicalize().unwrap())
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn spawns() -> usize {
        GIT_ROOT_SPAWNS.with(std::cell::Cell::get)
    }

    #[test]
    fn a_directory_is_asked_of_git_once_however_many_paths_are_under_it() {
        let repo = Scratch::new("git-root-memo");
        std::fs::create_dir_all(repo.0.join(".urzua")).unwrap();
        std::fs::write(repo.0.join(".urzua/config.yaml"), "schema_version: 2\n").unwrap();
        let init = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(&repo.0)
            .status()
            .unwrap();
        assert!(init.success());
        forget_git_roots();
        let before = spawns();
        for n in 0..300 {
            let file = repo.0.join(format!("notes/n{n}.txt"));
            assert_eq!(governed_roots(&file), vec![repo.0.clone()]);
        }
        // One answer for the repository's directory (the ancestors above it
        // hold no configuration, so git is not asked about them).
        assert_eq!(spawns() - before, 1);
        // A fresh request asks again.
        forget_git_roots();
        assert!(is_git_root(&repo.0));
        assert_eq!(spawns() - before, 2);
    }

    #[test]
    fn a_request_beyond_the_path_cap_hands_the_rest_back_as_absolute_paths() {
        let mut request = Request {
            paths: (0..514).map(|n| format!("p{n}")).collect(),
            ..Request::default()
        };
        request.contents = vec![Some("x".into()); 514];
        let rest = take_overflow(&mut request, Path::new("/work"));
        assert_eq!(request.paths.len(), 512);
        assert_eq!(request.contents.len(), 512);
        assert_eq!(rest, ["/work/p512", "/work/p513"]);
        assert!(take_overflow(&mut request, Path::new("/work")).is_empty());
    }

    #[test]
    fn a_raised_path_cap_judges_more_paths_and_hands_back_only_the_rest() {
        super::super::limits::raise_for_test("paths_per_call", 600);
        let mut request = Request {
            paths: (0..601).map(|n| format!("p{n}")).collect(),
            ..Request::default()
        };
        let rest = take_overflow(&mut request, Path::new("/work"));
        assert_eq!(request.paths.len(), 600);
        assert_eq!(rest, ["/work/p600"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_raised_symlink_hop_count_follows_a_longer_chain() {
        let dir = std::env::temp_dir().join(format!("urzua-hops-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("l0"), "x").unwrap();
        for n in 1..=45 {
            std::os::unix::fs::symlink(dir.join(format!("l{}", n - 1)), dir.join(format!("l{n}")))
                .unwrap();
        }
        let top = dir.canonicalize().unwrap().join("l45");
        assert_eq!(resolve_links(&top), None);
        super::super::limits::raise_for_test("symlink_hops", 45);
        assert_eq!(
            resolve_links(&top),
            Some(dir.canonicalize().unwrap().join("l0"))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
