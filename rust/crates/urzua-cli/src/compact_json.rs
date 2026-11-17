//! Pretty JSON with each array of scalars on one line.

/// Rewrites `serde_json::to_string_pretty` output so an array whose elements
/// are all strings, numbers, booleans or null reads `[a, b]` on the line of
/// its key. Arrays holding objects or arrays, and empty arrays, are unchanged.
/// Escaped newlines keep every JSON string on one line, so a line that ends in
/// `[` always opens an array.
pub(crate) fn inline_scalar_arrays(pretty: &str) -> String {
    let lines: Vec<&str> = pretty.lines().collect();
    let mut out = String::with_capacity(pretty.len());
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some((joined, next)) = collapse(&lines, i) {
            out.push_str(&joined);
            i = next;
        } else {
            out.push_str(line);
            i += 1;
        }
        out.push('\n');
    }
    if !pretty.ends_with('\n') {
        out.pop();
    }
    out
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The collapsed text of the array opened on `lines[at]` and the index after
/// its closing line, when every element is a scalar.
fn collapse(lines: &[&str], at: usize) -> Option<(String, usize)> {
    let open = lines[at];
    if !open.ends_with('[') {
        return None;
    }
    let inner = indent_of(open) + 2;
    let mut items = Vec::new();
    let mut j = at + 1;
    while j < lines.len() && indent_of(lines[j]) == inner {
        let item = lines[j].trim();
        if item.starts_with(['{', '[', '}', ']']) {
            return None;
        }
        items.push(item.trim_end_matches(','));
        j += 1;
    }
    let close = lines.get(j)?;
    if items.is_empty() || indent_of(close) != inner - 2 || !close.trim_start().starts_with(']') {
        return None;
    }
    Some((
        format!("{}{}{}", open, items.join(", "), close.trim_start()),
        j + 1,
    ))
}

#[cfg(test)]
mod tests {
    use super::inline_scalar_arrays;
    use serde_json::json;

    fn pretty(v: &serde_json::Value) -> String {
        serde_json::to_string_pretty(v).unwrap()
    }

    #[test]
    fn scalar_arrays_go_on_one_line_and_the_result_is_the_same_json() {
        let v = json!({
            "a": ["x", "y,z", 3, true, null],
            "b": {"c": ["[", "]"], "d": []},
            "e": [{"f": ["g"]}, {"h": 1}],
            "i": [["j"], ["k"]],
            "l": "tail ["
        });
        let text = inline_scalar_arrays(&pretty(&v));
        assert!(
            text.contains("\"a\": [\"x\", \"y,z\", 3, true, null],"),
            "{text}"
        );
        assert!(text.contains("\"c\": [\"[\", \"]\"],"), "{text}");
        assert!(text.contains("\"d\": []"), "{text}");
        assert!(text.contains("\"f\": [\"g\"]"), "{text}");
        assert!(text.contains("\"l\": \"tail [\""), "{text}");
        assert_eq!(serde_json::from_str::<serde_json::Value>(&text).unwrap(), v);
    }

    #[test]
    fn a_top_level_array_and_a_scalar_are_unchanged_in_meaning() {
        for v in [json!(["a", "b"]), json!([]), json!("s"), json!({})] {
            let text = inline_scalar_arrays(&pretty(&v));
            assert_eq!(serde_json::from_str::<serde_json::Value>(&text).unwrap(), v);
        }
    }
}
