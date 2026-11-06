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
        let fragment = json::parse(adapter.settings_fragment)
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

// ---------------------------------------------------------------- install

pub fn run_install(config: Option<PathBuf>, adapter: Option<String>, dry_run: bool) -> ExitCode {
    let env = match open(config) {
        Ok(e) => e,
        Err(m) => return not_run(m),
    };
    emit(&install(&env, adapter, dry_run))
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
    let mut conflicts = Vec::new();

    // Whole files: create, or leave if identical. A different file is a person's
    // (or an older bundle's, which `update` replaces), never overwritten here.
    let mut file_writes = Vec::new();
    for (path, content) in &want.files {
        match read(&env.root, path) {
            Err(m) => return report.refuse(m),
            Ok(None) => {
                report.item(
                    "file",
                    path,
                    None,
                    if dry_run { "would-create" } else { "created" },
                );
                file_writes.push((path.clone(), content.clone()));
            }
            Ok(Some(existing)) if existing == *content => {
                report.item("file", path, None, "unchanged");
            }
            Ok(Some(existing)) => {
                let owned = old_lock
                    .as_ref()
                    .and_then(|l| l.files.get(path))
                    .is_some_and(|h| *h == lock::hash(&existing));
                report.item(
                    "file",
                    path,
                    None,
                    if owned { "out-of-date" } else { "modified" },
                );
                conflicts.push(if owned {
                    format!("{path} is an older bundle file; run `urzua harness update`")
                } else {
                    format!("{path} exists and differs from the bundle; the install will not overwrite it (move it, or run `urzua harness update --force` after an install)")
                });
            }
        }
    }

    // Hook entries merged into the shared settings file.
    let mut shared: BTreeMap<String, Settings> = BTreeMap::new();
    for (file, entry) in &want.entries {
        if !shared.contains_key(file) {
            match load_settings(&env.root, file) {
                Ok(s) => {
                    shared.insert(file.clone(), s);
                }
                Err(m) => return report.refuse(m),
            }
        }
        let s = shared.get_mut(file).expect("just loaded");
        let key = entry.key();
        match settings::add(&mut s.doc, entry) {
            Err(m) => return report.refuse(format!("{file}: {m}")),
            Ok(settings::Merged::Added) => report.item(
                "entry",
                file,
                Some(&key),
                if dry_run { "would-merge" } else { "merged" },
            ),
            Ok(settings::Merged::Unchanged) => report.item("entry", file, Some(&key), "unchanged"),
            Ok(settings::Merged::Edited) => {
                report.item("entry", file, Some(&key), "modified");
                conflicts.push(format!(
                    "{file} holds an entry for '{key}' that differs from the bundle's (a person edited it, or it is an older bundle's); the install will not guess"
                ));
            }
        }
    }

    // The marked instructions block.
    let mut block_write = None;
    match &want.block {
        None => {
            report.item("block", "", None, "not-configured");
            report.notice(
                NoticeSeverity::Info,
                "harness.instructions_file is not set, so no agent-instructions block was written; the engine assumes no file name".to_string(),
            );
        }
        Some((file, rendered)) => {
            let text = match read(&env.root, file) {
                Ok(t) => t.unwrap_or_default(),
                Err(m) => return report.refuse(m),
            };
            match block::apply(&text, rendered) {
                Err(m) => return report.refuse(format!("{file}: {m}")),
                Ok(new) if new == text => report.item("block", file, None, "unchanged"),
                Ok(new) => {
                    report.item(
                        "block",
                        file,
                        None,
                        if dry_run { "would-write" } else { "written" },
                    );
                    block_write = Some((file.clone(), new));
                }
            }
        }
    }

    if !conflicts.is_empty() {
        report.items.retain(|_| true);
        return report.refuse(conflicts.join("; "));
    }
    for (file, s) in &shared {
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
    report.item(
        "lock",
        LOCK_PATH,
        None,
        if dry_run { "would-write" } else { "written" },
    );
    if dry_run {
        return report;
    }
    let result = (|| {
        for (path, content) in &file_writes {
            write(&env.root, path, content)?;
        }
        for (file, s) in &shared {
            save_settings(&env.root, file, s)?;
        }
        if let Some((file, text)) = &block_write {
            write(&env.root, file, text)?;
        }
        write(&env.root, LOCK_PATH, &new_lock.to_text())
    })();
    match result {
        Ok(()) => report,
        Err(m) => report.refuse(m),
    }
}

// ----------------------------------------------------------------- status

pub fn run_status(config: Option<PathBuf>) -> ExitCode {
    let env = match open(config) {
        Ok(e) => e,
        Err(m) => return not_run(m),
    };
    let lock = match load_lock(&env) {
        Ok(Some(l)) => l,
        Ok(None) => {
            return not_run(format!(
                "no {LOCK_PATH}: the harness is not installed (run `urzua harness install --adapter NAME`)"
            ))
        }
        Err(m) => return not_run(m),
    };
    emit(&status(&env, &lock))
}

fn status(env: &Env, lock: &Lock) -> HarnessReport {
    let mut report = HarnessReport::new("status", false, lock.adapters.clone());
    let mut drift = false;
    let want = match desired(env, &lock.adapters) {
        Ok(w) => w,
        Err(m) => return report.refuse(m),
    };
    let mut mark =
        |report: &mut HarnessReport, kind, path: &str, key: Option<&str>, state: &str| {
            drift |= state != "ok";
            report.item(kind, path, key, state);
        };
    if lock.bundle_version != VERSION {
        let state = "out-of-date";
        let older = lock.bundle_version.as_str() < VERSION;
        report.notice(
            NoticeSeverity::Warning,
            if older {
                format!("the installed bundle is {} and this binary is {VERSION}: run `urzua harness update`", lock.bundle_version)
            } else {
                format!("the installed bundle is {} and this binary is only {VERSION}: update urzua", lock.bundle_version)
            },
        );
        mark(&mut report, "lock", LOCK_PATH, None, state);
    }
    for (path, locked) in &lock.files {
        let state = match read(&env.root, path) {
            Err(m) => return report.refuse(m),
            Ok(None) => "missing",
            Ok(Some(t)) if lock::hash(&t) != *locked => "modified",
            Ok(Some(_)) => match want.files.get(path) {
                Some(bundle) if lock::hash(bundle) != *locked => "out-of-date",
                _ => "ok",
            },
        };
        mark(&mut report, "file", path, None, state);
    }
    let mut cache: BTreeMap<String, Option<J>> = BTreeMap::new();
    for e in &lock.entries {
        if !cache.contains_key(&e.file) {
            let doc = match load_settings(&env.root, &e.file) {
                Ok(s) => s.text.map(|_| s.doc),
                Err(m) => return report.refuse(m),
            };
            cache.insert(e.file.clone(), doc);
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
        mark(&mut report, "entry", &e.file, Some(&e.key()), state);
    }
    for b in &lock.blocks {
        let state = match read(&env.root, &b.file) {
            Err(m) => return report.refuse(m),
            Ok(None) => "missing",
            Ok(Some(t)) => match block::extract(&t) {
                Err(_) | Ok(None) => "missing",
                Ok(Some(found)) if lock::hash(found) != b.hash => "modified",
                Ok(Some(_)) => match &want.block {
                    Some((_, rendered)) if lock::hash(rendered) != b.hash => "out-of-date",
                    _ => "ok",
                },
            },
        };
        mark(&mut report, "block", &b.file, None, state);
    }
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

// ----------------------------------------------------------------- update

pub fn run_update(config: Option<PathBuf>, force: bool, dry_run: bool) -> ExitCode {
    let env = match open(config) {
        Ok(e) => e,
        Err(m) => return not_run(m),
    };
    let lock = match load_lock(&env) {
        Ok(Some(l)) => l,
        Ok(None) => {
            return not_run(format!(
                "no {LOCK_PATH}: the harness is not installed (run `urzua harness install --adapter NAME`)"
            ))
        }
        Err(m) => return not_run(m),
    };
    emit(&update(&env, &lock, force, dry_run))
}

fn update(env: &Env, old: &Lock, force: bool, dry_run: bool) -> HarnessReport {
    let mut report = HarnessReport::new("update", dry_run, old.adapters.clone());
    let want = match desired(env, &old.adapters) {
        Ok(w) => w,
        Err(m) => return report.refuse(m),
    };
    let mut lock = Lock::new(old.adapters.clone());
    let mut kept = false;
    let (mut file_writes, mut deletes) = (Vec::new(), Vec::new());
    let past = |s: &'static str, would: &'static str| if dry_run { would } else { s };

    for (path, content) in &want.files {
        let locked = old.files.get(path);
        let want_hash = lock::hash(content);
        let state = match read(&env.root, path) {
            Err(m) => return report.refuse(m),
            Ok(None) => {
                file_writes.push((path.clone(), content.clone()));
                lock.files.insert(path.clone(), want_hash);
                past("created", "would-create")
            }
            Ok(Some(t)) if t == *content => {
                lock.files.insert(path.clone(), want_hash);
                "unchanged"
            }
            Ok(Some(t)) if locked.is_some_and(|h| *h == lock::hash(&t)) || force => {
                file_writes.push((path.clone(), content.clone()));
                lock.files.insert(path.clone(), want_hash);
                if locked.is_some_and(|h| *h == lock::hash(&t)) {
                    past("updated", "would-update")
                } else {
                    past("forced", "would-force")
                }
            }
            Ok(Some(_)) => {
                kept = true;
                // The old hash stays, so `status` keeps reporting the change.
                if let Some(h) = locked {
                    lock.files.insert(path.clone(), h.clone());
                }
                "kept-modified"
            }
        };
        report.item("file", path, None, state);
    }
    for (path, locked) in &old.files {
        if want.files.contains_key(path) {
            continue;
        }
        match read(&env.root, path) {
            Err(m) => return report.refuse(m),
            Ok(None) => report.item("file", path, None, "gone"),
            Ok(Some(t)) if lock::hash(&t) == *locked || force => {
                deletes.push(path.clone());
                report.item("file", path, None, past("removed", "would-remove"));
            }
            Ok(Some(_)) => {
                kept = true;
                lock.files.insert(path.clone(), locked.clone());
                report.item("file", path, None, "kept-modified");
            }
        }
    }

    let mut shared: BTreeMap<String, Settings> = BTreeMap::new();
    for (file, entry) in &want.entries {
        if !shared.contains_key(file) {
            match load_settings(&env.root, file) {
                Ok(s) => {
                    shared.insert(file.clone(), s);
                }
                Err(m) => return report.refuse(m),
            }
        }
        let s = shared.get_mut(file).expect("just loaded");
        let key = entry.key();
        let locked = old.entries.iter().find(|e| same(e, file, entry));
        let found = settings::find(&s.doc, &entry.event, &entry.matcher, &entry.handler).cloned();
        let state = match found {
            None => {
                let _ = settings::add(&mut s.doc, entry);
                past("merged", "would-merge")
            }
            Some(h) if h == entry.hook => "unchanged",
            Some(h) if locked.is_some_and(|l| l.hash == lock::hash(&h.compact())) || force => {
                if let Err(m) = settings::replace(&mut s.doc, entry) {
                    return report.refuse(format!("{file}: {m}"));
                }
                past("updated", "would-update")
            }
            Some(_) => {
                kept = true;
                "kept-modified"
            }
        };
        report.item("entry", file, Some(&key), state);
        let hash = if state == "kept-modified" {
            locked
                .map(|l| l.hash.clone())
                .unwrap_or_else(|| entry.hash())
        } else {
            entry.hash()
        };
        lock.entries.push(entry_hash(file, entry, hash));
    }
    for e in &old.entries {
        if want.entries.iter().any(|(f, w)| same(e, f, w)) {
            continue;
        }
        let Some(s) = shared.get_mut(&e.file) else {
            continue;
        };
        let key = e.key();
        match settings::find(&s.doc, &e.event, &e.matcher, &e.handler) {
            None => report.item("entry", &e.file, Some(&key), "gone"),
            Some(h) if lock::hash(&h.compact()) == e.hash || force => {
                settings::remove(&mut s.doc, &e.event, &e.matcher, &e.handler);
                report.item(
                    "entry",
                    &e.file,
                    Some(&key),
                    past("removed", "would-remove"),
                );
            }
            Some(_) => {
                kept = true;
                lock.entries.push(e.clone());
                report.item("entry", &e.file, Some(&key), "kept-modified");
            }
        }
    }

    let mut block_writes: Vec<(String, String)> = Vec::new();
    if let Some((file, rendered)) = &want.block {
        let text = match read(&env.root, file) {
            Ok(t) => t.unwrap_or_default(),
            Err(m) => return report.refuse(m),
        };
        let locked = old.blocks.iter().find(|b| b.file == *file);
        let current = match block::extract(&text) {
            Ok(c) => c.map(str::to_string),
            Err(m) => return report.refuse(format!("{file}: {m}")),
        };
        let state = match &current {
            Some(c) if c == rendered => "unchanged",
            Some(c) if locked.is_some_and(|l| l.hash == lock::hash(c)) || force => {
                past("updated", "would-update")
            }
            Some(_) => "kept-modified",
            None => past("written", "would-write"),
        };
        if state == "kept-modified" {
            kept = true;
            if let Some(l) = locked {
                lock.blocks.push(l.clone());
            }
        } else {
            lock.blocks.push(BlockHash {
                file: file.clone(),
                hash: lock::hash(rendered),
            });
            if state != "unchanged" {
                match block::apply(&text, rendered) {
                    Ok(new) => block_writes.push((file.clone(), new)),
                    Err(m) => return report.refuse(format!("{file}: {m}")),
                }
            }
        }
        report.item("block", file, None, state);
    }

    report.item("lock", LOCK_PATH, None, past("written", "would-write"));
    let (unknown, unprovided) = harness_problems(env, &old.adapters);
    for p in unknown.iter().chain(&unprovided) {
        report.notice(NoticeSeverity::Warning, p.clone());
    }
    report.problems.extend(unknown);
    report.problems.extend(unprovided);
    if !dry_run {
        let result = (|| {
            for (path, content) in &file_writes {
                write(&env.root, path, content)?;
            }
            for path in &deletes {
                delete(&env.root, path)?;
            }
            for (file, s) in &shared {
                save_settings(&env.root, file, s)?;
            }
            for (file, text) in &block_writes {
                write(&env.root, file, text)?;
            }
            write(&env.root, LOCK_PATH, &lock.to_text())
        })();
        if let Err(m) = result {
            return report.refuse(m);
        }
    }
    if kept {
        report.notice(
            NoticeSeverity::Warning,
            "a file a person changed was left as it is (pass --force to overwrite it)".to_string(),
        );
        report = report.drift();
    }
    report
}

// -------------------------------------------------------------- uninstall

pub fn run_uninstall(config: Option<PathBuf>, dry_run: bool) -> ExitCode {
    let env = match open(config) {
        Ok(e) => e,
        Err(m) => return not_run(m),
    };
    let lock = match load_lock(&env) {
        Ok(Some(l)) => l,
        Ok(None) => {
            return not_run(format!(
                "no {LOCK_PATH}: the harness is not installed, nothing to remove"
            ))
        }
        Err(m) => return not_run(m),
    };
    emit(&uninstall(&env, &lock, dry_run))
}

fn uninstall(env: &Env, lock: &Lock, dry_run: bool) -> HarnessReport {
    let mut report = HarnessReport::new("uninstall", dry_run, lock.adapters.clone());
    let past = |s: &'static str, would: &'static str| if dry_run { would } else { s };
    let mut kept = false;
    let mut deletes = Vec::new();

    for (path, locked) in &lock.files {
        let state = match read(&env.root, path) {
            Err(m) => return report.refuse(m),
            Ok(None) => "missing",
            Ok(Some(t)) if lock::hash(&t) == *locked => {
                deletes.push(path.clone());
                past("removed", "would-remove")
            }
            Ok(Some(_)) => {
                kept = true;
                "kept-modified"
            }
        };
        report.item("file", path, None, state);
    }

    let mut shared: BTreeMap<String, Settings> = BTreeMap::new();
    for e in &lock.entries {
        if !shared.contains_key(&e.file) {
            match load_settings(&env.root, &e.file) {
                Ok(s) => {
                    shared.insert(e.file.clone(), s);
                }
                Err(m) => return report.refuse(m),
            }
        }
        let s = shared.get_mut(&e.file).expect("just loaded");
        let state = match settings::find(&s.doc, &e.event, &e.matcher, &e.handler) {
            None => "missing",
            Some(h) if lock::hash(&h.compact()) == e.hash => {
                settings::remove(&mut s.doc, &e.event, &e.matcher, &e.handler);
                past("removed", "would-remove")
            }
            Some(_) => {
                kept = true;
                "kept-modified"
            }
        };
        report.item("entry", &e.file, Some(&e.key()), state);
    }

    let mut block_writes = Vec::new();
    for b in &lock.blocks {
        let text = match read(&env.root, &b.file) {
            Err(m) => return report.refuse(m),
            Ok(t) => t,
        };
        let state = match text.as_deref().map(|t| (t, block::extract(t))) {
            None => "missing",
            Some((_, Err(_) | Ok(None))) => "missing",
            Some((t, Ok(Some(found)))) if lock::hash(found) == b.hash => {
                match block::remove(t) {
                    Ok(new) => block_writes.push((b.file.clone(), new)),
                    Err(m) => return report.refuse(format!("{}: {m}", b.file)),
                }
                past("removed", "would-remove")
            }
            Some(_) => {
                kept = true;
                "kept-modified"
            }
        };
        report.item("block", &b.file, None, state);
    }
    report.item("lock", LOCK_PATH, None, past("removed", "would-remove"));

    if !dry_run {
        let result = (|| {
            for path in &deletes {
                delete(&env.root, path)?;
            }
            for (file, s) in &shared {
                if s.doc.is_empty_container() && s.text.is_some() {
                    // Nothing but what the install added: the file goes with it.
                    delete(&env.root, file)?;
                } else if s.text.is_some() {
                    save_settings(&env.root, file, s)?;
                }
            }
            for (file, text) in &block_writes {
                if text.trim().is_empty() {
                    delete(&env.root, file)?;
                } else {
                    write(&env.root, file, text)?;
                }
            }
            // The lock goes last.
            delete(&env.root, LOCK_PATH)
        })();
        if let Err(m) = result {
            return report.refuse(m);
        }
    }
    if kept {
        report.notice(
            NoticeSeverity::Warning,
            "a file or entry a person changed was left in place; remove it by hand".to_string(),
        );
        report = report.drift();
    }
    report
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
    match load_lock(&env) {
        Err(m) => out.push((Level::Warn, "harness-installed", m)),
        Ok(None) => out.push((
            Level::Warn,
            "harness-installed",
            format!(
                "harness is declared but {LOCK_PATH} does not exist: run `urzua harness install`"
            ),
        )),
        Ok(Some(lock)) => {
            let report = status(&env, &lock);
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
