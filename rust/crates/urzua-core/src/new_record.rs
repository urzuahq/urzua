//! `urzua new`: render a record's initial content. Pure -- the caller reads
//! any existing template, resolves identity/today, computes the next
//! display number, and performs the actual file write; this module only
//! computes text.

/// The next unused display number, given the filenames already in a record
/// type's directory. Never reuses a number even if one was deleted --
/// cross-references elsewhere may still assume the old numbering held.
///
/// Reads both filename shapes (ADR-0036): legacy `NNNN-slug.md` (number is
/// the first segment) and the current default `TYPE-NNNN-slug.md` (number
/// is the second segment) -- a directory with a mix of old and new
/// filenames still finds the true max across both, never colliding.
pub fn next_display_number(filenames: &[String]) -> u32 {
    filenames
        .iter()
        .filter_map(|name| {
            let mut parts = name.split('-');
            let first = parts.next()?;
            if let Ok(n) = first.parse::<u32>() {
                return Some(n);
            }
            parts.next()?.parse::<u32>().ok()
        })
        .max()
        .map_or(1, |highest| highest + 1)
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
    pub title: &'a str,
    pub stable_id: &'a str,
    pub author: &'a str,
    pub today: &'a str,
}

/// Render from an existing `.urzua/templates/<type>.md` (blockquote/bold-list
/// shapes, whatever this corpus already uses): substitutes the filename
/// number and title into the H1, fills `Date`/`Author`, and inserts a
/// `Stable-Id` line into the header -- everything else (Status choices,
/// Deciders, Embodiment) stays the template's own placeholder text for a
/// human to pick, same as it always has been.
pub fn render_from_template(template: &str, params: &NewRecordParams) -> String {
    let number = format!("{}", params.display_number);
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
        let stable_id_line = if lines[idx].trim_start().starts_with('>') {
            format!("> Stable-Id: {}", params.stable_id)
        } else {
            format!("- **Stable-Id:** {}", params.stable_id)
        };
        lines.insert(idx + 1, stable_id_line);
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
    mapping.insert(
        Value::String("Stable-Id".to_string()),
        Value::String(params.stable_id.to_string()),
    );
    for field in required_fields {
        let value = match field.as_str() {
            "Date" => Value::String(params.today.to_string()),
            "Author" => Value::String(params.author.to_string()),
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
    out.push_str(&format!("# {} — {}\n", params.display_number, params.title));
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
            display_number: 42,
            title: "Use a real title",
            stable_id: "01ABC",
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
            display_number: 28,
            title: "X",
            stable_id: "01ABC",
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
            display_number: 1,
            title: "X",
            stable_id: "01ABC",
            author: "beau",
            today: "2026-09-06",
        };
        let result = render_from_template(template, &params);
        assert!(result.contains("> Status: Proposed | Accepted | Rejected"));
    }

    #[test]
    fn next_display_number_skips_past_the_highest_existing_prefix() {
        let filenames = vec![
            "0001-first.md".to_string(),
            "0016-a-bold-list-header-shape.md".to_string(),
            "0009-ninth.md".to_string(),
        ];
        assert_eq!(next_display_number(&filenames), 17);
    }

    #[test]
    fn next_display_number_finds_the_max_across_mixed_legacy_and_type_prefixed_filenames() {
        // ADR-0036: a directory can hold both shapes at once (existing
        // legacy files never get renamed); the true max must span both.
        let filenames = vec![
            "0001-first.md".to_string(),
            "ADR-0036-newer.md".to_string(),
            "0009-ninth.md".to_string(),
        ];
        assert_eq!(next_display_number(&filenames), 37);
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
            display_number: 1,
            title: "X",
            stable_id: "01ABC",
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
            display_number: 4,
            title: "X",
            stable_id: "01ABC",
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
            display_number: 1,
            title: "X",
            stable_id: "01ABC",
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
}
