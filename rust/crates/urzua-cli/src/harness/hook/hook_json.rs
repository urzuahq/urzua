//! The hook's JSON spelling: `", "` and `": "` separators, every non-ASCII
//! character escaped (`dumps`); and the notebook form, indented one space with
//! non-ASCII kept (`dumps_indent1`). The agent parses either; a fixed spelling
//! keeps the output comparable byte for byte in tests.

use super::super::json::J;

/// One line, `", "` and `": "` separators, every non-ASCII character escaped.
pub fn dumps(v: &J) -> String {
    let mut out = String::new();
    write(v, &mut out, None, true);
    out
}

/// Indented by one space per level, non-ASCII kept.
pub fn dumps_indent1(v: &J) -> String {
    let mut out = String::new();
    write(v, &mut out, Some(0), false);
    out
}

fn newline(out: &mut String, level: Option<usize>) {
    if let Some(n) = level {
        out.push('\n');
        out.push_str(&" ".repeat(n));
    }
}

fn write(v: &J, out: &mut String, level: Option<usize>, ascii: bool) {
    let inner = level.map(|n| n + 1);
    let sep = if level.is_some() { "," } else { ", " };
    match v {
        J::Null => out.push_str("null"),
        J::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        J::Num(n) => out.push_str(n),
        J::Str(s) => string(s, out, ascii),
        J::Arr(items) if items.is_empty() => out.push_str("[]"),
        J::Obj(members) if members.is_empty() => out.push_str("{}"),
        J::Arr(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(sep);
                }
                newline(out, inner);
                write(item, out, inner, ascii);
            }
            newline(out, level);
            out.push(']');
        }
        J::Obj(members) => {
            out.push('{');
            for (i, (k, val)) in members.iter().enumerate() {
                if i > 0 {
                    out.push_str(sep);
                }
                newline(out, inner);
                string(k, out, ascii);
                out.push_str(": ");
                write(val, out, inner, ascii);
            }
            newline(out, level);
            out.push('}');
        }
    }
}

fn string(s: &str, out: &mut String, ascii: bool) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c if ascii && !c.is_ascii() => {
                let mut units = [0u16; 2];
                for u in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{u:04x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A string member list, as the hook output is built.
pub fn obj(members: Vec<(&str, J)>) -> J {
    J::Obj(
        members
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

pub fn s(text: &str) -> J {
    J::Str(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::json::parse;

    #[test]
    fn the_spelling_is_fixed() {
        let v = parse(r#"{"a":"é😀\u0001","b":[1,{}],"c":[]}"#).unwrap();
        assert_eq!(
            dumps(&v),
            r#"{"a": "\u00e9\ud83d\ude00\u0001", "b": [1, {}], "c": []}"#
        );
        assert_eq!(
            dumps_indent1(&v),
            "{\n \"a\": \"é😀\\u0001\",\n \"b\": [\n  1,\n  {}\n ],\n \"c\": []\n}"
        );
    }
}
