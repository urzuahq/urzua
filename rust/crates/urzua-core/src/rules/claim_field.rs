//! The structured claim a release fragment declares (`ADR-85`): a frontmatter
//! key whose value is a one-line list of the records the change closes.
//!
//! Deliberately small. It reads one key, named by configuration, from the
//! `---` block a file begins with, and ignores every other key (the release
//! tool's own bump keys among them). It is not a YAML parser: the value is a
//! one-line flow list `[a, b]`, a single id, or `[]`, and anything else is
//! reported as malformed rather than guessed at. The release tool pinned in
//! `prepare-release.yml` rejects a block list and a list continued over two
//! lines, so the form this reads is the form that tool accepts.

use super::*;

// Implements: ADR-85 (the structured claim field)
/// What a claim file says about the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimField {
    /// The file does not begin with a closed `---` block, so it is not a
    /// fragment: out of scope, never given a finding.
    NoFrontmatter,
    /// The file tries to open a `---` block and the release tool cannot read
    /// it: the fence is never closed, or a blank line comes before it. Under
    /// `strict` this is `claim.field-malformed`; otherwise the file is read as
    /// no frontmatter, as before. `line` is the opening fence, one-based.
    BrokenFence { line: usize, reason: String },
    /// A frontmatter block without the key.
    Missing,
    /// The key and the ids it lists, in order (empty for `[]`). `line` is the
    /// key's own line, one-based.
    Declared { line: usize, ids: Vec<String> },
    /// The key is present and its value is not a one-line list of record ids.
    Malformed { line: usize, reason: String },
}

/// The zero-based line indexes of the frontmatter block, both fences
/// included, or `None` when the file does not begin with a closed one.
pub fn frontmatter_lines(text: &str) -> Option<std::ops::Range<usize>> {
    let mut lines = text.lines();
    let first = lines.next()?.trim_start_matches('\u{feff}');
    if first.trim_end() != "---" {
        return None;
    }
    let close = lines.position(|l| l.trim_end() == "---")?;
    Some(0..close + 2)
}

/// Reads `key` from the frontmatter block of `text`.
pub fn read_claim_field(text: &str, key: &str) -> ClaimField {
    let Some(block) = frontmatter_lines(text) else {
        return broken_fence(text).unwrap_or(ClaimField::NoFrontmatter);
    };
    let lines: Vec<&str> = text.lines().collect();
    // The lines between the fences.
    let inner = block.start + 1..block.end - 1;
    let mut found: Option<usize> = None;
    for i in inner.clone() {
        let Some(rest) = lines[i].strip_prefix(key) else {
            continue;
        };
        if !rest.trim_start().starts_with(':') {
            continue;
        }
        if found.is_some() {
            return ClaimField::Malformed {
                line: i + 1,
                reason: format!("`{key}` is declared twice in the frontmatter"),
            };
        }
        found = Some(i);
    }
    let Some(at) = found else {
        return ClaimField::Missing;
    };
    // `strip_prefix` and the `:` test above make this infallible.
    let value = lines[at]
        .strip_prefix(key)
        .and_then(|rest| rest.trim_start().strip_prefix(':'))
        .unwrap_or_default()
        .trim();
    let next = lines[at + 1..inner.end].first().copied();
    match parse_value(key, value, next) {
        Ok(ids) => ClaimField::Declared { line: at + 1, ids },
        Err(reason) => ClaimField::Malformed {
            line: at + 1,
            reason,
        },
    }
}

/// A file that tries to open a block the release tool (knope) would refuse
/// with `could_not_read_changeset`: the first non-blank line is `---` and the
/// file either has a blank line before it or never closes the block.
fn broken_fence(text: &str) -> Option<ClaimField> {
    let mut lines = text.lines().map(|l| l.trim_start_matches('\u{feff}'));
    let mut at = 0;
    let mut first = lines.next()?;
    let leading_blank = first.trim().is_empty();
    while first.trim().is_empty() {
        first = lines.next()?;
        at += 1;
    }
    if first.trim_end() != "---" {
        return None;
    }
    if leading_blank {
        return Some(ClaimField::BrokenFence {
            line: at + 1,
            reason: "has a blank line before its opening `---`, which the release tool refuses to read as frontmatter; start the file with the fence".to_string(),
        });
    }
    // Reached only when `frontmatter_lines` found no closing fence.
    Some(ClaimField::BrokenFence {
        line: at + 1,
        reason: "opens a `---` frontmatter block that is never closed, which the release tool refuses to read; add the closing `---`".to_string(),
    })
}

fn parse_value(key: &str, value: &str, next: Option<&str>) -> Result<Vec<String>, String> {
    if value.is_empty() {
        let continued = next.map(str::trim_start);
        return Err(match continued {
            Some(n) if n == "-" || n.starts_with("- ") => format!(
                "`{key}` is a block list; write the ids on the key's own line as `{key}: [A-1, B-2]`, the form the release tool reads"
            ),
            Some(n) if n.len() != next.map_or(0, str::len) => format!(
                "`{key}` has no value on its own line; write the ids on one line as `{key}: [A-1, B-2]`"
            ),
            _ => format!("`{key}` has no value; write `{key}: []` if the change closes nothing"),
        });
    }
    if let Some(open) = value.strip_prefix('[') {
        let Some(inner) = open.strip_suffix(']') else {
            return Err(format!(
                "`{key}` is a list that does not close on its own line; write the whole list on one line"
            ));
        };
        if inner.trim().is_empty() {
            return Ok(Vec::new());
        }
        return inner.split(',').map(|item| one_id(key, item)).collect();
    }
    one_id(key, value).map(|id| vec![id])
}

fn one_id(key: &str, item: &str) -> Result<String, String> {
    let item = item.trim();
    let id = match (item.chars().next(), item.chars().last()) {
        (Some(a @ ('"' | '\'')), Some(b)) if a == b && item.len() >= 2 => &item[1..item.len() - 1],
        _ => item,
    };
    if id.is_empty() {
        return Err(format!("`{key}` has an empty entry"));
    }
    if !is_record_reference(id) {
        return Err(format!(
            "`{key}` has `{id}`, which is not a record id; the value is one id or a one-line list of ids such as `[A-1, B-2]`"
        ));
    }
    Ok(id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(text: &str) -> Vec<String> {
        match read_claim_field(text, "closes") {
            ClaimField::Declared { ids, .. } => ids,
            other => panic!("expected a declared list, got {other:?}"),
        }
    }

    fn malformed(text: &str) -> (usize, String) {
        match read_claim_field(text, "closes") {
            ClaimField::Malformed { line, reason } => (line, reason),
            other => panic!("expected malformed, got {other:?}"),
        }
    }

    #[test]
    fn a_one_line_list_is_read_in_order_observed_failing() {
        assert_eq!(
            ids("---\ndefault: patch\ncloses: [BUG-60, BUG-61]\n---\n\nBody.\n"),
            ["BUG-60", "BUG-61"]
        );
        assert_eq!(
            read_claim_field("---\ndefault: patch\ncloses: [BUG-60]\n---\n", "closes"),
            ClaimField::Declared {
                line: 3,
                ids: vec!["BUG-60".to_string()]
            }
        );
    }

    #[test]
    fn a_single_id_and_the_empty_list_are_valid_observed_failing() {
        assert_eq!(ids("---\ncloses: BUG-62\n---\n"), ["BUG-62"]);
        assert_eq!(ids("---\ncloses: []\n---\n"), Vec::<String>::new());
        assert_eq!(ids("---\ncloses: [ ]\n---\n"), Vec::<String>::new());
        assert_eq!(
            ids("---\ncloses:   [ BUG-1 ,MILE-2 ]  \n---\n"),
            ["BUG-1", "MILE-2"]
        );
    }

    #[test]
    fn a_quoted_id_is_read_as_the_id_observed_failing() {
        assert_eq!(ids("---\ncloses: \"BUG-62\"\n---\n"), ["BUG-62"]);
        assert_eq!(
            ids("---\ncloses: ['BUG-1', \"BUG-2\"]\n---\n"),
            ["BUG-1", "BUG-2"]
        );
    }

    #[test]
    fn a_block_list_is_malformed_and_named_observed_failing() {
        let (line, reason) = malformed("---\ndefault: patch\ncloses:\n  - BUG-62\n---\n");
        assert_eq!(line, 3);
        assert!(reason.contains("block list"), "{reason}");
        let (_, reason) = malformed("---\ncloses:\n- BUG-62\n---\n");
        assert!(reason.contains("block list"), "{reason}");
    }

    #[test]
    fn a_list_continued_over_lines_is_malformed_observed_failing() {
        let (line, reason) = malformed("---\ncloses: [BUG-60,\n  BUG-61]\n---\n");
        assert_eq!(line, 2);
        assert!(reason.contains("one line"), "{reason}");
    }

    #[test]
    fn a_value_that_is_not_a_list_or_an_id_is_malformed_observed_failing() {
        for value in [
            "\"BUG-60, BUG-61\"",
            "BUG-60, BUG-61",
            "fixed",
            "[BUG-60, nope]",
            "[BUG-60,, BUG-61]",
            "[BUG-60,]",
            "[[BUG-60]]",
            "bug-60",
            "BUG-",
        ] {
            let (line, reason) = malformed(&format!("---\ncloses: {value}\n---\n"));
            assert_eq!(line, 2, "{value}");
            assert!(!reason.is_empty(), "{value}");
        }
    }

    #[test]
    fn a_key_with_no_value_and_no_list_is_malformed_observed_failing() {
        let (line, reason) = malformed("---\ncloses:\n---\n");
        assert_eq!(line, 2);
        assert!(reason.contains("no value"), "{reason}");
    }

    #[test]
    fn a_key_declared_twice_is_malformed_observed_failing() {
        let (line, reason) = malformed("---\ncloses: [BUG-1]\ncloses: [BUG-2]\n---\n");
        assert_eq!(line, 3);
        assert!(reason.contains("twice"), "{reason}");
    }

    #[test]
    fn a_file_without_a_closed_frontmatter_block_is_not_a_fragment_observed_failing() {
        assert_eq!(
            read_claim_field("# Notes\n\ncloses: [BUG-1]\n", "closes"),
            ClaimField::NoFrontmatter
        );
        assert_eq!(read_claim_field("", "closes"), ClaimField::NoFrontmatter);
    }

    #[test]
    fn an_unterminated_fence_or_a_blank_line_before_it_is_a_broken_fence_observed_failing() {
        for text in [
            "---\ncloses: [BUG-1]\n",
            "\n---\ncloses: [BUG-1]\n---\n",
            "  \n---\ncloses: [BUG-1]\n",
        ] {
            assert!(
                matches!(
                    read_claim_field(text, "closes"),
                    ClaimField::BrokenFence { .. }
                ),
                "{text:?}"
            );
        }
        // Not an attempt at a block: a horizontal rule mid-file, or none.
        assert_eq!(
            read_claim_field("# Notes\n\n---\ncloses: [BUG-1]\n", "closes"),
            ClaimField::NoFrontmatter
        );
    }

    #[test]
    fn a_block_without_the_key_is_missing_and_other_keys_are_ignored_observed_failing() {
        assert_eq!(
            read_claim_field("---\ndefault: patch\n---\n\nBody.\n", "closes"),
            ClaimField::Missing
        );
        assert_eq!(
            read_claim_field("---\n---\n", "closes"),
            ClaimField::Missing
        );
        // Neither a longer key, a nested one, nor the key in the body is it.
        assert_eq!(
            read_claim_field(
                "---\nunrelated:\n  closes: [BUG-1]\ncloses_more: [BUG-2]\n---\ncloses: [BUG-3]\n",
                "closes"
            ),
            ClaimField::Missing
        );
    }

    #[test]
    fn the_key_is_whatever_configuration_names_observed_failing() {
        assert_eq!(
            read_claim_field("---\nfixes: [BUG-9]\n---\n", "fixes"),
            ClaimField::Declared {
                line: 2,
                ids: vec!["BUG-9".to_string()]
            }
        );
        assert_eq!(
            read_claim_field("---\nfixes: [BUG-9]\n---\n", "closes"),
            ClaimField::Missing
        );
    }

    #[test]
    fn crlf_line_endings_and_a_byte_order_mark_are_read_observed_failing() {
        assert_eq!(
            ids("---\r\ndefault: patch\r\ncloses: [BUG-1]\r\n---\r\n"),
            ["BUG-1"]
        );
        assert_eq!(ids("\u{feff}---\ncloses: [BUG-1]\n---\n"), ["BUG-1"]);
    }

    #[test]
    fn the_frontmatter_range_covers_both_fences_observed_failing() {
        assert_eq!(frontmatter_lines("---\na: b\n---\nbody\n"), Some(0..3));
        assert_eq!(frontmatter_lines("body\n"), None);
        assert_eq!(frontmatter_lines("---\na: b\n"), None);
    }
}
