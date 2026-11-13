//! The record header as a closed structure (RFC-0010): located by anchoring
//! to the metadata shape, never a positional slice, and parsed by exactly one
//! function so no check runs its own regex over raw text.

/// One field as found in the header region, with its line number (1-indexed,
/// relative to the whole document) for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderField {
    pub key: crate::values::FieldName,
    pub value: String,
    pub line: usize,
}

/// Not `Option<&str>`: that has an `.unwrap_or(...)` a caller can reach for
/// without deciding what a miss means (`RFC-40`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldRead<'a> {
    Present(&'a str),
    Missing,
    Unreadable,
}

/// The parsed header: an ordered list of fields plus the region's own
/// boundaries, so closure questions (duplicate keys, "is this the whole
/// header") have something to be asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub fields: Vec<HeaderField>,
    /// 1-indexed, inclusive line range of the header region. `None` if no
    /// header-shaped region was found at all -- a reportable finding, never
    /// an empty result silently treated as "no fields."
    pub region: Option<(usize, usize)>,
    /// Why `region` is `None`, when there is a specific reason worth
    /// surfacing verbatim (BUG-0012): the YAML deserializer's own line and
    /// reason, or the name of a removed header form the record is still
    /// written in (MILE-85). `None` whenever `region` is `Some`, and `None`
    /// for a plain "nothing header-shaped found".
    pub parse_error: Option<String>,
}

impl Header {
    /// No header-shaped region was found, or it was but failed to parse
    /// (`region: None` covers both -- see the field's own doc comment). A
    /// candidate in this state has no value to classify: `Outcome::Unreadable`,
    /// never `Outcome::Absent` (`BUG-125`). One method rather than each rule
    /// re-deriving the same check keeps the nine call sites that need it from
    /// drifting apart.
    pub fn is_unreadable(&self) -> bool {
        self.region.is_none()
    }

    /// First value for `key`, if present. A repeated key is a closure
    /// violation the caller should check for separately via
    /// [`Header::duplicate_keys`] -- this method deliberately doesn't hide
    /// that by picking one silently.
    ///
    /// Exact (`ADR-57`): matching loosely here made two rules disagree about
    /// one field -- `header.field-set-consistency` called `status` undeclared
    /// while `header.required-fields` read it as satisfying `Status`.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|f| f.key.as_str() == key)
            .map(|f| f.value.as_str())
    }

    /// `get`, folding case. For the engine's own reserved keys --
    /// `Stable-Id`, `Rule`, `Scope`, `Expires` -- never the adopter's
    /// declared vocabulary.
    ///
    /// `ADR-57`'s exact match is about a field the *adopter* named, where two
    /// spellings could be two different things (`Maße`/`Masse`) and folding
    /// them is a guess the engine has no standing to make. A reserved key is
    /// not adopter vocabulary at all: there is exactly one field named
    /// `Stable-Id`, the engine itself invented the name, and a record
    /// spelling it `stable-id` has made a typo, not declared a different
    /// field. Reading it exactly here does not honour a declaration -- it
    /// silently accepts a second, conflicting write (`migrate ids`) or drops
    /// a waiver that should apply (`waiver::load_suppressions`), which is the
    /// defect class `ADR-57` was written to close, reopened for a case
    /// `ADR-57`'s own reasoning never covered.
    pub fn get_reserved(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|f| f.key.as_str().eq_ignore_ascii_case(key))
            .map(|f| f.value.as_str())
    }

    /// `get`, with the miss case decided once (`RFC-40`) instead of per
    /// caller. A differently-cased key is still `Missing`: `header
    /// .field-case-mismatch` is the one place that reports it.
    pub fn read_declared(&self, key: &str) -> FieldRead<'_> {
        if self.is_unreadable() {
            return FieldRead::Unreadable;
        }
        match self.get(key) {
            Some(value) => FieldRead::Present(value),
            None => FieldRead::Missing,
        }
    }

    /// Exact (`ADR-57`): `Status` and `status` are two fields, so writing both
    /// is not this defect. The undeclared one is reported by
    /// `header.field-set-consistency`, which names the declared spelling.
    pub fn duplicate_keys(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut dupes = Vec::new();
        for f in &self.fields {
            if !seen.insert(&f.key) && !dupes.contains(&f.key.to_string()) {
                dupes.push(f.key.to_string());
            }
        }
        dupes
    }
}

/// How a type's records carry their header (RFC-0010: declared per type, never
/// sniffed).
///
/// `yaml-frontmatter` is the one line-of-text shape: `---`-delimited YAML at
/// the top of the document, deserialized structurally. The blockquote
/// (`> Key: Value`) and bold-list (`- **Key:** Value`) shapes were removed
/// (MILE-85, ADR-33): a config declaring either fails to load, and a record
/// still written in one is reported by name (see [`legacy_header_form`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeaderShape {
    /// `---`-delimited YAML at the top of the document (RFC-0016/ADR-0017).
    #[default]
    YamlFrontmatter,
    /// This type's records carry no header block at all (`ADR-50`): a
    /// Nygard-style corpus whose metadata lives in prose, not a parseable
    /// shape. `header.required-fields` skips such a type rather than
    /// reporting a missing region on every record.
    None,
}

pub fn parse(content: &str) -> Header {
    parse_with_shape(content, HeaderShape::default())
}

/// Parse the header using an explicitly declared shape (RFC-0010: the shape
/// is declared per profile, never sniffed).
pub fn parse_with_shape(content: &str, shape: HeaderShape) -> Header {
    match shape {
        HeaderShape::YamlFrontmatter => parse_yaml_frontmatter(content),
        // A `none`-shaped type declares it has nowhere for a header to be
        // (`ADR-50`): no region, no fields, no parse error -- there is
        // nothing here to have failed to parse.
        HeaderShape::None => Header {
            fields: Vec::new(),
            region: None,
            parse_error: None,
        },
    }
}

/// The removed header form a document is still written in, when its top
/// (before the first `##` heading) carries one: `> Key: value` or
/// `> **Key:** value` blockquote lines, or `- **Key:** value` bold-list lines.
/// The form with more such lines wins (a body Y-statement quote is one line
/// beside a real header's several; an equal count goes to the earlier form),
/// and `None` means neither appears, so a document with no header at all is
/// not mislabelled. Used only to name the form in a diagnostic; nothing here
/// reads a field (MILE-85).
pub fn legacy_header_form(content: &str) -> Option<&'static str> {
    let (mut quote, mut bold) = (0usize, 0usize);
    let mut first_quote = usize::MAX;
    let mut first_bold = usize::MAX;
    for (idx, line) in content.lines().enumerate() {
        let t = line.trim_start();
        if t.starts_with("##") && t.trim_start_matches('#').starts_with([' ', '\t']) {
            break;
        }
        if let Some(rest) = t.strip_prefix('>') {
            let rest = rest.trim_start();
            if is_key_value(rest) || rest.starts_with("**") && rest.contains(":**") {
                quote += 1;
                first_quote = first_quote.min(idx);
            }
        } else if let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
            if rest.starts_with("**") && rest.contains(":**") {
                bold += 1;
                first_bold = first_bold.min(idx);
            }
        }
    }
    match (quote, bold) {
        (0, 0) => None,
        (q, b) if q > b || (q == b && first_quote < first_bold) => Some("blockquote"),
        _ => Some("bold-list"),
    }
}

/// `Key: value` with a plain key (letters, digits, space, `-`, `_`, `/`),
/// the colon followed by a space or the end of the line.
fn is_key_value(s: &str) -> bool {
    let Some((key, value)) = s.split_once(':') else {
        return false;
    };
    let key = key.trim();
    key.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '/'))
        && (value.is_empty() || value.starts_with(' '))
}

/// Read a `fields: {from: prefix-lines}` header (RFC-33, MILE-98): bare
/// `Key: value` lines between the title and the first `##` heading, as in a
/// Nygard-style record (`Date: 2016-02-12`).
///
/// A key is an ASCII letter followed by letters, digits, `-` or `_`, and the
/// colon must be followed by a space or end the line, so a URL or a sentence
/// is not a field. Lines that are not field-shaped (the title, blanks, prose)
/// are skipped, not region-ending. The scan stops at the first heading of
/// depth two or more. The region spans the first to the last field line; no
/// field line at all is `region: None`, reportable and never an empty header.
pub fn parse_prefix_lines(content: &str) -> Header {
    let mut fields = Vec::new();
    for (idx, raw) in content.lines().enumerate() {
        let line = raw.trim_end();
        if line.starts_with("##") && line.trim_start_matches('#').starts_with([' ', '\t']) {
            break;
        }
        if let Some(field) = prefix_line_field(line, idx + 1) {
            fields.push(field);
        }
    }
    let region = match (fields.first(), fields.last()) {
        (Some(first), Some(last)) => Some((first.line, last.line)),
        _ => None,
    };
    Header {
        fields,
        region,
        parse_error: None,
    }
}

fn prefix_line_field(line: &str, line_no: usize) -> Option<HeaderField> {
    let (key, value) = line.split_once(':')?;
    let mut chars = key.chars();
    let valid_key = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid_key || !(value.is_empty() || value.starts_with(' ')) {
        return None;
    }
    Some(HeaderField {
        key: crate::values::FieldName::new(key),
        value: value.trim().to_string(),
        line: line_no,
    })
}

/// Parse a `---`-delimited YAML frontmatter block (RFC-0016/ADR-0017).
///
/// Unlike the line-recognizer shapes above, this goes through a real YAML
/// deserializer rather than a bespoke parser -- closure (only the declared
/// keys, each once) and typing come from the deserializer, not from a
/// hand-written theory of what counts as "the header." A document that
/// doesn't open with `---`, whose frontmatter never closes, or whose content
/// isn't a top-level mapping has `region: None` -- reportable, same contract
/// every other shape honors, never a silently empty header.
fn parse_yaml_frontmatter(content: &str) -> Header {
    // A leading byte-order mark is not part of the document: every other
    // reader of a record strips it, and left in place it hid the opening `---`.
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let mut lines = content.lines();
    let Some(first) = lines.next() else {
        return header_without_region(None);
    };
    if first.trim_end() != "---" {
        // A removed form is named, not mistaken for a record with no header
        // (MILE-85): the rule reporting the unreadable header carries this.
        return header_without_region(legacy_header_form(content).map(|form| {
            format!(
                "the header is written as a {form} ({}), a form urzua no longer reads; rewrite it as YAML frontmatter (`---` ... `---` at the top of the file)",
                if form == "blockquote" { "`> Key: value`" } else { "`- **Key:** value`" }
            )
        }));
    }

    let mut yaml_text = String::new();
    let mut closing_line: Option<usize> = None;
    for (idx, line) in lines.enumerate() {
        let line_no = idx + 2; // +1 for 1-indexing, +1 for the opening `---`
        if line.trim_end() == "---" {
            closing_line = Some(line_no);
            break;
        }
        yaml_text.push_str(line);
        yaml_text.push('\n');
    }

    let Some(closing_line) = closing_line else {
        return header_without_region(None);
    };

    // Unlike the two early returns above ("no anchor line matched at all"),
    // a value here means real YAML syntax was attempted and failed --
    // yaml_serde's own error already carries a line and reason (BUG-0012),
    // worth surfacing verbatim instead of collapsing into the same generic
    // "no header-shaped region found" every other None case reports.
    let parsed = yaml_serde::from_str::<yaml_serde::Value>(&yaml_text);
    let mapping = match parsed {
        Ok(yaml_serde::Value::Mapping(mapping)) => mapping,
        Ok(_) => {
            return header_without_region(Some(
                "YAML parse error: frontmatter must be a mapping of key: value pairs".to_string(),
            ));
        }
        Err(e) => {
            return header_without_region(Some(format!("YAML parse error: {e}")));
        }
    };

    Header {
        fields: frontmatter_fields(&yaml_text, &mapping),
        region: Some((1, closing_line)),
        parse_error: None,
    }
}

/// A header with no fields and no region, carrying why it could not be read.
fn header_without_region(parse_error: Option<String>) -> Header {
    Header {
        fields: Vec::new(),
        region: None,
        parse_error,
    }
}

/// The top-level keys of a frontmatter mapping, each with the source text of
/// its value and the line it sits on.
fn frontmatter_fields(yaml_text: &str, mapping: &yaml_serde::Mapping) -> Vec<HeaderField> {
    mapping
        .iter()
        .map(|(key, value)| {
            // A non-string key (`1: x`, `true: x`) keeps its stringified form
            // rather than vanishing from the header.
            let key = match key {
                yaml_serde::Value::Null => "null".to_string(),
                other => stringify_yaml_value(other),
            };
            let line = find_key_line(yaml_text, &key);
            let value = line
                .and_then(|line| scalar_source_text(yaml_text, line, value))
                .unwrap_or_else(|| stringify_yaml_value(value));
            HeaderField {
                key: crate::values::FieldName::new(key),
                value,
                line: line.unwrap_or(2),
            }
        })
        .collect()
}

/// Best-effort line lookup for a top-level YAML key, for diagnostics only --
/// parsing correctness never depends on this succeeding. A flow-style
/// mapping (`{a: 1, b: 2}` on one line) falls back to the block's first
/// content line via the caller's default.
fn find_key_line(yaml_text: &str, key: &str) -> Option<usize> {
    let find = |top_level_only: bool| {
        yaml_text.lines().enumerate().find_map(|(idx, line)| {
            if top_level_only && line.starts_with(char::is_whitespace) {
                return None;
            }
            let trimmed = line.trim_start();
            (trimmed == format!("{key}:") || trimmed.starts_with(&format!("{key}: ")))
                .then_some(idx + 2)
        })
    };
    // An unindented line is the top-level key; an indented one of the same name
    // belongs to a nested mapping and is only a fallback.
    find(true).or_else(|| find(false))
}

/// The text a number or boolean was written as (`1.10`, `0x1F`, `1e3`), read
/// from its own line, so the header holds what the file says and not a
/// re-rendering of the parsed value. `None` when the value is not a plain
/// single-line number or boolean, or the text does not read back as the same
/// value; the caller then falls back to the parsed rendering.
fn scalar_source_text(
    yaml_text: &str,
    line_no: usize,
    value: &yaml_serde::Value,
) -> Option<String> {
    if !matches!(
        value,
        yaml_serde::Value::Number(_) | yaml_serde::Value::Bool(_)
    ) {
        return None;
    }
    let line = yaml_text.lines().nth(line_no.checked_sub(2)?)?;
    // Only a top-level key's line: an indented line of the same name belongs
    // to a nested mapping or a block scalar, not to this value.
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let (_, rest) = line.split_once(':')?;
    // A comment starts at a `#` that follows a space or a tab.
    let comment = rest
        .char_indices()
        .find(|&(i, c)| c == '#' && rest[..i].ends_with([' ', '\t']))
        .map_or(rest.len(), |(i, _)| i);
    let rest = rest[..comment].trim();
    if rest.is_empty() || rest.contains(['\'', '"', '[', '{', '&', '*', '!', '|', '>']) {
        return None;
    }
    (yaml_serde::from_str::<yaml_serde::Value>(rest)
        .ok()
        .as_ref()
        == Some(value))
    .then(|| rest.to_string())
}

fn stringify_yaml_value(value: &yaml_serde::Value) -> String {
    use yaml_serde::Value;
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Sequence(items) => items
            .iter()
            .map(stringify_yaml_value)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Mapping(_) | Value::Tagged(_) => yaml_serde::to_string(value)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_scalars_keep_the_text_the_file_wrote() {
        let h = parse_yaml_frontmatter(
            "---\nVersion: 1.10\nA: 0x1F\nC: 1e3 # note\nD: True\nE: 7\n---\n",
        );
        assert_eq!(h.get("Version"), Some("1.10"));
        assert_eq!(h.get("A"), Some("0x1F"));
        assert_eq!(h.get("C"), Some("1e3"));
        assert_eq!(h.get("D"), Some("True"));
        assert_eq!(h.get("E"), Some("7"));
    }

    #[test]
    fn a_tab_before_a_comment_and_a_nested_key_of_the_same_name_do_not_leak_into_the_value() {
        let h = parse_yaml_frontmatter("---\nA: 1.10\t# note\n---\n");
        assert_eq!(h.get("A"), Some("1.10"));
        let h = parse_yaml_frontmatter("---\nMeta:\n  Version: 1.1\nVersion: 1.10\n---\n");
        assert_eq!(h.get("Version"), Some("1.10"));
    }

    #[test]
    fn a_leading_bom_does_not_hide_the_frontmatter() {
        let h = parse_yaml_frontmatter("\u{feff}---\nStatus: Accepted\n---\n");
        assert_eq!(h.get("Status"), Some("Accepted"));
        assert_eq!(h.region, Some((1, 3)));
    }

    #[test]
    fn a_non_string_key_is_kept_not_dropped() {
        let h = parse_yaml_frontmatter("---\n1: x\ntrue: y\n---\n");
        assert_eq!(h.get("1"), Some("x"));
        assert_eq!(h.get("true"), Some("y"));
    }

    #[test]
    fn prefix_lines_reads_bare_key_value_lines_before_the_first_section() {
        let doc = "# 1. Record architecture decisions\n\nDate: 2016-02-12\nStatus: Accepted\n\n## Context\n\nDecoy: not a header\n";
        let h = parse_prefix_lines(doc);
        assert_eq!(h.get("Date"), Some("2016-02-12"));
        assert_eq!(h.get("Status"), Some("Accepted"));
        assert_eq!(h.get("Decoy"), None);
        assert_eq!(h.region, Some((3, 4)));
    }

    #[test]
    fn prefix_lines_does_not_read_prose_or_urls_as_fields() {
        let doc = "# T\n\nSee https://example.com for more.\nA sentence: with a colon.\nDate: 2020-01-01\n";
        let h = parse_prefix_lines(doc);
        assert_eq!(
            h.fields.iter().map(|f| f.key.as_str()).collect::<Vec<_>>(),
            ["Date"]
        );
    }

    #[test]
    fn prefix_lines_with_no_field_line_has_no_region_not_an_empty_header() {
        let h = parse_prefix_lines("# T\n\nJust prose.\n\n## Status\n\nAccepted\n");
        assert!(h.fields.is_empty());
        assert_eq!(h.region, None);
        assert!(h.is_unreadable());
    }

    #[test]
    fn prefix_lines_keeps_an_empty_value_and_stops_at_a_deeper_heading_only_when_spaced() {
        let h = parse_prefix_lines("# T\n\nDate:\n###\nStatus: x\n### Deep\nLate: y\n");
        assert_eq!(h.get("Date"), Some(""));
        assert_eq!(h.get("Status"), Some("x"));
        assert_eq!(h.get("Late"), None);
    }

    #[test]
    fn get_reserved_answers_for_a_differently_cased_key() {
        // The opposite of `get`, deliberately: Stable-Id is not adopter
        // vocabulary, there is exactly one field by that name, and a record
        // spelling it `stable-id` has made a typo -- migrate_ids assigning a
        // second id on top of it is BUG-97's shape reopened for a case
        // ADR-57's own reasoning never covered.
        let h = parse("---\nstable-id: 01ABC\n---\n# T\n");
        assert_eq!(h.get_reserved("Stable-Id"), Some("01ABC"));
        assert_eq!(h.get_reserved("STABLE-ID"), Some("01ABC"));
    }

    #[test]
    fn get_does_not_answer_for_a_differently_cased_key() {
        // ADR-57: the record wrote a different name, so the declared one is
        // absent. Answering here let header.required-fields read the field as
        // satisfied while header.field-set-consistency called it undeclared.
        let h = parse("---\nstatus: Accepted\n---\n# T\n");
        assert_eq!(h.get("status"), Some("Accepted"));
        assert_eq!(h.get("Status"), None);
        assert_eq!(h.get("STATUS"), None);
    }

    #[test]
    fn two_spellings_of_one_word_are_two_keys_not_a_duplicate() {
        let h = parse("---\nStatus: A\nstatus: B\n---\n# T\n");
        assert!(
            h.duplicate_keys().is_empty(),
            "ADR-57 makes these two fields: {:?}",
            h.duplicate_keys()
        );
        assert_eq!(h.get("Status"), Some("A"));
        assert_eq!(h.get("status"), Some("B"));
    }

    #[test]
    fn duplicate_keys_are_detected_not_first_wins() {
        // A YAML mapping cannot carry a repeated key, so the header is built
        // by hand: the closure check is still part of the contract.
        let field = |value: &str, line| HeaderField {
            key: crate::values::FieldName::new("Status"),
            value: value.to_string(),
            line,
        };
        let h = Header {
            fields: vec![field("Draft", 2), field("Accepted", 3)],
            region: Some((1, 4)),
            parse_error: None,
        };
        assert_eq!(h.duplicate_keys(), vec!["Status".to_string()]);
    }

    #[test]
    fn no_header_shaped_region_is_none_not_empty() {
        let doc = "# Title\n\nJust prose, no metadata block.\n";
        let h = parse(doc);
        assert_eq!(h.region, None);
        assert!(h.fields.is_empty());
    }

    #[test]
    fn a_blockquote_header_is_named_as_an_unsupported_form_not_just_absent() {
        // MILE-85: the shape is no longer read, and a record still written in
        // it must say so rather than look like a record with no header.
        let doc = "# Title\n\n> Status: Draft\n> Date: 2026-08-11\n";
        let h = parse_with_shape(doc, HeaderShape::YamlFrontmatter);
        assert_eq!(h.region, None);
        let e = h.parse_error.expect("the legacy form is named");
        assert!(
            e.contains("blockquote") && e.contains("no longer read"),
            "{e}"
        );
    }

    #[test]
    fn a_bold_list_header_is_named_as_an_unsupported_form() {
        let doc = "# Title\n\n- **Status:** Accepted\n- **Date:** 2026-08-14\n\n## Context\n";
        let h = parse_with_shape(doc, HeaderShape::YamlFrontmatter);
        let e = h.parse_error.expect("the legacy form is named");
        assert!(
            e.contains("bold-list") && e.contains("no longer read"),
            "{e}"
        );
    }

    #[test]
    fn a_body_quote_before_a_bold_list_header_does_not_hide_the_bold_list() {
        let doc = "# T\n\n> **Decision:** In the context of a\n> facing b\n\n- **Status:** Accepted\n- **Date:** 2026-01-01\n";
        assert_eq!(legacy_header_form(doc), Some("bold-list"));
    }

    #[test]
    fn a_body_blockquote_or_bullet_after_the_first_section_is_not_a_legacy_header() {
        let doc = "# Title\n\n## Context\n\n> Status: quoted\n\n- **Note:** bullet\n";
        let h = parse_with_shape(doc, HeaderShape::YamlFrontmatter);
        assert_eq!(h.parse_error, None);
    }

    #[test]
    fn parses_yaml_frontmatter() {
        let doc = "---\nstatus: Accepted\ndate: 2026-09-05\nreviewers:\n  - alice\n  - bob\n---\n# Title\n";
        let h = parse_with_shape(doc, HeaderShape::YamlFrontmatter);
        assert_eq!(h.get("status"), Some("Accepted"));
        assert_eq!(h.get("date"), Some("2026-09-05"));
        assert_eq!(h.get("reviewers"), Some("alice, bob"));
        assert_eq!(h.region, Some((1, 7)));
    }

    #[test]
    fn yaml_frontmatter_that_never_closes_is_no_region_not_a_panic() {
        // Observed failing before it's trusted: a document that opens `---`
        // but never closes it must be reportable, not silently swallowed and
        // not a crash on malformed structural input.
        let doc = "---\nstatus: Accepted\n# no closing delimiter\n";
        let h = parse_with_shape(doc, HeaderShape::YamlFrontmatter);
        assert_eq!(h.region, None);
        assert!(h.fields.is_empty());
    }

    #[test]
    fn a_document_not_opening_with_frontmatter_has_no_yaml_region() {
        let doc = "# Title\n\nStatus: Draft\n";
        let h = parse_with_shape(doc, HeaderShape::YamlFrontmatter);
        assert_eq!(h.region, None);
    }

    #[test]
    fn invalid_yaml_inside_the_delimiters_is_no_region_not_a_panic() {
        let doc = "---\nstatus: [unterminated\n---\n# Title\n";
        let h = parse_with_shape(doc, HeaderShape::YamlFrontmatter);
        assert_eq!(h.region, None);
        assert!(h.fields.is_empty());
    }

    #[test]
    fn invalid_yaml_surfaces_the_real_parse_error_observed_failing() {
        // BUG-0012: before this fix, region: None was the only signal --
        // yaml_serde's own error (which already carries a line and reason)
        // was discarded, forcing manual byte-level diagnosis every time.
        let doc = "---\nstatus: [unterminated\n---\n# Title\n";
        let h = parse_with_shape(doc, HeaderShape::YamlFrontmatter);
        assert_eq!(h.region, None);
        assert!(
            h.parse_error.is_some_and(|e| !e.is_empty()),
            "expected a real parse error message, got None"
        );
    }

    #[test]
    fn a_value_starting_with_a_backtick_surfaces_the_real_parse_error_observed_failing() {
        // The exact live case BUG-0012 was found from: a backtick is a
        // reserved YAML indicator character and cannot start an unquoted
        // plain scalar (hit on 9 real spec files' Subject field this
        // session).
        let doc = "---\nSubject: `oops\nStatus: Draft\n---\n# Title\n";
        let h = parse_with_shape(doc, HeaderShape::YamlFrontmatter);
        assert_eq!(h.region, None);
        assert!(
            h.parse_error.is_some_and(|e| !e.is_empty()),
            "expected a real parse error message, got None"
        );
    }

    #[test]
    fn a_top_level_scalar_is_no_region_with_a_specific_reason() {
        // Valid YAML, but not the mapping shape a header requires -- distinct
        // from a genuine syntax error, so it gets its own diagnosis rather
        // than yaml_serde's Err (there is none; this parses successfully).
        let doc = "---\njust a plain string\n---\n# Title\n";
        let h = parse_with_shape(doc, HeaderShape::YamlFrontmatter);
        assert_eq!(h.region, None);
        assert!(h.fields.is_empty());
        assert!(h.parse_error.is_some_and(|e| e.contains("mapping")));
    }
}
