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
    let stem = file_name.strip_suffix(".md")?;

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
pub fn next_display_number(filenames: &[String]) -> u32 {
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
) -> u32 {
    filenames
        .iter()
        .filter_map(|name| number_of(name))
        .max()
        .map_or(1, |highest| highest + 1)
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
) -> NumberAssignment {
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
) -> NumberAssignment {
    let local_next = next_display_number_by(local, number_of);
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
    let number = by_number.keys().next_back().map_or(local_next, |m| m + 1);
    NumberAssignment {
        number,
        skipped: by_number.into_iter().collect(),
    }
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
    /// The status a new record starts at: the first value of the type's
    /// declared `statuses.values`. `None` when the type declares none, and the
    /// field stays blank for a human.
    pub initial_status: Option<&'a str>,
    /// The field that holds the status, as the type declares it
    /// (`RecordTypeConfig::relation_field`), not a literal.
    pub status_field: &'a str,
    pub title: &'a str,
    /// `None` when the type's closed field set does not declare `Stable-Id`,
    /// which would reject it.
    pub stable_id: Option<&'a str>,
    pub author: &'a str,
    pub today: &'a str,
}

impl NewRecordParams<'_> {
    /// What stands for the number in the H1: `ADR-4` for a type with a declared
    /// identity, the bare number otherwise.
    fn h1_number(&self) -> String {
        match self.h1_prefix {
            Some(prefix) => format!("{prefix}-{}", self.display_number),
            None => self.display_number.to_string(),
        }
    }
}

/// Render from an existing `.urzua/templates/<type>.md` (blockquote/bold-list
/// shapes, whatever this corpus already uses): substitutes the filename
/// number and title into the H1, fills `Date`/`Author`, and inserts a
/// `Stable-Id` line into the header -- everything else (Status choices,
/// Deciders, Embodiment) stays the template's own placeholder text for a
/// human to pick, same as it always has been.
pub fn render_from_template(template: &str, params: &NewRecordParams) -> String {
    let number = params.h1_number();
    let mut lines: Vec<String> = template.lines().map(|l| l.to_string()).collect();

    // Only the H1 names *this* record's number -- a template's own body can
    // legitimately contain other "NNNN"-shaped placeholders (e.g.
    // `Derives-from: RFC-NNNN (optional)`) that must stay untouched, not get
    // overwritten with this record's own number.
    if let Some(h1) = lines.first_mut() {
        *h1 = h1.replacen("NNNN", &number, 1);
        *h1 = h1.replacen("Title", params.title, 1);
    }

    // Fill Date and Author on their own template lines, wherever they are --
    // don't assume a fixed line index, since blockquote and bold-list
    // headers order fields differently.
    for line in &mut lines {
        let is_author_line = line.contains("Author:")
            && (line.trim_start().starts_with('>') || line.trim_start().starts_with("- **"));
        if line.contains("Date:") && line.contains("YYYY-MM-DD") {
            *line = line.replace("YYYY-MM-DD", params.today);
        } else if is_author_line {
            *line = line.replacen("name", params.author, 1);
        }
    }

    // Insert Stable-Id right after the first header line, matching
    // `migrate ids`'s own insertion point.
    let first_header_line = lines
        .iter()
        .position(|l| l.trim_start().starts_with('>') || l.trim_start().starts_with("- **"));
    if let Some(idx) = first_header_line {
        if let Some(stable_id) = params.stable_id {
            let stable_id_line = if lines[idx].trim_start().starts_with('>') {
                format!("> Stable-Id: {stable_id}")
            } else {
                format!("- **Stable-Id:** {stable_id}")
            };
            lines.insert(idx + 1, stable_id_line);
        }
    }

    lines.join("\n") + "\n"
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
pub fn render_synthetic_yaml(params: &NewRecordParams, required_fields: &[String]) -> String {
    use yaml_serde::Value;

    let mut mapping = yaml_serde::Mapping::new();
    if let Some(stable_id) = params.stable_id {
        mapping.insert(
            Value::String("Stable-Id".to_string()),
            Value::String(stable_id.to_string()),
        );
    }
    for field in required_fields {
        let value = match field.as_str() {
            "Date" => Value::String(params.today.to_string()),
            "Author" => Value::String(params.author.to_string()),
            f if f == params.status_field => params
                .initial_status
                .map_or(Value::Null, |s| Value::String(s.to_string())),
            _ => Value::Null,
        };
        mapping.insert(Value::String(field.clone()), value);
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
    fn render_from_template_fills_number_title_date_author_and_stable_id() {
        let template = "# NNNN — Title\n\n> Status: Proposed\n> Date: YYYY-MM-DD\n> Author: name\n";
        let params = NewRecordParams {
            h1_prefix: None,
            initial_status: None,
            status_field: "Status",
            display_number: 42,
            title: "Use a real title",
            stable_id: Some("01ABC"),
            author: "beau",
            today: "2026-09-06",
        };
        let result = render_from_template(template, &params);
        assert!(result.contains("# 42 — Use a real title"));
        assert!(result.contains("> Date: 2026-09-06"));
        assert!(result.contains("> Author: beau"));
        assert!(result.contains("> Stable-Id: 01ABC"));
    }

    #[test]
    fn render_from_template_does_not_touch_an_nnnn_placeholder_outside_the_h1() {
        let template = "# NNNN — Title\n\n> Derives-from: RFC-NNNN (optional)\n> Author: name\n";
        let params = NewRecordParams {
            h1_prefix: None,
            initial_status: None,
            status_field: "Status",
            display_number: 28,
            title: "X",
            stable_id: Some("01ABC"),
            author: "beau",
            today: "2026-09-06",
        };
        let result = render_from_template(template, &params);
        assert!(result.contains("# 28 — X"));
        assert!(result.contains("> Derives-from: RFC-NNNN (optional)"));
    }

    #[test]
    fn render_from_template_preserves_status_choices_untouched() {
        let template = "# NNNN — Title\n\n> Status: Proposed | Accepted | Rejected\n";
        let params = NewRecordParams {
            h1_prefix: None,
            initial_status: None,
            status_field: "Status",
            display_number: 1,
            title: "X",
            stable_id: Some("01ABC"),
            author: "beau",
            today: "2026-09-06",
        };
        let result = render_from_template(template, &params);
        assert!(result.contains("> Status: Proposed | Accepted | Rejected"));
    }

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
        let got = next_display_number_with_claims(&local, &claims);
        assert_eq!(got.number, 5);
        assert_eq!(
            got.skipped,
            vec![
                (2, vec!["feat-a".to_string(), "origin/feat-b".to_string()]),
                (4, vec!["origin/feat-b".to_string()]),
            ]
        );
        assert_eq!(next_display_number_with_claims(&local, &[]).number, 2);
    }

    #[test]
    fn next_display_number_skips_past_the_highest_existing_prefix() {
        let filenames = vec![
            "ADR-1-first.md".to_string(),
            "ADR-16-a-bold-list-header-shape.md".to_string(),
            "ADR-9-ninth.md".to_string(),
        ];
        assert_eq!(next_display_number(&filenames), 17);
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
        assert_eq!(next_display_number(&mixed), 13);

        let nygard_only = vec![
            "0001-record-architecture-decisions.md".to_string(),
            "0009-help-scripts.md".to_string(),
        ];
        assert_eq!(next_display_number(&nygard_only), 10);
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
        assert_eq!(next_display_number(&names), 8);
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
        assert_eq!(next_display_number(&mixed), 2);
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
    fn next_display_number_rejects_a_malformed_directory_entry_observed_failing() {
        // A directory entry missing the .md extension, or with an empty type
        // prefix, must not silently contribute to the computed max -- the
        // shape must be validated in full, not just enough to find a number.
        let filenames = vec![
            "ADR-999".to_string(),      // no .md extension at all
            "-999-slug.md".to_string(), // empty type prefix
            "ADR-8-real.md".to_string(),
        ];
        assert_eq!(next_display_number(&filenames), 9);
    }

    #[test]
    fn next_display_number_starts_at_one_for_an_empty_directory() {
        assert_eq!(next_display_number(&[]), 1);
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
            initial_status: None,
            status_field: "Status",
            display_number: 1,
            title: "X",
            stable_id: Some("01ABC"),
            author: "beau",
            today: "2026-09-06",
        };
        let required = vec!["Status".to_string(), "Severity".to_string()];
        let result = render_synthetic_yaml(&params, &required);
        assert!(result.starts_with("---\n"));
        // A real YAML serializer renders an unset field as the `null` scalar,
        // not a bare trailing colon -- both are valid YAML for "unset," but
        // only the former is what `yaml_serde::to_string` actually emits.
        assert!(result.contains("Status: null\n"));
        assert!(result.contains("Severity: null\n"));
        assert!(result.contains("# 1 — X"));
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
            initial_status: None,
            status_field: "Status",
            display_number: 4,
            title: "X",
            stable_id: Some("01ABC"),
            author: "Doe, J: Platform",
            today: "2026-09-06",
        };
        let result = render_synthetic_yaml(&params, &["Author".to_string()]);
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
            initial_status: None,
            status_field: "Status",
            display_number: 1,
            title: "X",
            stable_id: Some("01ABC"),
            author: "0.2",
            today: "2026-09-06",
        };
        let result = render_synthetic_yaml(&params, &["Author".to_string()]);
        let header =
            crate::header::parse_with_shape(&result, crate::header::HeaderShape::YamlFrontmatter);
        assert_eq!(header.get("Author"), Some("0.2"));
    }

    #[test]
    fn template_body_starts_at_the_first_section_heading() {
        let template = "# NNNN — Title\n\n> Status: Planned\n> Phase: 0\n\n## What\n\nDeliver X.\n\n## Why\n\nBecause Y.\n";
        let body = template_body(template).unwrap();
        assert!(body.starts_with("## What"));
        assert!(body.contains("## Why"));
        assert!(!body.contains("Status: Planned"));
    }

    #[test]
    fn template_body_is_none_without_a_section_heading() {
        assert_eq!(template_body("# NNNN — Title\n\n> Status: Planned\n"), None);
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
        let n = next_display_number_by(&names, &|name| declared_number(&four, name));
        assert_eq!(n, 8, "ADR-9-short.md is off-pattern and takes no number");
    }

    #[test]
    fn the_status_field_starts_at_the_initial_status_and_stays_blank_without_one() {
        let mut params = NewRecordParams {
            display_number: 4,
            h1_prefix: Some("ADR"),
            initial_status: Some("Proposed"),
            status_field: "State",
            title: "T",
            stable_id: None,
            author: "a",
            today: "2026-11-04",
        };
        let rendered = render_synthetic_yaml(&params, &["State".to_string()]);
        assert!(rendered.contains("State: Proposed\n"), "{rendered}");
        assert!(!rendered.contains("Stable-Id"), "{rendered}");
        assert!(rendered.contains("# ADR-4 — T\n"), "{rendered}");
        params.initial_status = None;
        let rendered = render_synthetic_yaml(&params, &["State".to_string()]);
        assert!(rendered.contains("State: null"), "{rendered}");
    }
}
