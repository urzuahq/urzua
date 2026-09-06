//! `urzua new`: render a record's initial content. Pure -- the caller reads
//! any existing template, resolves identity/today, computes the next
//! display number, and performs the actual file write; this module only
//! computes text.

/// The next unused display number, given the filenames already in a record
/// type's directory. Never reuses a number even if one was deleted --
/// cross-references elsewhere may still assume the old numbering held.
pub fn next_display_number(filenames: &[String]) -> u32 {
    filenames
        .iter()
        .filter_map(|name| name.split('-').next())
        .filter_map(|prefix| prefix.parse::<u32>().ok())
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
    let number = format!("{:04}", params.display_number);
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
pub fn render_synthetic_yaml(params: &NewRecordParams, required_fields: &[String]) -> String {
    let mut out = String::from("---\n");
    out.push_str(&format!("Stable-Id: {}\n", params.stable_id));
    out.push_str(&format!("Date: {}\n", params.today));
    out.push_str(&format!("Author: {}\n", params.author));
    for field in required_fields {
        if field == "Date" || field == "Author" {
            continue;
        }
        out.push_str(&format!("{field}: \n"));
    }
    out.push_str("---\n");
    out.push_str(&format!(
        "# {:04} — {}\n",
        params.display_number, params.title
    ));
    out
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
        assert!(result.contains("# 0042 — Use a real title"));
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
        assert!(result.contains("# 0028 — X"));
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
        assert!(result.contains("Status: \n"));
        assert!(result.contains("Severity: \n"));
        assert!(result.contains("# 0001 — X"));
    }
}
