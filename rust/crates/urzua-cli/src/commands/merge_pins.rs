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
    pub(crate) ours: String,
    pub(crate) theirs: String,
    base: String,
    /// Every merge base git found; stage 1 is the record at one of them, or
    /// at git's virtual merge of them.
    pub(crate) bases: Vec<String>,
    /// A merge or cherry-pick brings theirs into this branch; a rebase
    /// replays this branch's commit (theirs) onto ours.
    pub(crate) incoming_is_theirs: bool,
}

/// Why a pin could not be recomputed.
pub(crate) enum Unverified {
    /// The pinned file is itself still unmerged: resolve it first.
    Unmerged(PathBuf),
    Other(String),
}

impl From<String> for Unverified {
    fn from(e: String) -> Self {
        Unverified::Other(e)
    }
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
        let bases: Vec<String> =
            urzua_io::git::stdout_in(repo_root, &["merge-base", "--all", &ours, &theirs]).map(
                |b| {
                    String::from_utf8_lossy(&b)
                        .split_whitespace()
                        .map(str::to_string)
                        .collect()
                },
            )?;
        let base = bases
            .first()
            .cloned()
            .ok_or("the two commits have no merge base")?;
        return Ok(Heads {
            ours,
            theirs,
            base,
            bases,
            incoming_is_theirs: true,
        });
    }
    for head in ["REBASE_HEAD", "CHERRY_PICK_HEAD"] {
        if let Some(theirs) = rev(repo_root, head) {
            let base = rev(repo_root, &format!("{theirs}^"))
                .ok_or_else(|| format!("{head} has no parent to merge against"))?;
            return Ok(Heads {
                ours,
                theirs,
                bases: vec![base.clone()],
                base,
                incoming_is_theirs: head == "CHERRY_PICK_HEAD",
            });
        }
    }
    Err("git is in no merge, rebase or cherry-pick (no MERGE_HEAD, REBASE_HEAD or CHERRY_PICK_HEAD), so the two committed states cannot be checked".into())
}

pub(crate) fn show(repo_root: &Path, spec: &str) -> Result<Vec<u8>, String> {
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
) -> Result<String, Unverified> {
    let locator = urzua_core::locator::parse_locator(&pin.spelling)
        .map_err(|e| format!("its locator does not parse: {e}"))?;
    let path = locator.path.clone();
    if unmerged.contains(Path::new(&path)) {
        return Err(Unverified::Unmerged(PathBuf::from(path)));
    }
    let texts = crate::file_texts::FileTexts::new(repo_root.to_path_buf(), config);
    if crate::pins::read_files_pin(repo_root, config, &texts, &path).is_some() {
        return Err(format!(
            "{path} is a directory or glob pin, which is not checked against the two commits"
        )
        .into());
    }
    let at = |commit: &str| show(repo_root, &format!("{commit}:{path}"));
    check_narrowing(pin, &locator)?;
    for a in attested_sides(heads, pin, &locator)? {
        let (side, file) = (a.side, a.locator.path.clone());
        let bytes = show(repo_root, &format!("{}:{file}", a.commit))
            .map_err(|_| format!("{file} does not exist at {side}"))?;
        let got = hash_under_scope(config, &a.locator, &bytes)
            .map_err(|e| format!("{file} at {side} cannot be hashed: {e}"))?;
        if Some(&got) != a.want.as_ref() {
            return Err(format!(
                "{file} at {side} does not hash to that side's pin: the file changed there without a re-pin"
            )
            .into());
        }
    }
    let index = show(repo_root, &format!(":0:{path}"))
        .map_err(|_| format!("{path} is not in the index as a merged file"))?;
    plain_file_everywhere(repo_root, heads, &path)?;
    let working = std::fs::read(repo_root.join(&path))
        .map_err(|e| format!("{path} cannot be read in the working tree: {e}"))?;
    if working != index {
        let lf = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
        if lf(&working) == lf(&index) {
            return Err(format!(
                "{path} in the working tree differs from the index only in line endings: git's line-ending conversion (core.autocrlf or an eol attribute) wrote it on checkout, which is not a hand edit, but the pin is read from the working tree, so the hashes would not agree"
            )
            .into());
        }
        return Err(format!(
            "{path} in the working tree differs from the index: it was edited by hand during the merge"
        )
        .into());
    }
    let base = at(&heads.base).unwrap_or_default();
    match merge_three(&at(&heads.ours)?, &base, &at(&heads.theirs)?)? {
        (true, merged) if merged == index => {}
        _ => {
            return Err(format!(
                "{path} in the index is not git's automatic merge of the two commits"
            )
            .into())
        }
    }
    let base_text = String::from_utf8_lossy(&base).into_owned();
    let merged_text = String::from_utf8_lossy(&index).into_owned();
    let scoped: Vec<Attested> = attested_sides(heads, pin, &locator)?
        .into_iter()
        .filter(|a| a.locator.scope.is_some())
        .collect();
    let mut spans = Vec::new();
    for a in &scoped {
        spans.push(side_span(repo_root, config, a, &base_text)?);
    }
    for (a, span) in scoped.iter().zip(&spans) {
        check_merged_span(config, a, span, &merged_text, scoped.len() > 1)?;
    }
    Ok(hash_under_scope(config, &locator, &index)
        .map_err(|e| format!("{path} cannot be hashed: {e}"))?)
}

/// The lines of `text`, each with its line ending, so a comparison is exact.
fn lines_of(text: &str) -> Vec<&str> {
    text.split_inclusive('\n').collect()
}

/// A side that pinned a scope attests that scope and nothing else, so its file
/// must equal the base outside the scope's span (the same lines before it and
/// after it). Returns the side's own text and the scope's span in it.
fn side_span(
    repo_root: &Path,
    config: &Config,
    a: &Attested,
    base: &str,
) -> Result<(String, urzua_core::locator::Span), String> {
    let (side, file) = (a.side, a.locator.path.clone());
    let bytes = show(repo_root, &format!("{}:{file}", a.commit))
        .map_err(|_| format!("{file} does not exist at {side}"))?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let span = scope_span(config, a, &text, &format!("at {side}"))?;
    let (b, s) = (lines_of(base), lines_of(&text));
    let (before, after) = (span.start - 1, s.len().saturating_sub(span.end));
    let confined = b.len() >= before + after
        && b[..before] == s[..before]
        && b[b.len() - after..] == s[s.len() - after..];
    if !confined {
        return Err(format!(
            "{file} at {side} changed code outside what that side pinned ('{}'): the merged pin would cover lines nobody attested",
            a.locator.display()
        ));
    }
    Ok((text, span))
}

fn scope_span(
    config: &Config,
    a: &Attested,
    text: &str,
    where_: &str,
) -> Result<urzua_core::locator::Span, String> {
    let file = &a.locator.path;
    let scope = a.locator.scope.as_ref().ok_or("the locator has no scope")?;
    urzua_core::locator::select_resolver(&config.locator_resolvers, file)
        .and_then(|r| urzua_core::locator::resolve_scope(text, r, scope))
        .map_err(|e| format!("{file} {where_} cannot be resolved: {e}"))
}

/// The scope resolved in the merged file must keep the lines before it that the
/// side has (a merge that moved it is refused) and, when every pin of the
/// merge is scoped (`both`), the lines after it as well: the merged span then
/// lies inside each side's span, so a merge that combined both sides' changes
/// across the span's edge pins no line a side did not pin. With a side that
/// pinned the whole file, that side attests every line, so only the lines
/// before the scope are held.
fn check_merged_span(
    config: &Config,
    a: &Attested,
    (side_text, side_span): &(String, urzua_core::locator::Span),
    merged: &str,
    both: bool,
) -> Result<(), String> {
    let file = &a.locator.path;
    let (s, m) = (lines_of(side_text), lines_of(merged));
    let merged_span = scope_span(config, a, merged, "in the merged file")?;
    let (before, after) = (side_span.start - 1, s.len().saturating_sub(side_span.end));
    let (m_before, m_after) = (
        merged_span.start - 1,
        m.len().saturating_sub(merged_span.end),
    );
    if m[..m_before] != s[..before] || (both && m[m.len() - m_after..] != s[s.len() - after..]) {
        return Err(format!(
            "the merged span of '{}' in {file} does not start and end where {}'s does: the merge moved it or combined both sides' changes across its edge, so the merged pin would cover lines that side did not pin",
            a.locator.display(),
            a.side
        ));
    }
    Ok(())
}

/// A locator one side replaced is recomputed only when the old one is the whole
/// file and the new one a scope of that same file. That is a precondition, not
/// the whole argument: the merged file also carries the changer's edits outside
/// its scope and the other side's edits anywhere, so `side_span` and `check_merged_span` still has
/// to hold. An old range or symbol is refused outright: its re-pin covers what
/// that scope held in the other side's file, whose lines can sit elsewhere than
/// in the changer's.
fn check_narrowing(pin: &PinSides, new: &Locator) -> Result<(), String> {
    let Some(moved) = &pin.moved_from else {
        return Ok(());
    };
    let old = urzua_core::locator::parse_locator(&moved.spelling)
        .map_err(|e| format!("its old locator '{}' does not parse: {e}", moved.spelling))?;
    let (new_name, old_name) = (&pin.spelling, &moved.spelling);
    if old.path != new.path {
        return Err(format!(
            "one side replaced '{old_name}' with '{new_name}', a different file, and the other side's re-pin of '{old_name}' does not cover it; only a whole file narrowed to a scope of itself is resolved on merge"
        ));
    }
    if old.scope.is_some() {
        return Err(format!(
            "one side replaced '{old_name}' with '{new_name}', and '{old_name}' is a range or symbol, not the whole file: the other side's re-pin of it covers what it held in that side's file, which may not be the lines '{new_name}' covers in the merged file; only a whole file narrowed to a scope of itself is resolved on merge"
        ));
    }
    Ok(())
}

/// One side of a merge and the pin it attests: its commit, the locator the pin
/// is over and the hash the file must have under it.
struct Attested {
    side: &'static str,
    commit: String,
    locator: Locator,
    want: Option<String>,
}

/// Each side's commit, the locator that side's pin is over and the hash it
/// must have. A pin one side wrote in place of another locator is attested at
/// that side by its own pin and at the other side by that old locator's.
fn attested_sides(
    heads: &Heads,
    pin: &PinSides,
    locator: &Locator,
) -> Result<Vec<Attested>, String> {
    let ours = ("our commit (HEAD)", heads.ours.clone());
    let theirs = ("their commit", heads.theirs.clone());
    let Some(moved) = &pin.moved_from else {
        return Ok(vec![
            attested(ours, locator.clone(), pin.ours.clone()),
            attested(theirs, locator.clone(), pin.theirs.clone()),
        ]);
    };
    let old = urzua_core::locator::parse_locator(&moved.spelling)
        .map_err(|e| format!("its old locator '{}' does not parse: {e}", moved.spelling))?;
    let (changer, other) = if moved.changed_by_ours {
        (ours, theirs)
    } else {
        (theirs, ours)
    };
    let changer_pin = if moved.changed_by_ours {
        &pin.ours
    } else {
        &pin.theirs
    };
    Ok(vec![
        attested(changer, locator.clone(), changer_pin.clone()),
        attested(other, old, Some(moved.other_hash.clone())),
    ])
}

fn attested(side: (&'static str, String), locator: Locator, want: Option<String>) -> Attested {
    Attested {
        side: side.0,
        commit: side.1,
        locator,
        want,
    }
}

/// A regular file of one mode at both commits and in the index, and not a
/// symlink in the working tree: a mode change or a link is no re-pin.
fn plain_file_everywhere(repo_root: &Path, heads: &Heads, path: &str) -> Result<(), String> {
    let mode_at = |commit: &str| -> Option<String> {
        let out = urzua_io::git::stdout_in(repo_root, &["ls-tree", commit, "--", path]).ok()?;
        String::from_utf8_lossy(&out)
            .split_whitespace()
            .next()
            .map(str::to_string)
    };
    let index_mode = urzua_io::git::stdout_in(repo_root, &["ls-files", "-s", "--", path])
        .ok()
        .and_then(|out| {
            String::from_utf8_lossy(&out)
                .split_whitespace()
                .next()
                .map(str::to_string)
        });
    let modes = [
        ("our commit (HEAD)", mode_at(&heads.ours)),
        ("their commit", mode_at(&heads.theirs)),
        ("the index", index_mode),
    ];
    for (where_, mode) in &modes {
        if !matches!(mode.as_deref(), Some("100644" | "100755")) {
            return Err(format!(
                "{path} is not a regular file at {where_} (mode {}): a symlink or another kind of entry is not checked against the two commits",
                mode.as_deref().unwrap_or("none")
            ));
        }
    }
    if modes.iter().any(|(_, m)| *m != modes[0].1) {
        return Err(format!(
            "{path} changed its file mode between the two commits or in the index, which no re-pin attests"
        ));
    }
    let meta = std::fs::symlink_metadata(repo_root.join(path))
        .map_err(|e| format!("{path} cannot be read in the working tree: {e}"))?;
    if meta.file_type().is_symlink() {
        return Err(format!(
            "{path} is a symlink in the working tree, so the file a pin would read is not the merged file"
        ));
    }
    Ok(())
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
