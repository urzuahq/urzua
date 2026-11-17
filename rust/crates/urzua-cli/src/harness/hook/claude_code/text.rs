//! Text helpers of the adapter, each matching the Python it replaced, so a
//! payload is read the same way it was before.

use std::path::Path;

/// Python's `str.isspace` for one character.
pub fn py_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

fn py_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Python's `str.splitlines()`.
fn splitlines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let breaks = matches!(
            c,
            '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{1c}' | '\u{1d}' | '\u{1e}' | '\u{85}' | '\u{2028}' | '\u{2029}'
        );
        if !breaks {
            continue;
        }
        out.push(&text[start..i]);
        let mut end = i + c.len_utf8();
        if c == '\r' {
            if let Some((j, '\n')) = chars.peek().copied() {
                chars.next();
                end = j + 1;
            }
        }
        start = end;
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

/// After `\b` + `word` at `at` in `body`: whether `word` starts a word there.
fn word_at(body: &str, at: usize, word: &str) -> bool {
    body[at..].starts_with(word) && body[..at].chars().next_back().is_none_or(|c| !py_word(c))
}

fn skip_space(t: &str) -> &str {
    t.trim_start_matches(py_space)
}

/// `\b{key}\s*:\s*{quote}{value}\b` anywhere in `body`.
fn has_setting(body: &str, key: &str, value: &str, quote: bool) -> bool {
    body.match_indices(key).any(|(at, _)| {
        if !word_at(body, at, key) {
            return false;
        }
        let rest = skip_space(&body[at + key.len()..]);
        let Some(rest) = rest.strip_prefix(':') else {
            return false;
        };
        let mut rest = skip_space(rest);
        if quote {
            if let Some(r) = rest.strip_prefix(['"', '\'']) {
                if r.starts_with(value) {
                    rest = r;
                }
            }
        }
        rest.strip_prefix(value)
            .is_some_and(|r| r.chars().next().is_none_or(|c| !py_word(c)))
    })
}

/// Whether the configuration found at or above `d` turns on acceptance or
/// protected-paths without `on_unjudged: allow` (a text scan), or `None` when
/// no configuration is found.
pub fn config_closes(d: &Path) -> Option<bool> {
    let start = super::super::lexical_absolute(d);
    let mut dir = start.as_path();
    let text = loop {
        match std::fs::read(dir.join(".urzua/config.yaml")) {
            Ok(b) => break String::from_utf8_lossy(&b).to_string(),
            Err(_) => dir = dir.parent()?,
        }
    };
    let lines = splitlines(&text);
    for (i, line) in lines.iter().enumerate() {
        let body_start = line.trim_start_matches(py_space);
        let indent = line.chars().count() - body_start.chars().count();
        let Some(after) = ["acceptance", "protected-paths"]
            .iter()
            .find_map(|k| body_start.strip_prefix(k))
        else {
            continue;
        };
        let Some(mut body) = skip_space(after).strip_prefix(':').map(|b| skip_space(b).to_string())
        else {
            continue;
        };
        if body.is_empty() || body.starts_with('#') {
            body = String::new();
            for next in &lines[i + 1..] {
                let lead = next.chars().count() - next.trim_start_matches(py_space).chars().count();
                if !next.trim_matches(py_space).is_empty() && lead <= indent {
                    break;
                }
                body.push(' ');
                body.push_str(next);
            }
        }
        if has_setting(&body, "enabled", "true", false)
            && !has_setting(&body, "on_unjudged", "allow", true)
        {
            return Some(true);
        }
    }
    Some(false)
}

/// The reason inside a translator message, without its own prefix and closing
/// advice: the engine words the refusal or notice around it.
pub fn reason_of(message: &str) -> String {
    for head in [
        "urzua harness: urzua did NOT judge this edit (",
        "urzua harness: urzua did NOT judge this call (",
    ] {
        if let Some(rest) = message.strip_prefix(head) {
            if let Some(end) = rest.find("). It is allowed ") {
                return rest[..end].to_string();
            }
        }
    }
    message.to_string()
}

/// `[A-Z]?[a-z]+|[A-Z]+(?![a-z])|[0-9]+`, lowercased.
pub fn words_of(text: &str) -> Vec<String> {
    let c: Vec<char> = text.chars().collect();
    let (up, low, dig) = (
        |x: char| x.is_ascii_uppercase(),
        |x: char| x.is_ascii_lowercase(),
        |x: char| x.is_ascii_digit(),
    );
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let run = |from: usize, f: &dyn Fn(char) -> bool| c[from..].iter().take_while(|x| f(**x)).count();
        let len = if low(c[i]) {
            run(i, &low)
        } else if up(c[i]) && c.get(i + 1).is_some_and(|x| low(*x)) {
            1 + run(i + 1, &low)
        } else if up(c[i]) {
            let n = run(i, &up);
            if c.get(i + n).is_some_and(|x| low(*x)) {
                n - 1
            } else {
                n
            }
        } else if dig(c[i]) {
            run(i, &dig)
        } else {
            0
        };
        if len == 0 {
            i += 1;
            continue;
        }
        out.push(c[i..i + len].iter().collect::<String>().to_lowercase());
        i += len;
    }
    out
}

/// A leading `~`, `$HOME` or `${HOME}` replaced by the home directory.
fn home_expanded(value: &str) -> Option<String> {
    let head = ["~", "$HOME", "${HOME}"].into_iter().find(|h| {
        value
            .strip_prefix(h)
            .is_some_and(|r| r.is_empty() || r.starts_with('/'))
    })?;
    let home = std::env::var("HOME").ok().filter(|h| !h.is_empty() && h != "~")?;
    Some(format!("{home}{}", &value[head.len()..]))
}

/// `urllib.parse.unquote`.
fn unquote(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = std::str::from_utf8(&b[i + 1..i + 3]).ok();
            if let Some(v) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// The path `urllib.parse.urlparse` finds in a `file:` URL.
fn url_path(value: &str) -> String {
    let mut rest: String = value[5..].chars().filter(|c| !matches!(c, '\t' | '\r' | '\n')).collect();
    if let Some(after) = rest.strip_prefix("//") {
        let end = after.find(['/', '?', '#']).unwrap_or(after.len());
        rest = after[end..].to_string();
    }
    if let Some(at) = rest.find('#') {
        rest.truncate(at);
    }
    if let Some(at) = rest.find('?') {
        rest.truncate(at);
    }
    let last = rest.rfind('/').unwrap_or(0);
    if let Some(at) = rest[last..].find(';') {
        rest.truncate(last + at);
    }
    rest
}

fn starts_with_url_scheme(value: &str) -> bool {
    let mut chars = value.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    let rest = &value[1..];
    let scheme_len = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
        .count();
    rest[scheme_len..].starts_with("://")
}

/// What a string names as a file, as (path, extras): `path` is the value a
/// server would use once it trims surrounding whitespace and cuts a C string
/// at a NUL, or `None`; `extras` are the other readings, looked at only for a
/// refusal. See the module notes of `mcp`.
pub fn as_paths(value: &str, max_path: usize) -> (Option<String>, Vec<String>) {
    let mut extras = Vec::new();
    let mut value = value.to_string();
    if value.contains('\0') {
        extras.push(value.replace('\0', ""));
        value = value.split('\0').next().unwrap_or("").to_string();
    }
    let mut value = value.trim_matches(py_space).to_string();
    if value.is_empty() || value.contains('\n') {
        return (None, Vec::new());
    }
    if value.len() >= 5 && value[..5].eq_ignore_ascii_case("file:") {
        extras.push(super::super::normpath(&value));
        value = unquote(&url_path(&value));
        if value.is_empty() {
            return (None, Vec::new());
        }
    } else if starts_with_url_scheme(&value) {
        return (None, Vec::new());
    }
    if let Some(home) = home_expanded(&value) {
        extras.push(super::super::normpath(&value));
        value = home;
    }
    let normal = super::super::normpath(&value);
    if value.chars().count() > max_path {
        extras.push(value);
        let keep = (normal.chars().count() <= max_path).then_some(normal);
        return (keep, extras);
    }
    if normal != value {
        extras.push(normal);
    }
    (Some(value), extras)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_split_as_the_regex_did() {
        assert_eq!(words_of("filePath"), ["file", "path"]);
        assert_eq!(words_of("ABCdef_x2"), ["ab", "cdef", "x", "2"]);
        assert_eq!(words_of("write_FILE"), ["write", "file"]);
    }

    #[test]
    fn a_file_url_is_the_path_it_carries() {
        assert_eq!(as_paths("FILE:///a/b%20c?x#y", 4096).0.unwrap(), "/a/b c");
        assert_eq!(as_paths("file:/a;p", 4096).0.unwrap(), "/a");
        assert_eq!(as_paths("https://x/y", 4096).0, None);
        assert_eq!(as_paths(" a\0b ", 4096).0.unwrap(), "a");
    }

    #[test]
    fn the_config_scan_reads_block_and_inline_forms() {
        let dir = std::env::temp_dir().join(format!("urzua-closes-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(".urzua")).unwrap();
        for (text, want) in [
            ("harness:\n  guardrails:\n    acceptance:\n      enabled: true\n", true),
            ("harness:\n  guardrails:\n    acceptance: {enabled: true, on_unjudged: 'allow'}\n", false),
            ("harness:\n  guardrails:\n    protected-paths:\n      enabled: false\n", false),
        ] {
            std::fs::write(dir.join(".urzua/config.yaml"), text).unwrap();
            assert_eq!(config_closes(&dir), Some(want), "{text}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
