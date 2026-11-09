//! Scoped evidence pins (`ADR-74`, `MILE-185`): the locator grammar, the
//! declared symbol resolvers and the pure resolution of a scope to a span.
//! Pure: callers read files and hand in text.

// Implements: ADR-74
use serde::{Deserialize, Serialize};

pub const PIN_MARKER: &str = "@sha256:";

/// One locator as written after its `category:` prefix (`ADR-74` point 1):
///
/// ```text
/// locator := path [ "#" scope ] [ "@" anchor ]
/// scope   := range | symbol
/// range   := "L" N [ "-L" M ]      N and M decimal, no leading zero, 1 <= N <= M
/// symbol  := seg ( "::" ( seg | "<" seg ">" ) )*     seg := [A-Za-z_] [A-Za-z0-9_.-]*
/// anchor  := "sha256:" 64 lowercase hex  |  7 to 64 lowercase hex (a commit)
/// ```
pub const GRAMMAR: &str = "path[#scope][@anchor]: scope is a symbol (`Name`, `Type::member`, `Type::<Trait>::member`) or a range (`L<n>` or `L<n>-L<m>`); anchor is `sha256:` and 64 lowercase hex, or a 7 to 64 digit lowercase hex commit";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locator {
    /// The path, lexically normalised like every locator path.
    pub path: String,
    pub scope: Option<Scope>,
    pub anchor: Option<Anchor>,
    /// The text before the anchor, as written and trimmed: what a re-pin edits
    /// in place, so a scope keeps its spelling.
    pub spelling: String,
}

impl Locator {
    /// The locator as shown in reports: the normalised path and the scope.
    pub fn display(&self) -> String {
        match &self.scope {
            Some(scope) => format!("{}#{scope}", self.path),
            None => self.path.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// 1-based inclusive lines.
    Range {
        start: usize,
        end: usize,
    },
    Symbol(Vec<Segment>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Name(String),
    /// `<Trait>`: a trait impl of the container before it.
    Trait(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Anchor {
    /// `@sha256:<64 lowercase hex>`: a content pin (`ADR-67`).
    Pin(String),
    /// `@<7 to 64 lowercase hex>`: the commit anchor reserved for `MILE-84`.
    Commit(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LocatorError {
    #[error("malformed line range '#{0}': a range is `L<n>` or `L<n>-L<m>`, decimal with no leading zero, 1 <= n <= m")]
    MalformedRange(String),
    #[error("malformed pin '{0}': a pin is `@sha256:` and exactly 64 lowercase hex digits")]
    MalformedPin(String),
    #[error("carries both a `@sha256:` pin and a commit anchor; a locator takes one anchor")]
    BothAnchors,
    #[error("'{0}' is a glob, and a glob cannot carry a scope")]
    GlobWithScope(String),
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scope::Range { start, end } if start == end => write!(f, "L{start}"),
            Scope::Range { start, end } => write!(f, "L{start}-L{end}"),
            Scope::Symbol(segments) => {
                for (i, seg) in segments.iter().enumerate() {
                    if i > 0 {
                        f.write_str("::")?;
                    }
                    match seg {
                        Segment::Name(n) => f.write_str(n)?,
                        Segment::Trait(t) => write!(f, "<{t}>")?,
                    }
                }
                Ok(())
            }
        }
    }
}

fn is_lower_hex(s: &str) -> bool {
    s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// A trailing anchor and what precedes it, in the order `ADR-74` fixes: a
/// `@sha256:` pin of exactly 64 lowercase hex, else a `@` and 7 to 64
/// lowercase hex (a commit).
fn split_anchor(text: &str) -> (&str, Option<Anchor>) {
    if let Some((rest, hash)) = text.rsplit_once(PIN_MARKER) {
        if hash.len() == 64 && is_lower_hex(hash) {
            return (rest, Some(Anchor::Pin(hash.to_string())));
        }
    }
    if let Some((rest, hex)) = text.rsplit_once('@') {
        if (7..=64).contains(&hex.len()) && is_lower_hex(hex) {
            return (rest, Some(Anchor::Commit(hex.to_string())));
        }
    }
    (text, None)
}

fn is_seg(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

fn parse_symbol(text: &str) -> Option<Vec<Segment>> {
    let mut segments = Vec::new();
    for (i, piece) in text.split("::").enumerate() {
        if let Some(inner) = piece.strip_prefix('<').and_then(|p| p.strip_suffix('>')) {
            // `<Trait>` is never the first segment.
            if i == 0 || !is_seg(inner) {
                return None;
            }
            segments.push(Segment::Trait(inner.to_string()));
        } else if is_seg(piece) {
            segments.push(Segment::Name(piece.to_string()));
        } else {
            return None;
        }
    }
    Some(segments)
}

fn parse_number(s: &str) -> Option<usize> {
    if s.is_empty() || s.starts_with('0') || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn parse_range(tail: &str) -> Result<Scope, LocatorError> {
    let bad = || LocatorError::MalformedRange(tail.to_string());
    let body = tail.strip_prefix('L').ok_or_else(bad)?;
    let (start, end) = match body.split_once("-L") {
        Some((a, b)) => (
            parse_number(a).ok_or_else(bad)?,
            parse_number(b).ok_or_else(bad)?,
        ),
        None => {
            let n = parse_number(body).ok_or_else(bad)?;
            (n, n)
        }
    };
    if start > end {
        return Err(bad());
    }
    Ok(Scope::Range { start, end })
}

/// Parses one locator (the text after `spec:`, `code:` or `test:`), in the
/// order `ADR-74` point 1 fixes: (1) a trailing `@sha256:` pin, (2) otherwise a
/// trailing commit anchor, (3) in what remains, the text after the last `#`
/// as the scope only when it matches `scope`; otherwise the whole text is the
/// path. A scope beginning `L` and a digit is a range or malformed, never a
/// symbol. Pure.
pub fn parse_locator(raw: &str) -> Result<Locator, LocatorError> {
    let raw = raw.trim();
    let (rest, anchor) = split_anchor(raw);
    if anchor.is_some() {
        // The other kind of anchor still sitting before this one.
        let (_, second) = split_anchor(rest);
        if second.is_some() {
            return Err(LocatorError::BothAnchors);
        }
    }
    if let Some(at) = rest.rfind(PIN_MARKER) {
        return Err(LocatorError::MalformedPin(rest[at..].to_string()));
    }
    let mut path = rest;
    let mut scope = None;
    if let Some((before, tail)) = rest.rsplit_once('#') {
        let mut chars = tail.chars();
        let range_like =
            chars.next() == Some('L') && chars.next().is_some_and(|c| c.is_ascii_digit());
        if range_like {
            scope = Some(parse_range(tail)?);
            path = before;
        } else if let Some(segments) = parse_symbol(tail) {
            scope = Some(Scope::Symbol(segments));
            path = before;
        }
    }
    if scope.is_some() && crate::rules::is_path_glob(path) {
        return Err(LocatorError::GlobWithScope(path.to_string()));
    }
    Ok(Locator {
        path: crate::rules::normalize_locator(path),
        scope,
        anchor,
        spelling: rest.trim().to_string(),
    })
}

/// One `Realized-by` segment: its category, the locator text as written and
/// the parse of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatorEntry {
    pub category: String,
    pub raw: String,
    pub parsed: Result<Locator, LocatorError>,
}

/// Every locator of a `Realized-by` value, in the order written. An
/// unrecognised category is dropped, as everywhere else.
pub fn realized_by_locators(value: &str) -> Vec<LocatorEntry> {
    realized_by_entries(value)
        .into_iter()
        .map(|(category, locator)| LocatorEntry {
            category: category.to_string(),
            raw: locator.trim().to_string(),
            parsed: parse_locator(locator),
        })
        .collect()
}

/// The categories a `Realized-by` entry can carry: the declared evidence
/// categories.
const CATEGORIES: [&str; 3] = ["spec", "code", "test"];

/// True when `rest` begins an entry: a category, then `:`.
fn starts_entry(rest: &str) -> bool {
    rest.trim_start()
        .split_once(':')
        .is_some_and(|(prefix, _)| CATEGORIES.contains(&prefix.trim()))
}

/// Splits a `Realized-by` value into `(category, locator text)` entries, the
/// one splitter every reader of the field shares. A comma separates entries
/// only when a `category:` prefix follows it; any other comma belongs to the
/// path (`BUG-129`). Text before the first category prefix belongs to no
/// entry. A path that itself contains `,<category>:` is the one ambiguous
/// shape; it reads as two entries.
pub fn realized_by_entries(value: &str) -> Vec<(&str, &str)> {
    let mut starts = Vec::new();
    if starts_entry(value) {
        starts.push(0);
    }
    for (i, _) in value.match_indices(',') {
        if starts_entry(&value[i + 1..]) {
            starts.push(i + 1);
        }
    }
    let mut out = Vec::new();
    for (n, &start) in starts.iter().enumerate() {
        let end = starts.get(n + 1).map_or(value.len(), |next| next - 1);
        if let Some((prefix, locator)) = value[start..end].split_once(':') {
            out.push((prefix.trim(), locator));
        }
    }
    out
}

/// The non-blank text of a `Realized-by` value that belongs to no recognised
/// entry: everything before the first `spec:`/`code:`/`test:` prefix, or the
/// whole value when it holds none (a bare path, `Code:`, `tests:`). The shared
/// splitter drops it; `embodiment.locator-exists` reports it.
pub fn realized_by_unrecognised(value: &str) -> Option<&str> {
    let mut first = None;
    if starts_entry(value) {
        first = Some(0);
    } else {
        for (i, _) in value.match_indices(',') {
            if starts_entry(&value[i + 1..]) {
                first = Some(i);
                break;
            }
        }
    }
    let lead = value[..first.unwrap_or(value.len())]
        .trim()
        .trim_end_matches(',')
        .trim();
    (!lead.is_empty()).then_some(lead)
}

/// What a locator path names among the tracked files (`BUG-176`): its kind and
/// the files it covers, sorted. `kind` is `None` when the path names nothing
/// (no tracked file, no tracked file below it, and not a glob).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatorFiles {
    pub kind: Option<crate::config::LocatorKind>,
    pub files: Vec<String>,
}

// BUG-176: a directory or glob locator is the tracked files it matches
/// Classifies `path` and lists the tracked files it covers. A path that is
/// itself a file (`exists` holds for it) is a file, whatever characters it holds; otherwise a
/// path with a glob character is a glob (matched with the `MILE-158` matcher),
/// and any other is a directory when a tracked file sits below it. Only files
/// for which `exists` holds count, so a staged deletion matches nothing (`exists`
/// must itself require the path be tracked). An
/// `Err` is a glob that cannot compile. Pure.
pub fn match_locator_files(
    path: &str,
    tracked: &[std::path::PathBuf],
    exists: &dyn Fn(&str) -> bool,
) -> Result<LocatorFiles, String> {
    use crate::config::LocatorKind;
    let live = |p: &std::path::PathBuf| p.to_str().filter(|s| exists(s)).map(str::to_string);
    if exists(path) {
        return Ok(LocatorFiles {
            kind: Some(LocatorKind::File),
            files: vec![path.to_string()],
        });
    }
    let (kind, mut files): (LocatorKind, Vec<String>) = if crate::rules::is_path_glob(path) {
        let re = crate::rules::path_glob_regex(path)?;
        (
            LocatorKind::Glob,
            tracked
                .iter()
                .filter(|p| re.is_match(&p.to_string_lossy()))
                .filter_map(live)
                .collect(),
        )
    } else {
        (
            LocatorKind::Directory,
            tracked
                .iter()
                .filter(|p| crate::rules::path_is_published(p, path))
                .filter_map(live)
                .collect(),
        )
    };
    files.sort();
    files.dedup();
    // A directory with nothing below it is not a directory; a glob is a glob
    // whether or not it matches.
    let kind = (kind == LocatorKind::Glob || !files.is_empty()).then_some(kind);
    Ok(LocatorFiles { kind, files })
}

/// The text a directory or glob pin hashes (`BUG-176`): one line per covered
/// file, `<path>\t<sha256 of the file>`, sorted by path. `pairs` is the files
/// with their content hashes. The pin is the hash of this text, so it moves
/// when a file's content changes, a file is added or removed, or a file moves.
pub fn files_pin_text(pairs: &[(String, String)]) -> String {
    let mut sorted: Vec<&(String, String)> = pairs.iter().collect();
    sorted.sort();
    sorted
        .iter()
        .map(|(path, hash)| format!("{path}\t{hash}\n"))
        .collect()
}

/// 1-based inclusive lines of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// A file as the CLI read it (core touches no filesystem). Shared so one read
/// serves the existence, drift and re-baseline call sites (`ADR-74` point 9).
pub type SharedText = std::rc::Rc<FileText>;

/// A reader that finds nothing; for callers with no filesystem (property tests).
pub fn no_file_text(_path: &str) -> SharedText {
    std::rc::Rc::new(FileText::Missing)
}

/// A file as the CLI read it (core touches no filesystem).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileText {
    Missing,
    NotUtf8,
    Text(String),
}

/// Why a scope did not resolve (`ADR-74` point 6). The message is the reason
/// `embodiment.locator-exists` reports.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScopeError {
    #[error("the file is not valid UTF-8, so it cannot carry a scoped pin")]
    NotUtf8,
    #[error("no locator resolver covers this file, so a symbol scope is unsupported for its type (a line range always works)")]
    NoResolver,
    #[error("more than one locator resolver covers this file ({0}); declare one")]
    MultipleResolvers(String),
    #[error("{0}")]
    NotFound(String),
    #[error("ambiguous: {} candidates, at lines {}", lines.len(), lines.iter().map(usize::to_string).collect::<Vec<_>>().join(", "))]
    Ambiguous { lines: Vec<usize> },
    #[error("the end of the item is not found (its end kind never closes before the end of the file or its container)")]
    EndNotFound,
    #[error("the range lies beyond the end of the file, which has {lines} lines")]
    RangeBeyondEnd { lines: usize },
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("commit anchor not yet supported")]
    CommitAnchor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EndKind {
    BraceBalanced,
    Indent,
    Heading,
    BlankLine,
    Semicolon,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Spans {
    /// Exactly one span must match.
    #[default]
    One,
    /// Every match is a candidate span (a type with several `impl` blocks).
    All,
}

/// A container of members: its own start pattern (`{symbol}` placeholder, plus
/// `{trait}` for a `trait_container`), end kind and span multiplicity.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerDecl {
    pub start: String,
    pub end: EndKind,
    #[serde(default)]
    pub spans: Spans,
}

/// A member found at direct-child depth inside a container span, with its own
/// start pattern and end kind.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MemberDecl {
    pub start: String,
    pub end: EndKind,
}

/// A repository-declared symbol resolver (`ADR-74` point 5; `SPEC-3`
/// `locator_resolvers`). The engine ships no language knowledge.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocatorResolver {
    pub name: String,
    /// The files this resolver covers.
    pub glob: String,
    /// Recognises the first line of a plain symbol; `{symbol}` is substituted
    /// after regular-expression escaping.
    pub start: String,
    pub end: EndKind,
    /// Contiguous lines directly above the first line that match are part of
    /// the span (attributes, doc comments). Without it they are not pinned.
    #[serde(default)]
    pub leading: Option<String>,
    #[serde(default)]
    pub container: Option<ContainerDecl>,
    /// The item inside a container span. One mapping, or a list of mappings
    /// when members end differently (a Rust `fn` is `brace_balanced`, a struct
    /// field is `indent`): a name is looked for with every pattern, and the
    /// matches are counted together.
    #[serde(
        default,
        rename = "member",
        deserialize_with = "one_or_many",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub members: Vec<MemberDecl>,
    /// Like `container`, with `{trait}` and `{symbol}` placeholders, for
    /// `Type::<Trait>` and `Type::<Trait>::member`.
    #[serde(default)]
    pub trait_container: Option<ContainerDecl>,
}

fn one_or_many<'de, D>(d: D) -> Result<Vec<MemberDecl>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(MemberDecl),
        Many(Vec<MemberDecl>),
    }
    Ok(match OneOrMany::deserialize(d)? {
        OneOrMany::One(m) => vec![m],
        OneOrMany::Many(v) => v,
    })
}

const SYMBOL: &str = "{symbol}";
const TRAIT: &str = "{trait}";

/// A regex compiled once per distinct pattern text and shared by clone. The
/// key is the whole pattern, so two configs that declare different resolvers
/// never see each other's entry; a pattern that does not compile is cached as
/// the error message, so the failure is reported the same way every time.
fn cached_regex(
    pattern: &str,
    on_error: impl Fn(&regex::Error) -> String,
) -> Result<regex::Regex, String> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<String, Result<regex::Regex, String>>>> = OnceLock::new();
    let compiled = || regex::Regex::new(pattern).map_err(|e| on_error(&e));
    match CACHE.get_or_init(Default::default).lock() {
        Ok(mut cache) => cache
            .entry(pattern.to_string())
            .or_insert_with(compiled)
            .clone(),
        Err(_) => compiled(),
    }
}

fn compile(
    template: &str,
    symbol: &str,
    trait_name: Option<&str>,
) -> Result<regex::Regex, ScopeError> {
    let mut pattern = template.replace(SYMBOL, &regex::escape(symbol));
    if let Some(t) = trait_name {
        pattern = pattern.replace(TRAIT, &regex::escape(t));
    }
    cached_regex(&pattern, |e| {
        format!("a resolver pattern does not compile: {e}")
    })
    .map_err(ScopeError::Unsupported)
}

/// The anchored regex of a resolver glob, compiled once per glob text.
fn glob_regex(glob: &str) -> Option<regex::Regex> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<String, Option<regex::Regex>>>> = OnceLock::new();
    let build = || crate::rules::path_glob_regex(glob).ok();
    match CACHE.get_or_init(Default::default).lock() {
        Ok(mut cache) => cache.entry(glob.to_string()).or_insert_with(build).clone(),
        Err(_) => build(),
    }
}

/// Config-load validation (`ADR-64` clause 5): a malformed glob or pattern, a
/// start without its placeholder, a container end kind that cannot track
/// direct children, a member with nothing to be a member of, two resolvers
/// with one name or one glob. A file covered by two different globs is
/// reported where it is resolved, since the file set is not known here.
pub fn validate_resolvers(resolvers: &[LocatorResolver]) -> Result<(), String> {
    let check = |what: &str, template: &str, trait_too: bool| -> Result<(), String> {
        if !template.contains(SYMBOL) {
            return Err(format!("{what} must contain the `{SYMBOL}` placeholder"));
        }
        if trait_too && !template.contains(TRAIT) {
            return Err(format!("{what} must contain the `{TRAIT}` placeholder"));
        }
        compile(template, "x", trait_too.then_some("T"))
            .map(|_| ())
            .map_err(|_| format!("{what} is not a valid pattern"))
    };
    for (i, r) in resolvers.iter().enumerate() {
        let n = &r.name;
        if n.trim().is_empty() {
            return Err("a resolver needs a non-empty `name`".to_string());
        }
        if resolvers[..i].iter().any(|o| o.name == r.name) {
            return Err(format!("two resolvers are named '{n}'"));
        }
        if let Some(o) = resolvers[..i].iter().find(|o| o.glob == r.glob) {
            return Err(format!(
                "resolvers '{}' and '{n}' declare the same glob '{}': a file is covered by exactly one",
                o.name, r.glob
            ));
        }
        if crate::rules::path_glob_regex(&r.glob).is_err() {
            return Err(format!(
                "resolver '{n}' glob '{}' is not a valid glob",
                r.glob
            ));
        }
        check(&format!("resolver '{n}' start"), &r.start, false)?;
        if let Some(l) = &r.leading {
            regex::Regex::new(l)
                .map_err(|e| format!("resolver '{n}' leading pattern is not valid: {e}"))?;
        }
        if let Some(c) = &r.container {
            check(&format!("resolver '{n}' container start"), &c.start, false)?;
        }
        if let Some(c) = &r.trait_container {
            check(
                &format!("resolver '{n}' trait_container start"),
                &c.start,
                true,
            )?;
        }
        for m in &r.members {
            check(&format!("resolver '{n}' member start"), &m.start, false)?;
        }
        for (what, c) in [
            ("container", &r.container),
            ("trait_container", &r.trait_container),
        ] {
            if let Some(c) = c {
                if !matches!(
                    c.end,
                    EndKind::BraceBalanced | EndKind::Indent | EndKind::Heading
                ) {
                    return Err(format!(
                        "resolver '{n}' {what} end must be brace_balanced, indent or heading: only those say which lines are direct children"
                    ));
                }
            }
        }
        if !r.members.is_empty() && r.container.is_none() && r.trait_container.is_none() {
            return Err(format!(
                "resolver '{n}' declares a member but no container or trait_container to find it in"
            ));
        }
        if r.members.is_empty() && r.container.is_some() {
            return Err(format!(
                "resolver '{n}' declares a container but no member to find in it"
            ));
        }
    }
    Ok(())
}

/// The resolver whose glob covers `path`: none, one, or an error when several
/// do (never the first).
pub fn select_resolver<'a>(
    resolvers: &'a [LocatorResolver],
    path: &str,
) -> Result<Option<&'a LocatorResolver>, ScopeError> {
    let covering: Vec<&LocatorResolver> = resolvers
        .iter()
        .filter(|r| glob_regex(&r.glob).is_some_and(|re| re.is_match(path)))
        .collect();
    match covering.as_slice() {
        [] => Ok(None),
        [one] => Ok(Some(one)),
        many => Err(ScopeError::MultipleResolvers(
            many.iter()
                .map(|r| r.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        )),
    }
}

/// The lines of a text: a leading BOM dropped, split on `\n`, at most one
/// trailing `\r` removed per line, and no empty last line for a final newline.
fn lines_of(text: &str) -> Vec<&str> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines: Vec<&str> = text
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();
    if text.is_empty() || text.ends_with('\n') {
        lines.pop();
    }
    lines
}

/// The pinned bytes of a span (`ADR-74` point 3): the span's lines, each with
/// at most one trailing `\r` removed, joined by `\n`, no trailing newline.
pub fn span_text(text: &str, span: Span) -> String {
    let lines = lines_of(text);
    let from = span.start.saturating_sub(1).min(lines.len());
    let to = span.end.min(lines.len());
    lines[from..to].join("\n")
}

fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}

fn indent_of(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).count()
}

fn heading_level(line: &str) -> Option<usize> {
    let hashes = line.bytes().take_while(|b| *b == b'#').count();
    let rest = &line[hashes..];
    ((1..=6).contains(&hashes) && (rest.is_empty() || rest.starts_with(' '))).then_some(hashes)
}

/// Brace depth at the end of each line, relative to the start of `lines`.
fn brace_delta(line: &str) -> i64 {
    line.chars().fold(0, |d, c| match c {
        '{' => d + 1,
        '}' => d - 1,
        _ => d,
    })
}

/// The last line (0-based) of the item starting at `first`, never past
/// `limit`, by `kind` (`ADR-74` point 5). `None` when the kind cannot find an
/// end.
fn end_of(kind: EndKind, lines: &[&str], first: usize, limit: usize) -> Option<usize> {
    match kind {
        EndKind::BraceBalanced => {
            let (mut depth, mut opened) = (0i64, false);
            for (i, line) in lines.iter().enumerate().take(limit + 1).skip(first) {
                opened |= line.contains('{');
                depth += brace_delta(line);
                if opened && depth <= 0 {
                    return Some(i);
                }
            }
            None
        }
        EndKind::Indent => {
            let base = indent_of(lines[first]);
            let mut last = first;
            for (i, line) in lines.iter().enumerate().take(limit + 1).skip(first + 1) {
                if is_blank(line) {
                    continue;
                }
                if indent_of(line) <= base {
                    break;
                }
                last = i;
            }
            Some(last)
        }
        EndKind::Heading => {
            let level = heading_level(lines[first])?;
            let mut last = first;
            for (i, line) in lines.iter().enumerate().take(limit + 1).skip(first + 1) {
                if heading_level(line).is_some_and(|l| l <= level) {
                    break;
                }
                if !is_blank(line) {
                    last = i;
                }
            }
            Some(last)
        }
        EndKind::BlankLine => {
            for (i, line) in lines.iter().enumerate().take(limit + 1).skip(first + 1) {
                if is_blank(line) {
                    return Some(i - 1);
                }
            }
            Some(limit.min(lines.len().saturating_sub(1)))
        }
        EndKind::Semicolon => {
            let mut depth = 0i64;
            for (i, line) in lines.iter().enumerate().take(limit + 1).skip(first) {
                depth += brace_delta(line);
                if depth == 0 && line.trim_end().ends_with(';') {
                    return Some(i);
                }
            }
            None
        }
    }
}

/// A candidate span, 0-based inclusive, before `leading` is applied.
type Raw = (usize, usize);

struct Resolution<'a> {
    lines: &'a [&'a str],
    leading: Option<regex::Regex>,
}

impl Resolution<'_> {
    fn finish(&self, (first, end): Raw) -> Span {
        let mut start = first;
        if let Some(re) = &self.leading {
            while start > 0 && re.is_match(self.lines[start - 1]) {
                start -= 1;
            }
        }
        Span {
            start: start + 1,
            end: end + 1,
        }
    }

    fn matching(&self, re: &regex::Regex, from: usize, to: usize) -> Vec<usize> {
        (from..=to)
            .filter(|i| re.is_match(self.lines[*i]))
            .collect()
    }

    /// The spans a container declaration matches for `symbol` (and `trait`).
    fn container_spans(
        &self,
        decl: &ContainerDecl,
        symbol: &str,
        trait_name: Option<&str>,
        what: &str,
    ) -> Result<Vec<Raw>, ScopeError> {
        let re = compile(&decl.start, symbol, trait_name)?;
        let last = self.lines.len().saturating_sub(1);
        let starts = if self.lines.is_empty() {
            Vec::new()
        } else {
            self.matching(&re, 0, last)
        };
        if starts.is_empty() {
            return Err(ScopeError::NotFound(format!("{what} not found")));
        }
        if decl.spans == Spans::One && starts.len() > 1 {
            return Err(ScopeError::Ambiguous {
                lines: starts.iter().map(|i| i + 1).collect(),
            });
        }
        starts
            .into_iter()
            .map(|s| {
                end_of(decl.end, self.lines, s, last)
                    .map(|e| (s, e))
                    .ok_or(ScopeError::EndNotFound)
            })
            .collect()
    }

    /// The lines of a container span that are its direct children.
    fn direct_children(&self, kind: EndKind, (first, end): Raw) -> Vec<usize> {
        let inside = first + 1..=end;
        match kind {
            EndKind::BraceBalanced => {
                let mut depth = brace_delta(self.lines[first]);
                let mut out = Vec::new();
                for i in inside {
                    if depth == 1 {
                        out.push(i);
                    }
                    depth += brace_delta(self.lines[i]);
                }
                out
            }
            EndKind::Indent => {
                let child = inside
                    .clone()
                    .find(|i| !is_blank(self.lines[*i]))
                    .map(|i| indent_of(self.lines[i]));
                inside
                    .filter(|i| {
                        Some(indent_of(self.lines[*i])) == child && !is_blank(self.lines[*i])
                    })
                    .collect()
            }
            EndKind::Heading => {
                let level = heading_level(self.lines[first]).map(|l| l + 1);
                inside
                    .filter(|i| level.is_some() && heading_level(self.lines[*i]) == level)
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn member_in(
        &self,
        resolver: &LocatorResolver,
        container_kind: EndKind,
        spans: &[Raw],
        name: &str,
    ) -> Result<Span, ScopeError> {
        if resolver.members.is_empty() {
            return Err(ScopeError::Unsupported(format!(
                "resolver '{}' declares no member pattern",
                resolver.name
            )));
        }
        let patterns: Vec<regex::Regex> = resolver
            .members
            .iter()
            .map(|m| compile(&m.start, name, None))
            .collect::<Result<_, _>>()?;
        // (line, container span, which member pattern matched it). A line two
        // patterns both match is two candidates, never first-pattern-wins.
        let mut found: Vec<(usize, Raw, usize)> = Vec::new();
        for span in spans {
            for line in self.direct_children(container_kind, *span) {
                for (k, re) in patterns.iter().enumerate() {
                    if re.is_match(self.lines[line]) {
                        found.push((line, *span, k));
                    }
                }
            }
        }
        match found.as_slice() {
            [] => Err(ScopeError::NotFound(format!("member '{name}' not found"))),
            [(line, container, k)] => {
                end_of(resolver.members[*k].end, self.lines, *line, container.1)
                    .map(|e| self.finish((*line, e)))
                    .ok_or(ScopeError::EndNotFound)
            }
            many => Err(ScopeError::Ambiguous {
                lines: many.iter().map(|(l, _, _)| l + 1).collect(),
            }),
        }
    }
}

/// Resolves a scope to the span of whole lines it names (`ADR-74` points 4-5).
/// A range needs no resolver. A symbol is found by `resolver`, which is
/// `None` when no resolver covers the file. Ambiguity is an error, never the
/// first match. Pure.
pub fn resolve_scope(
    text: &str,
    resolver: Option<&LocatorResolver>,
    scope: &Scope,
) -> Result<Span, ScopeError> {
    if matches!(scope, Scope::Range { .. }) {
        return resolve_scope_uncached(text, resolver, scope);
    }
    // A check resolves the same symbol scope in the same text twice (the rule
    // that checks it resolves, then the drift judge reads its pin), and a
    // resolution scans every line. The result is a function of exactly
    // (text, resolver, scope). The memo keeps no copy of the text: it keys on
    // the text's SHA-256 and compares the resolver and scope in full on a hit,
    // so a different config or file text never sees another's result. A
    // SHA-256 collision between two different file texts is not a practical
    // concern (the pins already rely on the same hash to tell texts apart), so
    // equal digests are taken to mean equal text.
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    struct Entry {
        resolver: Option<LocatorResolver>,
        scope: Scope,
        result: Result<Span, ScopeError>,
    }
    static MEMO: OnceLock<Mutex<HashMap<[u8; 32], Vec<Entry>>>> = OnceLock::new();
    let key: [u8; 32] = Sha256::digest(text.as_bytes()).into();
    let Ok(mut memo) = MEMO.get_or_init(Default::default).lock() else {
        return resolve_scope_uncached(text, resolver, scope);
    };
    let bucket = memo.entry(key).or_default();
    if let Some(hit) = bucket
        .iter()
        .find(|e| e.resolver.as_ref() == resolver && &e.scope == scope)
    {
        return hit.result.clone();
    }
    let result = resolve_scope_uncached(text, resolver, scope);
    bucket.push(Entry {
        resolver: resolver.cloned(),
        scope: scope.clone(),
        result: result.clone(),
    });
    result
}

fn resolve_scope_uncached(
    text: &str,
    resolver: Option<&LocatorResolver>,
    scope: &Scope,
) -> Result<Span, ScopeError> {
    let lines = lines_of(text);
    let segments = match scope {
        Scope::Range { start, end } => {
            if *end > lines.len() {
                return Err(ScopeError::RangeBeyondEnd { lines: lines.len() });
            }
            return Ok(Span {
                start: *start,
                end: *end,
            });
        }
        Scope::Symbol(segments) => segments,
    };
    let resolver = resolver.ok_or(ScopeError::NoResolver)?;
    let cx = Resolution {
        lines: &lines,
        leading: resolver
            .leading
            .as_deref()
            .map(|l| {
                cached_regex(l, |e| format!("the leading pattern does not compile: {e}"))
                    .map_err(ScopeError::Unsupported)
            })
            .transpose()?,
    };
    let name = |s: &Segment| match s {
        Segment::Name(n) | Segment::Trait(n) => n.clone(),
    };
    let unsupported = |why: &str| {
        Err(ScopeError::Unsupported(format!(
            "{why} (resolver '{}')",
            resolver.name
        )))
    };
    match segments.as_slice() {
        [Segment::Name(symbol)] => {
            let re = compile(&resolver.start, symbol, None)?;
            if lines.is_empty() {
                return Err(ScopeError::NotFound(format!("'{symbol}' matches nothing")));
            }
            let hits = cx.matching(&re, 0, lines.len() - 1);
            match hits.as_slice() {
                [] => Err(ScopeError::NotFound(format!("'{symbol}' matches nothing"))),
                [first] => end_of(resolver.end, &lines, *first, lines.len() - 1)
                    .map(|e| cx.finish((*first, e)))
                    .ok_or(ScopeError::EndNotFound),
                many => Err(ScopeError::Ambiguous {
                    lines: many.iter().map(|i| i + 1).collect(),
                }),
            }
        }
        [Segment::Name(container), Segment::Name(member)] => {
            let Some(decl) = &resolver.container else {
                return unsupported("a member of a type needs a `container` declaration");
            };
            let spans =
                cx.container_spans(decl, container, None, &format!("container '{container}'"))?;
            cx.member_in(resolver, decl.end, &spans, member)
        }
        [Segment::Name(container), Segment::Trait(trait_name)]
        | [Segment::Name(container), Segment::Trait(trait_name), Segment::Name(_)] => {
            let Some(decl) = &resolver.trait_container else {
                return unsupported("the `::<Trait>::` form needs a `trait_container` declaration");
            };
            let spans = cx.container_spans(
                decl,
                container,
                Some(trait_name),
                &format!("impl of '{trait_name}' for '{container}'"),
            )?;
            match segments.get(2) {
                None => match spans.as_slice() {
                    [one] => Ok(cx.finish(*one)),
                    many => Err(ScopeError::Ambiguous {
                        lines: many.iter().map(|(s, _)| s + 1).collect(),
                    }),
                },
                Some(m) => cx.member_in(resolver, decl.end, &spans, &name(m)),
            }
        }
        _ => unsupported("only one container level is supported; a member of a member is not"),
    }
}

#[cfg(test)]
mod tests;
