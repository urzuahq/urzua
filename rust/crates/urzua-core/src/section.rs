//! Splits a record body into its `##` sections (RFC-33 layer 1, MILE-98).
//!
//! Plain data for later rules: this module decides nothing about a
//! section's meaning, only where each one starts and ends. It reads no
//! declaration and emits no finding.

/// One section of a record body: a `##` heading and, when the type declares a
/// `depth`, the deeper headings nested under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// The heading text: trimmed, with any closing `#` sequence removed.
    /// Empty for a bare `##`.
    pub heading: String,
    /// The heading's level: the number of `#`, so 2 for `##`, 3 for `###`.
    pub level: u8,
    /// The index, in the same list, of the nearest preceding section of a
    /// shallower level that is still open; `None` for a section that sits
    /// under no other (every `##`).
    pub parent: Option<usize>,
    /// 1-based line of the heading line.
    pub line: usize,
    /// 1-based line of the section's last line (inclusive): the line before
    /// the next heading of the same or a shallower level, or the last line of
    /// the body. A section's range includes the sections nested in it.
    pub end_line: usize,
    /// The lines after the heading line through `end_line`, each followed by
    /// `\n`; carriage returns are not included. Nested headings are part of it.
    pub body: String,
}

/// Splits `body` into its `##` sections, in document order. Equivalent to
/// [`parse_sections_to_depth`] at depth 1.
pub fn parse_sections(body: &str, first_line: usize) -> Vec<Section> {
    parse_sections_to_depth(body, first_line, 1)
}

/// Splits `body` into sections, `depth` levels counting from `##`: depth 1
/// reads `##` only, 2 reads `##` and `###`, and so on (`sections.depth`).
/// Sections are listed in document order, a section before those nested in it.
///
/// `first_line` is the 1-based line number of the first line of `body`
/// within its file, so a caller that strips a header region first still gets
/// file-relative numbers. Text before the first heading belongs to no section.
/// Duplicate headings are kept as separate sections; reporting them is a
/// rule's job. Only ATX headings count: a heading inside a fenced code block,
/// an HTML comment (`<!-- ... -->`, on one line or several), or indented four or more spaces, is not one. A heading deeper than `depth`
/// is body text of the sections around it. Lines may end `\n` or `\r\n`.
pub fn parse_sections_to_depth(body: &str, first_line: usize, depth: u8) -> Vec<Section> {
    let deepest = depth.saturating_add(1);
    let mut lines: Vec<&str> = body.split('\n').collect();
    // A trailing newline ends the last line; it does not start another.
    if lines.last() == Some(&"") {
        lines.pop();
    }

    let mut sections: Vec<Section> = Vec::new();
    // Indexes of the sections still open, shallowest first.
    let mut open: Vec<usize> = Vec::new();
    let mut fence: Option<Fence> = None;
    let mut in_comment = false;
    for (i, raw) in lines.iter().enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let number = first_line + i;
        let mut heading = None;
        if in_comment {
            // Everything up to the closing `-->` is comment text, a `##` line
            // included.
            in_comment = !line.contains("-->");
        } else if let Some(f) = fence {
            if f.is_closed_by(line) {
                fence = None;
            }
        } else if let Some(f) = Fence::opening(line) {
            fence = Some(f);
        } else if let Some(unclosed) = html_comment_opening(line) {
            in_comment = unclosed;
        } else {
            heading = atx_heading(line).filter(|(level, _)| (2..=deepest).contains(level));
        }
        if let Some((level, _)) = heading {
            open.retain(|&j| sections[j].level < level);
        }
        // The line belongs to every section still open, a heading line to its
        // ancestors only.
        for &j in &open {
            sections[j].end_line = number;
            sections[j].body.push_str(line);
            sections[j].body.push('\n');
        }
        if let Some((level, text)) = heading {
            sections.push(Section {
                heading: text,
                level,
                parent: open.last().copied(),
                line: number,
                end_line: number,
                body: String::new(),
            });
            open.push(sections.len() - 1);
        }
    }
    sections
}

/// `Some(true)` when `line` opens an HTML comment it does not close, `Some(false)`
/// when it opens and closes one, `None` when it does not start with `<!--`.
fn html_comment_opening(line: &str) -> Option<bool> {
    let rest = indented_at_most_three(line)?.strip_prefix("<!--")?;
    Some(!rest.contains("-->"))
}

/// An open fenced code block: the marker character and its run length.
#[derive(Clone, Copy)]
struct Fence {
    marker: char,
    len: usize,
}

impl Fence {
    fn opening(line: &str) -> Option<Fence> {
        let rest = indented_at_most_three(line)?;
        let marker = rest.chars().next().filter(|c| *c == '`' || *c == '~')?;
        let len = rest.chars().take_while(|c| *c == marker).count();
        // An info string after a backtick fence may not itself contain one.
        if len < 3 || (marker == '`' && rest[len..].contains('`')) {
            return None;
        }
        Some(Fence { marker, len })
    }

    fn is_closed_by(self, line: &str) -> bool {
        let Some(rest) = indented_at_most_three(line) else {
            return false;
        };
        let run = rest.chars().take_while(|c| *c == self.marker).count();
        run >= self.len && rest[run..].trim().is_empty()
    }
}

/// `line` without its leading spaces, or `None` when it has four or more
/// (or a tab), which makes it indented code rather than markup.
fn indented_at_most_three(line: &str) -> Option<&str> {
    let rest = line.trim_start_matches(' ');
    (line.len() - rest.len() <= 3 && !rest.starts_with('\t')).then_some(rest)
}

/// The level and text when `line` is an ATX heading (1 to 6 `#`).
fn atx_heading(line: &str) -> Option<(u8, String)> {
    let rest = indented_at_most_three(line)?;
    let hashes = rest.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = &rest[hashes..];
    if !(rest.is_empty() || rest.starts_with([' ', '\t'])) {
        return None;
    }
    let text = rest.trim();
    let open = text.trim_end_matches('#');
    // A closing `#` sequence counts only when set off by whitespace.
    let text = if open.is_empty() || open.ends_with([' ', '\t']) {
        open.trim_end()
    } else {
        text
    };
    Some((hashes as u8, text.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headings(sections: &[Section]) -> Vec<&str> {
        sections.iter().map(|s| s.heading.as_str()).collect()
    }

    #[test]
    fn a_body_with_no_sections_yields_none() {
        assert!(parse_sections("", 1).is_empty());
        assert!(parse_sections("Just prose.\n\n# Title\n\n### Not h2\n", 1).is_empty());
    }

    #[test]
    fn sections_carry_heading_line_end_line_and_body() {
        let body = "# Title\n\nintro\n\n## Context\nwhy\n\n## Decision\nwhat\nmore\n";
        let s = parse_sections(body, 1);
        assert_eq!(headings(&s), ["Context", "Decision"]);
        assert_eq!((s[0].line, s[0].end_line), (5, 7));
        assert_eq!(s[0].body, "why\n\n");
        assert_eq!((s[1].line, s[1].end_line), (8, 10));
        assert_eq!(s[1].body, "what\nmore\n");
    }

    #[test]
    fn first_line_offsets_every_number() {
        let s = parse_sections("## A\nx\n## B\n", 10);
        assert_eq!((s[0].line, s[0].end_line), (10, 11));
        assert_eq!((s[1].line, s[1].end_line), (12, 12));
    }

    #[test]
    fn a_section_may_be_empty_and_the_last_may_lack_a_trailing_newline() {
        let s = parse_sections("## A\n## B\ntext", 1);
        assert_eq!((s[0].line, s[0].end_line, s[0].body.as_str()), (1, 1, ""));
        assert_eq!(
            (s[1].line, s[1].end_line, s[1].body.as_str()),
            (2, 3, "text\n")
        );
    }

    #[test]
    fn duplicate_headings_stay_separate_sections_in_order() {
        let s = parse_sections("## Notes\na\n## Notes\nb\n", 1);
        assert_eq!(headings(&s), ["Notes", "Notes"]);
        assert_eq!((s[0].line, s[1].line), (1, 3));
    }

    #[test]
    fn a_heading_inside_a_fenced_block_is_not_a_section() {
        let body = "## Real\n```md\n## Fake\n```\n~~~\n## Also fake\n~~~\n## Second\n";
        let s = parse_sections(body, 1);
        assert_eq!(headings(&s), ["Real", "Second"]);
        assert_eq!(s[0].end_line, 7);
    }

    #[test]
    fn a_fence_closes_only_on_its_own_marker_and_at_least_its_length() {
        let body = "````\n```\n## fake\n```\n## still fake\n````\n## Real\n~~~\n## fake\n```\n## fake\n~~~\n## After\n";
        assert_eq!(headings(&parse_sections(body, 1)), ["Real", "After"]);
    }

    #[test]
    fn an_unclosed_fence_swallows_the_rest_of_the_body() {
        let s = parse_sections("## A\n```\n## B\n## C\n", 1);
        assert_eq!(headings(&s), ["A"]);
        assert_eq!(s[0].end_line, 4);
    }

    #[test]
    fn crlf_line_endings_give_the_same_sections() {
        let lf = parse_sections("## A\nx\n\n## B\ny\n", 1);
        let crlf = parse_sections("## A\r\nx\r\n\r\n## B\r\ny\r\n", 1);
        assert_eq!(lf, crlf);
        assert_eq!(crlf[0].heading, "A");
    }

    #[test]
    fn crlf_does_not_hide_a_fence_or_leak_a_carriage_return() {
        let s = parse_sections("## A\r\n```\r\n## Fake\r\n```\r\n## B\r\n", 1);
        assert_eq!(headings(&s), ["A", "B"]);
        assert!(!s[0].body.contains('\r'));
    }

    #[test]
    fn heading_text_is_trimmed_and_closing_hashes_removed() {
        let s = parse_sections("##   Spaced   \n## Closed ##\n## Hash#tag\n##\n", 1);
        assert_eq!(headings(&s), ["Spaced", "Closed", "Hash#tag", ""]);
    }

    #[test]
    fn only_a_true_h2_counts() {
        let body = "##NoSpace\n### H3\n#### H4\n    ## indented code\n   ## Three spaces ok\n";
        assert_eq!(headings(&parse_sections(body, 1)), ["Three spaces ok"]);
    }

    #[test]
    fn deeper_headings_stay_inside_their_h2_body() {
        let s = parse_sections("## A\n### Sub\ntext\n## B\n", 1);
        assert_eq!(s[0].body, "### Sub\ntext\n");
    }

    fn tree(body: &str, depth: u8) -> Vec<(u8, String, Option<usize>, usize, usize)> {
        parse_sections_to_depth(body, 1, depth)
            .into_iter()
            .map(|s| (s.level, s.heading, s.parent, s.line, s.end_line))
            .collect()
    }

    #[test]
    fn depth_one_is_exactly_parse_sections_and_every_section_is_level_two() {
        let body = "## A\n### Sub\ntext\n## B\n";
        assert_eq!(parse_sections_to_depth(body, 1, 1), parse_sections(body, 1));
        assert!(parse_sections(body, 1)
            .iter()
            .all(|s| s.level == 2 && s.parent.is_none()));
    }

    #[test]
    fn depth_two_makes_h3_a_section_nested_under_its_h2() {
        let body = "## A\n### One\nx\n### Two\ny\n## B\n### Three\nz\n";
        assert_eq!(
            tree(body, 2),
            vec![
                (2, "A".into(), None, 1, 5),
                (3, "One".into(), Some(0), 2, 3),
                (3, "Two".into(), Some(0), 4, 5),
                (2, "B".into(), None, 6, 8),
                (3, "Three".into(), Some(3), 7, 8),
            ]
        );
    }

    #[test]
    fn an_h2_body_still_holds_its_subsections_text_at_any_depth() {
        let s = parse_sections_to_depth("## A\n### Sub\ntext\n## B\n", 1, 2);
        assert_eq!(s[0].body, "### Sub\ntext\n");
        assert_eq!(s[1].body, "text\n");
    }

    #[test]
    fn a_deeper_heading_than_depth_allows_is_body_not_a_section() {
        let body = "## A\n### B\n#### C\nx\n";
        assert_eq!(tree(body, 2).len(), 2);
        assert_eq!(tree(body, 3).len(), 3);
        assert_eq!(tree(body, 3)[2], (4, "C".into(), Some(1), 3, 4));
    }

    #[test]
    fn an_h3_before_any_h2_has_no_parent_and_a_skipped_level_attaches_to_the_nearest_shallower() {
        let s = tree("### Lone\n## A\n#### Deep\n", 2);
        assert_eq!(s[0].2, None);
        let s = tree("## A\n#### Deep\n", 2);
        assert_eq!(s.len(), 1, "a #### is beyond depth 2 (levels 2..=3)");
        let s = tree("## A\n#### Deep\n", 3);
        assert_eq!(s[1], (4, "Deep".into(), Some(0), 2, 2));
    }

    #[test]
    fn an_h3_inside_a_fence_is_not_a_section() {
        let s = tree("## A\n```\n### Fake\n```\n### Real\n", 2);
        assert_eq!(s.len(), 2);
        assert_eq!(s[1].1, "Real");
    }
}
