//! The `harness.limits` in force for this process's harness work. The caps are
//! read deep inside path resolution and JSON parsing, so the configuration that
//! loads first sets them here instead of being threaded through every call; a
//! process that loads none keeps the built-in floors.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use urzua_core::harness::Limits;

use crate::session::Workspace;

// Another thread sees the built-in floors, never a value below them, so a
// missed activation can only judge less of a call, never read past a cap.
thread_local! {
    static ACTIVE: RefCell<Limits> = RefCell::new(Limits::default());
}

pub(crate) fn get(key: &str) -> usize {
    ACTIVE.with(|a| a.borrow().get_usize(key))
}

pub(crate) fn activate(limits: &Limits) {
    ACTIVE.with(|a| *a.borrow_mut() = limits.clone());
}

/// The limits of the configuration found from `hint` (or named by `config`),
/// else the built-in ones.
pub(crate) fn of(config: Option<PathBuf>, hint: Option<&Path>) -> Limits {
    let hint: Vec<PathBuf> = hint.map(Path::to_path_buf).into_iter().collect();
    Workspace::open(&hint, config)
        .ok()
        .and_then(|ws| ws.config.harness)
        .map(|h| h.limits)
        .unwrap_or_default()
}

/// Sets one limit for the calling test thread.
#[cfg(test)]
pub(crate) fn raise_for_test(key: &str, value: u64) {
    let limits = Limits::read(&yaml_serde::from_str(&format!("{key}: {value}")).unwrap()).unwrap();
    activate(&limits);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn another_thread_does_not_see_this_threads_raised_limits_only_the_floors() {
        raise_for_test("paths_per_call", 900);
        assert_eq!(get("paths_per_call"), 900);
        let there = std::thread::spawn(|| get("paths_per_call")).join().unwrap();
        assert_eq!(there, 512);
    }
}
