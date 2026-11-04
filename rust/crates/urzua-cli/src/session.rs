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
use urzua_core::report::CouldNotRun;
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
    pub fn open(root_hint: &[PathBuf], config_path: Option<PathBuf>) -> Result<Self, String> {
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
    pub full_text: std::collections::HashMap<PathBuf, String>,
}

impl Session {
    pub fn open(config_path: Option<PathBuf>) -> Result<Self, String> {
        Self::open_with(&[], config_path, |_, _, _| Ok(()))
    }

    /// `adjust` runs after discovery and before records load, for the command
    /// that narrows or refuses the discovered set (`check`'s argv overrides,
    /// `export`'s escaping-symlink refusal).
    pub fn open_with(
        root_hint: &[PathBuf],
        config_path: Option<PathBuf>,
        adjust: impl FnOnce(&Path, &Config, &mut DiscoveredFiles) -> Result<(), String>,
    ) -> Result<Self, String> {
        Self::load(Workspace::open(root_hint, config_path)?, adjust)
    }

    /// Discovers and loads from an already-opened workspace, for the command
    /// that must judge the config before it touches the corpus (`index`).
    pub fn load(
        workspace: Workspace,
        adjust: impl FnOnce(&Path, &Config, &mut DiscoveredFiles) -> Result<(), String>,
    ) -> Result<Self, String> {
        let Workspace {
            repo_root,
            config_path,
            config,
        } = workspace;
        let mut discovered =
            urzua_io::discover_tracked_files(&repo_root).map_err(|e| e.to_string())?;
        adjust(&repo_root, &config, &mut discovered)?;
        let (records, full_text) = load_records(
            &repo_root,
            &discovered.paths,
            &discovered.staged_deletions,
            &config,
        )?;
        Ok(Session {
            repo_root,
            config_path,
            config,
            discovered,
            records,
            full_text,
        })
    }
}

/// Emits a command's result, or the could-not-run report for its error.
pub(crate) fn emit_or_not_run(result: Result<ExitCode, String>) -> ExitCode {
    match result {
        Ok(code) => code,
        Err(e) => emit(&CouldNotRun::from(e)),
    }
}
