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

    /// The text of the repo-relative `path`; a file that is not valid UTF-8 is
    /// reported as such, and one that cannot be read as missing.
    pub fn get(&self, path: &str) -> SharedText {
        if let Some(hit) = self.cache.borrow().get(path) {
            return Rc::clone(hit);
        }
        let read = match std::fs::read(self.root.join(path)) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(text) => FileText::Text(text),
                Err(_) => FileText::NotUtf8,
            },
            Err(_) => FileText::Missing,
        };
        let shared = Rc::new(read);
        self.cache
            .borrow_mut()
            .insert(path.to_string(), Rc::clone(&shared));
        shared
    }
}
