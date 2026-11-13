//! `urzua new`: render a record's initial content. Pure -- the caller reads
//! any existing template, resolves identity/today, computes the next
//! display number, and performs the actual file write; this module only
//! computes text.

/// The next unused display number, given the filenames already in a record
/// type's directory. Never reuses a number even if one was deleted --
/// cross-references elsewhere may still assume the old numbering held.
///
/// Reads the current `TYPE-NNNN-slug.md` shape (number is the second
/// segment). ADR-36's amendment dropped acceptance of the legacy
/// `NNNN-slug.md` shape (BUG-9): it was never a real external-adopter case,
/// and this repo's own pre-ADR-36 filenames no longer exist in the corpus.
///
/// Validates the complete shape, not just enough to find a number:
/// `strip_suffix(".md")` requires the extension, and the three-way split
/// requires a nonempty type prefix, a numeric second segment, and a nonempty
/// slug -- a directory entry like `ADR-999` (no `.md`) or `-999-slug.md`
/// (empty prefix) is real, once a stray non-record file sits in a type's
/// directory, and must not silently affect the computed max.
/// The number and slug of a record filename, in either convention this project
/// has to read: `0001-slug.md` (Nygard-style, and this project's own history)
/// and `ADR-1-slug.md` (`ADR-36`). Returns `None` for anything else.
///
/// One definition, used by both `urzua new`'s numbering and `urzua init`'s
/// adopt scan: two separate recognisers accepting disjoint sets makes each
/// blind to exactly what the other requires (`BUG-37`).
pub fn parse_record_filename(file_name: &str) -> Option<(Option<&str>, u32)> {
    let stem = crate::record::strip_md_extension(file_name)?;

    let segments: Vec<&str> = stem.split('-').collect();
    // The number is the first all-digit segment, wherever it sits: a type
    // prefix may contain hyphens, so its position is not fixed.
    let at = segments.iter().position(|s| is_digit_segment(s))?;
    // Something must follow the number: a bare `ADR-2.md` is a fragment.
    if at + 1 >= segments.len() || segments[at + 1..].iter().all(|s| s.is_empty()) {
        return None;
    }

    if at == 0 {
        // Padding is not load-bearing, but a date is not a number: a
        // `YYYY-MM-DD-` filename would parse as record YYYY and numbering
        // never comes back below it. Checked as a real date, so a slug that
        // merely starts with two digit-shaped segments stays a record. The
        // year itself must be plausible (BUG-102): `0013-01-15-use-postgres.md`
        // has a month- and day-shaped second and third segment too, but 13
        // is not a year anyone dates a file with -- it is record 13. A record
        // number that reaches 1000-9999 (a genuinely plausible year) is not
        // resolvable by shape alone -- `2024-01-15-migrate-db.md` is
        // ambiguous on its face, and this heuristic still reads it as a date.
        // No filename-shape rule can close that; it is inherent to the
        // convention, not a gap in this bound.
        //
        // Month/day are not required to be zero-padded (BUG-112):
        // `2026-9-19-notes.md` is as much a date as `2026-09-19-notes.md`,
        // and the value range below already rejects anything that isn't a
        // plausible month or day regardless of digit count.
        let looks_dated = segments.len() >= 3
            && segments[0].len() == 4
            && matches!(segments[0].parse::<u32>(), Ok(1000..=9999))
            && [1, 2].iter().all(|&i| is_digit_segment(segments[i]))
            && matches!(segments[1].parse::<u32>(), Ok(1..=12))
            && matches!(segments[2].parse::<u32>(), Ok(1..=31));
        if looks_dated {
            return None;
        }
        return segments[0].parse().ok().map(|n| (None, n));
    }

    // `ADR-2-slug`, and `DOC-ADR-2-slug`: every segment before the number is
    // the prefix, and each must be upper-case -- digits allowed (`BUG-114`:
    // `RecordTypeConfig.prefix` doesn't forbid a prefix like `V2`, and a
    // segment reaching here is already guaranteed not all-digit, since `at`
    // selects the *first* such segment, so a mixed prefix creates no
    // ambiguity with which segment is the number).
    let prefix_end = segments[..at].join("-").len();
    if segments[..at].iter().all(|s| is_prefix_segment(s)) {
        return segments[at]
            .parse()
            .ok()
            .map(|n| (Some(&stem[..prefix_end]), n));
    }
    None
}

/// A record identifier's number segment: non-empty, all ASCII digits.
///
/// Shared with [`crate::rules::is_record_reference`] (`BUG-133`): the two "is
/// this a valid record identifier" recognisers -- one for a filename, one for
/// a bare reference token in prose -- must agree on this grammar, and a
/// second copy is how they drifted once already (`BUG-114`).
pub(crate) fn is_digit_segment(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// A record identifier's prefix segment: non-empty, ASCII upper-case or
/// digit (`BUG-114`: a type may declare a prefix like `V2`). Shared with
/// [`crate::rules::is_record_reference`]; see [`is_digit_segment`].
pub(crate) fn is_prefix_segment(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

/// The next unclaimed number for a type, across **both** filename conventions.
///
/// `BUG-9` excluded `NNNN-slug.md` from this count, arguing it "was never a
/// real external-adopter case". `MILE-51` is that case: in a corpus adopted
/// from `npryce/adr-tools`, every filename is `0001-`..`0009-`, the exclusion
/// found nothing to count, and `urzua new` wrote a second record numbered 1
/// (BUG-37). Miscounting from a stale legacy file is recoverable; writing a
/// duplicate number into someone else's corpus is not.
pub fn next_display_number(filenames: &[String]) -> Result<u32, String> {
    next_display_number_by(filenames, &|name| {
        parse_record_filename(name).map(|(_, number)| number)
    })
}

/// [`next_display_number`] with the notion of "a record's number" supplied by
/// the caller, so a type that declares an `identity` pattern is numbered by
/// that pattern rather than by the `TYPE-NNNN-slug.md` convention (`MILE-151`):
/// two readers of one filename grammar would disagree about which numbers are
/// taken.
pub fn next_display_number_by(
    filenames: &[String],
    number_of: &dyn Fn(&str) -> Option<u32>,
) -> Result<u32, String> {
    match filenames.iter().filter_map(|name| number_of(name)).max() {
        None => Ok(1),
        Some(highest) => highest.checked_add(1).ok_or_else(|| {
            format!("record number {highest} is the largest this tool can issue; no number is left to assign")
        }),
    }
}

/// The record number a declared identity pattern reads from a file name, when
/// it fits a `u32`.
pub fn declared_number(
    identity: &crate::document_model::CompiledIdentity,
    file_name: &str,
) -> Option<u32> {
    identity.identify(file_name).number?.parse().ok()
}

/// The widest zero-padding tried when fitting a declared pattern.
const MAX_PADDING: usize = 12;

/// The file name (with `.md`) that a declared identity pattern reads back as
/// this type's prefix and this number (`MILE-151`).
///
/// A regular expression cannot be run backwards, so this builds the
/// `PREFIX-NUMBER-slug` shape the project writes, zero-padding the number to
/// the narrowest width the pattern accepts (`\d+` takes none, `\d{4}` takes
/// four), and keeps it only if [`CompiledIdentity::identify`] gives back the
/// same prefix and the same number. A pattern no such name fits (a different
/// separator, a date in the name) is an `Err` naming the pattern, because
/// writing a name the type's own pattern does not read would mint a record no
/// identity-reading rule can see.
pub fn declared_file_name(
    identity: &crate::document_model::CompiledIdentity,
    pattern: &str,
    type_prefix: &str,
    number: u32,
    slug: &str,
) -> Result<String, String> {
    for width in 1..=MAX_PADDING {
        let name = format!("{type_prefix}-{number:0width$}-{slug}.md");
        let found = identity.identify(&name);
        if found.title_prefix.as_deref() == Some(type_prefix)
            && found.number.as_deref().and_then(|n| n.parse::<u32>().ok()) == Some(number)
        {
            return Ok(name);
        }
    }
    Err(format!(
        "the declared identity pattern '{pattern}' does not read a '{type_prefix}-{number}-{slug}.md' \
         style file name back as prefix '{type_prefix}' and number {number}, so `urzua new` cannot \
         name a record for it -- the pattern must accept the 'PREFIX-NNNN-slug' form"
    ))
}

/// The outcome of numbering with other refs' claims taken into account.
#[derive(Debug, PartialEq, Eq)]
pub struct NumberAssignment {
    pub number: u32,
    /// Numbers above the working tree's own highest that another ref already
    /// uses, with the ref names claiming each -- the ones `number` skipped.
    pub skipped: Vec<(u32, Vec<String>)>,
}

/// [`next_display_number`], also skipping past numbers other refs already
/// use (`MILE-89`). `claims` is `(ref name, filenames in the type's directory
/// on that ref)`. Pure: the caller gathers the claims from git.
pub fn next_display_number_with_claims(
    local: &[String],
    claims: &[(String, Vec<String>)],
) -> Result<NumberAssignment, String> {
    next_display_number_with_claims_by(local, claims, &|name| {
        parse_record_filename(name).map(|(_, number)| number)
    })
}

/// [`next_display_number_with_claims`] with the caller's reading of a record's
/// number; see [`next_display_number_by`].
pub fn next_display_number_with_claims_by(
    local: &[String],
    claims: &[(String, Vec<String>)],
    number_of: &dyn Fn(&str) -> Option<u32>,
) -> Result<NumberAssignment, String> {
    let local_next = next_display_number_by(local, number_of)?;
    let mut by_number: std::collections::BTreeMap<u32, Vec<String>> = Default::default();
    for (ref_name, names) in claims {
        for name in names {
            if let Some(n) = number_of(name) {
                if n >= local_next {
                    let refs = by_number.entry(n).or_default();
                    if !refs.contains(ref_name) {
                        refs.push(ref_name.clone());
                    }
                }
            }
        }
    }
    let number = match by_number.keys().next_back() {
        None => local_next,
        Some(m) => m.checked_add(1).ok_or_else(|| {
            format!(
                "record number {m} is the largest this tool can issue; no number is left to assign"
            )
        })?,
    };
    Ok(NumberAssignment {
        number,
        skipped: by_number.into_iter().collect(),
    })
}

/// Lowercase, hyphen-separated, ASCII-alphanumeric only -- matches this
/// corpus's own existing filenames (e.g. `0016-a-bold-list-header-shape...`).
pub fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut last_was_hyphen = true; // suppresses a leading hyphen
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if !last_was_hyphen {
            slug.push('-');
            last_was_hyphen = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    slug
}

pub struct NewRecordParams<'a> {
    pub display_number: u32,
    /// The type prefix the H1 opens with (`ADR-4 -- Title`), set when the type
    /// declares an `identity` pattern, whose title form is `PREFIX-N`
    /// (`RFC-44`). `None` keeps the bare number.
    pub h1_prefix: Option<&'a str>,
    /// The width the H1's number is zero-padded to: the width of the `number`
    /// capture the declared pattern reads from the file name written beside it
    /// (`BUG-188`), so the two ids agree. `0` is unpadded.
    pub h1_width: usize,
    /// The status a new record starts at: the first value of the type's
    /// declared `statuses.values`. `None` when the type declares none, and the
    /// field stays blank for a human.
    pub initial_status: Option<&'a str>,
    /// The field that holds the status, as the type declares it
    /// (`RecordTypeConfig::relation_field`), not a literal.
    pub status_field: &'a str,
    /// The fields filled with `today` and `author`, as the type declares them
    /// (`RecordTypeConfig::date_field`/`author_field`), not literals.
    pub date_field: &'a str,
    pub author_field: &'a str,
    /// The field filled with the title, when the type declares one
    /// (`RecordTypeConfig::title_field`).
    pub title_field: Option<&'a str>,
    /// The version field and the value it starts at, when the type declares
    /// them (`RecordTypeConfig::initial_version`).
    pub initial_version: Option<(&'a str, &'a str)>,
    pub title: &'a str,
    /// Always assigned (`ADR-21`): every record type gets one, whether or not
    /// the type's field set declares it.
    pub stable_id: &'a str,
    pub author: &'a str,
    pub today: &'a str,
}

impl NewRecordParams<'_> {
    /// What stands for the number in the H1: `ADR-4` for a type with a declared
    /// identity, the bare number otherwise.
    fn h1_number(&self) -> String {
        match self.h1_prefix {
            Some(prefix) => format!(
                "{prefix}-{:0width$}",
                self.display_number,
                width = self.h1_width
            ),
            None => self.display_number.to_string(),
        }
    }
}

/// Render from scratch with no template: YAML frontmatter (ADR-0017's
/// default for a record with no pre-existing convention to adopt), listing
/// every configured required field as a field a human still has to fill in.
///
/// `Stable-Id` is always assigned (ADR-21: every type gets one, declared or
/// not). `Date`/`Author` are only filled with their real value when the
/// type's own `required_fields` actually names them -- unconditionally
/// adding them regardless of what a type declares would put a field on the
/// record its own `known_fields`/`required_fields` never listed, tripping
/// `header.field-set-consistency` for every record `urzua new` creates
/// (found live: `milestone`/`bug` require neither field, so the first
/// yaml-frontmatter `milestone` created this way carried both unannounced).
///
/// Builds a real `yaml_serde::Mapping` and serializes it (BUG-0006) instead
/// of hand-formatting strings -- `Stable-Id`/`Date`/`Author` are real values
/// that can contain YAML-special characters (a colon, a leading `-`), and a
/// value that merely *looks* numeric (a version string like `"0.2"`) must
/// stay a string on reparse, not silently become a YAML number. The real
/// serializer's plain-scalar analysis quotes a value exactly when leaving it
/// bare would change what it parses back as -- verified by round-trip tests,
/// not assumed.
pub fn render_synthetic_yaml(
    params: &NewRecordParams,
    required_fields: &[String],
    template_fields: &[String],
) -> String {
    render_yaml_with(params, required_fields, template_fields, &[])
}

/// [`render_synthetic_yaml`] with header values the caller supplies: each
/// `(field, value)` replaces the field's blank value, or adds the field after
/// the others when the header has none (`urzua waive --from-report`).
fn render_yaml_with(
    params: &NewRecordParams,
    required_fields: &[String],
    template_fields: &[String],
    supplied: &[(&str, &str)],
) -> String {
    use yaml_serde::Value;

    let mut mapping = yaml_serde::Mapping::new();
    mapping.insert(
        Value::String("Stable-Id".to_string()),
        Value::String(params.stable_id.to_string()),
    );
    // The fields a declaration lets `new` fill beyond the date, author and
    // status: the title and the starting version (`MILE-74`).
    let declared_value = |field: &str| -> Option<Value> {
        if Some(field) == params.title_field {
            return Some(Value::String(params.title.to_string()));
        }
        params
            .initial_version
            .filter(|(version_field, _)| *version_field == field)
            .map(|(_, value)| Value::String(value.to_string()))
    };
    let value_for = |field: &str| match field {
        f if f == params.date_field => Value::String(params.today.to_string()),
        f if f == params.author_field => Value::String(params.author.to_string()),
        f if f == params.status_field => params
            .initial_status
            .map_or(Value::Null, |s| Value::String(s.to_string())),
        f => declared_value(f).unwrap_or(Value::Null),
    };
    for field in required_fields {
        // A type that requires `Stable-Id` already has the generated one above;
        // listing it again as a blank required field would overwrite it with null.
        if field == "Stable-Id" {
            continue;
        }
        mapping.insert(Value::String(field.clone()), value_for(field));
    }

    // Fields the template's header names beyond the required ones (`BUG-188`),
    // already filtered to those the type declares, stay blank for a human,
    // unless the type declares a value for them (`MILE-74`).
    for field in template_fields {
        let key = Value::String(field.clone());
        if !mapping.contains_key(&key) {
            mapping.insert(key, declared_value(field).unwrap_or(Value::Null));
        }
    }

    for (field, value) in supplied {
        mapping.insert(
            Value::String((*field).to_string()),
            Value::String((*value).to_string()),
        );
    }

    let yaml_body = yaml_serde::to_string(&Value::Mapping(mapping)).unwrap_or_default();
    let mut out = String::from("---\n");
    out.push_str(yaml_body.trim_start_matches("---\n"));
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("---\n");
    out.push_str(&format!("# {} — {}\n", params.h1_number(), params.title));
    out
}

/// What `scaffold_record` needs to build a record's initial text: the type's
/// declarations and the values only the caller can resolve (number, id,
/// author, date). Shared by every command that creates a record from a type
/// (`urzua new`, and `waive --from-report` after it).
pub struct ScaffoldSpec<'a> {
    pub type_config: &'a crate::config::RecordTypeConfig,
    pub compiled: Option<&'a crate::document_model::CompiledIdentity>,
    pub type_prefix: &'a str,
    pub filename: &'a str,
    pub display_number: u32,
    pub title: &'a str,
    pub stable_id: &'a str,
    pub author: &'a str,
    pub today: &'a str,
    /// A value for the `Reason` header (the field `Waiver` reads), when the
    /// caller drafts one. `None` leaves it as the type's blank.
    pub reason: Option<&'a str>,
    /// A value for the `Expires` header; `None` leaves it blank.
    pub expires: Option<&'a str>,
    /// Further `(field, value)` header pairs the caller supplies, written after
    /// the type's own fields or over their blank value.
    pub header_fields: &'a [(&'a str, &'a str)],
    /// Text appended after the template's body.
    pub extra_body: Option<&'a str>,
    /// The narrative of a first revision-log row, written under the log's own
    /// layout when the type declares a log. `None` writes none.
    pub log_row: Option<&'a str>,
}

impl<'a> ScaffoldSpec<'a> {
    /// The spec `urzua new` builds: nothing supplied beyond the type's own
    /// declarations.
    #[allow(clippy::too_many_arguments)]
    pub fn plain(
        type_config: &'a crate::config::RecordTypeConfig,
        compiled: Option<&'a crate::document_model::CompiledIdentity>,
        type_prefix: &'a str,
        filename: &'a str,
        display_number: u32,
        title: &'a str,
        stable_id: &'a str,
        author: &'a str,
        today: &'a str,
    ) -> Self {
        ScaffoldSpec {
            type_config,
            compiled,
            type_prefix,
            filename,
            display_number,
            title,
            stable_id,
            author,
            today,
            reason: None,
            expires: None,
            header_fields: &[],
            extra_body: None,
            log_row: None,
        }
    }
}

/// The initial text of a record: a synthesized YAML header from the type's
/// declarations, then the body scaffolding of `template` (the caller reads the
/// checked-in template, if any). Pure: no file is read or written.
///
/// BUG-3: the configured header shape wins unconditionally -- a template's own
/// (possibly stale) header is never used, only the field names it lists and its
/// body.
pub fn scaffold_record(spec: &ScaffoldSpec, template: Option<&str>) -> String {
    let type_config = spec.type_config;
    // The entry state of the type's own vocabulary, not a choice: moving a
    // record on from it stays a human decision.
    let status_decl = type_config.status_decl();
    let params = NewRecordParams {
        display_number: spec.display_number,
        // The H1 id is padded exactly as the file name is: the width of the
        // `number` the declared pattern reads back from it (`BUG-188`).
        h1_width: spec
            .compiled
            .and_then(|c| c.identify(spec.filename).number)
            .map_or(0, |n| n.len()),
        h1_prefix: spec.compiled.map(|_| spec.type_prefix),
        initial_status: status_decl
            .as_ref()
            .and_then(|d| d.values.first())
            .map(String::as_str),
        status_field: type_config.relation_field(crate::config::RelationRole::Status),
        date_field: type_config.date_field(),
        author_field: type_config.author_field(),
        title_field: type_config.title_field(),
        initial_version: type_config.initial_version(),
        title: spec.title,
        stable_id: spec.stable_id,
        author: spec.author,
        today: spec.today,
    };
    let template_fields = declared_template_fields(type_config, template);
    let mut supplied: Vec<(&str, &str)> = Vec::new();
    supplied.extend(spec.reason.map(|v| (REASON_FIELD, v)));
    supplied.extend(spec.expires.map(|v| (EXPIRES_FIELD, v)));
    supplied.extend(spec.header_fields.iter().copied());
    let mut out = render_yaml_with(
        &params,
        &type_config.required_fields,
        &template_fields,
        &supplied,
    );
    if let Some(body) = template.and_then(template_body) {
        out.push('\n');
        out.push_str(body);
    }
    if let Some(narrative) = spec.log_row {
        out = with_first_log_row(out, type_config, spec.today, narrative);
    }
    if let Some(extra) = spec.extra_body {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
        out.push_str(extra);
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

/// The header fields `Waiver` reads by name beyond `Rule` and `Scope`.
pub(crate) const REASON_FIELD: &str = "Reason";
pub(crate) const EXPIRES_FIELD: &str = "Expires";

/// `text` with one tool-authored row after the last line of its revision log's
/// table, laid out by the log's own header (`ADR-94`). A type with no readable
/// log, or none in the text, gets `text` unchanged.
fn with_first_log_row(
    text: String,
    type_config: &crate::config::RecordTypeConfig,
    today: &str,
    narrative: &str,
) -> String {
    let id = type_config.revision_log_id();
    let shape = type_config.revision_log_shape();
    let Some(Ok(log)) = crate::rules::find_revision_log_entries(&text, &id, &shape) else {
        return text;
    };
    let Ok(row) = crate::fix::log_row("> ", log.layout, &shape, today, narrative) else {
        return text;
    };
    let lines: Vec<&str> = text.lines().collect();
    let Some(marker) = crate::rules::log_marker_line(&lines, &id) else {
        return text;
    };
    let last = lines
        .iter()
        .enumerate()
        .skip(marker + 1)
        .take_while(|(_, l)| l.trim_start().starts_with('>'))
        .filter(|(_, l)| l.contains('|'))
        .map(|(i, _)| i)
        .last();
    let Some(last) = last else {
        return text;
    };
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    out.insert(last + 1, row);
    let mut joined = out.join("\n");
    joined.push('\n');
    joined
}

/// The header fields the template names beyond the required ones stay on the
/// new record, blank, when the type declares them (`BUG-188`); one the type
/// does not declare would only trip `header.field-set-consistency`, so it is
/// left out.
fn declared_template_fields(
    type_config: &crate::config::RecordTypeConfig,
    template: Option<&str>,
) -> Vec<String> {
    let declared = type_config.declared_fields();
    template
        .map(template_header_fields)
        .unwrap_or_default()
        .into_iter()
        .filter(|f| {
            type_config.known_fields.is_none()
                || declared.contains(&crate::values::FieldName::new(f.as_str()))
        })
        .collect()
}

/// The field names a template's header lists, in order: the `> Name: value`
/// lines of a blockquote header or the keys of a leading YAML frontmatter
/// block, read before the first `## ` section. Only the names are used
/// (`BUG-188`); the caller keeps those the type declares.
pub fn template_header_fields(template: &str) -> Vec<String> {
    let head = match template.find("\n## ") {
        Some(idx) => &template[..idx],
        None => template,
    };
    let mut names: Vec<String> = Vec::new();
    for line in head.lines() {
        let line = line.trim_start_matches('>').trim_start();
        let Some((name, _)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let plausible = !name.is_empty()
            && !name.starts_with('#')
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ' ' | '/'));
        if plausible && !names.iter().any(|n| n == name) {
            names.push(name.to_string());
        }
    }
    names
}

/// Each record type's template header as `field -> value`, keyed by the
/// type's name. Read by the caller from `.urzua/templates/<type>.md`, so
/// `urzua-core` never opens a file (`BUG-22`).
pub type TemplateValues =
    std::collections::HashMap<String, std::collections::HashMap<String, String>>;

/// The `field -> value` pairs a template's header carries: the same lines
/// [`template_header_fields`] reads, with the value after the first colon,
/// trimmed and with one pair of matching quotes removed. A template that
/// repeats a field keeps its first value.
pub fn template_header_values(template: &str) -> std::collections::HashMap<String, String> {
    let head = match template.find("\n## ") {
        Some(idx) => &template[..idx],
        None => template,
    };
    let mut values = std::collections::HashMap::new();
    for line in head.lines() {
        let line = line.trim_start_matches('>').trim_start();
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let plausible = !name.is_empty()
            && !name.starts_with('#')
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ' ' | '/'));
        if !plausible {
            continue;
        }
        let value = value.trim();
        let unquoted = ['"', '\'']
            .iter()
            .find_map(|q| value.strip_prefix(*q).and_then(|v| v.strip_suffix(*q)))
            .unwrap_or(value);
        values
            .entry(name.to_string())
            .or_insert_with(|| unquoted.trim().to_string());
    }
    values
}

/// A template's body -- everything from its first `## ` section heading
/// onward -- so a type declaring `yaml-frontmatter` (BUG-0003) still gets
/// its template's scaffolding (`## What`, `## Why`, the revision log)
/// spliced under synthesized frontmatter, instead of losing it entirely
/// because the header shapes don't match.
pub fn template_body(template: &str) -> Option<&str> {
    let idx = template.find("\n## ")?;
    Some(&template[idx + 1..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claims_on_other_refs_push_the_number_past_them() {
        let local = vec!["BUG-1-x.md".to_string()];
        let claims = vec![
            (
                "feat-a".to_string(),
                vec!["BUG-1-x.md".into(), "BUG-2-y.md".into()],
            ),
            (
                "origin/feat-b".to_string(),
                vec!["BUG-2-z.md".into(), "BUG-4-w.md".into()],
            ),
        ];
        let got = next_display_number_with_claims(&local, &claims).unwrap();
        assert_eq!(got.number, 5);
        assert_eq!(
            got.skipped,
            vec![
                (2, vec!["feat-a".to_string(), "origin/feat-b".to_string()]),
                (4, vec!["origin/feat-b".to_string()]),
            ]
        );
        assert_eq!(
            next_display_number_with_claims(&local, &[]).unwrap().number,
            2
        );
    }

    #[test]
    fn next_display_number_skips_past_the_highest_existing_prefix() {
        let filenames = vec![
            "ADR-1-first.md".to_string(),
            "ADR-16-a-bold-list-header-shape.md".to_string(),
            "ADR-9-ninth.md".to_string(),
        ];
        assert_eq!(next_display_number(&filenames).unwrap(), 17);
    }

    #[test]
    fn next_display_number_counts_both_filename_conventions() {
        // Replaces next_display_number_ignores_a_legacy_pre_type_prefix_filename,
        // which asserted BUG-9's exclusion using 0001 + ADR-9 -- a pair that
        // yields 10 whether legacy filenames are counted or ignored. It could
        // not fail, so it was never evidence for the behaviour it named.
        //
        // The exclusion itself is reversed (BUG-37): every filename in a corpus
        // adopted from a Nygard-style repository is `NNNN-slug.md`, so ignoring
        // them found nothing to count and `urzua new` wrote a duplicate.
        let mixed = vec!["0012-legacy.md".to_string(), "ADR-9-ninth.md".to_string()];
        assert_eq!(next_display_number(&mixed).unwrap(), 13);

        let nygard_only = vec![
            "0001-record-architecture-decisions.md".to_string(),
            "0009-help-scripts.md".to_string(),
        ];
        assert_eq!(next_display_number(&nygard_only).unwrap(), 10);
    }

    /// BUG-53: `2026-09-19-notes.md` parsed as record 2026, and since numbers
    /// are never reused the corpus was stuck above it permanently.
    /// A hyphenated type prefix is what `init` emits when two directories
    /// share a last component, and `new` derives the filename from it -- so a
    /// parser that assumed the prefix was one segment could not read back the
    /// files the tool itself wrote, and handed out the same number forever.
    #[test]
    fn a_hyphenated_type_prefix_parses_observed_failing() {
        assert_eq!(
            parse_record_filename("DOC-ADR-2-first-thing.md"),
            Some((Some("DOC-ADR"), 2))
        );
        assert_eq!(parse_record_filename("ADR-2-x.md"), Some((Some("ADR"), 2)));

        // A lower-case segment before the number is not a prefix.
        assert_eq!(parse_record_filename("doc-ADR-2-x.md"), None);
        // A number with nothing after it is a fragment.
        assert_eq!(parse_record_filename("DOC-ADR-2.md"), None);
        // A digit-bearing prefix is real (BUG-114): RecordTypeConfig.prefix
        // doesn't forbid `V2`, and a mixed-alnum prefix segment is never
        // confused with the number segment, since `at` already selects the
        // first *all*-digit one.
        assert_eq!(
            parse_record_filename("V2-3-migrate.md"),
            Some((Some("V2"), 3))
        );

        let names = vec!["DOC-ADR-1-a.md".to_string(), "DOC-ADR-7-b.md".to_string()];
        assert_eq!(next_display_number(&names).unwrap(), 8);
    }

    #[test]
    fn a_date_named_file_is_not_a_record_number_observed_failing() {
        assert_eq!(parse_record_filename("2026-09-19-meeting-notes.md"), None);
        assert_eq!(parse_record_filename("1999-01-01-x.md"), None);
        // Unpadded month/day are still a date (BUG-112): `2026-9-19-x.md` is
        // as much `2026-09-19-x.md` as it is, and pinning `next_display_number`
        // above 2026 is the same defect `BUG-53`/`BUG-102` already cover for
        // the padded shape.
        assert_eq!(parse_record_filename("2026-9-19-meeting-notes.md"), None);

        // A four-digit bound cannot discriminate -- a year is four digits --
        // so these must still parse.
        assert_eq!(parse_record_filename("2026-slug.md"), Some((None, 2026)));
        // Two 2-digit segments are not a date unless they are a plausible
        // month and day: an ADR titled "80-20 rule" is a record.
        assert_eq!(
            parse_record_filename("0013-80-20-rule.md"),
            Some((None, 13))
        );
        // A record number whose slug happens to start with a plausible
        // month and day is still a record: "13" is not a plausible year, so
        // `0013-01-15-use-postgres.md` must parse as record 13, never as an
        // unnumbered date (BUG-102) -- reissuing 13 collides with the file
        // that already claims it.
        assert_eq!(
            parse_record_filename("0013-01-15-use-postgres.md"),
            Some((None, 13))
        );
        assert_eq!(
            parse_record_filename("0001-record-architecture.md"),
            Some((None, 1))
        );

        let mixed = vec![
            "ADR-1-x.md".to_string(),
            "2026-09-19-meeting-notes.md".to_string(),
        ];
        assert_eq!(next_display_number(&mixed).unwrap(), 2);
    }

    #[test]
    fn parse_record_filename_reads_both_conventions_and_rejects_neither_shape() {
        assert_eq!(parse_record_filename("0001-slug.md"), Some((None, 1)));
        assert_eq!(parse_record_filename("7-slug.md"), Some((None, 7)));
        assert_eq!(
            parse_record_filename("ADR-12-slug.md"),
            Some((Some("ADR"), 12))
        );
        // A number with no slug behind it is a fragment, not a record.
        assert_eq!(parse_record_filename("0001.md"), None);
        assert_eq!(parse_record_filename("ADR-12.md"), None);
        assert_eq!(parse_record_filename("-999-slug.md"), None);
        assert_eq!(parse_record_filename("ADR-999"), None);
        assert_eq!(parse_record_filename("notes.md"), None);
    }

    #[test]
    fn parse_record_filename_reads_an_any_case_extension() {
        assert_eq!(parse_record_filename("ADR-5-x.MD"), Some((Some("ADR"), 5)));
        assert_eq!(parse_record_filename("0007-slug.Md"), Some((None, 7)));
        assert_eq!(parse_record_filename("ADR-5-x.mdx"), None);
    }

    #[test]
    fn next_display_number_rejects_a_malformed_directory_entry_observed_failing() {
        // A directory entry missing the .md extension, or with an empty type
        // prefix, must not silently contribute to the computed max -- the
        // shape must be validated in full, not just enough to find a number.
        let filenames = vec![
            "ADR-999".to_string(),      // no .md extension at all
            "-999-slug.md".to_string(), // empty type prefix
            "ADR-8-real.md".to_string(),
        ];
        assert_eq!(next_display_number(&filenames).unwrap(), 9);
    }

    #[test]
    fn next_display_number_fails_instead_of_overflowing() {
        let names = vec!["ADR-4294967295-x.md".to_string()];
        assert!(next_display_number(&names).is_err());
        assert!(next_display_number_with_claims(&names, &[]).is_err());
    }

    #[test]
    fn next_display_number_starts_at_one_for_an_empty_directory() {
        assert_eq!(next_display_number(&[]).unwrap(), 1);
    }

    #[test]
    fn slugify_lowercases_and_hyphenates_punctuation() {
        assert_eq!(
            slugify("A Bold-List Header Shape, Declared Per Profile"),
            "a-bold-list-header-shape-declared-per-profile"
        );
    }

    #[test]
    fn render_synthetic_yaml_lists_required_fields_as_blank() {
        let params = NewRecordParams {
            h1_prefix: None,
            h1_width: 0,
            initial_status: None,
            status_field: "Status",
            date_field: "Date",
            author_field: "Author",
            title_field: None,
            initial_version: None,
            display_number: 1,
            title: "X",
            stable_id: "01ABC",
            author: "beau",
            today: "2026-09-06",
        };
        let required = vec!["Status".to_string(), "Severity".to_string()];
        let result = render_synthetic_yaml(&params, &required, &[]);
        assert!(result.starts_with("---\n"));
        // A real YAML serializer renders an unset field as the `null` scalar,
        // not a bare trailing colon -- both are valid YAML for "unset," but
        // only the former is what `yaml_serde::to_string` actually emits.
        assert!(result.contains("Status: null\n"));
        assert!(result.contains("Severity: null\n"));
        assert!(result.contains("# 1 — X"));
    }

    #[test]
    fn a_required_stable_id_keeps_the_generated_value() {
        let params = NewRecordParams {
            h1_prefix: None,
            h1_width: 0,
            initial_status: None,
            status_field: "Status",
            date_field: "Date",
            author_field: "Author",
            title_field: None,
            initial_version: None,
            display_number: 1,
            title: "X",
            stable_id: "01ABC",
            author: "beau",
            today: "2026-09-06",
        };
        let required = vec!["Status".to_string(), "Stable-Id".to_string()];
        let result = render_synthetic_yaml(&params, &required, &[]);
        assert!(result.contains("Stable-Id: 01ABC\n"), "{result}");
        assert!(!result.contains("Stable-Id: null"), "{result}");
        assert_eq!(result.matches("Stable-Id").count(), 1, "{result}");
    }

    #[test]
    fn render_synthetic_yaml_escapes_a_value_with_an_embedded_colon() {
        // Real values `render_synthetic_yaml` writes are `Stable-Id`/`Date`/
        // `Author` -- `Author` is the one that's genuinely free text (e.g. a
        // "Last, First: Team" convention), the same shape of value as the
        // live MILE-4 `Blocked-on` case (`header::parse_key_value` only
        // splits on the first colon, so a colon-bearing value already
        // exists in this corpus). Verified through the real parser, not a
        // hand-edited string, so this proves the escaping `render_synthetic_
        // yaml` itself performs, not `yaml_serde`'s parser in isolation.
        let params = NewRecordParams {
            h1_prefix: None,
            h1_width: 0,
            initial_status: None,
            status_field: "Status",
            date_field: "Date",
            author_field: "Author",
            title_field: None,
            initial_version: None,
            display_number: 4,
            title: "X",
            stable_id: "01ABC",
            author: "Doe, J: Platform",
            today: "2026-09-06",
        };
        let result = render_synthetic_yaml(&params, &["Author".to_string()], &[]);
        let header =
            crate::header::parse_with_shape(&result, crate::header::HeaderShape::YamlFrontmatter);
        assert_eq!(header.get("Author"), Some("Doe, J: Platform"));
    }

    #[test]
    fn render_synthetic_yaml_round_trips_a_numeric_looking_string() {
        // A value that merely *looks* numeric must stay a string on
        // reparse, not silently become a YAML number -- checked through the
        // real parser `check`/`urzua new` themselves use.
        let params = NewRecordParams {
            h1_prefix: None,
            h1_width: 0,
            initial_status: None,
            status_field: "Status",
            date_field: "Date",
            author_field: "Author",
            title_field: None,
            initial_version: None,
            display_number: 1,
            title: "X",
            stable_id: "01ABC",
            author: "0.2",
            today: "2026-09-06",
        };
        let result = render_synthetic_yaml(&params, &["Author".to_string()], &[]);
        let header =
            crate::header::parse_with_shape(&result, crate::header::HeaderShape::YamlFrontmatter);
        assert_eq!(header.get("Author"), Some("0.2"));
    }

    #[test]
    fn template_header_values_read_blockquote_and_frontmatter_values() {
        let t = "# N — T\n\n> Status: Open | Fixed\n> Found-in: how/where: it was found\n> Empty:\n\n## What\nDate: nope\n";
        let v = template_header_values(t);
        assert_eq!(v.get("Status").map(String::as_str), Some("Open | Fixed"));
        assert_eq!(
            v.get("Found-in").map(String::as_str),
            Some("how/where: it was found")
        );
        assert_eq!(v.get("Empty").map(String::as_str), Some(""));
        assert!(!v.contains_key("Date"), "the body is not the header");
        let y = "---\nPhase: '0'\nTrack: \"t\"\n---\n# N\n";
        let v = template_header_values(y);
        assert_eq!(v.get("Phase").map(String::as_str), Some("0"));
        assert_eq!(v.get("Track").map(String::as_str), Some("t"));
    }

    #[test]
    fn template_body_starts_at_the_first_section_heading() {
        let template = "---\nStatus: Planned\nPhase: 0\n---\n# NNNN — Title\n\n## What\n\nDeliver X.\n\n## Why\n\nBecause Y.\n";
        let body = template_body(template).unwrap();
        assert!(body.starts_with("## What"));
        assert!(body.contains("## Why"));
        assert!(!body.contains("Status: Planned"));
    }

    #[test]
    fn template_body_is_none_without_a_section_heading() {
        assert_eq!(
            template_body("---\nStatus: Planned\n---\n# NNNN — Title\n"),
            None
        );
    }

    fn compiled(pattern: &str) -> crate::document_model::CompiledIdentity {
        crate::document_model::CompiledIdentity::compile(&crate::document_model::IdentityDecl {
            from: crate::document_model::IdentitySource::Filename,
            pattern: pattern.to_string(),
        })
        .unwrap()
    }

    #[test]
    fn a_declared_file_name_is_padded_to_the_width_the_pattern_requires() {
        let four = compiled(r"^(?P<prefix>ADR)-(?P<number>\d{4})-(?P<slug>.+)$");
        assert_eq!(
            declared_file_name(&four, "p", "ADR", 4, "x-y").unwrap(),
            "ADR-0004-x-y.md"
        );
        let open = compiled(r"^(?P<prefix>ADR)-(?P<number>\d+)-(?P<slug>.+)$");
        assert_eq!(
            declared_file_name(&open, "p", "ADR", 4, "x").unwrap(),
            "ADR-4-x.md",
            "an open-ended pattern keeps the unpadded default"
        );
    }

    #[test]
    fn a_pattern_no_name_fits_is_refused_naming_the_pattern() {
        let other = compiled(r"^(?P<prefix>ADR)_(?P<number>\d+)_(?P<slug>.+)$");
        let err = declared_file_name(&other, "the-pattern", "ADR", 4, "x").unwrap_err();
        assert!(err.contains("the-pattern"), "{err}");
        // A prefix the pattern pins to something else is not this type's.
        let pinned = compiled(r"^(?P<prefix>DOC)-(?P<number>\d+)-(?P<slug>.+)$");
        assert!(declared_file_name(&pinned, "p", "ADR", 4, "x").is_err());
    }

    #[test]
    fn numbering_by_a_declared_pattern_reads_only_what_it_matches() {
        let four = compiled(r"^(?P<prefix>ADR)-(?P<number>\d{4})-(?P<slug>.+)$");
        let names: Vec<String> = ["ADR-0007-a.md", "README.md", "ADR-9-short.md"]
            .map(String::from)
            .to_vec();
        let n = next_display_number_by(&names, &|name| declared_number(&four, name)).unwrap();
        assert_eq!(n, 8, "ADR-9-short.md is off-pattern and takes no number");
    }

    #[test]
    fn template_header_fields_reads_blockquote_and_frontmatter_names_in_order() {
        let t = "# NNNN — Title\n\n> Rule: r\n> Expires: YYYY-MM-DD (optional)\n> Rule: again\n\n## What\n\nNote: prose\n";
        assert_eq!(template_header_fields(t), ["Rule", "Expires"]);
        let y = "---\nStatus: x\nSeverity: y\n---\n# T\n\n## S\n";
        assert_eq!(template_header_fields(y), ["Status", "Severity"]);
    }

    #[test]
    fn date_and_author_are_filled_by_their_declared_field_names() {
        let params = NewRecordParams {
            display_number: 1,
            h1_prefix: None,
            h1_width: 0,
            initial_status: None,
            status_field: "Status",
            date_field: "Created",
            author_field: "Owner",
            title_field: None,
            initial_version: None,
            title: "T",
            stable_id: "01ABC",
            author: "beau",
            today: "2026-11-04",
        };
        let required = vec![
            "Created".to_string(),
            "Owner".to_string(),
            "Date".to_string(),
        ];
        let rendered = render_synthetic_yaml(&params, &required, &[]);
        assert!(
            rendered.contains("Created: '2026-11-04'") || rendered.contains("Created: 2026-11-04"),
            "{rendered}"
        );
        assert!(rendered.contains("Owner: beau"), "{rendered}");
        assert!(rendered.contains("Date: null"), "{rendered}");
    }

    #[test]
    fn the_declared_title_and_version_fields_are_filled_and_the_rest_stay_blank() {
        let params = NewRecordParams {
            display_number: 1,
            h1_prefix: None,
            h1_width: 0,
            initial_status: None,
            status_field: "Status",
            date_field: "Date",
            author_field: "Author",
            title_field: Some("Subject"),
            initial_version: Some(("Version", "0.1")),
            title: "Probe scaffold",
            stable_id: "01ABC",
            author: "beau",
            today: "2026-11-07",
        };
        let required = vec![
            "Version".to_string(),
            "Subject".to_string(),
            "Other".to_string(),
        ];
        let rendered = render_synthetic_yaml(&params, &required, &[]);
        assert!(rendered.contains("Subject: Probe scaffold"), "{rendered}");
        assert!(
            rendered.contains("Version: '0.1'") || rendered.contains("Version: \"0.1\""),
            "a version stays a string: {rendered}"
        );
        assert!(rendered.contains("Other: null"), "{rendered}");
        let undeclared = NewRecordParams {
            title_field: None,
            initial_version: None,
            ..params
        };
        let rendered = render_synthetic_yaml(&undeclared, &required, &[]);
        assert!(rendered.contains("Subject: null"), "{rendered}");
        assert!(rendered.contains("Version: null"), "{rendered}");
    }

    #[test]
    fn the_status_field_starts_at_the_initial_status_and_stays_blank_without_one() {
        let mut params = NewRecordParams {
            display_number: 4,
            h1_prefix: Some("ADR"),
            h1_width: 0,
            initial_status: Some("Proposed"),
            status_field: "State",
            date_field: "Date",
            author_field: "Author",
            title_field: None,
            initial_version: None,
            title: "T",
            stable_id: "01ABC",
            author: "a",
            today: "2026-11-04",
        };
        let rendered = render_synthetic_yaml(&params, &["State".to_string()], &[]);
        assert!(rendered.contains("State: Proposed\n"), "{rendered}");
        assert!(rendered.contains("Stable-Id: 01ABC"), "{rendered}");
        assert!(rendered.contains("# ADR-4 — T\n"), "{rendered}");
        params.initial_status = None;
        let rendered = render_synthetic_yaml(&params, &["State".to_string()], &[]);
        assert!(rendered.contains("State: null"), "{rendered}");
    }

    #[test]
    fn scaffold_record_builds_header_and_template_body_from_a_synthetic_type() {
        let type_config = crate::config::RecordTypeConfig {
            dir: "docs/x".to_string(),
            required_fields: vec!["Status".to_string(), "Author".to_string()],
            known_fields: Some(vec![
                "Status".to_string(),
                "Author".to_string(),
                "Note".to_string(),
            ]),
            ..Default::default()
        };
        let template =
            "---\nStatus: null\nNote: null\nOther: null\n---\n# 0 -- T\n\n## What\n\nbody\n";
        let spec = ScaffoldSpec {
            type_config: &type_config,
            compiled: None,
            type_prefix: "X",
            filename: "X-3-t.md",
            display_number: 3,
            title: "Synthetic",
            stable_id: "01SYNTH",
            author: "tester",
            today: "2026-11-13",
            reason: None,
            expires: None,
            header_fields: &[],
            extra_body: None,
            log_row: None,
        };
        let out = scaffold_record(&spec, Some(template));
        // Header: generated id, filled author, blank decisions, the declared
        // template field kept and the undeclared one dropped.
        assert!(out.starts_with("---\nStable-Id: 01SYNTH\n"), "{out}");
        assert!(out.contains("Author: tester\n"), "{out}");
        assert!(out.contains("Status: null\n"), "{out}");
        assert!(out.contains("Note: null\n"), "{out}");
        assert!(!out.contains("Other"), "{out}");
        assert!(out.contains("# 3 — Synthetic\n"), "{out}");
        // The template's body scaffolding follows the synthesized header.
        assert!(out.ends_with("\n## What\n\nbody\n"), "{out}");
        // Without a template there is only the header and the H1.
        let bare = scaffold_record(&spec, None);
        assert!(bare.ends_with("# 3 — Synthetic\n"), "{bare}");
    }

    fn waiver_type() -> crate::config::RecordTypeConfig {
        crate::config::RecordTypeConfig {
            dir: "docs/w".to_string(),
            required_fields: vec![
                "Rule".to_string(),
                "Scope".to_string(),
                "Reason".to_string(),
            ],
            known_fields: Some(vec!["Expires".to_string(), "Owner".to_string()]),
            revision_log: Some(crate::config::RevisionLogDecl {
                marker: Some("**Revision log**".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    const WAIVER_TEMPLATE: &str = "---\nRule: null\nExpires: null\nOwner: null\n---\n# N -- T\n\n## What\n\nbody\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n";

    /// The compiled-identity path: the H1 carries the type prefix and the number
    /// padded to the width the declared pattern reads back from the file name
    /// (`h1_prefix`, `h1_width`, `BUG-188`); without a declared identity it is
    /// the bare number.
    #[test]
    fn the_h1_is_padded_to_the_declared_identity_width_and_bare_without_one() {
        let type_config = waiver_type();
        let padded = compiled(r"^(?P<prefix>WAIVER)-(?P<number>\d{4})-(?P<slug>.+)$");
        let unpadded = compiled(r"^(?P<prefix>WAIVER)-(?P<number>\d+)-(?P<slug>.+)$");
        let h1 = |compiled: Option<&crate::document_model::CompiledIdentity>, filename: &str| {
            let spec = ScaffoldSpec::plain(
                &type_config,
                compiled,
                "WAIVER",
                filename,
                7,
                "T",
                "01ID",
                "a",
                "2026-11-13",
            );
            scaffold_record(&spec, None)
                .lines()
                .find(|l| l.starts_with("# "))
                .unwrap()
                .to_string()
        };
        assert_eq!(h1(Some(&padded), "WAIVER-0007-t.md"), "# WAIVER-0007 — T");
        assert_eq!(h1(Some(&unpadded), "WAIVER-7-t.md"), "# WAIVER-7 — T");
        assert_eq!(h1(None, "WAIVER-7-t.md"), "# 7 — T");
    }

    #[test]
    fn supplied_values_fill_the_header_and_the_body_and_log_follow_the_template() {
        let type_config = waiver_type();
        let spec = ScaffoldSpec {
            reason: Some("Pending"),
            expires: Some("2027-01-31"),
            header_fields: &[("Rule", "r.x"), ("Scope", "docs/a.md"), ("Owner", "me")],
            extra_body: Some("## Findings\n\n- docs/a.md:3 msg"),
            log_row: Some("Drafted."),
            ..ScaffoldSpec::plain(
                &type_config,
                None,
                "WAIVER",
                "WAIVER-3-t.md",
                3,
                "T",
                "01ID",
                "a",
                "2026-11-13",
            )
        };
        let out = scaffold_record(&spec, Some(WAIVER_TEMPLATE));
        assert!(
            out.starts_with(
                "---\nStable-Id: 01ID\nRule: r.x\nScope: docs/a.md\nReason: Pending\nExpires: "
            ),
            "{out}"
        );
        assert!(out.contains("2027-01-31"), "{out}");
        assert!(out.contains("Owner: me\n---\n"), "{out}");
        // The row is laid out by the log's own header, after the separator, and
        // the supplied body follows the whole template.
        assert!(
            out.contains("> |---|---|---|\n> | 2026-11-13 | Drafted. | **structural** |\n"),
            "{out}"
        );
        assert!(
            out.ends_with("\n## Findings\n\n- docs/a.md:3 msg\n"),
            "{out}"
        );
        let log_at = out.find("**Revision log**").unwrap();
        assert!(log_at < out.find("## Findings").unwrap(), "{out}");
    }

    #[test]
    fn nothing_supplied_leaves_the_scaffold_as_new_writes_it() {
        let type_config = waiver_type();
        let spec = ScaffoldSpec::plain(
            &type_config,
            None,
            "WAIVER",
            "WAIVER-3-t.md",
            3,
            "T",
            "01ID",
            "a",
            "2026-11-13",
        );
        let out = scaffold_record(&spec, Some(WAIVER_TEMPLATE));
        assert!(
            out.contains("Rule: null\nScope: null\nReason: null\nExpires: null\nOwner: null\n"),
            "{out}"
        );
        assert!(out.ends_with("> |---|---|---|\n"), "{out}");
    }
}
