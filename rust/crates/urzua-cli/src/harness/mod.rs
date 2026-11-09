//! `urzua harness` (`ADR-89`, `MILE-198`): installs an agent harness bundle into a
//! repository and runs its cores.
//!
//! - `install`, `status`, `update`, `uninstall` and `adapters` manage the bundle
//!   embedded in this binary (`bundle`), keeping `.urzua/harness.lock` so drift is
//!   detectable (`lock`). The files a person also edits are shared, not owned: the
//!   agent's settings file is merged (`settings`) and the instructions file holds
//!   one marked block (`block`).
//! - `run edit` and `run session-start` are the guardrail cores (`run`). They speak
//!   a neutral request and verdict; an adapter's translator is the only thing that
//!   knows an agent.
//!
//! Nothing here names an agent: an adapter is a directory of the bundle, found by
//! its name.

mod block;
mod bundle;
mod gate;
mod json;
mod lock;
mod post_command;
mod run;
mod settings;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::harness::{HarnessDecl, GUARDRAILS};
use urzua_core::report::{CouldNotRun, Notice, NoticeSeverity, Report};

use crate::emit;
use crate::session::Workspace;
use json::J;
use lock::{BlockHash, EntryHash, Lock, LOCK_PATH};

pub use post_command::run_post_command;
pub use run::{run_edit, run_session_start};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// One thing the bundle puts in the repository, and what became of it.
#[derive(serde::Serialize)]
struct Item {
    /// `file` (owned), `entry` (a hook entry merged into a shared settings file),
    /// `block` (the marked instructions block) or `lock`.
    kind: &'static str,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
    /// install: `created`, `unchanged`, `merged`, `would-create`...; status: `ok`,
    /// `missing`, `modified`, `out-of-date`; update: `updated`, `kept-modified`...
    state: String,
}

#[derive(serde::Serialize)]
struct HarnessReport {
    status: &'static str,
    action: &'static str,
    dry_run: bool,
    bundle_version: &'static str,
    adapters: Vec<String>,
    items: Vec<Item>,
    /// Guardrails the repository turned on that no chosen adapter can wire, and
    /// keys the build does not know: reported, never skipped.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    problems: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
    #[serde(skip)]
    exit: u8,
}

impl Report for HarnessReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.exit)
    }
}

impl HarnessReport {
    fn new(action: &'static str, dry_run: bool, adapters: Vec<String>) -> Self {
        HarnessReport {
            status: "ok",
            action,
            dry_run,
            bundle_version: VERSION,
            adapters,
            items: Vec::new(),
            problems: Vec::new(),
            notices: Vec::new(),
            exit: 0,
        }
    }

    fn item(&mut self, kind: &'static str, path: &str, key: Option<&str>, state: &str) {
        self.items.push(Item {
            kind,
            path: path.to_string(),
            key: key.map(str::to_string),
            state: state.to_string(),
        });
    }

    fn notice(&mut self, severity: NoticeSeverity, message: String) {
        self.notices.push(Notice {
            severity,
            subject: "harness".to_string(),
            message,
            detail: None,
        });
    }

    /// The run changed nothing because it refused.
    fn refuse(mut self, why: String) -> Self {
        self.status = "not-run";
        self.exit = 2;
        self.problems.push(why);
        self
    }

    fn drift(mut self) -> Self {
        self.status = "drift";
        self.exit = 1;
        self
    }
}

/// Everything an action reads: the repository, its configuration and the
/// bundle files the chosen adapters install.
struct Env {
    root: PathBuf,
    harness: Option<HarnessDecl>,
    /// Each declared record type and its directory, which the generated
    /// drafting commands are made from.
    types: Vec<(String, String)>,
}

fn declared_types(config: &urzua_core::config::Config) -> Vec<(String, String)> {
    let mut types: Vec<(String, String)> = config
        .record_types
        .iter()
        .map(|(name, t)| (name.clone(), t.dir.clone()))
        .collect();
    types.sort();
    types
}

fn entry_hash(file: &str, entry: &settings::Entry, hash: String) -> EntryHash {
    EntryHash {
        file: file.to_string(),
        event: entry.event.clone(),
        matcher: entry.matcher.clone(),
        handler: entry.handler.clone(),
        hash,
    }
}

/// Whether a lock entry and a bundle entry are the same hook (event, matcher, handler).
fn same(e: &EntryHash, file: &str, w: &settings::Entry) -> bool {
    e.file == file && e.event == w.event && e.matcher == w.matcher && e.handler == w.handler
}

/// What the bundle wants in the repository for a set of adapters.
struct Desired {
    files: BTreeMap<String, String>,
    entries: Vec<(String, settings::Entry)>,
    /// (file, rendered block).
    block: Option<(String, String)>,
}

fn desired(env: &Env, adapters: &[String]) -> Result<Desired, String> {
    let mut want = Desired {
        files: BTreeMap::new(),
        entries: Vec::new(),
        block: None,
    };
    for name in adapters {
        let adapter = bundle::adapter(name).ok_or_else(|| unknown_adapter(name))?;
        for (path, content) in adapter.files {
            want.files.insert(path.to_string(), content.to_string());
        }
        if let Some(dir) = adapter.manifest().commands_dir {
            for (path, content) in commands(adapter, &dir, &env.types)? {
                want.files.insert(path, content);
            }
        }
        let fragment = json::parse(&render_fragment(adapter)?)
            .map_err(|e| format!("the {name} settings fragment is invalid: {e}"))?;
        for entry in settings::entries(&fragment)? {
            want.entries.push((adapter.manifest().settings_file, entry));
        }
    }
    if let Some(file) = env
        .harness
        .as_ref()
        .and_then(|h| h.instructions_file.clone())
    {
        want.block = Some((file, block::render(bundle::INSTRUCTIONS)));
    }
    Ok(want)
}

/// The adapter's settings fragment with every `{{sha256:PATH}}` replaced by the
/// sha256 (hex) of the adapter's file installed at `PATH`. A hook command that
/// runs an installed file embeds that file's hash this way and refuses to run
/// the file when it no longer matches (`ADR-89`, amendment of 2026-11-08); the
/// value is written by `install` and `update` and read back by `status`.
fn render_fragment(adapter: &bundle::Adapter) -> Result<String, String> {
    let mut text = adapter.settings_fragment.to_string();
    while let Some(at) = text.find("{{sha256:") {
        let rest = &text[at + "{{sha256:".len()..];
        let end = rest.find("}}").ok_or_else(|| {
            format!(
                "the {} settings fragment has an unterminated {{{{sha256:",
                adapter.name
            )
        })?;
        let path = rest[..end].to_string();
        let (_, content) = adapter
            .files
            .iter()
            .find(|(p, _)| *p == path)
            .ok_or_else(|| {
                format!(
                    "the {} settings fragment names {path}, which the adapter does not install",
                    adapter.name
                )
            })?;
        let marker = format!("{{{{sha256:{path}}}}}");
        text = text.replace(&marker, &urzua_io::sha256_hex_of_str(content));
    }
    Ok(text)
}

/// The slash commands an adapter installs (`ADR-89` section 1): the one generic
/// drafting template rendered for every declared record type, and the lifecycle
/// commands as they are. The type name and directory are the only things filled
/// in, both read from the configuration; the engine adds no schema knowledge.
fn commands(
    adapter: &bundle::Adapter,
    dir: &str,
    types: &[(String, String)],
) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let lifecycle: Vec<&str> = adapter
        .commands
        .iter()
        .map(|(n, _)| n.trim_end_matches(".md"))
        .filter(|n| *n != "draft")
        .collect();
    for (name, content) in adapter.commands {
        if *name == "draft.md" {
            for (ty, ty_dir) in types {
                if lifecycle.contains(&ty.as_str()) {
                    return Err(format!(
                        "the record type '{ty}' has the name of the lifecycle command /{ty}, so its drafting command would collide with it; rename the type"
                    ));
                }
                out.push((
                    format!("{dir}/{ty}.md"),
                    content.replace("{{type}}", ty).replace("{{dir}}", ty_dir),
                ));
            }
        } else {
            out.push((format!("{dir}/{name}"), content.to_string()));
        }
    }
    Ok(out)
}

fn unknown_adapter(name: &str) -> String {
    format!(
        "'{name}' is not an adapter this build embeds (it ships: {})",
        bundle::ADAPTERS
            .iter()
            .map(|a| a.name)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn read(root: &Path, rel: &str) -> Result<Option<String>, String> {
    match std::fs::read_to_string(root.join(rel)) {
        Ok(t) => Ok(Some(t)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("could not read {rel}: {e}")),
    }
}

fn write(root: &Path, rel: &str, text: &str) -> Result<(), String> {
    let path = root.join(rel);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    }
    urzua_io::write_atomic(&path, text).map_err(|e| format!("could not write {rel}: {e}"))
}

/// Deletes a file and the empty directories it leaves, up to the repository root.
fn delete(root: &Path, rel: &str) -> Result<(), String> {
    let path = root.join(rel);
    std::fs::remove_file(&path).map_err(|e| format!("could not remove {rel}: {e}"))?;
    let mut dir = path.parent();
    while let Some(d) = dir {
        if d == root || std::fs::remove_dir(d).is_err() {
            break;
        }
        dir = d.parent();
    }
    Ok(())
}

/// A settings file as the merge sees it: its text and parse, or absent.
struct Settings {
    text: Option<String>,
    doc: J,
    /// The file was not in the canonical two-space form, so writing it back
    /// re-indents it.
    reformatted: bool,
}

fn load_settings(root: &Path, rel: &str) -> Result<Settings, String> {
    let text = read(root, rel)?;
    let doc = match &text {
        None => J::Obj(Vec::new()),
        Some(t) => json::parse(t).map_err(|e| {
            format!("{rel} is not valid JSON ({e}); the install will not guess, fix or move it")
        })?,
    };
    let reformatted = text
        .as_deref()
        .is_some_and(|t| t.trim_end_matches('\n') != doc.pretty());
    Ok(Settings {
        text,
        doc,
        reformatted,
    })
}

fn save_settings(root: &Path, rel: &str, s: &Settings) -> Result<(), String> {
    let ends_with_newline = s.text.as_deref().is_none_or(|t| t.ends_with('\n'));
    let mut out = s.doc.pretty();
    if ends_with_newline {
        out.push('\n');
    }
    write(root, rel, &out)
}

fn open(config: Option<PathBuf>) -> Result<Env, String> {
    let ws = Workspace::open(&[PathBuf::from(".")], config).map_err(|e| e.message)?;
    Ok(Env {
        root: ws.repo_root,
        types: declared_types(&ws.config),
        harness: ws.config.harness,
    })
}

fn not_run(message: impl Into<String>) -> ExitCode {
    emit(&CouldNotRun::from(message.into()))
}

fn load_lock(env: &Env) -> Result<Option<Lock>, String> {
    read(&env.root, LOCK_PATH)?
        .map(|t| Lock::parse(&t))
        .transpose()
}

/// The adapters a run applies to: `--adapter`, else `harness.adapters`.
fn chosen_adapters(env: &Env, flag: Option<String>) -> Vec<String> {
    match flag {
        Some(name) => vec![name],
        None => env
            .harness
            .as_ref()
            .map(|h| h.adapters.clone())
            .unwrap_or_default(),
    }
}

/// Guardrails the repository turned on that none of `adapters` can wire, and
/// the keys this build does not know.
fn harness_problems(env: &Env, adapters: &[String]) -> (Vec<String>, Vec<String>) {
    let (mut unknown, mut unprovided) = (Vec::new(), Vec::new());
    let Some(h) = &env.harness else {
        return (unknown, unprovided);
    };
    unknown.extend(h.problems.iter().cloned());
    for g in GUARDRAILS {
        if !h.enabled(g.name) {
            continue;
        }
        let provided = adapters.iter().any(|a| {
            bundle::adapter(a).is_some_and(|a| a.manifest().provides.iter().any(|p| p == g.name))
        });
        if !provided {
            unprovided.push(format!(
                "guardrail '{}' is enabled but no chosen adapter ({}) provides it in this bundle version",
                g.name,
                if adapters.is_empty() {
                    "none".to_string()
                } else {
                    adapters.join(", ")
                }
            ));
        }
    }
    (unknown, unprovided)
}

/// `past` is the tense of an item state: what happened, or what a dry run would do.
fn past(dry_run: bool, done: &'static str, would: &'static str) -> &'static str {
    if dry_run {
        would
    } else {
        done
    }
}

/// The settings file `file`, loaded once into `shared` and kept there while an
/// action merges, replaces or removes its entries.
fn shared_settings<'a>(
    root: &Path,
    shared: &'a mut BTreeMap<String, Settings>,
    file: &str,
) -> Result<&'a mut Settings, String> {
    match shared.entry(file.to_string()) {
        std::collections::btree_map::Entry::Occupied(o) => Ok(o.into_mut()),
        std::collections::btree_map::Entry::Vacant(v) => Ok(v.insert(load_settings(root, file)?)),
    }
}

/// Opens the repository and its lock. A repository whose harness is not
/// installed is a not-run; `hint` ends the message.
fn open_installed(config: Option<PathBuf>, hint: &str) -> Result<(Env, Lock), ExitCode> {
    let env = open(config).map_err(not_run)?;
    match load_lock(&env) {
        Ok(Some(lock)) => Ok((env, lock)),
        Ok(None) => Err(not_run(format!(
            "no {LOCK_PATH}: the harness is not installed{hint}"
        ))),
        Err(m) => Err(not_run(m)),
    }
}

const INSTALL_HINT: &str = " (run `urzua harness install --adapter NAME`)";

// ---------------------------------------------------------------- install

pub fn run_install(config: Option<PathBuf>, adapter: Option<String>, dry_run: bool) -> ExitCode {
    let env = match open(config) {
        Ok(e) => e,
        Err(m) => return not_run(m),
    };
    emit(&install(&env, adapter, dry_run))
}

/// What an install has decided to write, collected before anything is written.
#[derive(Default)]
struct InstallPlan {
    conflicts: Vec<String>,
    file_writes: Vec<(String, String)>,
    shared: BTreeMap<String, Settings>,
    block_write: Option<(String, String)>,
}

fn install(env: &Env, flag: Option<String>, dry_run: bool) -> HarnessReport {
    let adapters = chosen_adapters(env, flag);
    let mut report = HarnessReport::new("install", dry_run, adapters.clone());
    if adapters.is_empty() {
        report.notice(
            NoticeSeverity::Info,
            "no adapter is named (pass --adapter, or list adapters under `harness` in .urzua/config.yaml): nothing was installed and nothing is enabled".to_string(),
        );
        return report;
    }
    let (unknown, unprovided) = harness_problems(env, &adapters);
    if !unknown.is_empty() {
        return report.refuse(unknown.join("; "));
    }
    let want = match desired(env, &adapters) {
        Ok(w) => w,
        Err(m) => return report.refuse(m),
    };
    let old_lock = load_lock(env).unwrap_or(None);
    let mut plan = InstallPlan::default();
    let planned = plan_install_files(
        env,
        &want,
        old_lock.as_ref(),
        dry_run,
        &mut report,
        &mut plan,
    )
    .and_then(|()| plan_install_entries(env, &want, dry_run, &mut report, &mut plan))
    .and_then(|()| plan_install_block(env, &want, dry_run, &mut report, &mut plan));
    if let Err(m) = planned {
        return report.refuse(m);
    }

    // A hand-copied first-generation editor hook wired beside the bundle's.
    for (file, s) in &plan.shared {
        let legacy = settings::legacy_commands(&s.doc);
        if !legacy.is_empty() {
            report.notice(
                NoticeSeverity::Warning,
                settings::legacy_warning(file, &legacy),
            );
        }
    }
    if !plan.conflicts.is_empty() {
        // Nothing is written, so no item may say it was: report what the
        // install would do once the conflict is cleared.
        for item in &mut report.items {
            item.state = match item.state.as_str() {
                "created" => "would-create",
                "merged" => "would-merge",
                "written" => "would-write",
                other => other,
            }
            .to_string();
        }
        return report.refuse(plan.conflicts.join("; "));
    }
    for (file, s) in &plan.shared {
        if s.reformatted {
            report.notice(
                NoticeSeverity::Warning,
                format!("{file} was not in the two-space JSON form, so it is re-indented when written; its keys and entries are unchanged"),
            );
        }
    }
    for p in &unprovided {
        report.notice(NoticeSeverity::Warning, p.clone());
    }
    report.problems.extend(unprovided);

    let new_lock = install_lock(&want, adapters);
    report.item(
        "lock",
        LOCK_PATH,
        None,
        past(dry_run, "written", "would-write"),
    );
    if dry_run {
        return report;
    }
    match write_install(env, &plan, &new_lock) {
        Ok(()) => report,
        Err(m) => report.refuse(m),
    }
}

/// Whole files: create, or leave if identical. A different file is a person's
/// (or an older bundle's, which `update` replaces), never overwritten here.
fn plan_install_files(
    env: &Env,
    want: &Desired,
    old_lock: Option<&Lock>,
    dry_run: bool,
    report: &mut HarnessReport,
    plan: &mut InstallPlan,
) -> Result<(), String> {
    for (path, content) in &want.files {
        match read(&env.root, path)? {
            None => {
                report.item("file", path, None, past(dry_run, "created", "would-create"));
                plan.file_writes.push((path.clone(), content.clone()));
            }
            Some(existing) if existing == *content => {
                report.item("file", path, None, "unchanged");
            }
            Some(existing) => {
                let owned = old_lock
                    .and_then(|l| l.files.get(path))
                    .is_some_and(|h| *h == lock::hash(&existing));
                report.item(
                    "file",
                    path,
                    None,
                    if owned { "out-of-date" } else { "modified" },
                );
                plan.conflicts.push(if owned {
                    format!("{path} is an older bundle file; run `urzua harness update`")
                } else {
                    format!("{path} exists and differs from the bundle; the install will not overwrite a file it does not own: move or delete it and install again")
                });
            }
        }
    }
    Ok(())
}

/// Hook entries merged into the shared settings file.
fn plan_install_entries(
    env: &Env,
    want: &Desired,
    dry_run: bool,
    report: &mut HarnessReport,
    plan: &mut InstallPlan,
) -> Result<(), String> {
    for (file, entry) in &want.entries {
        let s = shared_settings(&env.root, &mut plan.shared, file)?;
        let key = entry.key();
        match settings::add(&mut s.doc, entry) {
            Err(m) => return Err(format!("{file}: {m}")),
            Ok(settings::Merged::Added) => report.item(
                "entry",
                file,
                Some(&key),
                past(dry_run, "merged", "would-merge"),
            ),
            Ok(settings::Merged::Unchanged) => report.item("entry", file, Some(&key), "unchanged"),
            Ok(settings::Merged::Edited) => {
                report.item("entry", file, Some(&key), "modified");
                plan.conflicts.push(format!(
                    "{file} holds an entry for '{key}' that differs from the bundle's (a person edited it, or it is an older bundle's); the install will not guess"
                ));
            }
        }
    }
    Ok(())
}

/// The marked instructions block.
fn plan_install_block(
    env: &Env,
    want: &Desired,
    dry_run: bool,
    report: &mut HarnessReport,
    plan: &mut InstallPlan,
) -> Result<(), String> {
    match &want.block {
        None => {
            report.item("block", "", None, "not-configured");
            report.notice(
                NoticeSeverity::Info,
                "harness.instructions_file is not set, so no agent-instructions block was written; the engine assumes no file name".to_string(),
            );
        }
        Some((file, rendered)) => {
            let text = read(&env.root, file)?.unwrap_or_default();
            match block::apply(&text, rendered) {
                Err(m) => return Err(format!("{file}: {m}")),
                Ok(new) if new == text => report.item("block", file, None, "unchanged"),
                Ok(new) => {
                    report.item("block", file, None, past(dry_run, "written", "would-write"));
                    plan.block_write = Some((file.clone(), new));
                }
            }
        }
    }
    Ok(())
}

fn install_lock(want: &Desired, adapters: Vec<String>) -> Lock {
    let mut new_lock = Lock::new(adapters);
    new_lock.adapters.sort();
    for (path, content) in &want.files {
        new_lock.files.insert(path.clone(), lock::hash(content));
    }
    for (file, entry) in &want.entries {
        new_lock.entries.push(entry_hash(file, entry, entry.hash()));
    }
    if let Some((file, rendered)) = &want.block {
        new_lock.blocks.push(BlockHash {
            file: file.clone(),
            hash: lock::hash(rendered),
        });
    }
    new_lock
}

fn write_install(env: &Env, plan: &InstallPlan, new_lock: &Lock) -> Result<(), String> {
    for (path, content) in &plan.file_writes {
        write(&env.root, path, content)?;
    }
    for (file, s) in &plan.shared {
        save_settings(&env.root, file, s)?;
    }
    if let Some((file, text)) = &plan.block_write {
        write(&env.root, file, text)?;
    }
    write(&env.root, LOCK_PATH, &new_lock.to_text())
}

// ----------------------------------------------------------------- status

pub fn run_status(config: Option<PathBuf>) -> ExitCode {
    let (env, lock) = match open_installed(config, INSTALL_HINT) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    emit(&status(&env, &lock))
}

/// A status report and whether any item in it has drifted.
struct StatusRun {
    report: HarnessReport,
    drift: bool,
}

impl StatusRun {
    fn mark(&mut self, kind: &'static str, path: &str, key: Option<&str>, state: &str) {
        self.drift |= state != "ok";
        self.report.item(kind, path, key, state);
    }
}

fn status(env: &Env, lock: &Lock) -> HarnessReport {
    let mut run = StatusRun {
        report: HarnessReport::new("status", false, lock.adapters.clone()),
        drift: false,
    };
    let want = match desired(env, &lock.adapters) {
        Ok(w) => w,
        Err(m) => return run.report.refuse(m),
    };
    status_bundle_version(lock, &mut run);
    let checked = status_files(env, lock, &want, &mut run)
        .and_then(|()| status_entries(env, lock, &want, &mut run))
        .and_then(|()| status_blocks(env, lock, &want, &mut run));
    if let Err(m) = checked {
        return run.report.refuse(m);
    }
    let StatusRun { mut report, drift } = run;
    report.notice(
        NoticeSeverity::Info,
        "a local settings file next to the project settings file (settings.local.json) and the user-level settings are neither read nor written: a hook entry wired only there is not seen".to_string(),
    );
    let (unknown, unprovided) = harness_problems(env, &lock.adapters);
    report.problems.extend(unknown);
    report.problems.extend(unprovided);
    if drift || !report.problems.is_empty() {
        report = report.drift();
    }
    report
}

fn status_bundle_version(lock: &Lock, run: &mut StatusRun) {
    if lock.bundle_version == VERSION {
        return;
    }
    let older = lock.bundle_version.as_str() < VERSION;
    run.report.notice(
        NoticeSeverity::Warning,
        if older {
            format!("the installed bundle is {} and this binary is {VERSION}: run `urzua harness update`", lock.bundle_version)
        } else {
            format!("the installed bundle is {} and this binary is only {VERSION}: update urzua", lock.bundle_version)
        },
    );
    run.mark("lock", LOCK_PATH, None, "out-of-date");
}

fn status_files(env: &Env, lock: &Lock, want: &Desired, run: &mut StatusRun) -> Result<(), String> {
    for (path, locked) in &lock.files {
        let state = match read(&env.root, path)? {
            None => "missing",
            Some(t) if lock::hash(&t) != *locked => "modified",
            Some(_) => match want.files.get(path) {
                Some(bundle) if lock::hash(bundle) != *locked => "out-of-date",
                _ => "ok",
            },
        };
        run.mark("file", path, None, state);
    }
    // A file the bundle now wants that the lock does not know yet (a command for
    // a record type added since the install) is missing until `update` writes it.
    // One that exists is a person's file under the bundle's name (`update` kept
    // it): it is not missing, and it is not the install's until `update --force`.
    for (path, bundle) in want
        .files
        .iter()
        .filter(|(p, _)| !lock.files.contains_key(*p))
    {
        let state = match read(&env.root, path)? {
            None => "missing",
            Some(t) if t == *bundle => "out-of-date",
            Some(_) => "modified",
        };
        run.mark("file", path, None, state);
    }
    Ok(())
}

fn status_entries(
    env: &Env,
    lock: &Lock,
    want: &Desired,
    run: &mut StatusRun,
) -> Result<(), String> {
    let mut cache: BTreeMap<String, Option<J>> = BTreeMap::new();
    for e in &lock.entries {
        if !cache.contains_key(&e.file) {
            let s = load_settings(&env.root, &e.file)?;
            cache.insert(e.file.clone(), s.text.map(|_| s.doc));
        }
        let state = match cache[&e.file]
            .as_ref()
            .and_then(|doc| settings::find(doc, &e.event, &e.matcher, &e.handler))
        {
            None => "missing",
            Some(h) if lock::hash(&h.compact()) != e.hash => "modified",
            Some(_) => match want.entries.iter().find(|(f, w)| same(e, f, w)) {
                Some((_, w)) if w.hash() != e.hash => "out-of-date",
                _ => "ok",
            },
        };
        run.mark("entry", &e.file, Some(&e.key()), state);
    }
    Ok(())
}

fn status_blocks(
    env: &Env,
    lock: &Lock,
    want: &Desired,
    run: &mut StatusRun,
) -> Result<(), String> {
    for b in &lock.blocks {
        let state = match read(&env.root, &b.file)? {
            None => "missing",
            Some(t) => match block::extract(&t) {
                Err(_) | Ok(None) => "missing",
                Ok(Some(found)) if lock::hash(found) != b.hash => "modified",
                Ok(Some(_)) => match &want.block {
                    Some((_, rendered)) if lock::hash(rendered) != b.hash => "out-of-date",
                    _ => "ok",
                },
            },
        };
        run.mark("block", &b.file, None, state);
    }
    Ok(())
}

// ----------------------------------------------------------------- update

pub fn run_update(config: Option<PathBuf>, force: bool, dry_run: bool) -> ExitCode {
    let (env, lock) = match open_installed(config, INSTALL_HINT) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    emit(&update(&env, &lock, force, dry_run))
}

/// An update in progress: the report, the lock it is building, and the writes
/// it has decided on.
struct UpdateRun<'a> {
    env: &'a Env,
    old: &'a Lock,
    want: Desired,
    force: bool,
    dry_run: bool,
    report: HarnessReport,
    lock: Lock,
    /// A file a person changed was left as it is.
    kept: bool,
    file_writes: Vec<(String, String)>,
    deletes: Vec<String>,
    shared: BTreeMap<String, Settings>,
    block_writes: Vec<(String, String)>,
}

fn update(env: &Env, old: &Lock, force: bool, dry_run: bool) -> HarnessReport {
    let report = HarnessReport::new("update", dry_run, old.adapters.clone());
    let want = match desired(env, &old.adapters) {
        Ok(w) => w,
        Err(m) => return report.refuse(m),
    };
    let mut run = UpdateRun {
        env,
        old,
        want,
        force,
        dry_run,
        report,
        lock: Lock::new(old.adapters.clone()),
        kept: false,
        file_writes: Vec::new(),
        deletes: Vec::new(),
        shared: BTreeMap::new(),
        block_writes: Vec::new(),
    };
    let planned = run
        .wanted_files()
        .and_then(|()| run.retired_files())
        .and_then(|()| run.wanted_entries())
        .and_then(|()| run.retired_entries())
        .and_then(|()| run.block());
    if let Err(m) = planned {
        return run.report.refuse(m);
    }

    run.report.item(
        "lock",
        LOCK_PATH,
        None,
        past(dry_run, "written", "would-write"),
    );
    let (unknown, unprovided) = harness_problems(env, &old.adapters);
    for p in unknown.iter().chain(&unprovided) {
        run.report.notice(NoticeSeverity::Warning, p.clone());
    }
    run.report.problems.extend(unknown);
    run.report.problems.extend(unprovided);
    if !dry_run {
        if let Err(m) = run.apply() {
            return run.report.refuse(m);
        }
    }
    let mut report = run.report;
    if run.kept {
        report.notice(
            NoticeSeverity::Warning,
            "a file a person changed was left as it is (pass --force to overwrite it)".to_string(),
        );
        report = report.drift();
    }
    report
}

impl UpdateRun<'_> {
    /// Files the bundle wants: create, update what the lock owns, keep what a
    /// person changed.
    fn wanted_files(&mut self) -> Result<(), String> {
        for (path, content) in &self.want.files {
            let locked = self.old.files.get(path);
            let want_hash = lock::hash(content);
            let state = match read(&self.env.root, path)? {
                None => {
                    self.file_writes.push((path.clone(), content.clone()));
                    self.lock.files.insert(path.clone(), want_hash);
                    past(self.dry_run, "created", "would-create")
                }
                Some(t) if t == *content => {
                    self.lock.files.insert(path.clone(), want_hash);
                    "unchanged"
                }
                Some(t) if locked.is_some_and(|h| *h == lock::hash(&t)) || self.force => {
                    self.file_writes.push((path.clone(), content.clone()));
                    self.lock.files.insert(path.clone(), want_hash);
                    if locked.is_some_and(|h| *h == lock::hash(&t)) {
                        past(self.dry_run, "updated", "would-update")
                    } else {
                        past(self.dry_run, "forced", "would-force")
                    }
                }
                Some(_) => {
                    self.kept = true;
                    // The old hash stays, so `status` keeps reporting the change.
                    if let Some(h) = locked {
                        self.lock.files.insert(path.clone(), h.clone());
                    }
                    "kept-modified"
                }
            };
            self.report.item("file", path, None, state);
        }
        Ok(())
    }

    /// Files the old lock owns that the bundle no longer wants.
    fn retired_files(&mut self) -> Result<(), String> {
        for (path, locked) in &self.old.files {
            if self.want.files.contains_key(path) {
                continue;
            }
            match read(&self.env.root, path)? {
                None => self.report.item("file", path, None, "gone"),
                Some(t) if lock::hash(&t) == *locked || self.force => {
                    self.deletes.push(path.clone());
                    self.report.item(
                        "file",
                        path,
                        None,
                        past(self.dry_run, "removed", "would-remove"),
                    );
                }
                Some(_) => {
                    self.kept = true;
                    self.lock.files.insert(path.clone(), locked.clone());
                    self.report.item("file", path, None, "kept-modified");
                }
            }
        }
        Ok(())
    }

    /// Hook entries the bundle wants in the shared settings files.
    fn wanted_entries(&mut self) -> Result<(), String> {
        for (file, entry) in &self.want.entries {
            let s = shared_settings(&self.env.root, &mut self.shared, file)?;
            let key = entry.key();
            let locked = self.old.entries.iter().find(|e| same(e, file, entry));
            let found =
                settings::find(&s.doc, &entry.event, &entry.matcher, &entry.handler).cloned();
            let state = match found {
                None => {
                    let _ = settings::add(&mut s.doc, entry);
                    past(self.dry_run, "merged", "would-merge")
                }
                Some(h) if h == entry.hook => "unchanged",
                Some(h)
                    if locked.is_some_and(|l| l.hash == lock::hash(&h.compact())) || self.force =>
                {
                    if let Err(m) = settings::replace(&mut s.doc, entry) {
                        return Err(format!("{file}: {m}"));
                    }
                    past(self.dry_run, "updated", "would-update")
                }
                Some(_) => {
                    self.kept = true;
                    "kept-modified"
                }
            };
            self.report.item("entry", file, Some(&key), state);
            let hash = if state == "kept-modified" {
                locked
                    .map(|l| l.hash.clone())
                    .unwrap_or_else(|| entry.hash())
            } else {
                entry.hash()
            };
            self.lock.entries.push(entry_hash(file, entry, hash));
        }
        Ok(())
    }

    /// Entries the old lock owns that the bundle no longer wants.
    fn retired_entries(&mut self) -> Result<(), String> {
        for e in &self.old.entries {
            if self.want.entries.iter().any(|(f, w)| same(e, f, w)) {
                continue;
            }
            let Some(s) = self.shared.get_mut(&e.file) else {
                continue;
            };
            let key = e.key();
            match settings::find(&s.doc, &e.event, &e.matcher, &e.handler) {
                None => self.report.item("entry", &e.file, Some(&key), "gone"),
                Some(h) if lock::hash(&h.compact()) == e.hash || self.force => {
                    settings::remove(&mut s.doc, &e.event, &e.matcher, &e.handler);
                    self.report.item(
                        "entry",
                        &e.file,
                        Some(&key),
                        past(self.dry_run, "removed", "would-remove"),
                    );
                }
                Some(_) => {
                    self.kept = true;
                    self.lock.entries.push(e.clone());
                    self.report
                        .item("entry", &e.file, Some(&key), "kept-modified");
                }
            }
        }
        Ok(())
    }

    /// The marked instructions block.
    fn block(&mut self) -> Result<(), String> {
        let Some((file, rendered)) = &self.want.block else {
            return Ok(());
        };
        let text = read(&self.env.root, file)?.unwrap_or_default();
        let locked = self.old.blocks.iter().find(|b| b.file == *file);
        let current = match block::extract(&text) {
            Ok(c) => c.map(str::to_string),
            Err(m) => return Err(format!("{file}: {m}")),
        };
        let state = match &current {
            Some(c) if c == rendered => "unchanged",
            Some(c) if locked.is_some_and(|l| l.hash == lock::hash(c)) || self.force => {
                past(self.dry_run, "updated", "would-update")
            }
            Some(_) => "kept-modified",
            None => past(self.dry_run, "written", "would-write"),
        };
        if state == "kept-modified" {
            self.kept = true;
            if let Some(l) = locked {
                self.lock.blocks.push(l.clone());
            }
        } else {
            self.lock.blocks.push(BlockHash {
                file: file.clone(),
                hash: lock::hash(rendered),
            });
            if state != "unchanged" {
                match block::apply(&text, rendered) {
                    Ok(new) => self.block_writes.push((file.clone(), new)),
                    Err(m) => return Err(format!("{file}: {m}")),
                }
            }
        }
        self.report.item("block", file, None, state);
        Ok(())
    }

    fn apply(&self) -> Result<(), String> {
        let root = &self.env.root;
        for (path, content) in &self.file_writes {
            write(root, path, content)?;
        }
        for path in &self.deletes {
            delete(root, path)?;
        }
        for (file, s) in &self.shared {
            save_settings(root, file, s)?;
        }
        for (file, text) in &self.block_writes {
            write(root, file, text)?;
        }
        write(root, LOCK_PATH, &self.lock.to_text())
    }
}

// -------------------------------------------------------------- uninstall

pub fn run_uninstall(config: Option<PathBuf>, dry_run: bool) -> ExitCode {
    let (env, lock) = match open_installed(config, ", nothing to remove") {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    emit(&uninstall(&env, &lock, dry_run))
}

/// An uninstall in progress: the report and the removals it has decided on.
struct UninstallRun<'a> {
    env: &'a Env,
    lock: &'a Lock,
    dry_run: bool,
    report: HarnessReport,
    /// A file or entry a person changed was left in place.
    kept: bool,
    deletes: Vec<String>,
    shared: BTreeMap<String, Settings>,
    block_writes: Vec<(String, String)>,
}

fn uninstall(env: &Env, lock: &Lock, dry_run: bool) -> HarnessReport {
    let mut run = UninstallRun {
        env,
        lock,
        dry_run,
        report: HarnessReport::new("uninstall", dry_run, lock.adapters.clone()),
        kept: false,
        deletes: Vec::new(),
        shared: BTreeMap::new(),
        block_writes: Vec::new(),
    };
    let planned = run
        .files()
        .and_then(|()| run.entries())
        .and_then(|()| run.blocks());
    if let Err(m) = planned {
        return run.report.refuse(m);
    }
    run.report.item(
        "lock",
        LOCK_PATH,
        None,
        past(dry_run, "removed", "would-remove"),
    );

    if !dry_run {
        if let Err(m) = run.apply() {
            return run.report.refuse(m);
        }
    }
    let mut report = run.report;
    if run.kept {
        report.notice(
            NoticeSeverity::Warning,
            "a file or entry a person changed was left in place; remove it by hand".to_string(),
        );
        report = report.drift();
    }
    report
}

impl UninstallRun<'_> {
    fn files(&mut self) -> Result<(), String> {
        for (path, locked) in &self.lock.files {
            let state = match read(&self.env.root, path)? {
                None => "missing",
                Some(t) if lock::hash(&t) == *locked => {
                    self.deletes.push(path.clone());
                    past(self.dry_run, "removed", "would-remove")
                }
                Some(_) => {
                    self.kept = true;
                    "kept-modified"
                }
            };
            self.report.item("file", path, None, state);
        }
        Ok(())
    }

    fn entries(&mut self) -> Result<(), String> {
        for e in &self.lock.entries {
            let s = shared_settings(&self.env.root, &mut self.shared, &e.file)?;
            let state = match settings::find(&s.doc, &e.event, &e.matcher, &e.handler) {
                None => "missing",
                Some(h) if lock::hash(&h.compact()) == e.hash => {
                    settings::remove(&mut s.doc, &e.event, &e.matcher, &e.handler);
                    past(self.dry_run, "removed", "would-remove")
                }
                Some(_) => {
                    self.kept = true;
                    "kept-modified"
                }
            };
            self.report.item("entry", &e.file, Some(&e.key()), state);
        }
        Ok(())
    }

    fn blocks(&mut self) -> Result<(), String> {
        for b in &self.lock.blocks {
            let text = read(&self.env.root, &b.file)?;
            let state = match text.as_deref().map(|t| (t, block::extract(t))) {
                None => "missing",
                Some((_, Err(_) | Ok(None))) => "missing",
                Some((t, Ok(Some(found)))) if lock::hash(found) == b.hash => {
                    match block::remove(t) {
                        Ok(new) => self.block_writes.push((b.file.clone(), new)),
                        Err(m) => return Err(format!("{}: {m}", b.file)),
                    }
                    past(self.dry_run, "removed", "would-remove")
                }
                Some(_) => {
                    self.kept = true;
                    "kept-modified"
                }
            };
            self.report.item("block", &b.file, None, state);
        }
        Ok(())
    }

    fn apply(&self) -> Result<(), String> {
        let root = &self.env.root;
        for path in &self.deletes {
            delete(root, path)?;
        }
        for (file, s) in &self.shared {
            if s.doc.is_empty_container() && s.text.is_some() {
                // Nothing but what the install added: the file goes with it.
                delete(root, file)?;
            } else if s.text.is_some() {
                save_settings(root, file, s)?;
            }
        }
        for (file, text) in &self.block_writes {
            if text.trim().is_empty() {
                delete(root, file)?;
            } else {
                write(root, file, text)?;
            }
        }
        // The lock goes last.
        delete(root, LOCK_PATH)
    }
}

// --------------------------------------------------------------- adapters

#[derive(serde::Serialize)]
struct AdapterEntry {
    name: String,
    description: String,
    provides: Vec<String>,
    settings_file: String,
    files: Vec<String>,
    requires_interpreters: Vec<String>,
    requires_env: Vec<String>,
}

#[derive(serde::Serialize)]
struct AdaptersReport {
    status: &'static str,
    bundle_version: &'static str,
    adapters: Vec<AdapterEntry>,
}

impl Report for AdaptersReport {
    fn notices(&self) -> &[Notice] {
        &[]
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::SUCCESS
    }
}

pub fn run_adapters() -> ExitCode {
    emit(&AdaptersReport {
        status: "ok",
        bundle_version: VERSION,
        adapters: bundle::ADAPTERS
            .iter()
            .map(|a| {
                let m = a.manifest();
                AdapterEntry {
                    name: m.name,
                    description: m.description,
                    provides: m.provides,
                    settings_file: m.settings_file,
                    files: a.files.iter().map(|(p, _)| p.to_string()).collect(),
                    requires_interpreters: m.requires.interpreters,
                    requires_env: m.requires.env,
                }
            })
            .collect(),
    })
}

/// The adapters the lock names, for `doctor` and `session-start`.
pub(crate) fn installed_lock(root: &Path) -> Result<Option<Lock>, String> {
    read(root, LOCK_PATH)?.map(|t| Lock::parse(&t)).transpose()
}

pub(crate) fn engine_version() -> &'static str {
    VERSION
}

/// How serious one `doctor` finding about the harness is.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Level {
    Ok,
    Warn,
}

/// What `urzua doctor` says about the `authority` table (`ADR-89` section 4):
/// what it enforces, what it cannot, and that the host's settings are the host's.
/// Said whenever the table is declared, so it never reads as more protection than
/// it is.
fn authority_checks(config: &urzua_core::config::Config) -> Vec<(Level, &'static str, String)> {
    let mut out = vec![(
        Level::Ok,
        "authority-host",
        "authority is declared. The required status check, branch protection and any CODEOWNERS file are the host's settings, which urzua can neither read nor verify, so it does not claim them. Locally it refuses an agent's edit tools (a Bash command is reported, not stopped); in CI diff.decider-required proves authorship only, so with a single maintainer it cannot tell an agent acting under the maintainer's token from the maintainer (the accepted risk of ADR-89). A distinct agent identity, with CODEOWNERS on the protected paths and branch protection requiring the code owner, is a future option and is not configured here.".to_string(),
    )];
    let rule_on = config
        .rules
        .get(urzua_core::rules::RULE_DIFF_DECIDER_REQUIRED)
        .is_some_and(|s| s.level != urzua_core::config::RuleLevel::Off);
    if !rule_on {
        out.push((
            Level::Warn,
            "authority-host",
            format!(
                "authority is declared but the rule {} is not enabled, so CI checks no gated change against it",
                urzua_core::rules::RULE_DIFF_DECIDER_REQUIRED
            ),
        ));
    }
    out
}

/// Whether the harness is installed and matches its lock.
fn installed_checks(env: &Env) -> Vec<(Level, &'static str, String)> {
    let mut out = Vec::new();
    match load_lock(env) {
        Err(m) => out.push((Level::Warn, "harness-installed", m)),
        Ok(None) => out.push((
            Level::Warn,
            "harness-installed",
            format!(
                "harness is declared but {LOCK_PATH} does not exist: run `urzua harness install`"
            ),
        )),
        Ok(Some(lock)) => {
            let report = status(env, &lock);
            let drifted: Vec<String> = report
                .items
                .iter()
                .filter(|i| i.state != "ok")
                .map(|i| {
                    format!(
                        "{} {} is {}",
                        i.kind,
                        i.key.as_deref().unwrap_or(&i.path),
                        i.state
                    )
                })
                .collect();
            if drifted.is_empty() {
                out.push((
                    Level::Ok,
                    "harness-installed",
                    format!(
                        "the bundle ({}) is installed and matches its lock",
                        lock.bundle_version
                    ),
                ));
            }
            for d in drifted {
                out.push((
                    Level::Warn,
                    "harness-installed",
                    format!("{d} (run `urzua harness status`; `urzua harness update` repairs it)"),
                ));
            }
        }
    }
    out
}

/// A hand-copied first-generation editor hook wired beside the bundle's.
fn legacy_hook_checks(env: &Env, adapters: &[String]) -> Vec<(Level, &'static str, String)> {
    let mut out = Vec::new();
    if let Ok(want) = desired(env, adapters) {
        let files: std::collections::BTreeSet<&String> =
            want.entries.iter().map(|(f, _)| f).collect();
        for file in files {
            if let Ok(s) = load_settings(&env.root, file) {
                let legacy = settings::legacy_commands(&s.doc);
                if !legacy.is_empty() {
                    out.push((
                        Level::Warn,
                        "harness-legacy-hook",
                        settings::legacy_warning(file, &legacy),
                    ));
                }
            }
        }
    }
    out
}

/// What `urzua doctor` reports about the harness, when `harness` is declared:
/// unknown keys, guardrails no adapter provides, whether and how well it is
/// installed, and the escape hatch of a guardrail set to `on_unjudged: deny`.
pub(crate) fn doctor_checks(
    root: &Path,
    config: &urzua_core::config::Config,
) -> Vec<(Level, &'static str, String)> {
    let mut out = Vec::new();
    if config.authority.is_some() {
        out.extend(authority_checks(config));
    }
    let Some(harness) = config.harness.clone() else {
        return out;
    };
    let env = Env {
        root: root.to_path_buf(),
        harness: Some(harness.clone()),
        types: declared_types(config),
    };
    let adapters = chosen_adapters(&env, None);
    let (unknown, unprovided) = harness_problems(&env, &adapters);
    if unknown.is_empty() {
        out.push((
            Level::Ok,
            "harness-config",
            format!(
                "harness is declared (adapters: {})",
                if adapters.is_empty() {
                    "none".to_string()
                } else {
                    adapters.join(", ")
                }
            ),
        ));
    }
    for p in unknown.into_iter().chain(unprovided) {
        out.push((Level::Warn, "harness-config", p));
    }
    for name in &adapters {
        if bundle::adapter(name).is_none() {
            out.push((Level::Warn, "harness-config", unknown_adapter(name)));
        }
    }
    out.extend(installed_checks(&env));
    out.extend(legacy_hook_checks(&env, &adapters));
    if harness.any_enabled() && harness.outside_writes.is_none() {
        out.push((
            Level::Warn,
            "harness-outside-writes",
            "harness.outside_writes is not declared while guardrails are enabled, so an edit-tool write into a linked worktree of this repository (for example one at an older commit that holds no gates) is not judged by this repository's gates; set `outside_writes: shared` under harness (allow keeps today's behaviour, deny refuses any write outside the repository except under allow_paths)".to_string(),
        ));
    }
    for g in GUARDRAILS {
        if harness.enabled(g.name)
            && harness.on_unjudged(g.name) == urzua_core::harness::OnUnjudged::Deny
        {
            out.push((
                Level::Ok,
                "harness-escape-hatch",
                format!("{} is set to on_unjudged: deny, so an agent's edit is refused while it cannot judge; to repair it, a person edits outside the agent harness (a plain editor or a terminal), where no hook runs; the override of ADR-78 is not accepted for this verdict", g.name),
            ));
        }
    }
    out
}
