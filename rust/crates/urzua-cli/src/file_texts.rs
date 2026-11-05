//! A file's text, read once per run (`ADR-74` point 9): the existence rule, the
//! drift check and re-baselining resolve scopes in the same files, and a file
//! cited by several records is read one time, as `BUG-135` did for git.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use urzua_core::locator::{FileText, SharedText};

pub(crate) struct FileTexts {
    root: PathBuf,
    cache: RefCell<HashMap<String, SharedText>>,
}

impl FileTexts {
    pub fn new(root: PathBuf) -> Self {
        FileTexts {
            root,
            cache: RefCell::new(HashMap::new()),
        }
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
