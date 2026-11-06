//! The marked agent-instructions block (`ADR-89` section 2): rewritten between
//! its markers, appended once when absent, never touched outside them. Running
//! it twice yields identical bytes.

pub const BEGIN: &str = "<!-- urzua-harness:begin -->";
pub const END: &str = "<!-- urzua-harness:end -->";

/// The block with its markers, no trailing newline.
pub fn render(body: &str) -> String {
    format!("{BEGIN}\n{}\n{END}", body.trim_end_matches('\n'))
}

/// (start, end) byte range of the block with markers, `end` after the end marker.
fn span(text: &str) -> Result<Option<(usize, usize)>, String> {
    match (text.find(BEGIN), text.find(END)) {
        (None, None) => Ok(None),
        (Some(b), Some(e)) if b < e => Ok(Some((b, e + END.len()))),
        _ => Err(format!(
            "the instructions file has a '{BEGIN}' without a later '{END}' (or the reverse); fix the markers by hand, the install will not guess"
        )),
    }
}

/// The block as it stands in `text`, markers included.
pub fn extract(text: &str) -> Result<Option<&str>, String> {
    Ok(span(text)?.map(|(b, e)| &text[b..e]))
}

/// `text` with the block set to `block`: replaced in place between the
/// markers, or appended when absent.
pub fn apply(text: &str, block: &str) -> Result<String, String> {
    Ok(match span(text)? {
        Some((b, e)) => format!("{}{}{}", &text[..b], block, &text[e..]),
        None if text.is_empty() => format!("{block}\n"),
        None => {
            let mut out = text.to_string();
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
            out.push_str(block);
            out.push('\n');
            out
        }
    })
}

/// `text` without the block, and without the blank line `apply` added before it.
pub fn remove(text: &str) -> Result<String, String> {
    let Some((b, mut e)) = span(text)? else {
        return Ok(text.to_string());
    };
    if text[e..].starts_with('\n') {
        e += 1;
    }
    let mut start = b;
    if text[..b].ends_with("\n\n") {
        start -= 1;
    }
    Ok(format!("{}{}", &text[..start], &text[e..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applying_twice_is_identical_and_only_the_block_changes() {
        let block = render("one\ntwo");
        let original = "# Agents\n\nKeep this.\n";
        let once = apply(original, &block).unwrap();
        assert!(once.starts_with(original));
        assert_eq!(apply(&once, &block).unwrap(), once);
        let newer = render("changed");
        let updated = apply(&once, &newer).unwrap();
        assert!(updated.starts_with(original) && updated.contains("changed"));
        assert!(!updated.contains("one\ntwo"));
    }

    #[test]
    fn text_after_the_block_is_left_alone() {
        let block = render("x");
        let text = format!("top\n\n{block}\n\nbottom\n");
        assert_eq!(
            apply(&text, &render("y")).unwrap(),
            text.replace("\nx\n", "\ny\n")
        );
    }

    #[test]
    fn remove_restores_the_original() {
        let block = render("x");
        for original in ["", "# A\n", "# A\n\ntext\n"] {
            let installed = apply(original, &block).unwrap();
            assert_eq!(remove(&installed).unwrap(), original, "{original:?}");
        }
        assert_eq!(remove("no block\n").unwrap(), "no block\n");
    }

    #[test]
    fn a_marker_without_its_partner_is_refused_not_guessed() {
        assert!(apply(&format!("{BEGIN}\nx\n"), &render("y")).is_err());
        assert!(extract(&format!("x\n{END}\n")).is_err());
        assert!(remove(&format!("{END}\n{BEGIN}\n")).is_err());
    }
}
