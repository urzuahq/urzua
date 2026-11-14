//! `urzua migrate filenames` (MILE-134, SPEC-14): the plan for renaming a corpus's
//! record files to the filename convention their type declares, and for rewriting the
//! citations of the renamed paths.
//!
//! Pure: the caller supplies the records, the paths that exist and the texts to scan, and
//! gets back what to rename, what to rewrite and what to report. It writes nothing. A
//! rename keeps the directory, the number and the slug, so a record's `Stable-Id` and
//! record id are unchanged and a diff pairs the old and new file as one record (ADR-89).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use crate::config::Config;
use crate::document_model::CompiledIdentity;
use crate::record::Record;

/// One record file to rename.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Rename {
    pub record_type: String,
    pub from: PathBuf,
    pub to: PathBuf,
    /// Whether the record carries a `Stable-Id` for a diff to pair the rename by.
    #[serde(skip)]
    pub has_stable_id: bool,
    /// The type prefix and number the new name carries: the record id it takes.
    #[serde(skip)]
    pub id: (String, u64),
}

/// Why a record file is left as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkipReason {
    /// The name has no leading number to carry into the new name.
    NoNumber,
    /// The number does not fit what an identity reads.
    NumberTooLarge,
    /// The declared pattern accepts no name built from this one.
    PatternRejects,
    /// The new name is already a file, or another rename's new name.
    TargetExists,
    /// The number is already another record's of the type, or two renames would take it:
    /// the files would share one record id, so none of them is renamed.
    NumberCollision,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Skipped {
    pub from: PathBuf,
    pub reason: SkipReason,
    pub message: String,
}

/// One citation of a renamed file, as it reads and as it will read.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Citation {
    pub file: PathBuf,
    pub line: usize,
    pub from: String,
    pub to: String,
}

/// A citation in a file that is not rewritten.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Unrewritten {
    pub file: PathBuf,
    pub line: usize,
    pub reference: String,
    pub renamed_to: PathBuf,
}

/// An evidence pin (`path@sha256:...`) on a file whose text this run changes, so the
/// recorded hash no longer matches it: `urzua fix --rebaseline` re-pins it, with a reason
/// a person attests.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct StalePin {
    /// The file holding the pin, by its path now.
    pub file: PathBuf,
    pub line: usize,
    pub locator: String,
    /// The pinned file whose text changes.
    pub changed_file: PathBuf,
}

#[derive(Debug, Default)]
pub struct FilenamePlan {
    pub stale_pins: Vec<StalePin>,
    pub renames: Vec<Rename>,
    pub skipped: Vec<Skipped>,
    /// Every rewritten citation, in file then line order.
    pub rewrites: Vec<Citation>,
    /// The new text of each file whose citations change, keyed by its current path.
    pub new_texts: BTreeMap<PathBuf, String>,
    pub unrewritten: Vec<Unrewritten>,
}

/// The widest zero-padding tried when fitting a declared pattern.
const MAX_PADDING: usize = 12;

/// The name (with `.md`) a declared pattern, or the `TYPE-NNNN-slug.md` convention
/// when none is declared, reads back as this prefix and the number the file already
/// carries. The leading digits of the old name are that number; padding is kept as
/// written when the pattern accepts it and widened to the narrowest width it does.
pub fn target_name(
    identity: Option<&CompiledIdentity>,
    prefix: &str,
    file_name: &str,
) -> Result<String, (SkipReason, String)> {
    let stem = crate::record::strip_md_extension(file_name).unwrap_or(file_name);
    // A name that already carries the prefix (`ADR-5-x.md`, mispadded for the pattern) has
    // its number after it.
    let stem = stem
        .strip_prefix(prefix)
        .and_then(|r| r.strip_prefix('-'))
        .filter(|r| r.starts_with(|c: char| c.is_ascii_digit()))
        .unwrap_or(stem);
    let digits = stem.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        return Err((
            SkipReason::NoNumber,
            format!("'{file_name}' has no leading number to carry; rename it by hand"),
        ));
    }
    let (written, rest) = stem.split_at(digits);
    let rest = rest.trim_start_matches(['-', '_', ' ']);
    let value: u64 = written.parse().map_err(|_| {
        (
            SkipReason::NumberTooLarge,
            format!("'{written}' is too large to be a record number"),
        )
    })?;
    let tail = if rest.is_empty() {
        String::new()
    } else {
        format!("-{rest}")
    };
    let Some(declared) = identity else {
        return Ok(format!("{prefix}-{written}{tail}.md"));
    };
    // The written padding first, then wider, then narrower: only the zeros change, never
    // the number's value.
    let widths = (written.len()..=MAX_PADDING).chain(1..written.len());
    for width in widths {
        let name = format!("{prefix}-{value:0width$}{tail}.md");
        let found = declared.identify(&name);
        if found.title_prefix.as_deref() == Some(prefix)
            && found.number.as_deref().and_then(|n| n.parse::<u64>().ok()) == Some(value)
        {
            return Ok(name);
        }
    }
    Err((
        SkipReason::PatternRejects,
        format!(
            "the declared identity pattern reads no '{prefix}-<{value} at any zero-padding>{tail}.md' name back as prefix '{prefix}' and number {value}"
        ),
    ))
}

/// The inputs of [`plan`].
pub struct PlanInput<'a> {
    pub records: &'a [Record],
    pub config: &'a Config,
    /// Every path that exists in the corpus: a new name that is one of these is a conflict.
    pub existing: &'a HashSet<PathBuf>,
    /// Texts whose citations are rewritten (the records, and other markdown files).
    pub rewritable: &'a HashMap<PathBuf, String>,
    /// Texts whose citations are only reported.
    pub report_only: &'a HashMap<PathBuf, String>,
}

pub fn plan(input: &PlanInput) -> FilenamePlan {
    let mut out = FilenamePlan::default();
    let mut claimed: HashSet<PathBuf> = HashSet::new();
    let mut by_path: Vec<&Record> = input
        .records
        .iter()
        .filter(|r| crate::rules::record_id(r).is_none())
        .collect();
    by_path.sort_by(|a, b| a.path.cmp(&b.path));
    for record in by_path {
        plan_one(input, record, &mut claimed, &mut out);
    }
    skip_number_collisions(input, &mut out);
    let moved: HashMap<PathBuf, PathBuf> = out
        .renames
        .iter()
        .map(|r| (r.from.clone(), r.to.clone()))
        .collect();
    if moved.is_empty() {
        return out;
    }
    let mut files: Vec<&PathBuf> = input.rewritable.keys().collect();
    files.sort();
    for file in files {
        let (text, citations) = rewrite_citations(file, &input.rewritable[file], &moved);
        if !citations.is_empty() {
            out.new_texts.insert(file.clone(), text);
            out.rewrites.extend(citations.into_iter().map(|(c, _)| c));
        }
    }
    out.stale_pins = stale_pins(input, &out.new_texts);
    let mut files: Vec<&PathBuf> = input.report_only.keys().collect();
    files.sort();
    for file in files {
        let (_, citations) = rewrite_citations(file, &input.report_only[file], &moved);
        out.unrewritten
            .extend(citations.into_iter().map(|(c, renamed_to)| Unrewritten {
                file: c.file,
                line: c.line,
                renamed_to,
                reference: c.from,
            }));
    }
    out
}

/// Pins (`path[#scope]@sha256:hex`) in any scanned text on a file whose text is rewritten.
fn stale_pins(input: &PlanInput, changed: &BTreeMap<PathBuf, String>) -> Vec<StalePin> {
    let pin = regex::Regex::new(r"[A-Za-z0-9_./\-]+(?:#[A-Za-z0-9_]+)?@sha256:[0-9a-fA-F]+")
        .expect("a fixed pattern");
    let mut found = Vec::new();
    let mut files: Vec<(&PathBuf, &String)> = input
        .rewritable
        .iter()
        .chain(input.report_only.iter())
        .collect();
    files.sort();
    for (file, text) in files {
        for m in pin.find_iter(text) {
            let locator = m.as_str();
            let path = locator.split(['#', '@']).next().unwrap_or_default();
            if let Some(changed_file) =
                normalize(Path::new(path)).filter(|p| changed.contains_key(p))
            {
                found.push(StalePin {
                    file: file.clone(),
                    line: text[..m.start()].matches('\n').count() + 1,
                    locator: locator.to_string(),
                    changed_file,
                });
            }
        }
    }
    found
}

fn plan_one(
    input: &PlanInput,
    record: &Record,
    claimed: &mut HashSet<PathBuf>,
    out: &mut FilenamePlan,
) {
    let from = record.path.clone();
    let file_name = from
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let identity = input
        .config
        .record_types
        .get(&record.record_type)
        .and_then(|t| {
            crate::document_model::declared_model(&record.record_type, t)
                .ok()
                .flatten()
        })
        .and_then(|m| m.resolve().ok())
        .and_then(|m| m.identity);
    let skip = |out: &mut FilenamePlan, reason, message| {
        out.skipped.push(Skipped {
            from: from.clone(),
            reason,
            message,
        })
    };
    let name = match target_name(identity.as_ref(), &record.type_prefix, &file_name) {
        Ok(name) => name,
        Err((reason, message)) => return skip(out, reason, message),
    };
    let to = from.with_file_name(&name);
    if input.existing.contains(&to) || !claimed.insert(to.clone()) {
        return skip(
            out,
            SkipReason::TargetExists,
            format!(
                "{} already exists or is another rename's new name; nothing is overwritten",
                to.display()
            ),
        );
    }
    let Some(number) = number_after_prefix(&record.type_prefix, &name) else {
        return skip(
            out,
            SkipReason::PatternRejects,
            format!("'{name}' carries no number after '{}-'", record.type_prefix),
        );
    };
    out.renames.push(Rename {
        record_type: record.record_type.clone(),
        from,
        to,
        has_stable_id: record.header.get_reserved("Stable-Id").is_some(),
        id: (record.type_prefix.clone(), number),
    });
}

fn number_after_prefix(prefix: &str, name: &str) -> Option<u64> {
    let rest = name.strip_prefix(prefix)?.strip_prefix('-')?;
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    rest[..digits].parse().ok()
}

/// Two files sharing a record id is a defect a name check cannot see (`5-x.md` and
/// `0005-y.md` get different names and one id), so a rename whose id another rename takes,
/// or a record already has, is skipped, every file of the clash.
fn skip_number_collisions(input: &PlanInput, out: &mut FilenamePlan) {
    let mut taken: HashMap<(String, u64), usize> = HashMap::new();
    for record in input.records {
        let Some(id) = crate::rules::record_id(record) else {
            continue;
        };
        if let Some(n) = number_after_prefix(&record.type_prefix, &id) {
            *taken.entry((record.type_prefix.clone(), n)).or_default() += 1;
        }
    }
    for rename in &out.renames {
        *taken.entry(rename.id.clone()).or_default() += 1;
    }
    let (clash, kept): (Vec<Rename>, Vec<Rename>) = std::mem::take(&mut out.renames)
        .into_iter()
        .partition(|r| taken.get(&r.id).copied().unwrap_or(0) > 1);
    out.renames = kept;
    out.skipped.extend(clash.into_iter().map(|r| Skipped {
        message: format!(
            "{}-{} would be the record id of more than one file; renaming only one would hide the clash, so none is renamed -- renumber one by hand",
            r.id.0, r.id.1
        ),
        from: r.from,
        reason: SkipReason::NumberCollision,
    }));
}

/// The path with `.` and `..` folded away; `None` when it climbs out of the root.
fn normalize(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            Component::Normal(p) => out.push(p),
            _ => return None,
        }
    }
    Some(out)
}

/// Which repository path a mention of `prefix + file name` in `file` means, in the
/// order it would be tried: a link starting `../` or naming only a file is relative to
/// the file; one starting `./` is relative to the file and then to the root; one starting `/` is relative to the repository root; any other
/// path is a repository path first and a relative one second.
fn candidates(file: &Path, prefix: &str, name: &str) -> Vec<PathBuf> {
    let dir = file.parent().unwrap_or(Path::new(""));
    let joined = format!("{prefix}{name}");
    let relative = normalize(&dir.join(&joined));
    let from_root = || normalize(Path::new(joined.trim_start_matches('/')));
    let list = if prefix.starts_with("//") {
        Vec::new()
    } else if prefix.starts_with('/') {
        vec![from_root()]
    } else if prefix.is_empty() || prefix.starts_with("../") {
        vec![relative]
    } else if prefix.starts_with("./") {
        // `./x` is the file's own directory in a link and the repository root in
        // prose and code comments; a moved file under either is a mention.
        vec![relative, from_root()]
    } else {
        vec![from_root(), relative]
    };
    list.into_iter().flatten().collect()
}

/// Rewrites every mention of a moved file in `text`, returning the new text and the
/// citations changed. A mention is a path token ending `.md` (a trailing sentence
/// period is not part of it) whose directory part, read as the file's own relative
/// link or as a repository path, leads to a moved file. Only the file name is
/// replaced; the directory part, a `#anchor` and everything around are untouched.
fn rewrite_citations(
    file: &Path,
    text: &str,
    moved: &HashMap<PathBuf, PathBuf>,
) -> (String, Vec<(Citation, PathBuf)>) {
    let token = regex::Regex::new(r"[A-Za-z0-9_./\-]+").expect("a fixed pattern");
    let mut out = String::with_capacity(text.len());
    let mut citations = Vec::new();
    let mut last = 0;
    for m in token.find_iter(text) {
        let raw = m.as_str();
        let trimmed = raw.trim_end_matches('.');
        if !trimmed.to_ascii_lowercase().ends_with(".md") {
            continue;
        }
        let split = trimmed.rfind('/').map_or(0, |i| i + 1);
        let (prefix, name) = trimmed.split_at(split);
        let Some(to) = candidates(file, prefix, name)
            .iter()
            .find_map(|c| moved.get(c))
        else {
            continue;
        };
        let new_name = to.file_name().and_then(|n| n.to_str()).unwrap_or(name);
        let start = m.start() + split;
        out.push_str(&text[last..start]);
        out.push_str(new_name);
        last = start + name.len();
        let citation = Citation {
            file: file.to_path_buf(),
            line: text[..m.start()].matches('\n').count() + 1,
            from: trimmed.to_string(),
            to: format!("{prefix}{new_name}"),
        };
        citations.push((citation, to.clone()));
    }
    out.push_str(&text[last..]);
    (out, citations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(pattern: &str) -> CompiledIdentity {
        CompiledIdentity::compile(&crate::document_model::IdentityDecl {
            from: crate::document_model::IdentitySource::Filename,
            pattern: pattern.to_string(),
        })
        .unwrap()
    }

    const PADDED: &str = r"^(?P<prefix>ADR)-(?P<number>\d{4})-(?P<slug>.+)$";

    #[test]
    fn the_target_keeps_the_number_and_slug_and_takes_the_declared_padding() {
        let id = declared(PADDED);
        assert_eq!(
            target_name(Some(&id), "ADR", "0005-x.md").unwrap(),
            "ADR-0005-x.md"
        );
        assert_eq!(
            target_name(Some(&id), "ADR", "5-x.md").unwrap(),
            "ADR-0005-x.md"
        );
        assert_eq!(
            target_name(None, "ADR", "0005-x.md").unwrap(),
            "ADR-0005-x.md"
        );
        assert_eq!(
            target_name(None, "ADR", "5_x y.MD").unwrap(),
            "ADR-5-x y.md"
        );
    }

    #[test]
    fn padding_narrows_or_widens_but_never_changes_the_value() {
        let id = declared(PADDED);
        assert_eq!(
            target_name(Some(&id), "ADR", "00012-z.md").unwrap(),
            "ADR-0012-z.md"
        );
        let (reason, message) = target_name(Some(&id), "ADR", "12345-x.md").unwrap_err();
        assert_eq!(reason, SkipReason::PatternRejects);
        assert!(!message.contains("NNNN"), "{message}");
    }

    #[test]
    fn a_prefixed_but_mispadded_name_keeps_its_number() {
        let id = declared(PADDED);
        assert_eq!(
            target_name(Some(&id), "ADR", "ADR-5-x.md").unwrap(),
            "ADR-0005-x.md"
        );
        assert_eq!(
            target_name(Some(&id), "ADR", "ADR-00005-x.md").unwrap(),
            "ADR-0005-x.md"
        );
    }

    #[test]
    fn a_name_without_a_number_or_that_the_pattern_rejects_is_a_reasoned_skip() {
        let id = declared(PADDED);
        let (reason, _) = target_name(Some(&id), "ADR", "README.md").unwrap_err();
        assert_eq!(reason, SkipReason::NoNumber);
        let (reason, _) =
            target_name(Some(&id), "ADR", "99999999999999999999999-x.md").unwrap_err();
        assert_eq!(reason, SkipReason::NumberTooLarge);
        let (reason, _) = target_name(Some(&id), "ADR", "12345-x.md").unwrap_err();
        assert_eq!(reason, SkipReason::PatternRejects);
    }

    fn moved() -> HashMap<PathBuf, PathBuf> {
        HashMap::from([(
            PathBuf::from("docs/adr/0001-a.md"),
            PathBuf::from("docs/adr/ADR-0001-a.md"),
        )])
    }

    #[test]
    fn citations_resolve_against_the_file_or_the_root_and_nothing_else() {
        let text = "[a](./0001-a.md#x) [b](0001-a.md) docs/adr/0001-a.md. /docs/adr/0001-a.md other/0001-a.md ../adr/0001-a.md";
        let (new, found) = rewrite_citations(Path::new("docs/adr/ADR-0002-b.md"), text, &moved());
        assert_eq!(
            new,
            "[a](./ADR-0001-a.md#x) [b](ADR-0001-a.md) docs/adr/ADR-0001-a.md. /docs/adr/ADR-0001-a.md other/0001-a.md ../adr/ADR-0001-a.md"
        );
        assert_eq!(found.len(), 5);
        assert_eq!(found[0].0.line, 1);
    }

    #[test]
    fn a_dot_slash_path_is_read_from_the_file_and_from_the_repository_root() {
        // From a file elsewhere in the tree, `./docs/adr/...` names the repository
        // path as often as a path under the file's own directory.
        let (new, found) = rewrite_citations(
            Path::new("docs/rfc/0003-c.md"),
            "see ./docs/adr/0001-a.md and [x](./docs/adr/0001-a.md#y)\n",
            &moved(),
        );
        assert_eq!(
            new,
            "see ./docs/adr/ADR-0001-a.md and [x](./docs/adr/ADR-0001-a.md#y)\n"
        );
        assert_eq!(found.len(), 2);
        // `../` stays relative to the file only.
        let (_, found) = rewrite_citations(
            Path::new("docs/rfc/0003-c.md"),
            "see ../docs/adr/0001-a.md\n",
            &moved(),
        );
        assert!(found.is_empty());
    }

    #[test]
    fn a_link_from_another_directory_resolves_against_that_directory() {
        let (new, found) = rewrite_citations(
            Path::new("docs/rfc/0003-c.md"),
            "see ../adr/0001-a.md and 0001-a.md\n",
            &moved(),
        );
        assert_eq!(new, "see ../adr/ADR-0001-a.md and 0001-a.md\n");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn a_longer_name_that_merely_ends_alike_is_not_a_mention() {
        let (new, found) = rewrite_citations(
            Path::new("README.md"),
            "x10001-a.md and docs/adr/0001-a.md.bak",
            &moved(),
        );
        assert_eq!(new, "x10001-a.md and docs/adr/0001-a.md.bak");
        assert!(found.is_empty());
    }
}
