//! Filesystem and git discovery, plus file reading. The impure half of the
//! split `urzua-core` describes in its own doc comment — see
//! ADR-0005 for why this is a separate crate, and ADR-0006 for why it shells
//! out to `git` rather than linking libgit2.
//!
//! Implements: ADR-0005, ADR-0006

#![forbid(unsafe_code)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("could not run `git {args}` in {dir}: {source}")]
    Spawn {
        args: String,
        dir: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("`git {args}` in {dir} exited with {status}: {stderr}")]
    NonZeroExit {
        args: String,
        dir: PathBuf,
        status: std::process::ExitStatus,
        stderr: String,
    },
    #[error("{base:?} is not a valid base ref: a ref may not begin with `-`")]
    InvalidRef { base: String },
    #[error("there is no merge base between {base:?} and HEAD: the histories are unrelated")]
    NoMergeBase { base: String },
    #[error("there is no merge base between {base:?} and HEAD because this is a shallow clone and the history that joins them is not fetched: fetch full history (e.g. `git fetch --unshallow`, or `fetch-depth: 0` in CI)")]
    ShallowClone { base: String },
    #[error("`git {args}` output was not valid UTF-8")]
    InvalidUtf8 { args: String },
}

/// The set of files git considers relevant, and how they were discovered.
/// Never `Silent` — a caller that cannot enumerate git state must know that,
/// rather than receiving an empty list indistinguishable from "no files."
#[derive(Debug, Clone)]
pub struct DiscoveredFiles {
    pub paths: Vec<PathBuf>,
    /// Paths staged for deletion. They are still tracked, so they appear in
    /// `paths`, and they are legitimately not on disk. Carried separately
    /// because without it a caller cannot tell a staged deletion from a file
    /// someone deleted and has not staged, and skipping both let a record
    /// vanish from the corpus with the run still reported clean (BUG-82).
    pub staged_deletions: std::collections::HashSet<PathBuf>,
    /// Tracked record paths missing from the working tree that nothing has
    /// staged for deletion (an `rm`, or a `mv` before `git add`). The caller
    /// that notices them also adds them to `staged_deletions`, so they are
    /// treated as removed; this set is what lets the run say so rather than
    /// let a record vanish in silence (BUG-82).
    pub unstaged_deletions: std::collections::HashSet<PathBuf>,
    pub source: DiscoverySource,
    /// The untracked, non-ignored files [`UntrackedInclusion`] added to `paths`
    /// (`ADR-92`); empty when no inclusion was asked for. Carried separately so
    /// a report can count them and `--base` can treat them as added.
    pub untracked_included: std::collections::HashSet<PathBuf>,
}

/// What a repository that opts in (`discovery.include_untracked`, `ADR-92`)
/// has discovery add: the untracked, non-ignored files directly under
/// `dirs` that `governs` accepts. `governs` is the caller's own ownership rule,
/// the one that loads records, so what discovery adds is exactly what would
/// load; this crate knows no record type.
pub struct UntrackedInclusion<'a> {
    /// The declared record directories, repository-relative.
    pub dirs: &'a [PathBuf],
    /// Whether a repository-relative path is a file some record type governs.
    pub governs: &'a dyn Fn(&Path) -> bool,
}

/// The untracked, non-ignored files that sit directly in one of `inclusion.dirs`
/// and that `inclusion.governs` accepts, sorted. Git's own ignore rules decide
/// "non-ignored" (`.gitignore`, `.git/info/exclude`, the global excludes), and
/// `-z` keeps a non-ASCII path verbatim (`BUG-87`).
pub fn untracked_governed_files(
    repo_root: &Path,
    inclusion: &UntrackedInclusion<'_>,
) -> Result<Vec<PathBuf>, DiscoveryError> {
    if inclusion.dirs.is_empty() {
        return Ok(Vec::new());
    }
    let mut args: Vec<String> = ["ls-files", "--others", "--exclude-standard", "-z", "--"]
        .iter()
        .map(|a| a.to_string())
        .collect();
    args.extend(inclusion.dirs.iter().map(|d| {
        // A literal directory, not a pathspec pattern.
        format!(":(literal){}/", d.display())
    }));
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let listed = run_git(repo_root, &args)?;
    let mut out: Vec<PathBuf> = listed
        .split('\0')
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .filter(|p| {
            p.parent()
                .is_some_and(|parent| inclusion.dirs.iter().any(|d| d == parent))
        })
        .filter(|p| (inclusion.governs)(p))
        .collect();
    out.sort();
    out.dedup();
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoverySource {
    /// The union of `git ls-files` (tracked) and staged files.
    GitTracked,
    /// Explicit paths on argv, read as given (`BUG-24`): git-tracked status
    /// is not consulted at all, unlike `GitTracked`'s default sweep.
    Argv,
    /// Only the records changed against a base ref (`MILE-123`): the tracked
    /// set is still what is indexed, the diff is what is reported on.
    GitDiff,
}

/// Discover the files a run reads, at or below `repo_root`: the files git
/// tracks or has staged, plus, when `untracked` is given (a repository that set
/// `discovery.include_untracked`, `ADR-92`), the untracked, non-ignored record
/// files directly under its declared directories. With `None` an untracked
/// scratch file never appears here, so a single stray file does not change what
/// every run examines (`ADR-56`). Never a raw filesystem walk either way: the
/// untracked set is git's own listing, restricted to the declared directories
/// and the caller's ownership rule, and the paths it adds are returned in
/// [`DiscoveredFiles::untracked_included`] as well as in `paths`.
pub fn discover_tracked_files(
    repo_root: &Path,
    untracked: Option<&UntrackedInclusion<'_>>,
) -> Result<DiscoveredFiles, DiscoveryError> {
    // `-z`, so paths arrive NUL-separated and verbatim. With `core.quotepath`
    // at its default git C-quotes any path holding a non-ASCII byte -- the
    // name arrives wrapped in quotes with the bytes escaped, its parent never
    // equals the declared `dir`, and the record drops out of the corpus with
    // no rule reporting it (BUG-87).
    let tracked = run_git(repo_root, &["ls-files", "-z"])?;
    // `--name-status`, one subprocess instead of two separate `--name-only`
    // calls: the deleted set is always a subset of the staged set for every
    // status but a rename/copy, so a second call for it was re-deriving what
    // this one call's own status codes already say. `-z` pairs each entry as
    // `<status>\0<path>\0`, except `R`/`C` (rename/copy), which git always
    // reports as `<status>\0<old-path>\0<new-path>\0` -- three fields, not
    // two (verified against real git output, not assumed from the docs). A
    // rename's old name is deliberately not counted as a staged deletion
    // here, matching the two-call version's own behavior: `--diff-filter=D`
    // never matched a rename either, since git classifies it `R`, not `D`.
    let staged = run_git(repo_root, &["diff", "--name-status", "--cached", "-z"])?;

    let mut staged_paths = Vec::new();
    let mut staged_deletions = std::collections::HashSet::new();
    let mut fields = staged.split('\0').filter(|f| !f.is_empty());
    while let Some(status) = fields.next() {
        let is_rename_or_copy = status.starts_with('R') || status.starts_with('C');
        let Some(first_path) = fields.next() else {
            break;
        };
        let path = if is_rename_or_copy {
            // The old name is the extra field a rename/copy carries; the
            // new name is the one that matters to a caller reading the
            // working tree today.
            let Some(new_path) = fields.next() else {
                break;
            };
            new_path
        } else {
            first_path
        };
        let path = PathBuf::from(path);
        if status.starts_with('D') {
            staged_deletions.insert(path.clone());
        }
        staged_paths.push(path);
    }

    let mut paths: Vec<PathBuf> = tracked
        .split('\0')
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .chain(staged_paths)
        .collect();
    paths.sort();
    paths.dedup();

    let mut untracked_included = std::collections::HashSet::new();
    if let Some(inclusion) = untracked {
        let known: std::collections::HashSet<&PathBuf> = paths.iter().collect();
        untracked_included = untracked_governed_files(repo_root, inclusion)?
            .into_iter()
            .filter(|p| !known.contains(p))
            .collect();
        paths.extend(untracked_included.iter().cloned());
        paths.sort();
    }

    Ok(DiscoveredFiles {
        paths,
        staged_deletions,
        unstaged_deletions: std::collections::HashSet::new(),
        source: DiscoverySource::GitTracked,
        untracked_included,
    })
}

/// What differs between the merge base of a ref and the working tree
/// (`MILE-123`).
#[derive(Debug, Clone)]
pub struct GitChanges {
    /// The merge base commit, so a caller can read a deleted file as it was.
    pub merge_base: String,
    /// Every path that differs, deletions included, repository-relative.
    pub changed: std::collections::HashSet<PathBuf>,
    /// The subset of `changed` that no longer exists in the working tree.
    pub deleted: std::collections::HashSet<PathBuf>,
    /// The subset of `changed` that did not exist at the merge base. With
    /// `--no-renames` a renamed file is a deletion plus an addition, so the
    /// new name is in here.
    pub added: std::collections::HashSet<PathBuf>,
}

impl GitChanges {
    /// Counts untracked records as added (`ADR-92` clause 6): a file with no
    /// merge-base copy is new by definition, so it is in `changed` and `added`
    /// exactly as a staged addition is.
    pub fn add_untracked<'a>(&mut self, paths: impl IntoIterator<Item = &'a PathBuf>) {
        for path in paths {
            self.changed.insert(path.clone());
            self.added.insert(path.clone());
        }
    }
}

/// Every path that differs between the merge base of `base` and `HEAD` and
/// the working tree, repository-relative (`MILE-123`). Merge base, not the
/// ref's tip: a pull request must not be charged for what its base branch
/// gained since the fork. The working tree, not `HEAD`: staged and unstaged
/// edits to tracked files are changes a local run must see. Untracked files
/// are not reported here, matching the tracked-only discovery (`ADR-6`); a
/// repository that includes them (`ADR-92`) adds them with
/// [`GitChanges::add_untracked`], so they count as added.
///
/// Renames are reported as a deletion plus an addition (`--no-renames`), so
/// the new name is always listed on its own. A deleted path is listed too,
/// and also in `deleted`: a record that was deleted is no longer in the
/// tree, but whatever referred to it is affected by the change.
pub fn changed_paths_since(repo_root: &Path, base: &str) -> Result<GitChanges, DiscoveryError> {
    // A ref beginning with `-` would be read by git as an option.
    if base.starts_with('-') {
        return Err(DiscoveryError::InvalidRef {
            base: base.to_string(),
        });
    }
    // `merge-base` exits 1 with nothing on stderr when the two commits share
    // no history; every other failure (an unknown ref, 128) keeps git's own
    // message, which names the ref.
    let merge_base = match run_git(repo_root, &["merge-base", base, "HEAD"]) {
        Ok(out) => out,
        Err(DiscoveryError::NonZeroExit { status, stderr, .. })
            if status.code() == Some(1) && stderr.trim().is_empty() =>
        {
            // A shallow clone hides the commits that join the two histories;
            // that, not unrelated histories, is the usual cause in CI.
            if is_shallow_repository(repo_root).unwrap_or(false) {
                return Err(DiscoveryError::ShallowClone {
                    base: base.to_string(),
                });
            }
            return Err(DiscoveryError::NoMergeBase {
                base: base.to_string(),
            });
        }
        Err(e) => return Err(e),
    };
    let merge_base = merge_base.trim().to_string();
    let diff = run_git(
        repo_root,
        &[
            "diff",
            "--name-status",
            "--no-renames",
            "-z",
            &merge_base,
            "--",
        ],
    )?;
    let mut changed = std::collections::HashSet::new();
    let mut deleted = std::collections::HashSet::new();
    let mut added = std::collections::HashSet::new();
    let mut fields = diff.split('\0').filter(|f| !f.is_empty());
    while let (Some(status), Some(path)) = (fields.next(), fields.next()) {
        let path = PathBuf::from(path);
        if status.starts_with('D') {
            deleted.insert(path.clone());
        }
        if status.starts_with('A') {
            added.insert(path.clone());
        }
        changed.insert(path);
    }
    Ok(GitChanges {
        merge_base,
        changed,
        deleted,
        added,
    })
}

/// The files directly in each of `dirs` at each of `commits`, as repository-relative
/// paths (`dir/name`), keyed by commit. One `git cat-file --batch-check` process
/// resolves every `commit:dir` to a tree, and `git ls-tree` then runs once per
/// *distinct* tree: the pull-request heads of a large repository mostly share a
/// directory's tree with `main`, so listing each tip separately (hundreds of
/// processes) is what made the open-pull-request check slow. A commit with no such
/// directory has no entry for it.
pub fn files_in_dirs_at_commits(
    repo_root: &Path,
    commits: &[String],
    dirs: &[String],
) -> Result<std::collections::HashMap<String, Vec<String>>, String> {
    use std::collections::HashMap;
    let mut wanted: Vec<(&String, &String)> = Vec::new();
    for commit in commits {
        for dir in dirs {
            wanted.push((commit, dir));
        }
    }
    let input: String = wanted
        .iter()
        .map(|(c, d)| format!("{c}:{}\n", d.trim_end_matches('/')))
        .collect();
    let out = cat_file_batch(repo_root, "--batch-check", input)?;
    let text = String::from_utf8_lossy(&out.stdout);
    let answers: Vec<&str> = text.lines().collect();
    if answers.len() != wanted.len() {
        return Err("git cat-file answered a different number of lines than asked".to_string());
    }
    // The distinct trees, in first-seen order.
    let mut distinct: Vec<&str> = Vec::new();
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut tree_of: Vec<Option<&str>> = Vec::with_capacity(wanted.len());
    for answer in &answers {
        let mut parts = answer.split(' ');
        match (parts.next(), parts.next()) {
            (Some(oid), Some("tree")) => {
                if seen.insert(oid) {
                    distinct.push(oid);
                }
                tree_of.push(Some(oid));
            }
            _ => tree_of.push(None),
        }
    }
    let listed = list_trees(repo_root, &distinct)?;
    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    for ((commit, dir), tree) in wanted.iter().zip(tree_of) {
        let Some(oid) = tree else {
            continue;
        };
        let dir = dir.trim_end_matches('/');
        result
            .entry((*commit).clone())
            .or_default()
            .extend(listed[oid].iter().map(|n| format!("{dir}/{n}")));
    }
    Ok(result)
}

/// Runs `git cat-file <mode>` over `input` and returns its output. The input is
/// written on its own thread: a full output pipe must not block it.
fn cat_file_batch(
    repo_root: &Path,
    mode: &str,
    input: String,
) -> Result<std::process::Output, String> {
    use std::io::Write;
    let mut child = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["cat-file", mode])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not run git: {e}"))?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(input.as_bytes());
    });
    let out = child
        .wait_with_output()
        .map_err(|e| format!("could not read git: {e}"))?;
    let _ = writer.join();
    if !out.status.success() {
        return Err(format!(
            "git cat-file failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(out)
}

/// The names of the files (not directories or submodules) directly in each tree,
/// read in one `git cat-file --batch` process: a repository with hundreds of pull
/// requests has over a thousand distinct trees, and a `git ls-tree` process for
/// each took longer than the rest of a `check` run.
fn list_trees<'a>(
    repo_root: &Path,
    oids: &[&'a str],
) -> Result<std::collections::HashMap<&'a str, Vec<String>>, String> {
    let width = match run_git(repo_root, &["rev-parse", "--show-object-format"])
        .map_err(|e| e.to_string())?
        .trim()
    {
        "sha256" => 32,
        _ => 20,
    };
    let input: String = oids.iter().map(|o| format!("{o}\n")).collect();
    let out = cat_file_batch(repo_root, "--batch", input)?;
    let bytes = out.stdout;
    let bad = || "git cat-file --batch printed an object this tool cannot read".to_string();
    let mut listed = std::collections::HashMap::new();
    let mut at = 0usize;
    for oid in oids {
        // `<oid> tree <size>\n`, then <size> bytes, then `\n`.
        let eol = bytes[at..]
            .iter()
            .position(|b| *b == b'\n')
            .ok_or_else(bad)?;
        let header = std::str::from_utf8(&bytes[at..at + eol]).map_err(|_| bad())?;
        let size: usize = header
            .rsplit(' ')
            .next()
            .and_then(|n| n.parse().ok())
            .ok_or_else(bad)?;
        if !header.contains(" tree ") {
            return Err(bad());
        }
        let body = bytes
            .get(at + eol + 1..at + eol + 1 + size)
            .ok_or_else(bad)?;
        at += eol + 1 + size + 1;
        let mut names = Vec::new();
        let mut i = 0;
        while i < body.len() {
            // `<mode> <name>\0<raw oid>`
            let space = body[i..].iter().position(|b| *b == b' ').ok_or_else(bad)?;
            let mode = &body[i..i + space];
            let nul = body[i + space..]
                .iter()
                .position(|b| *b == 0)
                .ok_or_else(bad)?;
            let name = String::from_utf8_lossy(&body[i + space + 1..i + space + nul]).into_owned();
            i += space + nul + 1 + width;
            if mode != b"40000" && mode != b"160000" {
                names.push(name);
            }
        }
        listed.insert(*oid, names);
    }
    Ok(listed)
}

/// A file as it was at `commit`: how a deleted record is read once it is
/// gone from the tree.
pub fn read_at_commit(
    repo_root: &Path,
    commit: &str,
    path: &Path,
) -> Result<String, DiscoveryError> {
    let spec = format!("{commit}:{}", path.display());
    run_git(repo_root, &["show", &spec])
}

/// A file as it was at `commit`, or `None` when the commit has no such path.
/// Absence is an answer; every other failure (a missing object, a copy that is
/// not UTF-8) is an error, so a caller cannot read "could not read" as "was
/// never there". Presence is asked of the tree (`ls-tree`), not read off git's
/// error wording, which is localised.
pub fn read_at_commit_if_present(
    repo_root: &Path,
    commit: &str,
    path: &Path,
) -> Result<Option<String>, DiscoveryError> {
    let listed = run_git(
        repo_root,
        &[
            "ls-tree",
            "--name-only",
            "-z",
            commit,
            "--",
            &path.display().to_string(),
        ],
    )?;
    if listed.is_empty() {
        return Ok(None);
    }
    read_at_commit(repo_root, commit, path).map(Some)
}

/// Read a file's content as UTF-8. The only I/O `urzua-core`'s parser needs;
/// `urzua-core` itself never touches a `Path`.
pub fn read_to_string(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

/// Where the evaluation date came from (`ADR-97`, amendment of 2026-11-08).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsOfSource {
    /// `--as-of` named it.
    Flag,
    /// The machine's clock, read once.
    Clock,
}

#[derive(Debug, Clone)]
struct EvaluationClock {
    date: String,
    /// The instant claim ages are measured to: the end of `date` (UTC) for a
    /// flag, so nothing dated on that day is in the future; the real instant
    /// for the clock.
    unix: i64,
    source: AsOfSource,
}

static CLOCK: std::sync::OnceLock<EvaluationClock> = std::sync::OnceLock::new();

/// The one place the wall clock is read. Every date and age comparison reads
/// the evaluation clock below; `clock_is_read_in_one_place` (urzua-cli tests)
/// fails if another call site appears.
fn system_clock() -> EvaluationClock {
    let now = chrono::Utc::now();
    EvaluationClock {
        date: now.date_naive().format("%Y-%m-%d").to_string(),
        unix: now.timestamp(),
        source: AsOfSource::Clock,
    }
}

/// Parses a strict `YYYY-MM-DD` calendar date.
fn parse_as_of(value: &str) -> Result<chrono::NaiveDate, String> {
    let shaped = value.len() == 10
        && value.bytes().enumerate().all(|(i, b)| match i {
            4 | 7 => b == b'-',
            _ => b.is_ascii_digit(),
        });
    if !shaped {
        return Err(format!(
            "--as-of '{value}' is not a date: write it as YYYY-MM-DD"
        ));
    }
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| format!("--as-of '{value}' is not a calendar date: write it as YYYY-MM-DD"))
}

/// Fixes the evaluation date for the whole run (`--as-of`). Call once, before
/// anything reads a date; a second call, or a call after a read, is refused.
pub fn set_as_of(value: &str) -> Result<(), String> {
    let date = parse_as_of(value)?;
    let end_of_day = date
        .and_hms_opt(23, 59, 59)
        .map(|t| t.and_utc().timestamp())
        .ok_or_else(|| format!("--as-of '{value}' is out of range"))?;
    CLOCK
        .set(EvaluationClock {
            date: date.format("%Y-%m-%d").to_string(),
            unix: end_of_day,
            source: AsOfSource::Flag,
        })
        .map_err(|_| "the evaluation date was already fixed for this run".to_string())
}

fn clock() -> &'static EvaluationClock {
    CLOCK.get_or_init(system_clock)
}

/// The evaluation date, `YYYY-MM-DD`: `--as-of` when given, else today in UTC
/// whatever the machine's time zone (read once per run). Waiver `Expires`, a
/// snapshot's date, a claim's anchor and every other date comparison use it,
/// so a waiver lapses at the same instant for every contributor and in CI.
/// `urzua-core`'s comparisons take it as a plain string and stay pure.
pub fn today() -> String {
    clock().date.clone()
}

/// Where [`today`] came from.
pub fn as_of_source() -> AsOfSource {
    clock().source
}

const GH_AUTH_TIMEOUT: Duration = Duration::from_secs(5);

/// Where a resolved identity came from (`ADR-89` section 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentitySource {
    /// The `gh` login that `gh api user` verified.
    GhVerified,
    /// `--by`, taken as given.
    Asserted,
    /// `git config user.name`.
    GitConfig,
}

/// The `attestation` configuration, as `resolve_identity_with` reads it
/// (`ADR-89` section 5). `urzua-io` does not know the configuration type, so
/// the caller passes the two decisions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IdentityPolicy {
    /// `identity: asserted`: `--by` is taken as given, a verified login is
    /// recorded beside it.
    pub asserted: bool,
    /// `require_verified: true`: no `gh` answer is an error, never a fallback.
    pub require_verified: bool,
}

/// `resolve_identity`'s result plus any non-fatal notice about it. Structured
/// rather than a bare `eprintln!` so a caller can fold `warning` into its own
/// JSON report instead of it landing as unparseable stderr prose.
#[derive(Debug)]
pub struct ResolvedIdentity {
    pub name: String,
    pub warning: Option<String>,
    pub source: IdentitySource,
    /// The `--by` a verified login replaced, when the two differ.
    pub replaced_by: Option<String>,
    /// The login `gh` verified, recorded beside an `asserted` name.
    pub verified_login: Option<String>,
}

/// `resolve_identity_with` under the default policy (`gh-verified`, nothing
/// required): the behaviour of a repository that declares no `attestation`.
pub fn resolve_identity(
    explicit: Option<&str>,
    repo_root: &Path,
) -> Result<ResolvedIdentity, String> {
    resolve_identity_with(explicit, repo_root, IdentityPolicy::default())
}

/// Identity tiering (RFC-0002 §2, amended by ADR-0031 in this repo's own
/// records; `ADR-89` section 5 makes the source configurable): under the
/// default a `gh api user` login wins even over an explicit `--by` (surfaced
/// via `warning` and `replaced_by` on divergence, never silently discarding
/// `--by`); `--by` wins over `git config user.name`. Errors rather than
/// defaulting when none of the three resolve.
pub fn resolve_identity_with(
    explicit: Option<&str>,
    repo_root: &Path,
    policy: IdentityPolicy,
) -> Result<ResolvedIdentity, String> {
    // Checked whichever tier wins: a `--by` that would corrupt the row is
    // refused even when a gh login supersedes it.
    if let Some(by) = explicit {
        row_safe_identity(by, "--by")?;
    }
    let resolved = resolve_identity_unchecked(explicit, repo_root, policy)?;
    row_safe_identity(&resolved.name, "the resolved identity")?;
    if let Some(login) = &resolved.verified_login {
        row_safe_identity(login, "the verified gh login")?;
    }
    Ok(resolved)
}

/// The identity is written into a revision-log table row, where a `|` adds a
/// cell and a line break splits the row. Refused, as `--reason` is, rather
/// than rewritten: a silently altered name is not the one that attested.
fn row_safe_identity(name: &str, source: &str) -> Result<(), String> {
    if name.chars().any(|c| c == '|' || c.is_control()) {
        return Err(format!(
            "{source} {name:?} contains '|' or a control character such as a line break; \
             it is written into a revision-log table row. Pass --by with a plain name \
             (or fix `git config user.name`)"
        ));
    }
    Ok(())
}

fn resolve_identity_unchecked(
    explicit: Option<&str>,
    repo_root: &Path,
    policy: IdentityPolicy,
) -> Result<ResolvedIdentity, String> {
    let verified = gh_authenticated_login();

    if policy.asserted {
        // `--by` as given, the verified login beside it (`ADR-89` section 5).
        // With no `--by` there is nothing asserted: the git configuration is
        // the next tier, as it is under `gh-verified`.
        let (name, source) = match explicit {
            Some(by) => (by.to_string(), IdentitySource::Asserted),
            None => (git_config_name(repo_root)?, IdentitySource::GitConfig),
        };
        return Ok(ResolvedIdentity {
            name,
            warning: None,
            source,
            replaced_by: None,
            verified_login: verified,
        });
    }

    if let Some(login) = verified {
        let replaced_by = explicit.filter(|by| *by != login).map(str::to_string);
        let warning = replaced_by.as_ref().map(|by| {
            format!(
                "--by '{by}' differs from the authenticated gh login '{login}' -- using '{login}'"
            )
        });
        return Ok(ResolvedIdentity {
            name: login,
            warning,
            source: IdentitySource::GhVerified,
            replaced_by,
            verified_login: None,
        });
    }

    if policy.require_verified {
        return Err(
            "attestation.require_verified is true and gh could not verify a login (`gh api user` failed or timed out); nothing was written. Log in with `gh auth login`, or set `attestation.identity: asserted` for a repository whose agents attest on behalf of a named person"
                .to_string(),
        );
    }

    if let Some(by) = explicit {
        return Ok(ResolvedIdentity {
            name: by.to_string(),
            warning: None,
            source: IdentitySource::Asserted,
            replaced_by: None,
            verified_login: None,
        });
    }

    Ok(ResolvedIdentity {
        name: git_config_name(repo_root)?,
        warning: None,
        source: IdentitySource::GitConfig,
        replaced_by: None,
        verified_login: None,
    })
}

fn git_config_name(repo_root: &Path) -> Result<String, String> {
    if let Ok(output) = Command::new("git")
        .args(["config", "user.name"])
        .current_dir(repo_root)
        .output()
    {
        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return Ok(name);
            }
        }
    }
    Err("could not resolve an identity (gh api user unavailable, git config user.name unset) -- pass --by explicitly".to_string())
}

/// `Command::output()` blocks indefinitely if `gh` hangs -- an
/// unauthenticated interactive prompt, a stalled network call. Bounded so a
/// hung `gh` falls through to the next identity tier instead of hanging
/// `fix --apply` forever (RFC-0002's undesigned timeout question).
fn gh_authenticated_login() -> Option<String> {
    let mut child = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let deadline = Instant::now() + GH_AUTH_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let mut stdout = String::new();
                child.stdout.take()?.read_to_string(&mut stdout).ok()?;
                let login = stdout.trim().to_string();
                return if login.is_empty() { None } else { Some(login) };
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return None,
        }
    }
}

/// A corpus-level lock for `urzua fix --apply` (RFC-0008 §4): apply is
/// single-process, and a second concurrent apply must fail loud rather than
/// interleave writes. `create_new` is atomic -- two processes racing to
/// create the same file, only one succeeds.
pub struct FixLock {
    path: PathBuf,
}

impl FixLock {
    pub fn acquire(repo_root: &Path) -> Result<Self, String> {
        let dir = repo_root.join(".urzua/cache");
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
        let path = dir.join("fix.lock");
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    format!(
                        "{} already exists -- another `urzua fix --apply` may be running; remove it if that's not the case",
                        path.display()
                    )
                } else {
                    format!("could not create {}: {e}", path.display())
                }
            })?;
        Ok(FixLock { path })
    }
}

impl Drop for FixLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// The one confinement check for every path the tool reads or writes on
/// behalf of a user-supplied or config-supplied name: resolve `path` (relative
/// paths are taken from `root`), following symlinks, and refuse anything that
/// lands outside `root`. A path that does not exist yet is resolved through its
/// deepest existing ancestor, so a new file below a symlinked directory that
/// points outside the repository is refused too. Returns the resolved path.
/// A dangling symlink is refused: writing through it would create its target,
/// wherever that is.
pub fn confine_to_root(root: &Path, path: &Path) -> Result<PathBuf, String> {
    use std::path::Component;
    let root_real = std::fs::canonicalize(root).map_err(|e| {
        format!(
            "could not resolve the repository root {}: {e}",
            root.display()
        )
    })?;
    let outside = || {
        format!(
            "{} resolves outside the repository; urzua will not read or write it",
            path.display()
        )
    };
    let full = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let mut existing = full.clone();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let real = loop {
        match std::fs::canonicalize(&existing) {
            Ok(real) => break real,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if std::fs::symlink_metadata(&existing).is_ok() {
                    return Err(format!(
                        "{} is a dangling symlink; urzua will not write through it",
                        existing.display()
                    ));
                }
                match (existing.components().next_back(), existing.parent()) {
                    (Some(Component::Normal(name)), Some(parent)) => {
                        tail.push(name.to_os_string());
                        existing = parent.to_path_buf();
                    }
                    _ => return Err(outside()),
                }
            }
            Err(e) => return Err(format!("could not resolve {}: {e}", path.display())),
        }
    };
    let mut resolved = real;
    resolved.extend(tail.iter().rev());
    if resolved.starts_with(&root_real) {
        Ok(resolved)
    } else {
        Err(outside())
    }
}

/// `write_atomic` for a file below the repository `root`: the destination is
/// checked with `confine_to_root` first, so nothing is written outside it.
pub fn write_atomic_in(root: &Path, path: &Path, text: &str) -> std::io::Result<()> {
    confine_to_root(root, path)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::PermissionDenied, e))?;
    write_atomic(path, text)
}

/// Replace `path` with `text` only once the whole text is on disk: written to a
/// temporary file in the same directory, then renamed over the target. A write
/// that fails (disk full) or a process killed part-way leaves the old file
/// intact instead of an empty or truncated record or configuration. An
/// existing file's permissions are kept; a symlink is written through, so the
/// link stays a link. A read-only file is refused, as a plain write would.
/// The temporary file is removed on failure.
pub fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    let target = match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => std::fs::canonicalize(path)?,
        _ => path.to_path_buf(),
    };
    let dir = match target.parent() {
        Some(d) if !d.as_os_str().is_empty() => d.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = dir.join(format!(".{name}.urzua-tmp-{}", std::process::id()));
    let result = (|| {
        // A file made read-only on purpose stays unwritable: rename would
        // replace it regardless of its own mode.
        if let Ok(meta) = std::fs::metadata(&target) {
            if meta.permissions().readonly() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "the file is read-only",
                ));
            }
        }
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        if let Ok(meta) = std::fs::metadata(&target) {
            std::fs::set_permissions(&temp, meta.permissions())?;
        }
        std::fs::rename(&temp, &target)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

/// The commit that last touched a specific 1-indexed line (RFC-0005 tier 1),
/// via `git blame`. Scoped to the line, not the whole file, so an unrelated
/// edit elsewhere in a record can't silently reset its own staleness clock
/// (ADR-0032). `Ok(None)` for anything blame can't resolve -- no history,
/// an untracked file -- never an error a caller has to specially handle.
pub fn commit_for_line(
    repo_root: &Path,
    path: &Path,
    line: usize,
) -> Result<Option<String>, String> {
    commit_for_line_in(repo_root, path, line, None)
}

/// As [`commit_for_line`], blaming `contents` (proposed text, not yet on
/// disk) instead of the working-tree file when given. A line the proposal
/// changes or adds is attributed to no commit: git reports an all-zero hash,
/// which the caller reads as "touched now" (`MILE-159`).
pub fn commit_for_line_in(
    repo_root: &Path,
    path: &Path,
    line: usize,
    contents: Option<&str>,
) -> Result<Option<String>, String> {
    let mut cmd = Command::new("git");
    cmd.args(["blame", "-L", &format!("{line},{line}"), "--porcelain"]);
    if contents.is_some() {
        cmd.args(["--contents", "-"]);
    }
    let output = run_blame(cmd, path, repo_root, contents)
        .map_err(|e| format!("could not run git blame on {}: {e}", path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // No history for the file or line (untracked, staged-new, unborn
        // HEAD) is the ordinary "nothing to date" answer; anything else is a
        // git failure the caller must be able to report (`MILE-159`).
        if is_no_history_stderr(&stderr) {
            return Ok(None);
        }
        return Err(format!(
            "git blame on {} failed: {}",
            path.display(),
            stderr.trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().next())
        .map(|s| s.to_string()))
}

/// One line's blame: its commit, none, or why git could not say.
type LineBlame = Result<Option<String>, String>;

/// The cache key: file, proposed contents (if blamed against them) and line.
type BlameKey = (PathBuf, Option<String>, usize);

/// One `git blame` per file and one `git show` per commit for a whole run,
/// instead of one process per line: a corpus of a thousand records costs as
/// many blame processes as it has files. Each line's answer is the one
/// [`commit_for_line_in`] gives for it. Make one per run, never reuse it after
/// a file has changed.
pub struct BlameCache {
    repo_root: PathBuf,
    lines: std::sync::Mutex<std::collections::HashMap<BlameKey, LineBlame>>,
    times: std::sync::Mutex<std::collections::HashMap<String, Option<i64>>>,
}

impl BlameCache {
    pub fn new(repo_root: &Path) -> Self {
        BlameCache {
            repo_root: repo_root.to_path_buf(),
            lines: Default::default(),
            times: Default::default(),
        }
    }

    /// Blames every not yet cached `(file, line)` of `work` (against `contents`
    /// when given): one `git blame` per file, with a `-L` range per wanted line
    /// (so git stops tracing a line once it is resolved), the files on a few
    /// threads. Later lookups of these lines spawn nothing.
    pub fn prefetch(&self, work: &[(PathBuf, usize)], contents: Option<&str>) {
        let key_contents = contents.map(str::to_string);
        let mut wanted: Vec<(&PathBuf, Vec<usize>)> = Vec::new();
        {
            let cached = self.lines.lock().unwrap_or_else(|e| e.into_inner());
            for (path, line) in work {
                if cached.contains_key(&(path.clone(), key_contents.clone(), *line)) {
                    continue;
                }
                match wanted.iter_mut().find(|(p, _)| *p == path) {
                    Some((_, lines)) if lines.contains(line) => {}
                    Some((_, lines)) => lines.push(*line),
                    None => wanted.push((path, vec![*line])),
                }
            }
        }
        if wanted.is_empty() {
            return;
        }
        let threads = std::thread::available_parallelism()
            .map_or(4, |n| n.get())
            .clamp(2, 8);
        let chunk = wanted.len().div_ceil(threads);
        let blamed: Vec<(PathBuf, Vec<(usize, LineBlame)>)> = std::thread::scope(|scope| {
            let handles: Vec<_> = wanted
                .chunks(chunk)
                .map(|part| {
                    scope.spawn(move || {
                        part.iter()
                            .map(|(p, lines)| {
                                (
                                    (*p).clone(),
                                    blame_lines(&self.repo_root, p, lines, contents),
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            wanted
                .chunks(chunk)
                .zip(handles)
                .flat_map(|(part, h)| blamed_or_each(&self.repo_root, part, contents, h.join()))
                .collect()
        });
        let mut cached = self.lines.lock().unwrap_or_else(|e| e.into_inner());
        for (path, answers) in blamed {
            for (line, answer) in answers {
                cached.insert((path.clone(), key_contents.clone(), line), answer);
            }
        }
    }

    /// As [`commit_for_line_in`], from the cache (blaming the line on a miss).
    pub fn commit_for_line(
        &self,
        path: &Path,
        line: usize,
        contents: Option<&str>,
    ) -> Result<Option<String>, String> {
        self.prefetch(&[(path.to_path_buf(), line)], contents);
        self.lines
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&(path.to_path_buf(), contents.map(str::to_string), line))
            .cloned()
            .unwrap_or(Ok(None))
    }

    /// [`commit_unix_time`], asked of git once per commit.
    pub fn commit_unix_time(&self, commit: &str) -> Option<i64> {
        if let Some(time) = self
            .times
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(commit)
        {
            return *time;
        }
        let time = commit_unix_time(&self.repo_root, commit);
        self.times
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(commit.to_string(), time);
        time
    }
}

/// A blame thread's answers, or -- when the thread panicked -- each of its lines
/// asked of git on its own, so an error surfaces exactly as
/// [`commit_for_line_in`] reports it rather than as "no commit".
fn blamed_or_each(
    repo_root: &Path,
    part: &[(&PathBuf, Vec<usize>)],
    contents: Option<&str>,
    joined: std::thread::Result<Vec<(PathBuf, Vec<(usize, LineBlame)>)>>,
) -> Vec<(PathBuf, Vec<(usize, LineBlame)>)> {
    joined.unwrap_or_else(|_| {
        part.iter()
            .map(|(p, lines)| {
                let each = lines
                    .iter()
                    .map(|l| (*l, commit_for_line_in(repo_root, p, *l, contents)))
                    .collect();
                ((*p).clone(), each)
            })
            .collect()
    })
}

/// `git blame --porcelain` of `lines` of `path` (of `contents` when given) in
/// one process. When that one process fails for a reason other than "no
/// history" -- a line outside the file, say -- each line is asked of git on its
/// own, so every line reports exactly what [`commit_for_line_in`] reports.
fn blame_lines(
    repo_root: &Path,
    path: &Path,
    lines: &[usize],
    contents: Option<&str>,
) -> Vec<(usize, LineBlame)> {
    let each = || {
        lines
            .iter()
            .map(|l| (*l, commit_for_line_in(repo_root, path, *l, contents)))
            .collect()
    };
    if lines.contains(&0) {
        return each();
    }
    let mut cmd = Command::new("git");
    cmd.args(["blame", "--porcelain"]);
    for line in lines {
        cmd.args(["-L", &format!("{line},{line}")]);
    }
    if contents.is_some() {
        cmd.args(["--contents", "-"]);
    }
    let Ok(output) = run_blame(cmd, path, repo_root, contents) else {
        return each();
    };
    if !output.status.success() {
        if is_no_history_stderr(&String::from_utf8_lossy(&output.stderr)) {
            return lines.iter().map(|l| (*l, Ok(None))).collect();
        }
        return each();
    }
    let commits = porcelain_line_commits(&String::from_utf8_lossy(&output.stdout));
    lines
        .iter()
        .map(|l| match commits.get(l) {
            Some(commit) => (*l, Ok(Some(commit.clone()))),
            None => (*l, commit_for_line_in(repo_root, path, *l, contents)),
        })
        .collect()
}

/// Runs a prepared `git blame` for `path`, feeding `contents` on stdin if given.
fn run_blame(
    mut cmd: Command,
    path: &Path,
    repo_root: &Path,
    contents: Option<&str>,
) -> std::io::Result<std::process::Output> {
    use std::io::Write;
    cmd.arg("--")
        .arg(path)
        .current_dir(repo_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(if contents.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
    let mut child = cmd.spawn()?;
    if let (Some(text), Some(mut stdin)) = (contents, child.stdin.take()) {
        // A broken pipe means git exited early (no history); its status says why.
        let _ = stdin.write_all(text.as_bytes());
    }
    child.wait_with_output()
}

/// The commit of each blamed line of `git blame --porcelain` output, by line.
/// A line's group header is `<commit> <original line> <final line> [<group size>]`;
/// the content lines start with a tab and the other headers with a word.
fn porcelain_line_commits(porcelain: &str) -> std::collections::HashMap<usize, String> {
    let mut commits = std::collections::HashMap::new();
    for line in porcelain.lines().filter(|l| !l.starts_with('\t')) {
        let mut parts = line.split_whitespace();
        let (Some(commit), Some(original), Some(final_line)) =
            (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let is_commit = commit.len() >= 40 && commit.bytes().all(|b| b.is_ascii_hexdigit());
        if let (true, Ok(_), Ok(n)) = (
            is_commit,
            original.parse::<usize>(),
            final_line.parse::<usize>(),
        ) {
            commits.insert(n, commit.to_string());
        }
    }
    commits
}

/// The commits that touched `path` after `since` (exclusive), newest first, as
/// `<short hash> <subject>`, at most 20: what `urzua reverify --pin-only` shows
/// for an unpinned locator that drifted since its `Realized-by` line was last
/// touched (`ADR-97` section 4). A git failure is an error, never "none".
pub fn commits_touching_since(
    repo_root: &Path,
    since: &str,
    path: &str,
) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args([
            "--literal-pathspecs",
            "log",
            "--max-count=20",
            "--format=%h %s",
            &format!("{since}..HEAD"),
            "--",
            path,
        ])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("could not run git log on {path}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git log on {path} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect())
}

/// The commit that last touched a whole file -- used for locator paths,
/// which (unlike a header field) aren't a single line (ADR-0032).
pub fn last_commit_for_path(repo_root: &Path, path: &Path) -> Result<Option<String>, String> {
    let output = Command::new("git")
        .args(["--literal-pathspecs", "log", "-1", "--format=%H", "--"])
        .arg(path)
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("could not run git log on {}: {e}", path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if is_no_history_stderr(&stderr) {
            return Ok(None);
        }
        return Err(format!(
            "git log on {} failed: {}",
            path.display(),
            stderr.trim()
        ));
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(if sha.is_empty() { None } else { Some(sha) })
}

/// The newest commit that last touched any of `paths` -- a directory or glob
/// locator's drift anchor (`BUG-176`). Paths are literal (never read as
/// pathspec magic) and are passed in batches, so a locator matching many files
/// does not exceed the argument limit; the newest of the batches' answers is
/// chosen by commit date.
pub fn last_commit_for_paths(
    repo_root: &Path,
    paths: &[PathBuf],
) -> Result<Option<String>, String> {
    let mut shas: Vec<String> = Vec::new();
    for batch in paths.chunks(400) {
        let output = Command::new("git")
            .args(["--literal-pathspecs", "log", "-1", "--format=%H", "--"])
            .args(batch)
            .current_dir(repo_root)
            .output()
            .map_err(|e| format!("could not run git log: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if is_no_history_stderr(&stderr) {
                return Ok(None);
            }
            return Err(format!("git log failed: {}", stderr.trim()));
        }
        let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !sha.is_empty() && !shas.contains(&sha) {
            shas.push(sha);
        }
    }
    if shas.len() <= 1 {
        return Ok(shas.pop());
    }
    let output = Command::new("git")
        .args(["log", "--no-walk=sorted", "-n", "1", "--format=%H"])
        .args(&shas)
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("could not run git log: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git log failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(if sha.is_empty() { None } else { Some(sha) })
}

/// True if `ancestor` came strictly before `descendant` in history.
/// `Ok(false)` when git says it is not an ancestor (exit 1) or the commits
/// are the same. Any other git outcome (a commit the repository does not
/// have, as in a shallow clone) is an `Err`, never a quiet "no drift"
/// (`MILE-159`).
pub fn commit_strictly_before(
    repo_root: &Path,
    ancestor: &str,
    descendant: &str,
) -> Result<bool, String> {
    if ancestor == descendant {
        return Ok(false);
    }
    let output = Command::new("git")
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("could not run git merge-base: {e}"))?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(format!(
            "git merge-base --is-ancestor {} {} failed: {}",
            &ancestor[..ancestor.len().min(12)],
            &descendant[..descendant.len().min(12)],
            String::from_utf8_lossy(&output.stderr).trim()
        )),
    }
}

/// Whether the repository is a shallow clone, which hides the history the
/// drift computation reads (`MILE-159`). `Err` when git cannot say.
pub fn is_shallow_repository(repo_root: &Path) -> Result<bool, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--is-shallow-repository"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("could not run git rev-parse: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git rev-parse --is-shallow-repository failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim() == "true")
}

/// git's wording for "this path or ref has no history here", which is an
/// answer, not a failure.
fn is_no_history_stderr(stderr: &str) -> bool {
    let s = stderr.to_ascii_lowercase();
    s.contains("no such path")
        || s.contains("does not have any commits")
        || s.contains("unknown revision")
        || s.contains("bad revision")
        || s.contains("ambiguous argument 'head'")
}

/// Committer time of `commit`, in seconds since the epoch (`MILE-129`).
/// `None` when git cannot say, so a grace window or claim age never fires on
/// a commit it could not date.
pub fn commit_unix_time(repo_root: &Path, commit: &str) -> Option<i64> {
    let output = Command::new("git")
        .args(["show", "-s", "--format=%ct", commit])
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

/// Seconds since the epoch at the evaluation date (`MILE-129`): now, or the
/// end of the `--as-of` day. Claim ages and audit grace measure to it.
pub fn now_unix() -> i64 {
    clock().unix
}

/// Lowercase hex SHA-256 of the file's bytes (`MILE-129`), or `None` when it
/// cannot be read (a missing locator is `embodiment.locator-exists`' finding,
/// not a hash mismatch).
pub fn sha256_hex_of_file(path: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).ok()?;
    let digest = Sha256::digest(bytes);
    Some(digest.iter().map(|b| format!("{b:02x}")).collect())
}

/// Lowercase hex SHA-256 of a string's UTF-8 bytes (`ADR-74`: the pinned
/// bytes of a scoped span).
pub fn sha256_hex_of_str(text: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn run_git(dir: &Path, args: &[&str]) -> Result<String, DiscoveryError> {
    let args_str = args.join(" ");
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|source| DiscoveryError::Spawn {
            args: args_str.clone(),
            dir: dir.to_path_buf(),
            source,
        })?;

    if !output.status.success() {
        return Err(DiscoveryError::NonZeroExit {
            args: args_str,
            dir: dir.to_path_buf(),
            status: output.status,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    String::from_utf8(output.stdout).map_err(|_| DiscoveryError::InvalidUtf8 { args: args_str })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    /// Serializes PATH-stubbing tests -- `std::env::set_var("PATH", ...)` is
    /// process-wide and Rust runs tests in parallel threads.
    static PATH_MUTEX: Mutex<()> = Mutex::new(());

    /// Runs only inside the child process `today_is_the_utc_date_in_every_time_zone`
    /// starts, where `TZ` is set before the process reads the clock.
    #[test]
    fn today_child() {
        if std::env::var_os("URZUA_TODAY_CHILD").is_none() {
            return;
        }
        let before = chrono::Utc::now().date_naive().to_string();
        let got = today();
        let after = chrono::Utc::now().date_naive().to_string();
        assert!(
            got == before || got == after,
            "TZ={:?}: today() gave {got}, the UTC date is {before}",
            std::env::var("TZ")
        );
    }

    /// `today()` is the UTC date whatever the machine's time zone is. Two zones
    /// 26 hours apart cannot both share the UTC date, so one of them fails a
    /// local-time implementation at any instant. Each zone runs in its own
    /// process, since the C library reads `TZ` once.
    #[test]
    fn today_is_the_utc_date_in_every_time_zone() {
        let exe = std::env::current_exe().unwrap();
        for zone in ["Pacific/Kiritimati", "Etc/GMT+12", "UTC"] {
            let out = Command::new(&exe)
                .args(["tests::today_child", "--exact", "--nocapture"])
                .env("TZ", zone)
                .env("URZUA_TODAY_CHILD", "1")
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "TZ={zone}: {}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }

    #[test]
    fn as_of_accepts_only_a_strict_calendar_date() {
        for ok in ["2026-11-08", "2024-02-29", "1999-12-31"] {
            assert!(parse_as_of(ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "tomorrow",
            "2026-1-5",
            "2026-13-01",
            "2026-02-30",
            "2025-02-29",
            "20261108",
            "2026-11-08T00:00:00Z",
            " 2026-11-08",
            "2026/11/08",
            "+026-11-08",
        ] {
            assert!(parse_as_of(bad).is_err(), "{bad:?}");
        }
    }

    fn init_repo(dir: &Path) {
        let run = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(dir)
                .output()
                .expect("git command should run")
        };
        run(&["init", "-q"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "test"]);
    }

    /// A fresh directory under the temp dir, removed on drop so a failing
    /// assertion cannot leave it behind (BUG-193).
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Scratch {
            static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("{label}-{}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Scratch(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    impl std::ops::Deref for Scratch {
        type Target = Path;
        fn deref(&self) -> &Path {
            &self.0
        }
    }

    impl AsRef<Path> for Scratch {
        fn as_ref(&self) -> &Path {
            &self.0
        }
    }

    #[test]
    fn discovers_only_tracked_and_staged_files_never_a_raw_walk() {
        let tmp = Scratch::new("urzua-io-test");
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp);

        fs::write(tmp.join("tracked.md"), "tracked").unwrap();
        Command::new("git")
            .args(["add", "tracked.md"])
            .current_dir(&tmp)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-q", "-m", "init"])
            .current_dir(&tmp)
            .output()
            .unwrap();

        // The untracked scratch file is the planted violation: a raw
        // filesystem walk would find it, and the check must not.
        fs::write(tmp.join("scratch.md"), "untracked scratch file").unwrap();

        let result = discover_tracked_files(&tmp, None).expect("discovery should succeed");
        let names: Vec<_> = result
            .paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        assert!(names.contains(&"tracked.md".to_string()));
        assert!(
            !names.contains(&"scratch.md".to_string()),
            "untracked scratch file must never be discovered: {names:?}"
        );
    }

    /// `BUG-147`'s own regression: merging the staged/deleted discovery into
    /// one `--name-status` call has to handle a staged rename correctly --
    /// `git diff --name-status` reports it as `R100\0old\0new\0`, three
    /// fields, not the two every other status line carries. A naive
    /// two-fields-per-record parser would misalign on exactly this input.
    #[test]
    fn a_staged_rename_is_discovered_under_its_new_name_only_observed_failing() {
        let tmp = Scratch::new("urzua-io-rename-test");
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp);
        // Pin rename detection locally: a developer's global `diff.renames=false`
        // would otherwise report this move as `D`+`A` and fail the test.
        Command::new("git")
            .args(["config", "diff.renames", "true"])
            .current_dir(&tmp)
            .output()
            .unwrap();

        // Identical, substantial content: git's default rename detection
        // needs enough similarity to classify this as `R`, not a plain `D`
        // followed by an unrelated `A`.
        let body = "line one\nline two\nline three\nline four\nline five\n".repeat(4);
        fs::write(tmp.join("old-name.md"), &body).unwrap();
        Command::new("git")
            .args(["add", "old-name.md"])
            .current_dir(&tmp)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-q", "-m", "init"])
            .current_dir(&tmp)
            .output()
            .unwrap();

        Command::new("git")
            .args(["mv", "old-name.md", "new-name.md"])
            .current_dir(&tmp)
            .output()
            .unwrap();

        let result = discover_tracked_files(&tmp, None).expect("discovery should succeed");
        let names: Vec<_> = result
            .paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        assert!(
            names.contains(&"new-name.md".to_string()),
            "the renamed file's new name must be discovered: {names:?}"
        );
        assert!(
            !names.contains(&"old-name.md".to_string()),
            "a rename's old name must not linger as a phantom entry: {names:?}"
        );
        assert!(
            !result
                .staged_deletions
                .contains(&PathBuf::from("old-name.md")),
            "a rename is not a deletion -- git classifies it R, not D: {:?}",
            result.staged_deletions
        );
    }

    /// `ADR-92`: with an inclusion, discovery adds the untracked, non-ignored
    /// files directly under the declared directories that the caller's rule
    /// governs, and nothing else; without one, none.
    #[test]
    fn an_inclusion_adds_only_untracked_non_ignored_files_the_rule_governs() {
        let tmp = Scratch::new("urzua-io-include");
        fs::create_dir_all(tmp.0.join("docs/adr/deeper")).unwrap();
        fs::create_dir_all(tmp.0.join("notes")).unwrap();
        init_repo(&tmp);
        fs::write(tmp.join(".gitignore"), "docs/adr/ignored.md\n").unwrap();
        fs::write(tmp.join("docs/adr/tracked.md"), "t").unwrap();
        for args in [&["add", "-A"][..], &["commit", "-q", "-m", "init"][..]] {
            Command::new("git")
                .args(args)
                .current_dir(&tmp)
                .output()
                .unwrap();
        }
        for p in [
            "docs/adr/draft.md",
            "docs/adr/ignored.md",
            "docs/adr/scratch.txt",
            "docs/adr/deeper/deep.md",
            "notes/outside.md",
        ] {
            fs::write(tmp.join(p), "x").unwrap();
        }
        let dirs = vec![PathBuf::from("docs/adr")];
        let governs = |p: &Path| p.extension().is_some_and(|e| e == "md");
        let inclusion = UntrackedInclusion {
            dirs: &dirs,
            governs: &governs,
        };

        let with = discover_tracked_files(&tmp, Some(&inclusion)).unwrap();
        let names: Vec<String> = with.paths.iter().map(|p| p.display().to_string()).collect();
        assert!(
            names.contains(&"docs/adr/draft.md".to_string()),
            "{names:?}"
        );
        assert!(
            names.contains(&"docs/adr/tracked.md".to_string()),
            "{names:?}"
        );
        for not_included in [
            "docs/adr/ignored.md",
            "docs/adr/scratch.txt",
            "docs/adr/deeper/deep.md",
            "notes/outside.md",
        ] {
            assert!(
                !names.contains(&not_included.to_string()),
                "{not_included}: {names:?}"
            );
        }
        assert_eq!(
            with.untracked_included,
            std::collections::HashSet::from([PathBuf::from("docs/adr/draft.md")])
        );

        let without = discover_tracked_files(&tmp, None).unwrap();
        assert!(without.untracked_included.is_empty());
        assert!(!without.paths.contains(&PathBuf::from("docs/adr/draft.md")));
    }

    #[test]
    fn untracked_files_count_as_added_in_a_change_set() {
        let mut changes = GitChanges {
            merge_base: "abc".to_string(),
            changed: std::collections::HashSet::new(),
            deleted: std::collections::HashSet::new(),
            added: std::collections::HashSet::new(),
        };
        let draft = PathBuf::from("docs/adr/draft.md");
        changes.add_untracked([&draft]);
        assert!(changes.changed.contains(&draft) && changes.added.contains(&draft));
        assert!(!changes.deleted.contains(&draft));
    }

    #[test]
    fn a_missing_git_repo_is_a_reported_error_not_a_silent_empty_list() {
        let tmp = Scratch::new("urzua-io-notgit");
        fs::create_dir_all(&tmp).unwrap();

        let result = discover_tracked_files(&tmp, None);
        assert!(
            result.is_err(),
            "a directory with no git repo must error, not return an empty list silently"
        );
    }

    /// A stub `gh` ahead of the real PATH, always exiting non-zero -- lets
    /// `resolve_identity`'s gh-unavailable fallback be tested deterministically
    /// without depending on whether this machine actually has `gh` authenticated.
    fn prepend_failing_gh_to_path() -> (Scratch, String) {
        prepend_stub_gh_to_path("exit 1\n")
    }

    /// A stub `gh` ahead of the real PATH that succeeds and reports `login`
    /// -- lets the gh-wins-even-over-explicit-`--by` tier be tested
    /// deterministically, the mirror of `prepend_failing_gh_to_path`.
    fn prepend_successful_gh_to_path(login: &str) -> (Scratch, String) {
        prepend_stub_gh_to_path(&format!("echo '{login}'\nexit 0\n"))
    }

    fn prepend_stub_gh_to_path(script_body: &str) -> (Scratch, String) {
        let stub_dir = Scratch::new("urzua-io-fake-gh");
        let stub_gh = stub_dir.join("gh");
        fs::write(&stub_gh, format!("#!/bin/sh\n{script_body}")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&stub_gh, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", stub_dir.display(), original_path));
        (stub_dir, original_path)
    }

    #[test]
    fn resolve_identity_falls_through_to_git_config_when_gh_is_unavailable() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_failing_gh_to_path();

        let tmp = Scratch::new("urzua-io-identity");
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp); // sets local user.name to "test"

        let result = resolve_identity(None, &tmp);

        std::env::set_var("PATH", original_path);

        let result = result.unwrap();
        assert_eq!(
            result.name, "test",
            "with gh unavailable, git config user.name must win over nothing"
        );
        assert!(result.warning.is_none());
    }

    /// gh unavailable, git config unset, explicit --by given -> --by
    /// resolves. Doesn't cover --by vs. a present git config -- see
    /// `resolve_identity_prefers_explicit_by_over_git_config`.
    #[test]
    fn resolve_identity_never_lets_explicit_by_override_a_verified_gh_login() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_failing_gh_to_path();

        let tmp = Scratch::new("urzua-io-identity-by");
        fs::create_dir_all(&tmp).unwrap();
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&tmp)
            .output()
            .unwrap();
        // Isolates from this machine's real global git config.
        std::env::set_var("GIT_CONFIG_GLOBAL", "/dev/null");
        std::env::set_var("GIT_CONFIG_SYSTEM", "/dev/null");

        let result = resolve_identity(Some("explicit-name"), &tmp);

        std::env::remove_var("GIT_CONFIG_GLOBAL");
        std::env::remove_var("GIT_CONFIG_SYSTEM");
        std::env::set_var("PATH", original_path);

        let result = result.unwrap();
        assert_eq!(result.name, "explicit-name");
        assert!(result.warning.is_none());
    }

    #[test]
    fn resolve_identity_prefers_explicit_by_over_git_config() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_failing_gh_to_path();

        let tmp = Scratch::new("urzua-io-identity-by2");
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp); // sets local user.name to "test"

        let result = resolve_identity(Some("explicit-name"), &tmp);

        std::env::set_var("PATH", original_path);

        let result = result.unwrap();
        assert_eq!(result.name, "explicit-name");
        assert!(result.warning.is_none());
    }

    #[test]
    fn write_atomic_replaces_the_file_and_leaves_no_temporary_behind() {
        let tmp = Scratch::new("urzua-io-atomic");
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("record.md");
        fs::write(&path, "old\n").unwrap();
        write_atomic(&path, "new\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");
        let names: Vec<_> = fs::read_dir(&*tmp).unwrap().collect();
        assert_eq!(names.len(), 1, "only the record remains: {names:?}");
    }

    #[test]
    fn write_atomic_failing_part_way_keeps_the_old_file() {
        let tmp = Scratch::new("urzua-io-atomic-fail");
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("record.md");
        fs::write(&path, "old\n").unwrap();
        // The temporary name is taken by a directory, so creating it fails
        // before anything is renamed: the target must be untouched.
        let temp = tmp.join(format!(".record.md.urzua-tmp-{}", std::process::id()));
        fs::create_dir(&temp).unwrap();
        assert!(write_atomic(&path, "new\n").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "old\n");
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_keeps_the_mode_and_writes_through_a_symlink() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = Scratch::new("urzua-io-atomic-link");
        fs::create_dir_all(&tmp).unwrap();
        let real = tmp.join("real.yaml");
        fs::write(&real, "a: 1\n").unwrap();
        fs::set_permissions(&real, fs::Permissions::from_mode(0o640)).unwrap();
        let link = tmp.join("link.yaml");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        write_atomic(&link, "a: 2\n").unwrap();
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_to_string(&real).unwrap(), "a: 2\n");
        assert_eq!(
            fs::metadata(&real).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[cfg(unix)]
    #[test]
    fn confine_to_root_refuses_paths_that_resolve_outside() {
        let tmp = Scratch::new("urzua-io-confine");
        let repo = tmp.join("repo");
        let outside = tmp.join("elsewhere");
        fs::create_dir_all(repo.join("docs")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.txt"), "s\n").unwrap();
        std::os::unix::fs::symlink(&outside, repo.join("out")).unwrap();
        std::os::unix::fs::symlink(outside.join("secret.txt"), repo.join("leak.txt")).unwrap();
        std::os::unix::fs::symlink(outside.join("nope.txt"), repo.join("dangling.txt")).unwrap();

        assert!(confine_to_root(&repo, Path::new("docs/new/deep.md")).is_ok());
        assert!(confine_to_root(&repo, &repo.join("docs/x.md")).is_ok());
        for bad in [
            "out/idx.md",
            "leak.txt",
            "dangling.txt",
            "../elsewhere/secret.txt",
            "docs/../../elsewhere/x",
            "docs/missing/../../../x",
        ] {
            assert!(
                confine_to_root(&repo, Path::new(bad)).is_err(),
                "{bad} should be refused"
            );
        }
        assert!(confine_to_root(&repo, &outside.join("secret.txt")).is_err());
        assert!(write_atomic_in(&repo, Path::new("out/idx.md"), "x\n").is_err());
        assert!(!outside.join("idx.md").exists());
    }

    #[test]
    fn resolve_identity_refuses_a_name_that_would_break_a_table_row() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_failing_gh_to_path();

        let tmp = Scratch::new("urzua-io-identity-pipe");
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp);

        let pipe = resolve_identity(Some("a | b"), &tmp);
        let newline = resolve_identity(Some("a\nb"), &tmp);
        // A git user.name is checked too, not only --by.
        Command::new("git")
            .args(["config", "user.name", "x | y"])
            .current_dir(&tmp)
            .output()
            .unwrap();
        let from_git = resolve_identity(None, &tmp);

        std::env::set_var("PATH", original_path);

        assert!(pipe.unwrap_err().contains("--by"));
        assert!(newline.is_err());
        assert!(from_git.unwrap_err().contains("resolved identity"));
    }

    #[test]
    fn resolve_identity_lets_gh_win_over_a_differing_explicit_by() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_successful_gh_to_path("gh-login");

        let tmp = Scratch::new("urzua-io-identity-gh");
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp);

        let result = resolve_identity(Some("explicit-name"), &tmp);

        std::env::set_var("PATH", original_path);

        let result = result.unwrap();
        assert_eq!(result.name, "gh-login");
        assert!(result.warning.unwrap().contains("explicit-name"));
    }

    fn quiet_repo(prefix: &str, with_git_name: bool) -> Scratch {
        let tmp = Scratch::new(prefix);
        fs::create_dir_all(&tmp).unwrap();
        if with_git_name {
            init_repo(&tmp); // sets local user.name to "test"
        } else {
            Command::new("git")
                .args(["init", "-q"])
                .current_dir(&tmp)
                .output()
                .unwrap();
        }
        tmp
    }

    const GH_VERIFIED: IdentityPolicy = IdentityPolicy {
        asserted: false,
        require_verified: false,
    };
    const GH_REQUIRED: IdentityPolicy = IdentityPolicy {
        asserted: false,
        require_verified: true,
    };
    const ASSERTED: IdentityPolicy = IdentityPolicy {
        asserted: true,
        require_verified: false,
    };

    #[test]
    fn gh_verified_names_the_replaced_by_as_a_field_and_the_source() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_successful_gh_to_path("gh-login");
        let tmp = quiet_repo("urzua-io-id-replaced", true);

        let result = resolve_identity_with(Some("typed"), &tmp, GH_VERIFIED);
        let agree = resolve_identity_with(Some("gh-login"), &tmp, GH_VERIFIED);

        std::env::set_var("PATH", original_path);

        let result = result.unwrap();
        assert_eq!(result.name, "gh-login");
        assert_eq!(result.source, IdentitySource::GhVerified);
        assert_eq!(result.replaced_by.as_deref(), Some("typed"));
        assert!(result.warning.unwrap().contains("typed"));
        let agree = agree.unwrap();
        assert_eq!(agree.source, IdentitySource::GhVerified);
        assert!(agree.replaced_by.is_none() && agree.warning.is_none());
    }

    #[test]
    fn gh_verified_without_a_gh_answer_records_by_as_asserted_and_git_as_git_config() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_failing_gh_to_path();
        let tmp = quiet_repo("urzua-io-id-fallback", true);

        let typed = resolve_identity_with(Some("typed"), &tmp, GH_VERIFIED);
        let git = resolve_identity_with(None, &tmp, GH_VERIFIED);

        std::env::set_var("PATH", original_path);

        assert_eq!(typed.unwrap().source, IdentitySource::Asserted);
        let git = git.unwrap();
        assert_eq!(
            (git.name.as_str(), git.source),
            ("test", IdentitySource::GitConfig)
        );
    }

    #[test]
    fn require_verified_refuses_every_fallback_when_gh_does_not_answer() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_failing_gh_to_path();
        let tmp = quiet_repo("urzua-io-id-required", true);

        let with_by = resolve_identity_with(Some("typed"), &tmp, GH_REQUIRED);
        let with_git = resolve_identity_with(None, &tmp, GH_REQUIRED);

        std::env::set_var("PATH", original_path);

        for err in [with_by.unwrap_err(), with_git.unwrap_err()] {
            assert!(
                err.contains("require_verified") && err.contains("gh"),
                "{err}"
            );
        }
    }

    #[test]
    fn require_verified_accepts_a_verified_login() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_successful_gh_to_path("gh-login");
        let tmp = quiet_repo("urzua-io-id-required-ok", true);

        let result = resolve_identity_with(None, &tmp, GH_REQUIRED);

        std::env::set_var("PATH", original_path);

        assert_eq!(result.unwrap().source, IdentitySource::GhVerified);
    }

    #[test]
    fn asserted_takes_by_as_given_with_the_verified_login_beside_it() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_successful_gh_to_path("gh-login");
        let tmp = quiet_repo("urzua-io-id-asserted", true);

        let result = resolve_identity_with(Some("typed"), &tmp, ASSERTED);

        std::env::set_var("PATH", original_path);

        let result = result.unwrap();
        assert_eq!(result.name, "typed");
        assert_eq!(result.source, IdentitySource::Asserted);
        assert_eq!(result.verified_login.as_deref(), Some("gh-login"));
        assert!(result.replaced_by.is_none());
        assert!(result.warning.is_none());
    }

    #[test]
    fn asserted_without_a_gh_answer_has_no_verified_login_and_without_by_uses_git_config() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_failing_gh_to_path();
        let tmp = quiet_repo("urzua-io-id-asserted-nogh", true);

        let typed = resolve_identity_with(Some("typed"), &tmp, ASSERTED);
        let git = resolve_identity_with(None, &tmp, ASSERTED);

        std::env::set_var("PATH", original_path);

        let typed = typed.unwrap();
        assert!(typed.verified_login.is_none());
        assert_eq!(typed.source, IdentitySource::Asserted);
        assert_eq!(git.unwrap().source, IdentitySource::GitConfig);
    }

    #[test]
    fn a_verified_login_that_would_break_a_row_is_refused_beside_asserted_too() {
        let _guard = PATH_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_stub_dir, original_path) = prepend_successful_gh_to_path("a|b");
        let tmp = quiet_repo("urzua-io-id-asserted-pipe", true);

        let result = resolve_identity_with(Some("typed"), &tmp, ASSERTED);

        std::env::set_var("PATH", original_path);

        assert!(result.is_err());
    }

    fn commit_all(dir: &Path, message: &str) -> String {
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-q", "-m", message])
            .current_dir(dir)
            .output()
            .unwrap();
        String::from_utf8(
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(dir)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string()
    }

    #[test]
    fn commit_strictly_before_is_true_only_in_the_real_commit_order() {
        let tmp = Scratch::new("urzua-io-order");
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp);

        fs::write(tmp.join("a.txt"), "1").unwrap();
        let first = commit_all(&tmp, "first");
        fs::write(tmp.join("a.txt"), "2").unwrap();
        let second = commit_all(&tmp, "second");

        assert!(
            commit_strictly_before(&tmp, &first, &second).unwrap(),
            "first really did happen before second"
        );
        assert!(
            !commit_strictly_before(&tmp, &second, &first).unwrap(),
            "second did not happen before first"
        );
        assert!(
            !commit_strictly_before(&tmp, &first, &first).unwrap(),
            "a commit is never strictly before itself"
        );
        // A commit the repository does not have is a git failure, not
        // "no drift" (`MILE-159`).
        let missing = "0123456789abcdef0123456789abcdef01234567";
        assert!(
            commit_strictly_before(&tmp, missing, &second).is_err(),
            "an unknown commit must surface as an error"
        );
        assert!(!is_shallow_repository(&tmp).unwrap());
    }

    #[test]
    fn last_commit_for_path_tracks_the_most_recent_touch() {
        let tmp = Scratch::new("urzua-io-lastcommit");
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp);

        fs::write(tmp.join("a.txt"), "1").unwrap();
        fs::write(tmp.join("b.txt"), "1").unwrap();
        let first = commit_all(&tmp, "add both");
        fs::write(tmp.join("a.txt"), "2").unwrap();
        let second = commit_all(&tmp, "touch a only");

        assert_eq!(
            last_commit_for_path(&tmp, &tmp.join("a.txt")).unwrap(),
            Some(second)
        );
        assert_eq!(
            last_commit_for_path(&tmp, &tmp.join("b.txt")).unwrap(),
            Some(first)
        );
    }

    #[test]
    fn last_commit_for_path_reads_glob_characters_literally() {
        let tmp = Scratch::new("urzua-io-lastcommit-literal");
        fs::create_dir_all(tmp.join("app/[id]")).unwrap();
        fs::create_dir_all(tmp.join("app/i")).unwrap();
        init_repo(&tmp);

        fs::write(tmp.join("app/[id]/page.tsx"), "1").unwrap();
        let literal = commit_all(&tmp, "add the bracketed page");
        fs::write(tmp.join("app/i/page.tsx"), "1").unwrap();
        commit_all(&tmp, "add a page the bracket class would match");

        assert_eq!(
            last_commit_for_path(&tmp, Path::new("app/[id]/page.tsx")).unwrap(),
            Some(literal)
        );
    }

    #[test]
    fn commit_for_line_is_scoped_to_the_line_not_the_whole_file() {
        let tmp = Scratch::new("urzua-io-blame");
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp);

        fs::write(tmp.join("doc.md"), "line one\nline two\n").unwrap();
        let first = commit_all(&tmp, "add both lines");
        fs::write(tmp.join("doc.md"), "line one\nline two CHANGED\n").unwrap();
        let second = commit_all(&tmp, "change line two only");

        assert_eq!(
            commit_for_line(&tmp, &tmp.join("doc.md"), 1).unwrap(),
            Some(first),
            "line 1 was never touched by the second commit"
        );
        assert_eq!(
            commit_for_line(&tmp, &tmp.join("doc.md"), 2).unwrap(),
            Some(second),
            "line 2 was changed by the second commit"
        );
    }

    #[test]
    fn blame_cache_answers_every_line_as_commit_for_line_in_does() {
        let tmp = Scratch::new("urzua-io-blame-cache");
        fs::create_dir_all(&tmp).unwrap();
        init_repo(&tmp);
        fs::write(tmp.join("doc.md"), "one\ntwo\nthree\n").unwrap();
        commit_all(&tmp, "add three lines");
        fs::write(tmp.join("doc.md"), "one\ntwo CHANGED\nthree\n").unwrap();
        commit_all(&tmp, "change line two");
        fs::write(tmp.join("untracked.md"), "x\n").unwrap();

        let cache = BlameCache::new(&tmp);
        let doc = tmp.join("doc.md");
        let work: Vec<(PathBuf, usize)> = [1, 2, 3, 0, 4]
            .iter()
            .map(|l| (PathBuf::from("doc.md"), *l))
            .chain([(PathBuf::from("untracked.md"), 1)])
            .collect();
        cache.prefetch(&work, None);
        for (path, line) in &work {
            assert_eq!(
                cache.commit_for_line(path, *line, None),
                commit_for_line_in(&tmp, path, *line, None),
                "{} line {line}",
                path.display()
            );
        }
        assert!(cache.commit_for_line(&doc, 2, None).unwrap().is_some());
        // Proposed contents are a separate answer from the working tree's.
        let proposed = "one\ntwo EDITED AGAIN\nthree\n";
        let work = [(PathBuf::from("doc.md"), 2)];
        cache.prefetch(&work, Some(proposed));
        assert_eq!(
            cache.commit_for_line(Path::new("doc.md"), 2, Some(proposed)),
            commit_for_line_in(&tmp, Path::new("doc.md"), 2, Some(proposed))
        );
    }

    #[test]
    fn a_panicked_blame_thread_falls_back_to_asking_git_per_line() {
        let tmp = Scratch::new("urzua-io-blame-panic");
        init_repo(&tmp);
        fs::write(tmp.join("doc.md"), "one\ntwo\n").unwrap();
        let commit = commit_all(&tmp, "add two lines");

        let path = PathBuf::from("doc.md");
        let part = [(&path, vec![1, 2, 9])];
        let panicked = std::thread::spawn(|| -> Vec<(PathBuf, Vec<(usize, LineBlame)>)> {
            panic!("simulated blame thread panic")
        })
        .join();
        assert!(panicked.is_err());
        let answers = blamed_or_each(&tmp, &part, None, panicked);
        assert_eq!(answers.len(), 1);
        let lines = &answers[0].1;
        assert_eq!(lines[0].1, Ok(Some(commit.clone())));
        assert_eq!(lines[1].1, Ok(Some(commit)));
        // A line past the end is the error commit_for_line_in gives, not Ok(None).
        assert_eq!(lines[2].1, commit_for_line_in(&tmp, &path, 9, None));
        assert!(lines[2].1.is_err());
    }

    /// Every line of `path` blamed in one batch answers as `commit_for_line_in`.
    fn assert_batch_matches_single(repo: &Path, path: &str, lines: usize) {
        let cache = BlameCache::new(repo);
        let work: Vec<(PathBuf, usize)> = (1..=lines).map(|l| (PathBuf::from(path), l)).collect();
        cache.prefetch(&work, None);
        for (p, line) in &work {
            let single = commit_for_line_in(repo, p, *line, None);
            assert!(
                matches!(single, Ok(Some(_))),
                "{path} line {line}: {single:?}"
            );
            assert_eq!(
                cache.commit_for_line(p, *line, None),
                single,
                "{path} line {line}"
            );
        }
    }

    #[test]
    fn batched_blame_matches_single_line_blame_across_a_rename() {
        let tmp = Scratch::new("urzua-io-blame-rename");
        init_repo(&tmp);
        fs::write(tmp.join("old.md"), "a\nb\nc\n").unwrap();
        commit_all(&tmp, "add old");
        fs::rename(tmp.join("old.md"), tmp.join("new.md")).unwrap();
        fs::write(tmp.join("new.md"), "a\nb CHANGED\nc\nd\n").unwrap();
        commit_all(&tmp, "rename and edit");
        assert_batch_matches_single(&tmp, "new.md", 4);
    }

    #[test]
    fn batched_blame_matches_single_line_blame_for_crlf_lines() {
        let tmp = Scratch::new("urzua-io-blame-crlf");
        init_repo(&tmp);
        fs::write(tmp.join("crlf.md"), "a\r\nb\r\nc\r\n").unwrap();
        commit_all(&tmp, "add crlf");
        fs::write(tmp.join("crlf.md"), "a\r\nb2\r\nc\r\n").unwrap();
        commit_all(&tmp, "edit b");
        assert_batch_matches_single(&tmp, "crlf.md", 3);
    }

    #[test]
    fn batched_blame_matches_single_line_blame_in_a_shallow_clone() {
        let tmp = Scratch::new("urzua-io-blame-shallow");
        init_repo(&tmp);
        fs::write(tmp.join("doc.md"), "a\nb\nc\n").unwrap();
        commit_all(&tmp, "first");
        fs::write(tmp.join("doc.md"), "a\nb2\nc\n").unwrap();
        commit_all(&tmp, "second");
        let clone = Scratch::new("urzua-io-blame-shallow-clone");
        let status = Command::new("git")
            .args(["clone", "--quiet", "--depth", "1"])
            .arg(format!("file://{}", tmp.display()))
            .arg(&*clone)
            .status()
            .unwrap();
        assert!(status.success());
        // Every line is a boundary commit in the shallow history.
        assert_batch_matches_single(&clone, "doc.md", 3);
    }
}
