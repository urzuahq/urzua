//! A file's text, read once per run (`ADR-74` point 9): the existence rule, the
//! drift check and re-baselining resolve scopes in the same files, and a file
//! cited by several records is read one time, as `BUG-135` did for git.

use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;

use urzua_core::locator::{FileText, SharedText};

pub(crate) struct FileTexts {
    root: PathBuf,
    /// The configuration discovery reads (`ADR-92`): the tracked files a
    /// directory or glob locator matches are the files `check` itself examines.
    config: urzua_core::config::Config,
    cache: RefCell<HashMap<String, SharedText>>,
    /// The tracked files, discovered on first use: only a directory or glob
    /// locator needs them (`BUG-176`).
    tracked: OnceCell<(Vec<PathBuf>, HashSet<PathBuf>)>,
}

impl FileTexts {
    pub fn new(root: PathBuf, config: &urzua_core::config::Config) -> Self {
        FileTexts {
            root,
            config: config.clone(),
            cache: RefCell::new(HashMap::new()),
            tracked: OnceCell::new(),
        }
    }

    // BUG-176: a directory or glob locator is the tracked files it matches
    /// What the locator path `path` names among the tracked files that are on
    /// disk: its kind and the files it covers, sorted.
    pub fn locator_files(&self, path: &str) -> Result<urzua_core::locator::LocatorFiles, String> {
        if self.tracked.get().is_none() {
            let list = crate::discovery::discover_files(&self.root, &self.config)
                .map_err(|e| format!("could not list the repository's files: {e}"))?
                .paths;
            let set = list.iter().cloned().collect();
            let _ = self.tracked.set((list, set));
        }
        let (list, set) = self.tracked.get().expect("set above");
        urzua_core::locator::match_locator_files(path, list, &|p| {
            let p = std::path::Path::new(p);
            set.contains(p) && self.root.join(p).is_file()
        })
    }

    /// `Err` with the reason when `path` resolves outside the repository.
    pub fn confined(&self, path: &str) -> Result<(), String> {
        urzua_io::confine_to_root(&self.root, std::path::Path::new(path)).map(|_| ())
    }

    /// The text of the repo-relative `path`; a file that is not valid UTF-8 is
    /// reported as such, and one that cannot be read as missing.
    pub fn get(&self, path: &str) -> SharedText {
        if let Some(hit) = self.cache.borrow().get(path) {
            return Rc::clone(hit);
        }
        // A path that resolves outside the repository (absolute, `..`, or a
        // symlink out) is never read: it reads as missing.
        let read = match self
            .confined(path)
            .ok()
            .map(|_| std::fs::read(self.root.join(path)))
        {
            Some(Ok(bytes)) => match String::from_utf8(bytes) {
                Ok(text) => FileText::Text(text),
                Err(_) => FileText::NotUtf8,
            },
            _ => FileText::Missing,
        };
        let shared = Rc::new(read);
        self.cache
            .borrow_mut()
            .insert(path.to_string(), Rc::clone(&shared));
        shared
    }
}

/// Runs `f` with the lookups a locator is judged by: a path is present when git
/// tracks it and it is on disk, as `embodiment.locator-exists` judges it. Used
/// by the commands that judge a Done-means item outside `check` (`fix`,
/// `roadmap`, `ADR-96`).
pub(crate) fn with_locator_env<R>(
    root: &std::path::Path,
    config: &urzua_core::config::Config,
    tracked: &[PathBuf],
    records: &[urzua_core::record::Record],
    f: impl FnOnce(&urzua_core::rules::LocatorEnv<'_>) -> R,
) -> R {
    let texts = FileTexts::new(root.to_path_buf(), config);
    let set: HashSet<&std::path::Path> = tracked.iter().map(|p| p.as_path()).collect();
    let exists = |p: &std::path::Path| root.join(p).try_exists().unwrap_or(true);
    let present = |p: &str| {
        let path = std::path::Path::new(p);
        set.contains(path) && exists(path)
    };
    let untracked = |p: &str| {
        let path = std::path::Path::new(p);
        !set.contains(path) && exists(path)
    };
    let read = |p: &str| texts.get(p);
    let env = urzua_core::rules::LocatorEnv {
        config,
        opts: config.rule_opts(urzua_core::rules::RULE_EMBODIMENT_LOCATOR_EXISTS),
        tracked,
        present: &present,
        read: &read,
        untracked_on_disk: &untracked,
        records,
    };
    f(&env)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Removes the fixture directory on every exit path, a panic included.
    struct Scratch(std::path::PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A failed file listing is reported, not read as "every locator names
    /// nothing" (which would call each existing path missing).
    #[test]
    fn locator_files_reports_a_discovery_failure() {
        let scratch = Scratch(
            std::env::temp_dir().join(format!("urzua-filetexts-nogit-{}", std::process::id())),
        );
        let dir = scratch.0.clone();
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "x\n").unwrap();
        // A `.git` file that is not a gitfile makes every git command fail here.
        std::fs::write(dir.join(".git"), "not a gitfile\n").unwrap();
        let config = urzua_core::config::parse("schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    required_fields: [Status]\n").unwrap();
        let texts = FileTexts::new(dir.clone(), &config);
        let result = texts.locator_files("a.rs");
        let err = result.expect_err("a directory that is not a git checkout cannot be listed");
        assert!(
            err.contains("could not list the repository's files"),
            "{err}"
        );
    }
}
