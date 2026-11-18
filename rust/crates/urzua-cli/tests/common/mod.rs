//! Shared helpers for the integration tests (BUG-193).
//!
//! Every fixture directory a test builds is owned by a [`TempDir`], which removes
//! it on drop, so a failing assertion or a panic mid-test cannot leave it behind.
//! `make test-no-leak` runs the whole suite under a fresh `TMPDIR` and fails when
//! anything is left in it.

// Each test crate compiles this module and uses only part of it.
#![allow(dead_code)]

pub mod waive;

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

/// The evaluation date every test that judges this repository's own live corpus passes as
/// `--as-of`. Claim expiry (ADR-97) is the records gate's job under the real clock; a test
/// that asserted a clean run under the real clock would go red on the first expiry with no
/// code change. A date in the past is safe: nothing reads a record dated after it as an
/// error. Acceptance cases with `corpus: "@repo"` carry the same literal in their YAML.
pub const OWN_CORPUS_AS_OF: &str = "2026-11-09";

/// The directory holding a `gh` that always fails, as a `gh` that is not logged
/// in does. `urzua fix --apply` and `urzua new` ask `gh api user` who is
/// attesting, and a logged-in `gh` on the machine running the tests would make a
/// live API call (up to the engine's five second wait when offline) and replace
/// the `--by` a test passed with its own login. Every test that runs the binary
/// puts this directory first on `PATH`, through [`urzua`] or
/// [`path_with_stub_gh`], so no test depends on the machine's `gh` or network.
///
/// It lives under `CARGO_TARGET_TMPDIR`, not the system temp dir, so the leak
/// check (`.github/scripts/test-no-leak.sh`) never sees it.
pub fn stub_gh_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        use std::os::unix::fs::PermissionsExt;
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("stub-gh");
        std::fs::create_dir_all(&dir).expect("create the stub gh directory");
        // Written under a per-process name and renamed into place, so test
        // binaries running at once never see a half-written script.
        let tmp = dir.join(format!("gh.{}", std::process::id()));
        std::fs::write(&tmp, "#!/bin/sh\nexit 1\n").expect("write the stub gh");
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
            .expect("make the stub gh executable");
        std::fs::rename(&tmp, dir.join("gh")).expect("install the stub gh");
        dir
    })
}

/// `PATH` with [`stub_gh_dir`] in front of the caller's own.
pub fn path_with_stub_gh() -> std::ffi::OsString {
    let mut paths = vec![stub_gh_dir().to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).expect("join PATH")
}

/// A command that runs the `urzua` binary with no reachable `gh`. Use this, never
/// `Command::new(env!("CARGO_BIN_EXE_urzua"))`: `no_real_gh_integration` fails
/// the build of a test that does.
pub fn urzua() -> std::process::Command {
    command_with_stub_gh(env!("CARGO_BIN_EXE_urzua"))
}

/// Any program, with no reachable `gh`: a released `urzua` a test compares
/// against, or a tool such as `kill`, takes the same stubbed `PATH` as the
/// binary under test.
pub fn command_with_stub_gh(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    command.env("PATH", path_with_stub_gh());
    command
}

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

/// A `git` command with the machine's configuration out of the way (review finding 7.2).
///
/// `rust/.cargo/config.toml` already exports these variables to every test process, so a
/// helper that spawns `git` directly is isolated too. They are set again here so the helper
/// stays isolated when a test binary runs outside cargo (a raw `target/debug/deps` run),
/// and [`git_environment_is_isolated`] fails when the cargo-level setting is dropped.
pub fn isolated_git() -> std::process::Command {
    let mut cmd = std::process::Command::new("git");
    cmd.env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "urzua test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "urzua test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com");
    cmd
}

/// Whether this test process sees the git isolation `rust/.cargo/config.toml` exports.
pub fn git_environment_is_isolated() -> bool {
    std::env::var("GIT_CONFIG_GLOBAL").as_deref() == Ok("/dev/null")
        && std::env::var("GIT_CONFIG_NOSYSTEM").as_deref() == Ok("1")
}

/// Runs `git` in `dir` and asserts it succeeded.
pub fn git(dir: &Path, args: &[&str]) {
    let status = isolated_git()
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

/// Whether mode bits stop this process reading a directory. They do not for root, nor with an
/// ACL that bypasses them, and a test that needs `chmod 000` to deny access proves nothing then.
pub fn permission_bits_are_enforced() -> bool {
    use std::os::unix::fs::PermissionsExt;
    let probe = TempDir::new("urzua-perm-probe");
    let locked = probe.join("locked");
    std::fs::create_dir(&locked).expect("create the probe directory");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000))
        .expect("lock the probe directory");
    let blocked = std::fs::read_dir(&locked).is_err();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755))
        .expect("unlock the probe directory");
    blocked
}

/// The one way a test that needs the permission bits to deny access starts (review finding
/// 7.3). Returns `true` when it can run. Where the bits are not enforced it never passes
/// silently: under CI (`CI` set) it fails, because CI runs as an ordinary user and a skip there
/// would hide a real regression; elsewhere it prints a `SKIPPED` line and returns `false`, and
/// the test returns at once.
pub fn permission_test_can_run(test: &str) -> bool {
    if permission_bits_are_enforced() {
        return true;
    }
    assert!(
        std::env::var_os("CI").is_none(),
        "{test}: permission bits do not block access here, so this test cannot run; CI must \
         run as an ordinary user"
    );
    eprintln!(
        "SKIPPED {test}: permission bits do not block access in this environment (running as root?)"
    );
    false
}

/// The UTC calendar day (days since 1970-01-01) the test process reads once, so every
/// fixture date and assertion of a run agree even when midnight UTC passes inside it.
fn today_epoch_day() -> i64 {
    static DAY: OnceLock<i64> = OnceLock::new();
    *DAY.get_or_init(|| {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock is after 1970")
            .as_secs();
        (secs / 86_400) as i64
    })
}

/// `YYYY-MM-DDT12:00:00Z`, `n` whole days before today (UTC, read once per process).
pub fn date_days_ago(n: u32) -> String {
    // Days to civil date (Howard Hinnant's algorithm).
    let z = today_epoch_day() - i64::from(n) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T12:00:00Z")
}

/// Stages everything and commits it as `days_ago` days ago (author and committer).
pub fn commit_all(dir: &Path, days_ago: u32) {
    git(dir, &["add", "-A"]);
    let date = date_days_ago(days_ago);
    let status = isolated_git()
        .args(["commit", "-q", "-m", "fixture"])
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success());
}

/// Appends an `authority` table to the fixture's configuration with one path
/// gate over `paths`, owned by the group `leads` (login `lead`).
pub fn gate_paths(dir: &Path, paths: &[&str]) {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(dir.join(".urzua/config.yaml"))
        .expect("open the fixture configuration");
    write!(
        f,
        "authority:\n  deciders:\n    leads: {{logins: [lead]}}\n  gates:\n    - {{paths: [{}], by: leads}}\n",
        paths.join(", ")
    )
    .unwrap();
}

/// A directory holding a `gh` that prints `login`, for a test that needs a
/// verified identity.
pub fn gh_login_dir(login: &str) -> TempDir {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new("urzua-gh-login");
    std::fs::write(dir.join("gh"), format!("#!/bin/sh\necho '{login}'\n")).unwrap();
    std::fs::set_permissions(dir.join("gh"), std::fs::Permissions::from_mode(0o755)).unwrap();
    dir
}

/// [`urzua`] with the `gh` of [`gh_login_dir`].
pub fn urzua_as(gh: &TempDir) -> std::process::Command {
    let mut command = urzua();
    let mut paths = vec![gh.path().to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    command.env("PATH", std::env::join_paths(paths).unwrap());
    command
}

/// `urzua harness hook claude-code <event>` run from `dir` with `payload` on
/// stdin, as an agent's hook command runs it. `engine`, when given, is the argv
/// of a stand-in engine (a debug build's seam); `project` is the session's
/// project (`CLAUDE_PROJECT_DIR`), `None` for a session whose project holds no
/// configuration.
pub fn run_hook(
    dir: &Path,
    event: &str,
    payload: &str,
    engine: Option<&[&str]>,
    project: Option<&Path>,
    envs: &[(&str, &std::ffi::OsStr)],
) -> std::process::Output {
    use std::io::Write;
    let mut c = urzua();
    c.args(["harness", "hook", "claude-code", event])
        .current_dir(dir)
        .env(
            "CLAUDE_PROJECT_DIR",
            project.map_or_else(
                || "/nonexistent-project".into(),
                |p| p.as_os_str().to_owned(),
            ),
        )
        .env_remove("URZUA_HOOK_ENGINE_FOR_TESTS");
    if let Some(argv) = engine {
        c.env(
            "URZUA_HOOK_ENGINE_FOR_TESTS",
            serde_json::to_string(argv).unwrap(),
        );
    }
    for (k, v) in envs {
        c.env(k, v);
    }
    c.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = c.spawn().unwrap();
    let _ = child.stdin.take().unwrap().write_all(payload.as_bytes());
    child.wait_with_output().unwrap()
}

/// `None` (the binary's own engine) for an argv that names `urzua`, else the
/// argv as a stand-in engine.
pub fn engine_or_own<'a>(engine: &'a [&'a str]) -> Option<&'a [&'a str]> {
    (engine.first() != Some(&"urzua")).then_some(engine)
}

/// `PATH` with this build's `urzua` first, for running an installed hook
/// command the way the agent runs it.
pub fn path_with_urzua() -> std::ffi::OsString {
    let bin = Path::new(env!("CARGO_BIN_EXE_urzua"))
        .parent()
        .unwrap()
        .to_path_buf();
    let mut dirs = vec![bin];
    dirs.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    std::env::join_paths(dirs).unwrap()
}
