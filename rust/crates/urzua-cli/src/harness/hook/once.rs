//! One judgement per hook call when the same hook is registered twice.
//!
//! Claude Code runs a plugin's copy of a hook beside the same hook in a
//! settings file, in parallel, and does not merge them. A repository that
//! commits `urzua harness install` and a machine that also has the plugin
//! would judge every edit twice: an override would be written to the override
//! log twice and every notice shown twice. So the first process to claim a call
//! judges it and leaves its output; the second waits for that output and prints
//! it unchanged, so the two can never disagree. The claim is keyed on the hash
//! of the whole payload, which carries the agent's id for the call, so an agent
//! cannot pre-claim a call it has not made yet; a claim that is older than a
//! minute, or whose output does not arrive in time, is ignored and the call is
//! judged again (judging twice is the safe failure, judging never is not). A
//! payload with no call id is always judged.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::super::json::{self, J};

const STALE: Duration = Duration::from_secs(60);
const WAIT: Duration = Duration::from_secs(20);

/// The id the agent gives this call, when the payload carries one.
fn call_id(payload: &[u8], event: &str) -> Option<String> {
    let doc = json::parse(std::str::from_utf8(payload).ok()?).ok()?;
    let key = if event == "session-start" {
        "session_id"
    } else {
        "tool_use_id"
    };
    match doc.get(key) {
        Some(J::Str(id)) if !id.is_empty() => Some(id.clone()),
        _ => None,
    }
}

fn claims_dir(project: &Path) -> Option<PathBuf> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(project)
        .args(["rev-parse", "--git-common-dir"])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let dir = String::from_utf8(out.stdout).ok()?;
    let dir = PathBuf::from(dir.trim_end_matches('\n'));
    let dir = if dir.is_absolute() {
        dir
    } else {
        project.join(dir)
    };
    Some(dir.join("urzua").join("hook-calls"))
}

fn age(path: &Path) -> Option<Duration> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    SystemTime::now().duration_since(modified).ok()
}

fn prune(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        if age(&e.path()).is_some_and(|a| a > STALE * 10) {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

pub fn run(
    project: Option<&Path>,
    adapter: &str,
    event: &str,
    payload: &[u8],
    judge: impl FnOnce() -> String,
) -> String {
    let Some(dir) = project
        .filter(|_| call_id(payload, event).is_some())
        .and_then(claims_dir)
    else {
        return judge();
    };
    if std::fs::create_dir_all(&dir).is_err() {
        return judge();
    }
    let mut keyed = format!("{adapter}\n{event}\n").into_bytes();
    keyed.extend_from_slice(payload);
    let key = urzua_io::sha256_hex_of_str(&String::from_utf8_lossy(&keyed));
    claimed(&dir, &key, judge)
}

/// Judges the call keyed `key` once among the processes that claim it in `dir`.
fn claimed(dir: &Path, key: &str, judge: impl FnOnce() -> String) -> String {
    let claim = dir.join(key);
    let done = dir.join(format!("{key}.done"));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&claim)
    {
        Ok(_) => {
            prune(dir);
            let out = judge();
            let partial = dir.join(format!("{key}.part"));
            if std::fs::write(&partial, &out).is_ok() {
                let _ = std::fs::rename(&partial, &done);
            }
            out
        }
        // Only a claim that exists means another process has the call; any
        // other failure to claim (a read-only directory, a full disk) is judged
        // here at once.
        Err(e) if e.kind() != std::io::ErrorKind::AlreadyExists => judge(),
        Err(_) if age(&claim).is_some_and(|a| a > STALE) => judge(),
        Err(_) => {
            let start = std::time::Instant::now();
            while start.elapsed() < WAIT {
                if let Ok(out) = std::fs::read_to_string(&done) {
                    return out;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            judge()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn a_claim_that_cannot_be_written_is_judged_at_once_not_after_the_wait() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("urzua-once-ro-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        // A process that may write anyway (root) cannot see the case.
        let writable = std::fs::write(dir.join("probe"), "x").is_ok();
        if !writable {
            let start = std::time::Instant::now();
            assert_eq!(claimed(&dir, "k", || "judged".to_string()), "judged");
            assert!(start.elapsed() < Duration::from_secs(2), "{:?}", start.elapsed());
        }
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_payload_without_a_call_id_is_not_claimed() {
        assert_eq!(call_id(br#"{"tool_name":"Write"}"#, "edit"), None);
        assert_eq!(
            call_id(br#"{"tool_use_id":"t1"}"#, "edit").as_deref(),
            Some("t1")
        );
        assert_eq!(
            call_id(br#"{"session_id":"s"}"#, "session-start").as_deref(),
            Some("s")
        );
    }
}
