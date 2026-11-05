//! Shared helpers for the integration tests (BUG-193).
//!
//! Every fixture directory a test builds is owned by a [`TempDir`], which removes
//! it on drop, so a failing assertion or a panic mid-test cannot leave it behind.
//! `make test-no-leak` runs the whole suite under a fresh `TMPDIR` and fails when
//! anything is left in it.

// Each test crate compiles this module and uses only part of it.
#![allow(dead_code)]

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A fresh, empty directory under the system temp dir, removed when dropped.
///
/// The name is `<label>-<pid>-<n>`: unique per process and per call, so two
/// tests of one run never share a directory and a crashed earlier run's
/// directory is never reused.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Creates `<temp dir>/<label>-<pid>-<n>`. `label` should start with
    /// `urzua-` so a stray directory is attributable; the leak check
    /// (`.github/scripts/test-no-leak.sh`) only sees `urzua-*` entries.
    pub fn new(label: &str) -> TempDir {
        assert!(
            label.starts_with("urzua-"),
            "TempDir label `{label}` must start with `urzua-` or the leak check cannot see it"
        );
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        // Canonical, so a test that compares paths the binary prints sees the
        // same spelling (a symlinked temp dir otherwise differs).
        let base = std::env::temp_dir()
            .canonicalize()
            .unwrap_or_else(|_| std::env::temp_dir());
        let path = base.join(format!("{label}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create the fixture directory");
        TempDir { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

impl Deref for TempDir {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for TempDir {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<std::ffi::OsStr> for TempDir {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.path.as_os_str()
    }
}

/// A single temp file, removed when dropped.
pub struct TempFile {
    path: PathBuf,
}

impl TempFile {
    pub fn new(label: &str) -> TempFile {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("{label}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        TempFile { path }
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

impl Deref for TempFile {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for TempFile {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<std::ffi::OsStr> for TempFile {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.path.as_os_str()
    }
}
