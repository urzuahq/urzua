//! Splits a record body into its `##` sections (RFC-33 layer 1, MILE-98).
//!
//! Plain data for later rules: this module decides nothing about a
//! section's meaning, only where each one starts and ends. It reads no
//! declaration and emits no finding.

/// One `##` section of a record body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// The heading text: trimmed, with any closing `#` sequence removed.
    /// Empty for a bare `##`.
    pub heading: String,
    /// 1-based line of the `##` heading line.
    pub line: usize,
    /// 1-based line of the section's last line (inclusive): the line before
    /// the next `##` heading, or the last line of the body.
    pub end_line: usize,
    /// The lines after the heading line through `end_line`, each followed by
    /// `\n`; carriage returns are not included.
    pub body: String,
}

/// Splits `body` into its `##` sections, in document order.
///
/// `first_line` is the 1-based line number of the first line of `body`
/// within its file, so a caller that strips a header region first still gets
/// file-relative numbers. Text before the first `##` belongs to no section.
/// Duplicate headings are kept as separate sections; reporting them is a
/// rule's job. Only ATX headings count: a `##` inside a fenced code block, or
/// indented four or more spaces, is not a heading. Lines may end `\n` or
/// `\r\n`.
pub fn parse_sections(body: &str, first_line: usize) -> Vec<Section> {
    let mut lines: Vec<&str> = body.split('\n').collect();
    // A trailing newline ends the last line; it does not start another.
    if lines.last() == Some(&"") {
        lines.pop();
    }

    let mut sections: Vec<Section> = Vec::new();
    let mut fence: Option<Fence> = None;
    for (i, raw) in lines.iter().enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let number = first_line + i;
        if let Some(open) = fence {
            if open.is_closed_by(line) {
                fence = None;
            }
        } else if let Some(open) = Fence::opening(line) {
            fence = Some(open);
        } else if let Some(heading) = h2_heading(line) {
            sections.push(Section {
                heading,
                line: number,
                end_line: number,
                body: String::new(),
            });
            continue;
        }
        if let Some(current) = sections.last_mut() {
            current.end_line = number;
            current.body.push_str(line);
            current.body.push('\n');
        }
    }
    sections
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

/// The heading text when `line` is an ATX `##` heading.
fn h2_heading(line: &str) -> Option<String> {
    let rest = indented_at_most_three(line)?.strip_prefix("##")?;
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
    Some(text.to_string())
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
}
