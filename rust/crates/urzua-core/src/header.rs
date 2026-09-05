//! The record header as a closed structure (RFC-0010): located by anchoring
//! to the metadata shape, never a positional slice, and parsed by exactly one
//! function so no check runs its own regex over raw text.

/// One field as found in the header region, with its line number (1-indexed,
/// relative to the whole document) for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderField {
    pub key: String,
    pub value: String,
    pub line: usize,
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
}

impl Header {
    /// First value for `key`, if present. A repeated key is a closure
    /// violation the caller should check for separately via
    /// [`Header::duplicate_keys`] -- this method deliberately doesn't hide
    /// that by picking one silently.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|f| f.key.eq_ignore_ascii_case(key))
            .map(|f| f.value.as_str())
    }

    pub fn duplicate_keys(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut dupes = Vec::new();
        for f in &self.fields {
            let lower = f.key.to_ascii_lowercase();
            if !seen.insert(lower.clone()) && !dupes.contains(&f.key) {
                dupes.push(f.key.clone());
            }
        }
        dupes
    }
}

/// Parse the header out of a document's full text.
///
/// The header is the maximal contiguous run of metadata-shaped lines,
/// anchored at the first such line. Three shapes are recognized, because
/// three are observed in real corpora (RFC-0010's motivation):
///
/// 1. Blockquote, one field per line: `> Key: Value`
/// 2. Blockquote, bold-labelled: `> **Key:** Value`
/// 3. Pipe-delimited single line: `Key1: v1 | Key2: v2 | Key3: v3`
///    (each segment may itself be bold-labelled)
///
/// A fourth shape is declared per profile rather than assumed (RFC-0010's
/// open question on where a differently-shaped header fits): a bold-labelled
/// markdown list with no blockquote wrapper at all, `- **Key:** Value`. Real
/// corpora were found using this shape without ever wrapping it in a `>`
/// block -- treating it as a blockquote-only variant would silently miss
/// every field in such a corpus.
///
/// A blank line, a non-metadata-shaped line, or end of document ends the
/// region. A document with no metadata-shaped line at all has `region: None`
/// -- reportable, never treated as a header with zero fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeaderShape {
    /// Blockquote-anchored: shapes 1-3 above. The long-standing default, so
    /// existing configs and callers that never declare a shape keep their
    /// current behavior unchanged.
    #[default]
    Blockquote,
    /// `- **Key:** Value`, anchored by a leading markdown list marker
    /// instead of a blockquote.
    BoldList,
}

pub fn parse(content: &str) -> Header {
    parse_with_shape(content, HeaderShape::Blockquote)
}

/// Parse the header using an explicitly declared shape (RFC-0010: the shape
/// is declared per profile, never sniffed).
pub fn parse_with_shape(content: &str, shape: HeaderShape) -> Header {
    let mut fields = Vec::new();
    let mut region: Option<(usize, usize)> = None;

    let line_body: fn(&str) -> Option<&str> = match shape {
        HeaderShape::Blockquote => blockquote_body,
        HeaderShape::BoldList => bold_list_body,
    };

    for (idx, raw_line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim_end();

        let Some(body) = line_body(line) else {
            if region.is_some() {
                break;
            }
            continue;
        };

        let line_fields = parse_metadata_line(body, line_no);
        if line_fields.is_empty() {
            if region.is_some() {
                break;
            }
            continue;
        }

        region = Some(match region {
            Some((start, _)) => (start, line_no),
            None => (line_no, line_no),
        });
        fields.extend(line_fields);
    }

    Header { fields, region }
}

/// Strip a leading `> ` (or bare `>`) blockquote marker. Returns `None` for a
/// line that isn't blockquote-shaped at all -- the anchor this parser uses,
/// since every observed shape in this corpus lives inside a leading
/// blockquote block.
fn blockquote_body(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix('>')?;
    Some(rest.strip_prefix(' ').unwrap_or(rest))
}

/// Strip a leading `- ` or `* ` markdown list marker. The bold-key body that
/// follows (`**Key:** Value`) is parsed by the same `parse_key_value` every
/// other shape uses -- only the anchor differs.
fn bold_list_body(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))?;
    Some(rest)
}

/// Parse one blockquote line's body into zero or more fields: either a
/// pipe-delimited run of `Key: Value` segments, or a single `Key: Value`
/// (plain or bold-labelled) line.
fn parse_metadata_line(body: &str, line_no: usize) -> Vec<HeaderField> {
    if body.contains(" | ") {
        return body
            .split(" | ")
            .filter_map(|segment| parse_key_value(segment, line_no))
            .collect();
    }
    parse_key_value(body, line_no).into_iter().collect()
}

/// Parse a single `Key: Value` or `**Key:** Value` segment. Deliberately
/// requires the colon to end the key -- `Supersedes / Superseded-by: —`
/// parses as one field with a compound key, which is exactly how that field
/// is declared in the RFC/ADR template, not a shape this parser needs to
/// special-case.
fn parse_key_value(segment: &str, line_no: usize) -> Option<HeaderField> {
    let segment = segment.trim();
    let segment = segment
        .strip_prefix("**")
        .and_then(|s| s.split_once(":**"))
        .map(|(key, value)| format!("{key}:{value}"))
        .unwrap_or_else(|| segment.to_string());

    let (key, value) = segment.split_once(':')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    Some(HeaderField {
        key: key.to_string(),
        value: value.trim().to_string(),
        line: line_no,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_blockquote_shape() {
        let doc = "# Title\n\n> Status: Draft\n> Date: 2026-08-11\n\n## Summary\n";
        let h = parse(doc);
        assert_eq!(h.get("Status"), Some("Draft"));
        assert_eq!(h.get("Date"), Some("2026-08-11"));
        assert_eq!(h.region, Some((3, 4)));
    }

    #[test]
    fn parses_pipe_delimited_shape() {
        let doc = "# Title\n\n> Version: 0.1 | Date: 2026-08-20 | Status: Draft\n> **Implements:** RFC-0001\n";
        let h = parse(doc);
        assert_eq!(h.get("Version"), Some("0.1"));
        assert_eq!(h.get("Status"), Some("Draft"));
        assert_eq!(h.get("Implements"), Some("RFC-0001"));
    }

    #[test]
    fn a_field_shaped_body_bullet_is_never_the_header() {
        // The exact bug class RFC-0010 documents: a body bullet in the
        // header's own shape must not satisfy a header field, because it
        // isn't in the header region at all (it's after a blank line/heading).
        let doc = "# Title\n\n> Status: Draft\n\n## Body\n\n- **Status:** decoy, not the header\n";
        let h = parse(doc);
        assert_eq!(h.get("Status"), Some("Draft"));
        assert_eq!(h.region, Some((3, 3)));
    }

    #[test]
    fn duplicate_keys_are_detected_not_first_wins() {
        let doc = "> Status: Draft\n> Status: Accepted\n";
        let h = parse(doc);
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
    fn parses_bold_list_shape() {
        let doc = "# Title\n\n> **Decision:** some prose.\n\n- **Status:** Accepted\n- **Date:** 2026-08-14\n- **Deciders:** A B\n\n## Context\n";
        let h = parse_with_shape(doc, HeaderShape::BoldList);
        assert_eq!(h.get("Status"), Some("Accepted"));
        assert_eq!(h.get("Date"), Some("2026-08-14"));
        assert_eq!(h.get("Deciders"), Some("A B"));
        assert_eq!(h.region, Some((5, 7)));
    }

    #[test]
    fn bold_list_shape_does_not_recognize_a_blockquote_header() {
        // Observed failing before it's trusted (SPEC-0002's discipline for
        // every rule): a corpus whose real header is blockquote-shaped must
        // report no region under the bold-list shape, not silently find
        // something else -- a misconfigured `header_shape` must be loud.
        let doc = "# Title\n\n> Status: Draft\n> Date: 2026-08-11\n";
        let h = parse_with_shape(doc, HeaderShape::BoldList);
        assert_eq!(h.region, None);
        assert!(h.fields.is_empty());
    }

    #[test]
    fn blockquote_shape_is_the_default_when_unspecified() {
        let doc = "# Title\n\n> Status: Draft\n";
        assert_eq!(parse(doc), parse_with_shape(doc, HeaderShape::Blockquote));
    }
}
