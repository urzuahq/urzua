//! `urzua init` adopt mode (SPEC-5). Adopt is the primary case: every
//! codebase this project's design was drawn from already had records before
//! it had tooling. Adopt never moves a file -- it reads the tree, proposes a
//! config, and writes only under `.urzua/`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::header::{self, HeaderShape};
use urzua_core::report::{CouldNotRun, Notice, NoticeSeverity, Report, ReportStatus};

use crate::discovery::find_repo_root;
use crate::{emit, NOTICE_HEADER_SHAPE, NOTICE_RULE_APPLICABILITY};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProposedRecordType {
    pub name: String,
    pub dir: String,
    pub record_count: usize,
    /// The type prefix the directory's filenames actually carry, when they all
    /// agree on one. `None` means the corpus uses the bare `NNNN-slug` shape,
    /// which carries no identity a rule can read (BUG-61).
    pub prefix: Option<String>,
    /// The header shape proposed for the type: always `yaml-frontmatter`, the
    /// only shape with a reader (MILE-85).
    pub header_shape: HeaderShape,
}

/// Propose one record type per directory holding record-shaped files. Adopt
/// mode: this only reads, it never writes or moves.
pub fn detect_record_types(discovered: &[PathBuf]) -> Vec<ProposedRecordType> {
    let mut counts: std::collections::BTreeMap<PathBuf, usize> = std::collections::BTreeMap::new();
    let mut prefixes: std::collections::BTreeMap<PathBuf, Vec<Option<String>>> =
        std::collections::BTreeMap::new();

    for rel_path in discovered {
        let Some(file_name) = rel_path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !is_record_shaped(file_name) {
            continue;
        }
        let Some(parent) = rel_path.parent() else {
            continue;
        };
        // A hidden directory (`.changeset`, `.github`) holds tool state, never
        // a record type, and a leading dot is not a legal type name or prefix.
        if is_hidden_path(rel_path) {
            continue;
        }
        // A record at the repository root has no directory that identifies its
        // type, and proposing "" would claim every file in the repository.
        if parent.as_os_str().is_empty() {
            continue;
        }
        *counts.entry(parent.to_path_buf()).or_insert(0) += 1;
        let detected = urzua_core::new_record::parse_record_filename(file_name)
            .and_then(|(prefix, _)| prefix.map(|p| p.to_string()));
        prefixes
            .entry(parent.to_path_buf())
            .or_default()
            .push(detected);
    }

    // Discovery matches a type's `dir` by path prefix, so a directory proposed
    // alongside one that contains it is already covered by the outer type. The
    // outer one wins: dropping it instead adopts an `archive/` subdirectory and
    // leaves its parent's records ungoverned, with `check` reporting success
    // over them (BUG-43).
    // One type per directory. Inferring which directories belong together
    // loses records whenever the inference is wrong, and an adopter can merge
    // two proposed types far more easily than they can notice a missing one.

    let mut names: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for dir in counts.keys() {
        *names.entry(proposed_name(dir)).or_insert(0) += 1;
    }

    counts
        .into_iter()
        .map(|(dir, record_count)| {
            let base = proposed_name(&dir);
            // Two directories can end in the same component (`doc/adr` and
            // `docs/adr`). A duplicated type name is a config that cannot load,
            // so qualify it rather than emit one.
            let name = if names.get(&base).copied().unwrap_or(0) > 1 {
                legal_type_name(
                    &dir.components()
                        .map(|c| c.as_os_str().to_string_lossy().to_string())
                        .collect::<Vec<_>>()
                        .join("-"),
                )
            } else {
                base
            };
            // Unanimous or nothing. A directory whose filenames disagree has
            // no single prefix to declare, and guessing one silently ungoverns
            // every record that does not match it.
            let seen = prefixes.get(&dir).cloned().unwrap_or_default();
            let prefix = match seen.first() {
                Some(first) if seen.iter().all(|p| p == first) => first.clone(),
                _ => None,
            };
            ProposedRecordType {
                name,
                dir: dir.to_string_lossy().to_string(),
                record_count,
                prefix,
                header_shape: HeaderShape::YamlFrontmatter,
            }
        })
        .collect()
}

/// How one record's header reads under the only shape with a reader.
enum HeaderReading {
    /// YAML frontmatter at the very start of the document, with fields.
    Frontmatter,
    /// A removed form (`blockquote` or `bold-list`, MILE-85): named, not read.
    Removed(&'static str),
    /// No header at all, or one that does not parse.
    Unreadable,
}

fn read_header(content: &str) -> HeaderReading {
    let yaml = header::parse_with_shape(content, HeaderShape::YamlFrontmatter);
    if matches!(yaml.region, Some((1, _))) && !yaml.fields.is_empty() {
        return HeaderReading::Frontmatter;
    }
    match header::legacy_header_form(content) {
        Some(form) => HeaderReading::Removed(form),
        None => HeaderReading::Unreadable,
    }
}

/// Report, for each proposed type, how many of its existing records `check`
/// will read. `yaml-frontmatter` is the only shape proposed (the blockquote
/// and bold-list shapes were removed, MILE-85), so a record in one of those
/// forms is named in a notice rather than proposed a shape that cannot read
/// it. Reads only; a record that cannot be read counts as unreadable.
pub fn infer_header_shapes(
    repo_root: &Path,
    discovered: &[PathBuf],
    proposed: &mut [ProposedRecordType],
) -> Vec<Notice> {
    let mut notices = Vec::new();
    for rt in proposed.iter_mut() {
        rt.header_shape = HeaderShape::YamlFrontmatter;
        let (mut yaml, mut blockquote, mut bold_list, mut unreadable) =
            (0usize, 0usize, 0usize, 0usize);
        for rel in discovered {
            let in_type = rel.parent() == Some(Path::new(&rt.dir))
                && rel
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(is_record_shaped);
            if !in_type {
                continue;
            }
            let reading = std::fs::read_to_string(repo_root.join(rel))
                .map(|content| read_header(&content))
                .unwrap_or(HeaderReading::Unreadable);
            match reading {
                HeaderReading::Frontmatter => yaml += 1,
                HeaderReading::Removed("blockquote") => blockquote += 1,
                HeaderReading::Removed(_) => bold_list += 1,
                HeaderReading::Unreadable => unreadable += 1,
            }
        }
        if blockquote + bold_list + unreadable == 0 {
            continue;
        }
        let unread: Vec<String> = [
            (blockquote, "in the removed blockquote form"),
            (bold_list, "in the removed bold-list form"),
            (unreadable, "with no readable header"),
        ]
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, what)| format!("{n} {what}"))
        .collect();
        notices.push(Notice {
            severity: NoticeSeverity::Warning,
            subject: NOTICE_HEADER_SHAPE.to_string(),
            message: format!(
                "yaml-frontmatter is proposed for {}, the only header shape urzua reads; {yaml} record(s) parse under it and {} will be reported as unreadable by `check` until their headers are rewritten as YAML frontmatter",
                rt.dir,
                unread.join(", ")
            ),
        });
    }
    notices
}

/// Notice subject for what adopt's directory scan found worth a second look.
pub(crate) const NOTICE_INIT_SCAN: &str = "init-scan";

fn has_header(content: &str) -> bool {
    !matches!(read_header(content), HeaderReading::Unreadable)
}

fn nested_either_way(a: &Path, b: &Path) -> bool {
    a.starts_with(b) || b.starts_with(a)
}

/// Split off each proposed type none of whose record-shaped files carries a
/// header. Such a directory holds numbered pages (site content, requirement
/// documents) far more often than records, and a type with nothing to read is
/// noise in the config, so it is not proposed; the notice names it so the
/// adopter can declare it by hand. Reads only.
pub fn drop_headerless(
    repo_root: &Path,
    discovered: &[PathBuf],
    proposed: Vec<ProposedRecordType>,
) -> (Vec<ProposedRecordType>, Vec<Notice>) {
    let (kept, dropped): (Vec<_>, Vec<_>) = proposed.into_iter().partition(|rt| {
        discovered.iter().any(|rel| {
            rel.parent() == Some(Path::new(&rt.dir))
                && rel
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(is_record_shaped)
                && std::fs::read_to_string(repo_root.join(rel))
                    .is_ok_and(|content| has_header(&content))
        })
    });
    if dropped.is_empty() {
        return (kept, Vec::new());
    }
    let listed: Vec<String> = dropped
        .iter()
        .map(|rt| format!("{} ({} file(s))", rt.dir, rt.record_count))
        .collect();
    let notice = Notice {
        severity: NoticeSeverity::Warning,
        subject: NOTICE_INIT_SCAN.to_string(),
        message: format!(
            "not proposed, because no file in these directories has a header, so they hold pages rather than records: {}; declare a record type by hand if one of them is a record directory",
            listed.join(", ")
        ),
    };
    (kept, vec![notice])
}

fn is_hidden_path(rel: &Path) -> bool {
    rel.components()
        .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
}

/// A header field that reads as a lifecycle state, the one thing numbered
/// file names would otherwise have told us makes a file a record.
fn has_status_like_field(content: &str) -> bool {
    header::parse_with_shape(content, HeaderShape::YamlFrontmatter)
        .fields
        .iter()
        .any(|f| f.key.as_str().eq_ignore_ascii_case("status"))
}

/// Propose directories whose files are not numbered records but do carry
/// headers (a flat spec directory named by slug, for example). A directory
/// qualifies when at least two of its markdown files have a header and those
/// are at least half of the directory's markdown files. A directory nested in,
/// or containing, an already proposed one is skipped: discovery matches by path
/// prefix, so the outer type would already cover it. Headers alone are not
/// enough (every docs-site page has front matter): at least two headed files
/// must carry a status-like field. The directories that had headers but no
/// status are returned second, so the notice can name them.
pub fn detect_unnumbered_record_types(
    repo_root: &Path,
    discovered: &[PathBuf],
    proposed: &[ProposedRecordType],
) -> (Vec<ProposedRecordType>, Vec<String>) {
    // (markdown files, with a header, with a status-like field)
    let mut dirs: std::collections::BTreeMap<PathBuf, (usize, usize, usize)> =
        std::collections::BTreeMap::new();
    for rel in discovered {
        let Some(file_name) = rel.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let lower = file_name.to_ascii_lowercase();
        if !lower.ends_with(".md")
            || file_name.starts_with('_')
            || lower == "readme.md"
            || lower == "index.md"
            || is_record_shaped(file_name)
        {
            continue;
        }
        let Some(parent) = rel.parent().filter(|p| !p.as_os_str().is_empty()) else {
            continue;
        };
        if is_hidden_path(rel) {
            continue;
        }
        if proposed
            .iter()
            .any(|rt| nested_either_way(parent, Path::new(&rt.dir)))
        {
            continue;
        }
        let entry = dirs.entry(parent.to_path_buf()).or_insert((0, 0, 0));
        entry.0 += 1;
        if let Ok(content) = std::fs::read_to_string(repo_root.join(rel)) {
            if has_header(&content) {
                entry.1 += 1;
            }
            if has_status_like_field(&content) {
                entry.2 += 1;
            }
        }
    }
    let mut taken: std::collections::BTreeSet<String> =
        proposed.iter().map(|rt| rt.name.clone()).collect();
    let mut out = Vec::new();
    let mut headers_only = Vec::new();
    for (dir, (total, headed, with_status)) in dirs {
        if headed < 2 || headed * 2 < total {
            continue;
        }
        if with_status < 2 {
            headers_only.push(dir.to_string_lossy().to_string());
            continue;
        }
        let base = proposed_name(&dir);
        let name = if taken.contains(&base) {
            legal_type_name(
                &dir.components()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join("-"),
            )
        } else {
            base
        };
        if !taken.insert(name.clone()) {
            continue;
        }
        out.push(ProposedRecordType {
            name,
            dir: dir.to_string_lossy().to_string(),
            record_count: total,
            prefix: None,
            header_shape: HeaderShape::YamlFrontmatter,
        });
    }
    (out, headers_only)
}

fn proposed_name(dir: &Path) -> String {
    let last = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    legal_type_name(&singular_type_name(&last))
}

/// A type name whose upper-cased form is a legal default prefix: lower-case
/// ASCII letters and digits in hyphen-separated segments, none all digits.
/// Whatever the directory is called, the config written for it must load.
fn legal_type_name(raw: &str) -> String {
    let mut segments: Vec<String> = Vec::new();
    for seg in raw
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
    {
        segments.push(if seg.chars().all(|c| c.is_ascii_digit()) {
            format!("n{seg}")
        } else {
            seg.to_string()
        });
    }
    if segments.is_empty() {
        "records".to_string()
    } else {
        segments.join("-")
    }
}

/// The prefix a type would load with: the declared one, else its upper-cased
/// name.
fn effective_prefix(rt: &ProposedRecordType) -> String {
    rt.prefix
        .clone()
        .unwrap_or_else(|| rt.name.to_ascii_uppercase())
}

/// Two types may not share a prefix, so a later directory whose prefix an
/// earlier one holds is not proposed (the config would not load). Named in a
/// notice, never dropped silently.
fn drop_prefix_clashes(
    proposed: Vec<ProposedRecordType>,
) -> (Vec<ProposedRecordType>, Vec<Notice>) {
    let mut taken: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    let mut kept = Vec::new();
    let mut clashes = Vec::new();
    for rt in proposed {
        let prefix = effective_prefix(&rt);
        match taken.get(&prefix) {
            Some(owner) => clashes.push(format!("{} (prefix {prefix}, held by {owner})", rt.dir)),
            None => {
                taken.insert(prefix, rt.dir.clone());
                kept.push(rt);
            }
        }
    }
    let notices = if clashes.is_empty() {
        Vec::new()
    } else {
        vec![Notice {
            severity: NoticeSeverity::Warning,
            subject: NOTICE_INIT_SCAN.to_string(),
            message: format!(
                "not proposed, because two record types may not share a prefix: {}; declare it by hand with a prefix of its own",
                clashes.join(", ")
            ),
        }]
    };
    (kept, notices)
}

/// A leading underscore is a template, never a record. Everything else defers
/// to the shared recogniser, so adopt mode and `new`'s numbering cannot accept
/// disjoint sets. Deliberately stricter than `record::is_governed_record_filename`
/// (`BUG-108`): a numberless file governs nothing for `check`, but adopt mode
/// specifically needs a number to detect the corpus's own convention.
fn is_record_shaped(file_name: &str) -> bool {
    !file_name.starts_with('_')
        && urzua_core::new_record::parse_record_filename(file_name).is_some()
}

/// `adr` -> `adr`, `rfc` -> `rfc`, `specs` -> `spec` -- adopt proposes the
/// type name a config would actually declare, not the directory's own name.
fn singular_type_name(dir_name: &str) -> String {
    dir_name.strip_suffix('s').unwrap_or(dir_name).to_string()
}

/// The rules `init` does not propose because the declaration each reads is one
/// `init` does not write, grouped by declaration, so the adopter knows what to
/// declare before enabling them (BUG-73).
fn withheld_rule_notices() -> Vec<Notice> {
    let mut by_declaration: Vec<(urzua_core::rules::Declaration, Vec<&'static str>)> = Vec::new();
    for def in urzua_core::rules::RULE_TABLE {
        let Some(declaration) = def.reads_declaration else {
            continue;
        };
        match by_declaration.iter_mut().find(|(d, _)| *d == declaration) {
            Some((_, rules)) => rules.push(def.id),
            None => by_declaration.push((declaration, vec![def.id])),
        }
    }
    if by_declaration.is_empty() {
        return Vec::new();
    }
    let clauses: Vec<String> = by_declaration
        .iter()
        .map(|(declaration, rules)| {
            format!("{} (declare `{}`)", rules.join(", "), declaration.key())
        })
        .collect();
    vec![Notice {
        severity: NoticeSeverity::Info,
        subject: NOTICE_RULE_APPLICABILITY.to_string(),
        message: format!(
            "not proposed, because each reads a per-type declaration init does not infer and would examine no record until it exists: {}; declare it on a record type, then enable the rule",
            clauses.join("; ")
        ),
    }]
}

/// Built through the real serializer rather than string-concatenated, so a
/// directory name containing YAML metacharacters cannot break out of its value
/// (BUG-36). The `Mapping` is ordered by `proposed`, so repeated runs over the
/// same corpus produce byte-identical output.
pub fn render_config_yaml(proposed: &[ProposedRecordType]) -> String {
    use yaml_serde::{Mapping, Value};

    let mut types = Mapping::new();
    for rt in proposed {
        let mut entry = Mapping::new();
        entry.insert(Value::from("dir"), Value::from(rt.dir.as_str()));
        entry.insert(Value::from("required_fields"), Value::Sequence(vec![]));
        // The one shape with a reader (MILE-85); `infer_header_shapes` notices
        // any existing record that is not already written in it.
        entry.insert(
            Value::from("header_shape"),
            Value::from(match rt.header_shape {
                HeaderShape::YamlFrontmatter => "yaml-frontmatter",
                HeaderShape::None => "none",
            }),
        );
        // Without this, the prefix defaults to the type name upper-cased --
        // which is the directory's name, not the filenames' -- and every rule
        // that reads a record's identity from its filename silently examines
        // nothing (BUG-61).
        if let Some(prefix) = &rt.prefix {
            entry.insert(Value::from("prefix"), Value::from(prefix.as_str()));
        }
        types.insert(Value::from(rt.name.as_str()), Value::Mapping(entry));
    }

    // Proposed at `warn`: an adopted corpus should be told what is irregular
    // without being blocked on it, and promoting a rule is the adopter's call.
    let mut rules = Mapping::new();
    for id in urzua_core::rules::ALL_RULES {
        // Adopt mode has nothing to say about a rule's options, and a rule
        // declared without the options it requires will not load.
        if urzua_core::config::rule_requires_options(id) {
            continue;
        }
        // Pairs are declared, never inferred: enabling these with none
        // declared is a load error, so adopt mode cannot propose them.
        if urzua_core::rules::rule_def(id).is_some_and(|d| d.requires_relation_pairs) {
            continue;
        }
        // The override log is declared, never inferred (`MILE-188`).
        if urzua_core::rules::rule_def(id).is_some_and(|d| d.requires_override_log) {
            continue;
        }
        // The authority table names logins and gates, which no corpus can supply
        // (`MILE-200`).
        if urzua_core::rules::rule_def(id).is_some_and(|d| d.requires_authority) {
            continue;
        }
        // A rule that reads a per-type declaration `init` leaves empty would
        // report `ran` over zero records (BUG-73). `init` infers nothing from
        // the corpus (ADR-33), so the adopter declares it and enables the rule;
        // `withheld_rule_notices` says which.
        if urzua_core::rules::rule_def(id).is_some_and(|d| d.reads_declaration.is_some()) {
            continue;
        }
        // A corpus whose filenames carry no type prefix has no identity for
        // these rules to read (ADR-36 declined the bare `NNNN-slug` shape), so
        // they would load, report `ran`, and examine zero records. Declining to
        // propose a rule that cannot fire is the same judgement already applied
        // to a rule whose required options adopt mode cannot supply.
        //
        // Every type, not any: these rules skip an individual record whose
        // filename yields no identity, so one prefix-less directory costs only
        // its own records -- dropping the rule outright would cost the rest of
        // the corpus its checks (BUG-70).
        if urzua_core::rules::rule_def(id).is_some_and(|d| d.needs_identity_index)
            && proposed.iter().all(|rt| rt.prefix.is_none())
        {
            continue;
        }
        rules.insert(Value::from(*id), Value::from("warn"));
    }

    let mut root = Mapping::new();
    root.insert(
        Value::from("schema_version"),
        Value::from(urzua_core::config::CURRENT_SCHEMA_VERSION),
    );
    root.insert(Value::from("record_types"), Value::Mapping(types));
    root.insert(Value::from("rules"), Value::Mapping(rules));

    // `.unwrap()`, not a swallowed default: `root` is built entirely from
    // plain strings/sequences/mappings, so serialization failing here would
    // be a bug in this function, not a config an adopter's input could
    // trigger. A silent empty body would write a config with no
    // `record_types`/`rules` and still report `written: true` -- exactly
    // the disclosure failure (`ADR-55`) this project otherwise closes by
    // panicking loudly (matching `emit()`'s own `serde_json::to_string_pretty(..).unwrap()`),
    // which the panic hook turns into structured JSON rather than raw stderr.
    let body = yaml_serde::to_string(&Value::Mapping(root)).unwrap();
    format!("# Generated by `urzua init` (adopt mode). Nothing was moved.\n\n{body}")
}

/// `urzua init`'s report (SPEC-5).
#[derive(serde::Serialize)]
struct InitReport {
    status: ReportStatus,
    dry_run: bool,
    config_path: PathBuf,
    proposed: Vec<ProposedRecordType>,
    written: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    config_yaml: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
}

impl Report for InitReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(0)
    }
}

/// Never clobbers an existing config; `--dry-run` proposes the config the
/// real run writes (its `config_yaml` is byte-identical to the written file)
/// and differs in the report's `dry_run` and `written` fields, and in the real
/// run omitting `config_yaml` rather than echoing the file back.
pub fn run(config: Option<PathBuf>, dry_run: bool) -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    // `--config X` names the file to write (SPEC-5); only the default location
    // is derived from the repository, so only it is confined to it.
    let explicit_config = config.is_some();
    let config_path = config.unwrap_or_else(|| crate::discovery::default_config_path(&repo_root));
    let config_exists = config_path.exists();
    // `--dry-run` writes nothing, so an existing config is no reason to refuse
    // it: it shows what adopt would propose today.
    if config_exists && !dry_run {
        return emit(&CouldNotRun::from(format!(
            "{} already exists -- refusing to overwrite. Edit it directly, or remove it to re-run adopt.",
            config_path.display()
        )));
    }

    let discovered = match urzua_io::discover_tracked_files(&repo_root, None) {
        Ok(d) => d,
        Err(e) => return emit(&CouldNotRun::from(e.to_string())),
    };

    let (mut proposed, mut notices) = drop_headerless(
        &repo_root,
        &discovered.paths,
        detect_record_types(&discovered.paths),
    );
    if config_exists {
        notices.push(Notice {
            severity: NoticeSeverity::Info,
            subject: NOTICE_INIT_SCAN.to_string(),
            message: format!(
                "{} already exists; this dry run shows what adopt would propose and nothing was written",
                config_path.display()
            ),
        });
    }
    let (unnumbered, headers_only) =
        detect_unnumbered_record_types(&repo_root, &discovered.paths, &proposed);
    if !headers_only.is_empty() {
        notices.push(Notice {
            severity: NoticeSeverity::Info,
            subject: NOTICE_INIT_SCAN.to_string(),
            message: format!(
                "not proposed, because their files carry headers but no status field, so they look like pages rather than records: {}; declare a record type by hand if one of them is a record directory",
                headers_only.join(", ")
            ),
        });
    }
    if !unnumbered.is_empty() {
        notices.push(Notice {
            severity: NoticeSeverity::Info,
            subject: NOTICE_INIT_SCAN.to_string(),
            message: format!(
                "proposed from headers alone, because their file names carry no record number: {}; check that each is a record directory",
                unnumbered
                    .iter()
                    .map(|rt| rt.dir.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
        proposed.extend(unnumbered);
    }
    // Disambiguation (above, on base name only) can still collide: a
    // disambiguated name can match an unrelated directory that already
    // proposes it (BUG-132). `render_config_yaml`'s `Mapping::insert` would
    // silently overwrite one type's entry with the other's, so refuse rather
    // than write a config that drops an entire directory's records with no
    // error.
    let mut names_to_dirs: std::collections::BTreeMap<&str, &str> =
        std::collections::BTreeMap::new();
    for rt in &proposed {
        if let Some(other_dir) = names_to_dirs.insert(rt.name.as_str(), rt.dir.as_str()) {
            return emit(&CouldNotRun::from(format!(
                "{} and {other_dir} both propose the record type name '{}' after disambiguation -- adopt cannot generate a config that would silently drop one. Reorganize one of these directories, or write .urzua/config.yaml by hand for now.",
                rt.dir, rt.name
            )));
        }
    }

    let (proposed, clash_notices) = drop_prefix_clashes(proposed);
    let mut proposed = proposed;
    notices.extend(clash_notices);
    if proposed.is_empty() {
        let scanned = notices
            .iter()
            .map(|n| n.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        return emit(&CouldNotRun::from(format!(
            "no record directory found in the tracked set -- nothing to adopt{}{scanned}",
            if scanned.is_empty() { "" } else { ": " }
        )));
    }

    let relative_config_path = config_path
        .strip_prefix(&repo_root)
        .unwrap_or(&config_path)
        .to_path_buf();

    notices.extend(infer_header_shapes(
        &repo_root,
        &discovered.paths,
        &mut proposed,
    ));
    notices.extend(withheld_rule_notices());

    let rendered = render_config_yaml(&proposed);
    // Whatever was proposed, the file written must load: a config `init` calls
    // ready that `check` then refuses is the worst outcome adopt can have.
    if let Err(e) = urzua_core::config::parse(&rendered) {
        return emit(&CouldNotRun::from(format!(
            "adopt generated a config that does not load ({e}); this is a bug in `urzua init`, nothing was written"
        )));
    }

    if dry_run {
        return emit(&InitReport {
            status: ReportStatus::Ok,
            dry_run: true,
            config_path: relative_config_path,
            proposed,
            written: false,
            config_yaml: Some(rendered),
            notices: notices.clone(),
        });
    }

    if !explicit_config {
        if let Err(e) = urzua_io::confine_to_root(&repo_root, &config_path) {
            return emit(&CouldNotRun::from(e));
        }
    }
    if let Some(parent) = config_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return emit(&CouldNotRun::from(format!(
                "could not create {}: {e}",
                parent.display()
            )));
        }
    }
    if let Err(e) = urzua_io::write_atomic(&config_path, &rendered) {
        return emit(&CouldNotRun::from(format!(
            "could not write {}: {e}",
            config_path.display()
        )));
    }

    emit(&InitReport {
        status: ReportStatus::Ok,
        dry_run: false,
        config_path: relative_config_path,
        proposed,
        written: true,
        config_yaml: None,
        notices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_one_type_per_populated_docs_subdirectory() {
        let discovered = vec![
            PathBuf::from("docs/adr/0001-x.md"),
            PathBuf::from("docs/adr/0002-y.md"),
            PathBuf::from("docs/rfc/0001-z.md"),
            PathBuf::from("README.md"),
            PathBuf::from("docs/adr/_template.md"),
        ];
        let proposed = detect_record_types(&discovered);

        let adr = proposed.iter().find(|p| p.name == "adr").unwrap();
        assert_eq!(adr.record_count, 2);
        assert_eq!(adr.dir, "docs/adr");

        let rfc = proposed.iter().find(|p| p.name == "rfc").unwrap();
        assert_eq!(rfc.record_count, 1);

        assert_eq!(
            proposed.len(),
            2,
            "README.md and the template must not create a type"
        );
    }

    /// BUG-132: disambiguation only compares base names, never the joined
    /// name it produces -- so a disambiguated name can still collide with an
    /// unrelated directory that already happens to be named that.
    #[test]
    fn a_disambiguated_name_can_still_collide_with_an_unrelated_directory_observed_failing() {
        let discovered: Vec<PathBuf> = ["x/y/0001-a.md", "z/y/0001-b.md", "x-y/0001-c.md"]
            .iter()
            .map(PathBuf::from)
            .collect();

        let proposed = detect_record_types(&discovered);
        let names: Vec<&str> = proposed.iter().map(|p| p.name.as_str()).collect();
        let collided = names.iter().filter(|n| **n == "x-y").count();
        assert_eq!(
            collided, 2,
            "x/y disambiguates to x-y, colliding with the unrelated x-y directory: {names:?}"
        );
    }

    #[test]
    fn specs_directory_proposes_singular_type_name() {
        let discovered = vec![PathBuf::from("docs/specs/0001-x.md")];
        let proposed = detect_record_types(&discovered);
        assert_eq!(proposed[0].name, "spec");
        assert_eq!(proposed[0].dir, "docs/specs");
    }

    /// BUG-36: the config was string-concatenated, so a directory name carrying
    /// the format's metacharacters escaped its value. Rendering through the real
    /// serializer is only a fix if the output parses back to the same string.
    #[test]
    fn a_directory_name_full_of_yaml_metacharacters_survives_render_and_reparse_observed_failing() {
        let hostile = "docs: [adr] #x\n  evil: true";
        let rendered = render_config_yaml(&[ProposedRecordType {
            name: "adr".to_string(),
            dir: hostile.to_string(),
            record_count: 1,
            prefix: None,
            header_shape: HeaderShape::YamlFrontmatter,
        }]);
        let parsed = urzua_core::config::parse(&rendered)
            .expect("init must never emit a config it cannot read back");
        assert_eq!(parsed.record_types.get("adr").unwrap().dir, hostile);
    }

    /// RFC-35: one type per directory holding records. No folding, no
    /// containers, no inference -- four heuristics guessed at the overlap
    /// prefix matching created, and each re-entered the last one's failure.
    /// The third silently dropped every record in a directory that had two
    /// record-bearing subdirectories (`BUG-43`'s own failure, re-entered).
    #[test]
    fn every_directory_holding_records_becomes_its_own_type() {
        let discovered: Vec<PathBuf> = [
            "docs/adr/0001-a.md",
            "docs/adr/0002-b.md",
            "docs/adr/0003-c.md",
            "docs/adr/archive/0009-old.md",
            "docs/adr/drafts/0010-new.md",
            "docs/rfc/0001-r.md",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();

        let proposed = detect_record_types(&discovered);
        let mut got: Vec<(String, usize)> = proposed
            .iter()
            .map(|p| (p.dir.clone(), p.record_count))
            .collect();
        got.sort();

        assert_eq!(
            got,
            vec![
                ("docs/adr".to_string(), 3),
                ("docs/adr/archive".to_string(), 1),
                ("docs/adr/drafts".to_string(), 1),
                ("docs/rfc".to_string(), 1),
            ]
        );

        // Every record is accounted for exactly once. The old fold lost three
        // of these entirely and `check` reported success over them.
        assert_eq!(got.iter().map(|(_, n)| n).sum::<usize>(), discovered.len());
    }

    /// BUG-44: a rule that cannot be declared without options must not be
    /// proposed bare -- the generated config would not load.
    #[test]
    fn the_generated_config_omits_rules_that_require_options() {
        let rendered = render_config_yaml(&[ProposedRecordType {
            name: "adr".to_string(),
            dir: "docs/adr".to_string(),
            record_count: 1,
            prefix: None,
            header_shape: HeaderShape::YamlFrontmatter,
        }]);
        assert!(!rendered.contains("claim.status-agreement"), "{rendered}");
        assert!(!rendered.contains("pointer.target-status"), "{rendered}");
        urzua_core::config::parse(&rendered).expect("init must emit a config that loads");
    }

    /// Two runs over the same corpus must produce the same bytes -- a
    /// `HashMap` in the render path would make this fail intermittently.
    #[test]
    fn rendering_is_deterministic_across_runs() {
        let proposed: Vec<_> = ["adr", "rfc", "spec", "bug", "milestone"]
            .iter()
            .map(|n| ProposedRecordType {
                name: n.to_string(),
                dir: format!("docs/{n}"),
                record_count: 1,
                prefix: None,
                header_shape: HeaderShape::YamlFrontmatter,
            })
            .collect();
        assert_eq!(render_config_yaml(&proposed), render_config_yaml(&proposed));
    }

    #[test]
    fn a_detected_prefix_is_written_so_the_proposed_rules_can_read_an_identity() {
        let proposed = vec![ProposedRecordType {
            name: "decision".to_string(),
            dir: "decisions".to_string(),
            record_count: 2,
            prefix: Some("ADR".to_string()),
            header_shape: HeaderShape::YamlFrontmatter,
        }];
        let yaml = render_config_yaml(&proposed);
        assert!(
            yaml.contains("prefix: ADR"),
            "without it the prefix defaults to DECISION and every identity rule goes inert: {yaml}"
        );
        assert!(yaml.contains("filename.title-consistency"));
    }

    #[test]
    fn one_prefix_less_type_does_not_disarm_the_types_that_have_a_prefix() {
        let proposed = vec![
            ProposedRecordType {
                name: "adr".to_string(),
                dir: "docs/adr".to_string(),
                record_count: 2,
                prefix: Some("ADR".to_string()),
                header_shape: HeaderShape::YamlFrontmatter,
            },
            ProposedRecordType {
                name: "note".to_string(),
                dir: "docs/notes".to_string(),
                record_count: 1,
                prefix: None,
                header_shape: HeaderShape::YamlFrontmatter,
            },
        ];
        let yaml = render_config_yaml(&proposed);
        assert!(
            yaml.contains("filename.title-consistency"),
            "adr carries a prefix, so the identity rules still address it: {yaml}"
        );
        assert!(yaml.contains("prefix: ADR"));
    }

    #[test]
    fn a_corpus_with_no_prefix_is_not_offered_rules_that_cannot_fire() {
        let proposed = vec![ProposedRecordType {
            name: "adr".to_string(),
            dir: "doc/adr".to_string(),
            record_count: 9,
            prefix: None,
            header_shape: HeaderShape::YamlFrontmatter,
        }];
        let yaml = render_config_yaml(&proposed);
        assert!(
            !yaml.contains("prefix:"),
            "there is none to declare: {yaml}"
        );
        for rule in [
            "filename.title-consistency",
            "relation.supersession-reciprocity",
            "pointer.resolution",
            "pointer.target-status",
            "narrative-field.stale",
            "relation.target-status-undeclared",
        ] {
            assert!(
                !yaml.contains(rule),
                "{rule} would report `ran` over zero records: {yaml}"
            );
        }
        assert!(
            yaml.contains("header.required-fields"),
            "rules that do not read an identity are still proposed: {yaml}"
        );
    }

    /// `BUG-73`: `init` writes `required_fields: []` and no `known_fields` or
    /// `pointer_fields`, so a rule that reads one of them examines nothing on
    /// the corpus it just adopted. It is not proposed, and the adopter is told
    /// what to declare to turn it on.
    #[test]
    fn a_rule_reading_a_declaration_init_does_not_write_is_not_proposed_observed_failing() {
        let proposed = vec![ProposedRecordType {
            name: "adr".to_string(),
            dir: "docs/adr".to_string(),
            record_count: 3,
            prefix: Some("ADR".to_string()),
            header_shape: HeaderShape::YamlFrontmatter,
        }];
        let yaml = render_config_yaml(&proposed);
        for rule in [
            "field.quality",
            "field.pending",
            "header.field-set-consistency",
            "header.pointer-field-clean",
        ] {
            assert!(
                !yaml.contains(&format!("{rule}:")),
                "{rule} reads a declaration init leaves empty, so it would examine zero records: {yaml}"
            );
        }
        assert!(
            yaml.contains("header.required-fields"),
            "rules that read no declaration are still proposed: {yaml}"
        );
    }

    #[test]
    fn the_withheld_rules_are_disclosed_with_the_declaration_each_needs() {
        let notices = withheld_rule_notices();
        assert_eq!(notices.len(), 1, "{notices:?}");
        let message = &notices[0].message;
        for needle in [
            "field.quality",
            "field.pending",
            "required_fields",
            "header.field-set-consistency",
            "known_fields",
            "header.pointer-field-clean",
            "pointer_fields",
        ] {
            assert!(message.contains(needle), "{needle} missing from: {message}");
        }
        assert_eq!(notices[0].subject, crate::NOTICE_RULE_APPLICABILITY);
    }
}
