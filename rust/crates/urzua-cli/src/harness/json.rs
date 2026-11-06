//! A small order-preserving JSON model for the agent settings file the install
//! merges into (`ADR-89` section 2). `serde_json::Value` sorts object keys, which
//! would reorder a person's file; this keeps member order and keeps a number as
//! the text it was written with, so every key and entry the install does not
//! touch comes back as it was.

#[derive(Clone, Debug, PartialEq)]
pub enum J {
    Null,
    Bool(bool),
    /// The number as written.
    Num(String),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    pub fn get(&self, key: &str) -> Option<&J> {
        match self {
            J::Obj(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut J> {
        match self {
            J::Obj(members) => members.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            J::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Sets a member, appending it when absent.
    pub fn set(&mut self, key: &str, value: J) {
        if let J::Obj(members) = self {
            match members.iter_mut().find(|(k, _)| k == key) {
                Some((_, v)) => *v = value,
                None => members.push((key.to_string(), value)),
            }
        }
    }

    pub fn remove(&mut self, key: &str) {
        if let J::Obj(members) = self {
            members.retain(|(k, _)| k != key);
        }
    }

    pub fn is_empty_container(&self) -> bool {
        matches!(self, J::Arr(a) if a.is_empty()) || matches!(self, J::Obj(o) if o.is_empty())
    }

    /// The canonical two-space form (`JSON.stringify(x, null, 2)`), no trailing newline.
    pub fn pretty(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, Some(0));
        out
    }

    /// One line, no spaces: the form a hash is taken over.
    pub fn compact(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, None);
        out
    }

    fn write(&self, out: &mut String, indent: Option<usize>) {
        match self {
            J::Null => out.push_str("null"),
            J::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            J::Num(n) => out.push_str(n),
            J::Str(s) => write_string(s, out),
            J::Arr(items) if items.is_empty() => out.push_str("[]"),
            J::Obj(members) if members.is_empty() => out.push_str("{}"),
            J::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    newline(out, indent.map(|n| n + 1));
                    item.write(out, indent.map(|n| n + 1));
                }
                newline(out, indent);
                out.push(']');
            }
            J::Obj(members) => {
                out.push('{');
                for (i, (key, value)) in members.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    newline(out, indent.map(|n| n + 1));
                    write_string(key, out);
                    out.push(':');
                    if indent.is_some() {
                        out.push(' ');
                    }
                    value.write(out, indent.map(|n| n + 1));
                }
                newline(out, indent);
                out.push('}');
            }
        }
    }
}

fn newline(out: &mut String, indent: Option<usize>) {
    if let Some(n) = indent {
        out.push('\n');
        for _ in 0..n {
            out.push_str("  ");
        }
    }
}

fn write_string(s: &str, out: &mut String) {
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
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Parses one JSON document; trailing text is an error.
pub fn parse(text: &str) -> Result<J, String> {
    let mut p = Parser {
        chars: text.chars().collect(),
        at: 0,
    };
    p.skip_ws();
    let value = p.value()?;
    p.skip_ws();
    if p.at != p.chars.len() {
        return Err(format!(
            "unexpected text after the document at offset {}",
            p.at
        ));
    }
    Ok(value)
}

struct Parser {
    chars: Vec<char>,
    at: usize,
}

impl Parser {
    fn skip_ws(&mut self) {
        while self.chars.get(self.at).is_some_and(|c| c.is_whitespace()) {
            self.at += 1;
        }
    }

    fn eat(&mut self, c: char) -> Result<(), String> {
        if self.chars.get(self.at) == Some(&c) {
            self.at += 1;
            Ok(())
        } else {
            Err(format!("expected '{c}' at offset {}", self.at))
        }
    }

    fn word(&mut self, w: &str, v: J) -> Result<J, String> {
        for c in w.chars() {
            self.eat(c)?;
        }
        Ok(v)
    }

    fn value(&mut self) -> Result<J, String> {
        match self.chars.get(self.at) {
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('"') => Ok(J::Str(self.string()?)),
            Some('t') => self.word("true", J::Bool(true)),
            Some('f') => self.word("false", J::Bool(false)),
            Some('n') => self.word("null", J::Null),
            Some(c) if *c == '-' || c.is_ascii_digit() => {
                let start = self.at;
                while self
                    .chars
                    .get(self.at)
                    .is_some_and(|c| c.is_ascii_digit() || "+-.eE".contains(*c))
                {
                    self.at += 1;
                }
                Ok(J::Num(self.chars[start..self.at].iter().collect()))
            }
            Some(c) => Err(format!("unexpected '{c}' at offset {}", self.at)),
            None => Err("the document ends inside a value".to_string()),
        }
    }

    fn object(&mut self) -> Result<J, String> {
        self.eat('{')?;
        let mut members = Vec::new();
        self.skip_ws();
        if self.chars.get(self.at) == Some(&'}') {
            self.at += 1;
            return Ok(J::Obj(members));
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            self.eat(':')?;
            self.skip_ws();
            members.push((key, self.value()?));
            self.skip_ws();
            match self.chars.get(self.at) {
                Some(',') => self.at += 1,
                Some('}') => {
                    self.at += 1;
                    return Ok(J::Obj(members));
                }
                _ => return Err(format!("expected ',' or '}}' at offset {}", self.at)),
            }
        }
    }

    fn array(&mut self) -> Result<J, String> {
        self.eat('[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.chars.get(self.at) == Some(&']') {
            self.at += 1;
            return Ok(J::Arr(items));
        }
        loop {
            self.skip_ws();
            items.push(self.value()?);
            self.skip_ws();
            match self.chars.get(self.at) {
                Some(',') => self.at += 1,
                Some(']') => {
                    self.at += 1;
                    return Ok(J::Arr(items));
                }
                _ => return Err(format!("expected ',' or ']' at offset {}", self.at)),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.eat('"')?;
        let mut s = String::new();
        loop {
            let Some(&c) = self.chars.get(self.at) else {
                return Err("the document ends inside a string".to_string());
            };
            self.at += 1;
            match c {
                '"' => return Ok(s),
                '\\' => {
                    let Some(&e) = self.chars.get(self.at) else {
                        return Err("the document ends inside an escape".to_string());
                    };
                    self.at += 1;
                    match e {
                        '"' => s.push('"'),
                        '\\' => s.push('\\'),
                        '/' => s.push('/'),
                        'n' => s.push('\n'),
                        'r' => s.push('\r'),
                        't' => s.push('\t'),
                        'b' => s.push('\u{8}'),
                        'f' => s.push('\u{c}'),
                        'u' => {
                            let first = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&first) {
                                self.eat('\\')?;
                                self.eat('u')?;
                                let second = self.hex4()?;
                                0x10000 + ((first - 0xD800) << 10) + (second.wrapping_sub(0xDC00))
                            } else {
                                first
                            };
                            s.push(char::from_u32(code).ok_or("an invalid unicode escape")?);
                        }
                        other => return Err(format!("an invalid escape '\\{other}'")),
                    }
                }
                c => s.push(c),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let digits: String = self.chars.iter().skip(self.at).take(4).collect();
        if digits.chars().count() != 4 {
            return Err("a short unicode escape".to_string());
        }
        self.at += 4;
        u32::from_str_radix(&digits, 16).map_err(|_| "an invalid unicode escape".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_canonical_document_round_trips_byte_for_byte() {
        let text = "{\n  \"hooks\": {\n    \"PreToolUse\": [\n      {\n        \"matcher\": \"Bash\",\n        \"hooks\": [\n          {\n            \"type\": \"command\",\n            \"command\": \"echo \\\"hi\\\" \\\\ \\n\"\n          }\n        ]\n      }\n    ]\n  },\n  \"n\": 1.50,\n  \"empty\": [],\n  \"obj\": {},\n  \"z\": null,\n  \"a\": true\n}";
        assert_eq!(parse(text).unwrap().pretty(), text);
    }

    #[test]
    fn member_order_and_number_spelling_are_kept() {
        let v = parse("{\"b\":1e5,\"a\":[2,3]}").unwrap();
        assert_eq!(v.compact(), "{\"b\":1e5,\"a\":[2,3]}");
    }

    #[test]
    fn unicode_escapes_decode_and_malformed_documents_are_refused() {
        assert_eq!(
            parse("\"\\u00e9\\ud83d\\ude00\"").unwrap(),
            J::Str("\u{e9}\u{1f600}".to_string())
        );
        for bad in ["", "{", "{\"a\":}", "[1,]", "{\"a\":1} x", "tru", "{'a':1}"] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }
}
