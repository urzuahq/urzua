//! The preamble every corpus command shares: find the repository root, load
//! the config, discover the tracked files, load the records. A command that
//! cannot get through it has not run at all, so every failure here is a
//! `Result<_, String>` that `emit_or_not_run` turns into the one could-not-run
//! emission (exit code 2); no command spells that sequence or its error arms
//! out itself.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::config::Config;
use urzua_core::record::Record;
use urzua_core::report::{CouldNotRun, Failure};
use urzua_io::DiscoveredFiles;

use crate::discovery::{default_config_path, find_repo_root, load_config, load_records};
use crate::emit;

/// The repository root and the config read from it.
pub(crate) struct Workspace {
    pub repo_root: PathBuf,
    pub config_path: PathBuf,
    pub config: Config,
}

impl Workspace {
    /// `root_hint` locates the repository (the first path wins; none means the
    /// current directory). `config_path` defaults to the repository's own.
    pub fn open(root_hint: &[PathBuf], config_path: Option<PathBuf>) -> Result<Self, Failure> {
        let repo_root = find_repo_root(root_hint)?;
        let config_path = config_path.unwrap_or_else(|| default_config_path(&repo_root));
        let config = load_config(&config_path)?;
        Ok(Workspace {
            repo_root,
            config_path,
            config,
        })
    }
}

/// A workspace with its tracked files discovered and every record loaded.
pub(crate) struct Session {
    pub repo_root: PathBuf,
    pub config_path: PathBuf,
    pub config: Config,
    pub discovered: DiscoveredFiles,
    pub records: Vec<Record>,
    /// Tracked record files that are symlinks out of their governed tree. They
    /// are in `discovered` but never loaded; `check` reports each as a finding
    /// and every other command refuses (`refusing_escaping_records`).
    pub escaping_records: Vec<urzua_core::rules::EscapingRecord>,
    pub full_text: std::collections::HashMap<PathBuf, String>,
}

impl Session {
    /// For every command that reads or writes records: a record that is a
    /// symlink out of its governed tree stops the run (exit 2), naming it.
    /// `check` alone keeps it and reports a finding (`open_with`).
    pub fn open(config_path: Option<PathBuf>) -> Result<Self, Failure> {
        Self::open_with(&[], config_path, |_, _, _| Ok(()))?.refusing_escaping_records()
    }

    /// Stops the run when a tracked record is a link, or sits behind one: the file is never read, and the refusal
    /// names it rather than letting the corpus shrink in silence.
    pub fn refusing_escaping_records(self) -> Result<Self, Failure> {
        match self.escaping_records.first() {
            Some(e) => Err(format!(
                "{} is a symlink {}; urzua refuses to read it",
                e.path.display(),
                e.reason
            )
            .into()),
            None => Ok(self),
        }
    }

    /// `adjust` runs after discovery and before records load, for the command
    /// that narrows or refuses the discovered set (`check`'s argv overrides).
    /// The result keeps `escaping_records`; a command that is not `check` calls
    /// `refusing_escaping_records`.
    pub fn open_with(
        root_hint: &[PathBuf],
        config_path: Option<PathBuf>,
        adjust: impl FnOnce(&Path, &Config, &mut DiscoveredFiles) -> Result<(), String>,
    ) -> Result<Self, Failure> {
        Self::load(Workspace::open(root_hint, config_path)?, adjust)
    }

    /// Discovers and loads from an already-opened workspace, for the command
    /// that must judge the config before it touches the corpus (`index`).
    pub fn load(
        workspace: Workspace,
        adjust: impl FnOnce(&Path, &Config, &mut DiscoveredFiles) -> Result<(), String>,
    ) -> Result<Self, Failure> {
        let Workspace {
            repo_root,
            config_path,
            config,
        } = workspace;
        let mut discovered =
            crate::discovery::discover_files(&repo_root, &config).map_err(|e| e.to_string())?;
        adjust(&repo_root, &config, &mut discovered)?;
        // A tracked record missing from the working tree is a removed record,
        // not a reason to refuse the whole run: it is the ordinary mid-rename
        // state. It is remembered so `check` can name it (BUG-82's concern is
        // a record vanishing in silence, which a notice answers).
        let missing: Vec<PathBuf> = discovered
            .paths
            .iter()
            .filter(|p| {
                crate::discovery::is_governed(&config, p)
                    && !discovered.staged_deletions.contains(*p)
                    && matches!(
                        std::fs::symlink_metadata(repo_root.join(p)),
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound
                    )
            })
            .cloned()
            .collect();
        for p in missing {
            discovered.staged_deletions.insert(p.clone());
            discovered.unstaged_deletions.insert(p);
        }
        // With the rule `off` the repository has chosen to read a record through a
        // link (`identity.record-symlink-escapes`, SPEC-2): nothing is refused.
        let escaping_records = if urzua_core::rules::effective_level(
            &config,
            urzua_core::rules::RULE_IDENTITY_RECORD_SYMLINK_ESCAPES,
        ) == urzua_core::config::RuleLevel::Off
        {
            Vec::new()
        } else {
            crate::discovery::escaping_records(&repo_root, &discovered.paths, &config)
        };
        let readable: Vec<PathBuf> = discovered
            .paths
            .iter()
            .filter(|p| !escaping_records.iter().any(|e| &e.path == *p))
            .cloned()
            .collect();
        let (records, full_text) =
            load_records(&repo_root, &readable, &discovered.staged_deletions, &config)?;
        Ok(Session {
            repo_root,
            config_path,
            config,
            discovered,
            records,
            escaping_records,
            full_text,
        })
    }
}

/// Emits a command's result, or the could-not-run report for its error.
pub(crate) fn emit_or_not_run(result: Result<ExitCode, Failure>) -> ExitCode {
    match result {
        Ok(code) => code,
        Err(e) => emit(&CouldNotRun::from(e)),
    }
}
