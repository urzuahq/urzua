//! Pretty JSON with each array of scalars, then each flat object, on one line.

/// Rewrites `serde_json::to_string_pretty` output in two passes. An array
/// whose elements are all strings, numbers, booleans or null reads `[a, b]`
/// on the line of its key. Then an object whose values are all scalars or
/// such arrays reads `{"k": v, ...}` on one line. Anything holding a nested
/// object or an array of objects, and every empty array or object, is
/// unchanged, so the result parses to the same data.
pub(crate) fn inline_flat(pretty: &str) -> String {
    let arrays = rewrite(pretty, collapse_array);
    rewrite(&arrays, collapse_object)
}

/// Escaped newlines keep every JSON string on one line, and a string line ends
/// in a quote, so a line that ends in `[` or `{` always opens a container.
type Collapse = fn(&[&str], usize) -> Option<(String, usize)>;

fn rewrite(text: &str, collapse: Collapse) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < lines.len() {
        if let Some((joined, next)) = collapse(&lines, i) {
            out.push_str(&joined);
            i = next;
        } else {
            out.push_str(lines[i]);
            i += 1;
        }
        out.push('\n');
    }
    if !text.ends_with('\n') {
        out.pop();
    }
    out
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The collapsed text of the array opened on `lines[at]` and the index after
/// its closing line, when every element is a scalar.
fn collapse_array(lines: &[&str], at: usize) -> Option<(String, usize)> {
    collapse(lines, at, '[', ']')
}

/// The same for an object, after arrays are collapsed: a member line that
/// opens a container (ends in `[` or `{`) keeps the object as it is.
fn collapse_object(lines: &[&str], at: usize) -> Option<(String, usize)> {
    collapse(lines, at, '{', '}')
}

fn collapse(lines: &[&str], at: usize, open_ch: char, close_ch: char) -> Option<(String, usize)> {
    let open = lines[at];
    if !open.ends_with(open_ch) {
        return None;
    }
    let inner = indent_of(open) + 2;
    let mut items = Vec::new();
    let mut j = at + 1;
    while j < lines.len() && indent_of(lines[j]) == inner {
        let item = lines[j].trim();
        let opens = item.ends_with(['{', '[']);
        let is_member_or_scalar = if open_ch == '{' {
            item.starts_with('"')
        } else {
            !item.starts_with(['{', '[', '}', ']'])
        };
        if opens || !is_member_or_scalar {
            return None;
        }
        items.push(item.trim_end_matches(','));
        j += 1;
    }
    let close = lines.get(j)?;
    if items.is_empty()
        || indent_of(close) != inner - 2
        || !close.trim_start().starts_with(close_ch)
    {
        return None;
    }
    Some((
        format!("{}{}{}", open, items.join(", "), close.trim_start()),
        j + 1,
    ))
}

#[cfg(test)]
mod tests {
    use super::inline_flat;
    use serde_json::json;

    fn pretty(v: &serde_json::Value) -> String {
        serde_json::to_string_pretty(v).unwrap()
    }

    #[test]
    fn flat_containers_go_on_one_line_and_the_result_is_the_same_json() {
        let v = json!({
            "a": ["x", "y,z", 3, true, null],
            "b": {"c": ["[", "]"], "d": [], "e": {}},
            "f": [{"g": ["h"], "i": 1}, {"j": {"k": 1}}],
            "l": [["m"], ["n"]],
            "o": "tail [",
            "p": {"q": "}", "r": "{"},
            "s": {"t": {"u": 1}, "v": 2}
        });
        let text = inline_flat(&pretty(&v));
        assert!(
            text.contains("\"a\": [\"x\", \"y,z\", 3, true, null],"),
            "{text}"
        );
        assert!(
            text.contains("\"p\": {\"q\": \"}\", \"r\": \"{\"}"),
            "{text}"
        );
        assert!(text.contains("{\"g\": [\"h\"], \"i\": 1}"), "{text}");
        assert!(text.contains("\"t\": {\"u\": 1}"), "{text}");
        assert!(
            !text.contains("\"s\": {\"t\""),
            "an object holding an object is not flat: {text}"
        );
        assert!(
            text.contains("\"d\": []") && text.contains("\"e\": {}"),
            "{text}"
        );
        assert!(text.contains("\"o\": \"tail [\""), "{text}");
        assert_eq!(serde_json::from_str::<serde_json::Value>(&text).unwrap(), v);
    }

    #[test]
    fn a_top_level_container_or_scalar_keeps_its_meaning() {
        for v in [
            json!(["a", "b"]),
            json!([]),
            json!("s"),
            json!({}),
            json!({"a": 1}),
            json!([{"a": 1}]),
        ] {
            let text = inline_flat(&pretty(&v));
            assert_eq!(serde_json::from_str::<serde_json::Value>(&text).unwrap(), v);
        }
    }
}
