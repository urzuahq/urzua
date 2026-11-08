//! The guardrail cores (`ADR-89` section 1): `urzua harness run edit` and
//! `urzua harness run session-start`.
//!
//! They speak a neutral request on stdin and a neutral verdict on stdout, and
//! know no agent. An adapter's translator turns an agent's hook payload into the
//! request and the verdict back into the agent's output; everything that used to
//! fail open in a `jq`, `bash` and `python3` chain (no `jq`, no binary, an
//! unreadable payload) is a verdict stated here, which `on_unjudged` can turn
//! from allow into deny for an agent.
//!
//! Request: `{event, tool, paths, content, actor, cwd, override?}`.
//! Verdict: `{verdict, path?, message, findings, override?}` with `verdict` one
//! of `allow`, `allow-with-warnings`, `block`, `allow-unjudged`,
//! `allow-override` and `not-governed`. Exit 0 for every verdict but `block`
//! (exit 1).

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
    #[allow(dead_code)]
    event: Option<String>,
    #[allow(dead_code)]
    tool: Option<String>,
    #[serde(default)]
    paths: Vec<String>,
    content: Option<String>,
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
}

impl Request {
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
}

impl Verdict {
    fn new(verdict: &'static str, path: Option<&str>, message: String) -> Self {
        Verdict {
            verdict,
            path: path.map(str::to_string),
            message,
            findings: Vec::new(),
            override_name: None,
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
        .and_then(|t| yaml_serde::from_str::<yaml_serde::Value>(&t).ok())
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
    Verdict::new(
        "block",
        Some(rel),
        format!(
            "urzua harness ({}): refused. {}. A person makes this change by editing outside the agent harness (a plain editor or a terminal), where no hook runs. The override of ADR-78 is not accepted for this verdict.{override_note}",
            denial.guardrail, denial.reason
        ),
    )
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
        Err(why) => unjudged(
            on_unreadable_request(config.as_deref()),
            Actor::Agent,
            None,
            &why,
        ),
    };
    emit(&verdict)
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
    let here = std::env::current_dir().unwrap_or_default();
    let root = crate::discovery::find_repo_root(&[PathBuf::from(".")]).unwrap_or(here);
    let path = request.paths.first().map(String::as_str);
    let guardrail = match path {
        None => ["protected-paths", "acceptance"]
            .into_iter()
            .find(|g| matches!(raw_policy(config, &root, g), (true, OnUnjudged::Deny))),
        Some(_) => request
            .paths
            .iter()
            .find_map(|p| targeted_guardrail(config, &root, Path::new(p))),
    };
    if let Some(guardrail) = guardrail {
        return unjudged_as(guardrail, OnUnjudged::Deny, Actor::Agent, path, why);
    }
    Verdict::new(
        "allow-unjudged",
        path,
        format!("urzua harness: urzua did NOT judge this call ({why}). It is allowed; run 'urzua harness status' and 'urzua check' before committing."),
    )
}

/// The denying guardrail that covers `path`, read from the raw configuration
/// (a protected path, the configuration itself, or a record directory of a
/// gated type), judging the path as written and as it resolves.
fn targeted_guardrail(config: Option<&Path>, root: &Path, path: &Path) -> Option<&'static str> {
    let config_path = config
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::discovery::default_config_path(root));
    let raw = gate::raw_gates(&std::fs::read_to_string(&config_path).ok()?)?;
    let config_rel = crate::discovery::relative_to_root(root, &config_path).ok();
    let lexical = crate::discovery::relative_to_root(root, path).ok();
    let resolved = resolved_rel(root, path);
    let mut entries = raw.paths.clone();
    entries.extend(raw.owned.iter().cloned());
    let linked = gate::hardlink_aliases(root, path, &entries, &raw.gated_dirs);
    lexical
        .iter()
        .chain(resolved.iter())
        .chain(linked.iter())
        .find_map(|rel| raw.covers(rel, config_rel.as_deref()))
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

/// Whether `dir` is the top of a real git working tree: `.git` exists and git
/// itself accepts it. An empty file or directory named `.git` is not one (git
/// rejects it, or walks past it to a repository above), so a file an agent can
/// write cannot make a directory a repository.
fn is_git_root(dir: &Path) -> bool {
    if !dir.join(".git").exists() {
        return false;
    }
    let Ok(out) = std::process::Command::new("git")
        .args(["-c", "safe.directory=*", "rev-parse", "--show-toplevel"])
        .current_dir(dir)
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
/// name. `None` after more than 40 links (a loop).
fn resolve_links(abs: &Path) -> Option<PathBuf> {
    let mut queue: std::collections::VecDeque<Step> = steps(abs).into();
    let mut cur = PathBuf::new();
    let mut followed = 0;
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
                    if followed > 40 {
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

fn judge_edit(config: Option<PathBuf>, mut request: Request) -> Verdict {
    let actor = request.actor();
    if let Some(cwd) = &request.cwd {
        let on = on_unreadable_request(config.as_deref());
        if std::env::set_current_dir(cwd).is_err() {
            return unjudged(
                on,
                actor,
                None,
                &format!("the request's cwd '{cwd}' cannot be entered"),
            );
        }
    }
    let Ok(cwd) = std::env::current_dir() else {
        return judge_edit_in(config, request);
    };
    // An explicit configuration path names the repository's file itself.
    let roots = if config.is_some() {
        Vec::new()
    } else {
        judging_roots(&request, &cwd)
    };
    let root = roots
        .first()
        .cloned()
        .or_else(|| plain_root(&request, &cwd));
    let Some(first_root) = root else {
        return judge_edit_in(config, request);
    };
    request.paths = request
        .paths
        .iter()
        .map(|p| absolute(&cwd, p).to_string_lossy().to_string())
        .collect();
    if roots.is_empty() {
        let _ = std::env::set_current_dir(first_root);
        return judge_edit_in(config, request);
    }
    // Every repository judges; a deny from any wins. Otherwise the verdict is
    // the first repository's that holds the file (the session's own first).
    let mut verdicts: Vec<(bool, Verdict)> = Vec::new();
    for root in &roots {
        let _ = std::env::set_current_dir(root);
        let verdict = judge_edit_in(config.clone(), request.clone());
        if verdict.verdict == "block" {
            return verdict;
        }
        verdicts.push((holds(root, &cwd, &request), verdict));
    }
    let at = verdicts.iter().position(|(held, _)| *held).unwrap_or(0);
    verdicts.swap_remove(at).1
}

/// Judges the edit in the repository the process is in.
fn judge_edit_in(config: Option<PathBuf>, request: Request) -> Verdict {
    let actor = request.actor();
    if let Some(why) = &request.adapter_error {
        return adapter_failure(config.as_deref(), &request, why);
    }
    let cwd_here = std::env::current_dir().unwrap_or_default();
    let ws = match Workspace::open(&[PathBuf::from(".")], config.clone()) {
        Ok(ws) => ws,
        Err(e) => {
            // The configuration does not load, so only the raw file can say what
            // the guardrail was told to do about an edit it cannot judge.
            let (enabled, on) = raw_policy(config.as_deref(), &cwd_here, "editor-hook");
            let why = format!("the configuration does not load: {}", e.message);
            if let Some(verdict) = raw_gate_denial(config.as_deref(), &request, &why) {
                return verdict;
            }
            return if enabled {
                unjudged(on, actor, request.paths.first().map(String::as_str), &why)
            } else {
                unjudged(
                    OnUnjudged::Allow,
                    actor,
                    request.paths.first().map(String::as_str),
                    &why,
                )
            };
        }
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
        if let ([path], Some(content)) = (request.paths.as_slice(), request.content.as_deref()) {
            let lexical = crate::discovery::relative_to_root(&repo_root, Path::new(path)).ok();
            let resolved = resolved_rel(&repo_root, Path::new(path));
            // A hard link is the file it shares an inode with (Unix).
            let mut entries = gate::protected_entries(&loaded);
            entries.extend(gate::engine_owned(loaded.override_log.as_deref()));
            let linked = gate::hardlink_aliases(
                &repo_root,
                Path::new(path),
                &entries,
                &gate::gated_record_dirs(&loaded),
            );
            for rel in lexical
                .iter()
                .chain(resolved.iter().filter(|r| Some(*r) != lexical.as_ref()))
                .chain(linked.iter())
            {
                if let Some(denial) = gate::judge(&repo_root, &loaded, rel, content) {
                    return denied(&denial, &rel.to_string_lossy(), &request);
                }
            }
        }
    }
    let Some(harness) = loaded.harness.as_ref().filter(|h| h.enabled("editor-hook")) else {
        return unjudged(
            OnUnjudged::Allow,
            actor,
            request.paths.first().map(String::as_str),
            "the editor-hook guardrail is not enabled under harness.guardrails in .urzua/config.yaml",
        );
    };
    let on = harness.on_unjudged("editor-hook");
    let [path] = request.paths.as_slice() else {
        return unjudged(on, actor, None, "the request must name exactly one path");
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

fn judge_record(j: &Judging<'_>, content: String, request: &Request) -> Verdict {
    const P: &str = "urzua harness (editor-hook)";
    let rel = j.rel_text;
    let report = match judge_proposed(j.config_arg.clone(), j.rel.clone(), content) {
        Err(e) if e.message.starts_with("no record type governs ") => {
            return Verdict::new(
                "not-governed",
                Some(rel),
                format!("{rel} is not a governed record; nothing to judge"),
            )
        }
        Err(e) => {
            return unjudged(
                j.on,
                j.actor,
                Some(rel),
                &format!("urzua reported not-run: {}", e.message),
            )
        }
        Ok(r) if r.status == ReportStatus::NotRun => {
            return unjudged(
                j.on,
                j.actor,
                Some(rel),
                "urzua reported not-run: no enabled rule examined the record",
            )
        }
        Ok(r) => r,
    };

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

    // Errors the record on disk already has are not this edit's (`block_on:
    // introduced`, the default). A baseline run that cannot judge leaves every
    // error introduced. `block_on: all` blocks any unwaived error and skips it.
    let block_all = j.config.block_on.as_deref() == Some("all");
    let (mut introduced, mut existing) = (errors.clone(), Vec::<&Finding>::new());
    let baseline_report;
    if !block_all && !errors.is_empty() {
        if let Ok(on_disk) = std::fs::read_to_string(j.repo_root.join(&j.rel)) {
            if let Ok(r) = judge_proposed(j.config_arg.clone(), j.rel.clone(), on_disk) {
                if matches!(r.status, ReportStatus::Ok | ReportStatus::FindingsPresent) {
                    baseline_report = r;
                    let keys: HashSet<String> = live(&baseline_report)
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

    let mut note = histnote.clone();
    if !existing.is_empty() {
        note.push_str(&format!(
            "\nAlready failing on disk before this edit (not blocked):\n{}",
            describe(&existing, true)
        ));
    }
    if !others.is_empty() {
        note.push_str(&format!(
            "\nWarnings (shown, not blocking):\n{}",
            describe(&others, true)
        ));
    }

    if introduced.is_empty() {
        let shown: Vec<&Finding> = existing.iter().chain(&others).copied().collect();
        let (kind, message) = if shown.is_empty() {
            if histnote.is_empty() {
                (
                    "allow",
                    format!(
                        "{P}: urzua judged {rel}; no findings. {}",
                        ran_summary(&report)
                    ),
                )
            } else {
                (
                    "allow-with-warnings",
                    format!(
                        "{P}: urzua judged {rel} with what it can see; no findings. {}{histnote}",
                        ran_summary(&report)
                    ),
                )
            }
        } else {
            (
                "allow-with-warnings",
                format!(
                    "{P}: urzua judged {rel}; nothing this edit introduced blocks it. {}{note}",
                    ran_summary(&report)
                ),
            )
        };
        let mut v = Verdict::new(kind, Some(rel), message);
        v.findings = to_json(&shown);
        return v;
    }

    // A "run:" hint repairs the record on disk, so it is shown only for a
    // finding the disk already has; an introduced finding is listed without it.
    let list = describe(&introduced, false);
    let mut v = Verdict::new("block", Some(rel), String::new());
    v.findings = to_json(&introduced);

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
            if let Err(e) = persist_override(j.repo_root, log, &name, rel, &introduced, &reason) {
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
    if config.is_none() {
        enter_governed_cwd(request.project_dir.as_deref());
    }
    emit(&session_start(config))
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
    if !harness.as_ref().is_some_and(|h| h.enabled("session-start")) {
        return Verdict::new(
            "allow",
            None,
            format!("{P}: the session-start guardrail is not enabled under harness.guardrails"),
        );
    }
    let harness = harness.expect("checked above");
    if harness.enabled("post-command") {
        // The commit the first post-command check of the session starts from.
        super::post_command::seed_head(&ws.repo_root);
        super::post_command::seed_protected(&ws.repo_root, &ws.config_path, &ws.config);
    }
    let mut problems: Vec<Problem> = Vec::new();
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
    let lock = match super::installed_lock(&ws.repo_root) {
        Ok(l) => l,
        Err(m) => {
            problems.push(Problem {
                check: "lock",
                message: m,
            });
            None
        }
    };
    let Some(lock) = lock else {
        problems.push(Problem {
            check: "lock",
            message: format!(
                "{} is missing: the harness is not installed (run `urzua harness install`)",
                lock::LOCK_PATH
            ),
        });
        return finish(problems, &harness);
    };
    let version = super::engine_version();
    if lock.bundle_version != version {
        let older = lock.bundle_version.as_str() < version;
        problems.push(Problem {
            check: "version",
            message: if older {
                format!("the installed bundle ({}) is older than this binary ({version}): run `urzua harness update`", lock.bundle_version)
            } else {
                format!("the installed bundle ({}) is newer than this binary ({version}): update urzua", lock.bundle_version)
            },
        });
    }

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
                    message: format!("'{interpreter}', which the {name} adapter's translator needs, is not on PATH: its hook commands exit 127, which the agent treats as non-blocking and tells you nothing, so edits go unjudged"),
                });
            }
        }
        for var in &manifest.requires.env {
            if std::env::var_os(var).is_none_or(|v| v.is_empty()) {
                problems.push(Problem {
                    check: "environment",
                    message: format!("{var} is not set in this session. The {name} adapter's hook commands find the project from it; without it they fall back to the outermost directory above the current one that holds .urzua/config.yaml, which is the same project only while the session was started inside it"),
                });
            }
        }
    }

    // The hook entries are wired in the agent's project settings.
    let mut docs: std::collections::BTreeMap<String, Option<super::json::J>> = Default::default();
    for e in &lock.entries {
        let doc = docs.entry(e.file.clone()).or_insert_with(|| {
            super::load_settings(&ws.repo_root, &e.file)
                .ok()
                .and_then(|s| s.text.map(|_| s.doc))
        });
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
            Some(_) => {}
        }
    }

    // The files the install owns are the ones it wrote: a translator that was
    // neutered or deleted passes every other check here and judges nothing.
    for (path, locked) in &lock.files {
        let customisable = command_dirs.iter().any(|d| path.starts_with(d.as_str()));
        match super::read(&ws.repo_root, path) {
            Ok(None) => problems.push(Problem {
                check: "file",
                message: format!("{path} is missing: it is part of the installed harness and the guardrail that uses it cannot run (run `urzua harness status`, then `urzua harness update`)"),
            }),
            Ok(Some(text)) if !customisable && lock::hash(&text) != *locked => {
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

    // A setting that switches every hook off, which no hook entry can show.
    for name in &lock.adapters {
        let Some(adapter) = super::bundle::adapter(name) else {
            continue;
        };
        let manifest = adapter.manifest();
        let Some(doc) = docs
            .entry(manifest.settings_file.clone())
            .or_insert_with(|| {
                super::load_settings(&ws.repo_root, &manifest.settings_file)
                    .ok()
                    .and_then(|s| s.text.map(|_| s.doc))
            })
            .as_ref()
        else {
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

    for message in crate::commands::doctor::git_hook_problems(&ws.repo_root) {
        problems.push(Problem {
            check: "git-hook",
            message,
        });
    }
    finish(problems, &harness)
}

fn finish(problems: Vec<Problem>, harness: &urzua_core::harness::HarnessDecl) -> Verdict {
    const P: &str = "urzua harness (session-start)";
    if problems.is_empty() {
        return Verdict::new(
            "allow",
            None,
            format!("{P}: the harness is installed and wired"),
        );
    }
    let mut message = format!(
        "{P}: {} thing(s) would make a guardrail fail open or not run:",
        problems.len()
    );
    for p in &problems {
        message.push_str(&format!("\n  - [{}] {}", p.check, p.message));
    }
    if urzua_core::harness::GUARDRAILS
        .iter()
        .any(|g| harness.enabled(g.name) && harness.on_unjudged(g.name) == OnUnjudged::Deny)
    {
        message.push_str(&format!(
            "\nA guardrail is set to on_unjudged: deny, so an agent's edit is refused while it cannot judge. {HATCH}"
        ));
    }
    let mut v = Verdict::new("allow-with-warnings", None, message);
    v.findings = problems
        .iter()
        .map(|p| serde_json::to_value(p).expect("a problem serializes"))
        .collect();
    v
}
