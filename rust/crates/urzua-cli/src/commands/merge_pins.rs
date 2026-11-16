//! Whether a pin `fix --conflicts` would recompute is git's own merge of two
//! attested states (`MILE-284`). The merged file is hashed only when the
//! pinned file at our commit hashes to our side's pin, at their commit to
//! their side's pin, and the index holds exactly git's automatic merge of the
//! two with the working tree equal to it. Anything else is a state nobody
//! attested, which needs a person's re-pin.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use urzua_core::config::Config;
use urzua_core::conflicts::PinSides;
use urzua_core::locator::Locator;

/// The commits whose record versions are stages 2 and 3, and their base.
pub(crate) struct Heads {
    ours: String,
    theirs: String,
    base: String,
}

fn rev(repo_root: &Path, spec: &str) -> Option<String> {
    urzua_io::git::stdout_in(repo_root, &["rev-parse", "-q", "--verify", spec])
        .ok()
        .map(|b| String::from_utf8_lossy(&b).trim().to_string())
}

/// Stage 2 is always `HEAD`. Stage 3 is `MERGE_HEAD` in a merge and the
/// commit being replayed (`REBASE_HEAD`, `CHERRY_PICK_HEAD`) otherwise, whose
/// parent is the base git merged against.
pub(crate) fn heads(repo_root: &Path) -> Result<Heads, String> {
    let ours = rev(repo_root, "HEAD").ok_or("there is no HEAD commit")?;
    if let Some(theirs) = rev(repo_root, "MERGE_HEAD") {
        let base = urzua_io::git::stdout_in(repo_root, &["merge-base", &ours, &theirs])
            .map(|b| String::from_utf8_lossy(&b).trim().to_string())?;
        return Ok(Heads { ours, theirs, base });
    }
    for head in ["REBASE_HEAD", "CHERRY_PICK_HEAD"] {
        if let Some(theirs) = rev(repo_root, head) {
            let base = rev(repo_root, &format!("{theirs}^"))
                .ok_or_else(|| format!("{head} has no parent to merge against"))?;
            return Ok(Heads { ours, theirs, base });
        }
    }
    Err("git is in no merge, rebase or cherry-pick (no MERGE_HEAD, REBASE_HEAD or CHERRY_PICK_HEAD), so the two committed states cannot be checked".into())
}

fn show(repo_root: &Path, spec: &str) -> Result<Vec<u8>, String> {
    urzua_io::git::stdout_in(repo_root, &["show", spec])
}

/// What `bytes` hashes to under the pin's scope, the way a pin is read.
fn hash_under_scope(config: &Config, locator: &Locator, bytes: &[u8]) -> Result<String, String> {
    let Some(scope) = &locator.scope else {
        return Ok(urzua_io::sha256_hex_of_bytes(bytes));
    };
    let text = std::str::from_utf8(bytes).map_err(|_| "the file is not UTF-8".to_string())?;
    if text.starts_with('\u{feff}') {
        return Err("the file starts with a byte-order mark (BOM)".into());
    }
    let span = urzua_core::locator::select_resolver(&config.locator_resolvers, &locator.path)
        .and_then(|resolver| urzua_core::locator::resolve_scope(text, resolver, scope))
        .map_err(|e| e.to_string())?;
    Ok(urzua_io::sha256_hex_of_str(
        &urzua_core::locator::span_text(text, span),
    ))
}

/// The merged pin hash, or which condition failed.
pub(crate) fn verified_hash(
    repo_root: &Path,
    config: &Config,
    unmerged: &BTreeSet<PathBuf>,
    heads: &Heads,
    pin: &PinSides,
) -> Result<String, String> {
    let locator = urzua_core::locator::parse_locator(&pin.spelling)
        .map_err(|e| format!("its locator does not parse: {e}"))?;
    let path = locator.path.clone();
    if unmerged.contains(Path::new(&path)) {
        return Err(format!("the pinned file {path} is itself unmerged"));
    }
    let texts = crate::file_texts::FileTexts::new(repo_root.to_path_buf(), config);
    if crate::pins::read_files_pin(repo_root, config, &texts, &path).is_some() {
        return Err(format!(
            "{path} is a directory or glob pin, which is not checked against the two commits"
        ));
    }
    let at = |commit: &str| show(repo_root, &format!("{commit}:{path}"));
    for (side, commit, want) in [
        ("our commit (HEAD)", &heads.ours, &pin.ours),
        ("their commit", &heads.theirs, &pin.theirs),
    ] {
        let bytes = at(commit).map_err(|_| format!("{path} does not exist at {side}"))?;
        let got = hash_under_scope(config, &locator, &bytes)
            .map_err(|e| format!("{path} at {side} cannot be hashed: {e}"))?;
        if Some(&got) != want.as_ref() {
            return Err(format!(
                "{path} at {side} does not hash to that side's pin: the file changed there without a re-pin"
            ));
        }
    }
    let index = show(repo_root, &format!(":0:{path}"))
        .map_err(|_| format!("{path} is not in the index as a merged file"))?;
    let working = std::fs::read(repo_root.join(&path))
        .map_err(|e| format!("{path} cannot be read in the working tree: {e}"))?;
    if working != index {
        return Err(format!(
            "{path} in the working tree differs from the index: it was edited by hand during the merge"
        ));
    }
    let base = at(&heads.base).unwrap_or_default();
    match merge_three(&at(&heads.ours)?, &base, &at(&heads.theirs)?)? {
        (true, merged) if merged == index => {}
        _ => {
            return Err(format!(
                "{path} in the index is not git's automatic merge of the two commits"
            ))
        }
    }
    hash_under_scope(config, &locator, &index).map_err(|e| format!("{path} cannot be hashed: {e}"))
}

/// `git merge-file -p` of three texts: whether it merged cleanly, and the
/// result.
pub(crate) fn merge_three(
    ours: &[u8],
    base: &[u8],
    theirs: &[u8],
) -> Result<(bool, Vec<u8>), String> {
    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("urzua-conflicts-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create a scratch dir: {e}"))?;
    let result = (|| {
        for (name, bytes) in [("ours", ours), ("base", base), ("theirs", theirs)] {
            std::fs::write(dir.join(name), bytes).map_err(|e| format!("could not write: {e}"))?;
        }
        let out = urzua_io::git::GitRun::new()
            .args(["merge-file", "-p", "ours", "base", "theirs"])
            .cwd(&dir)
            .output()
            .map_err(|e| format!("could not run git: {}", e.into_io()))?;
        match out.status.code() {
            Some(0) => Ok((true, out.stdout)),
            Some(n) if n > 0 => Ok((false, out.stdout)),
            _ => Err(format!(
                "git merge-file failed: {}",
                urzua_io::git::stderr_text(&out)
            )),
        }
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}
