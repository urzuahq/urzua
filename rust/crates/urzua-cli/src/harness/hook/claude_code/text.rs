//! Text helpers of the adapter: how it reads whitespace, lines, words, URLs
//! and paths in a payload and in the configuration.

use std::path::Path;

/// Whitespace as the adapter reads it: Unicode whitespace and the four
/// separator controls (U+001C to U+001F).
pub fn is_blank(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// What the configuration that governs a directory says about an edit nobody
/// judged, read without the engine.
#[derive(Debug, PartialEq, Eq)]
pub enum Unjudged {
    /// No `.urzua/config.yaml` at or above the directory: not governed.
    NotGoverned,
    /// Read with certainty as letting an unjudged edit through.
    Open,
    /// Refusing, or not readable with certainty (it does not parse, cannot be
    /// read, or holds a value of a shape the rule does not accept).
    Closed,
}

/// The rule the hook command's text fallback also follows: a configuration is
/// open only when it parses and neither `acceptance` nor `protected-paths`
/// under `harness.guardrails` is enabled without `on_unjudged: allow`. A
/// guardrail counts as off only when `enabled` is absent, null or `false`, and
/// as allowing only when `on_unjudged` is exactly the string `allow`; anything
/// else is closed.
pub fn unjudged_at(d: &Path) -> Unjudged {
    let start = super::super::lexical_absolute(d);
    let Some(path) = start
        .ancestors()
        .map(|a| a.join(".urzua/config.yaml"))
        .find(|p| p.exists())
    else {
        return Unjudged::NotGoverned;
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Unjudged::Closed;
    };
    let Ok(doc) = yaml_serde::from_str::<yaml_serde::Value>(urzua_core::config::strip_bom(&text))
    else {
        return Unjudged::Closed;
    };
    if open(&doc) {
        Unjudged::Open
    } else {
        Unjudged::Closed
    }
}

fn open(doc: &yaml_serde::Value) -> bool {
    use yaml_serde::Value as V;
    let guardrails = match doc.get("harness") {
        None | Some(V::Null) => return true,
        Some(h @ V::Mapping(_)) => match h.get("guardrails") {
            None | Some(V::Null) => return true,
            Some(g @ V::Mapping(_)) => g,
            Some(_) => return false,
        },
        Some(_) => return false,
    };
    ["acceptance", "protected-paths"].iter().all(|name| {
        let g = match guardrails.get(*name) {
            None | Some(V::Null) => return true,
            Some(g @ V::Mapping(_)) => g,
            Some(_) => return false,
        };
        let off = matches!(
            g.get("enabled"),
            None | Some(V::Null) | Some(V::Bool(false))
        );
        let allows = matches!(g.get("on_unjudged"), Some(V::String(s)) if s == "allow");
        off || allows
    })
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
        let run =
            |from: usize, f: &dyn Fn(char) -> bool| c[from..].iter().take_while(|x| f(**x)).count();
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
    let home = std::env::var("HOME")
        .ok()
        .filter(|h| !h.is_empty() && h != "~")?;
    Some(format!("{home}{}", &value[head.len()..]))
}

/// Percent-decoding: `%XX` becomes the byte, an invalid sequence is kept, and
/// the result is read as UTF-8 with replacement.
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

/// The path of a `file:` URL: after the host, before `?` and `#`, without the
/// `;parameters` of its last segment.
fn url_path(value: &str) -> String {
    let mut rest: String = value[5..]
        .chars()
        .filter(|c| !matches!(c, '\t' | '\r' | '\n'))
        .collect();
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
    let mut value = value.trim_matches(is_blank).to_string();
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
    fn words_split_at_case_changes_and_digits() {
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
    fn a_configuration_is_open_only_when_it_parses_and_says_so() {
        let dir = std::env::temp_dir().join(format!("urzua-unjudged-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(".urzua")).unwrap();
        let sub = dir.join("a/b");
        std::fs::create_dir_all(&sub).unwrap();
        for (text, want) in [
            (
                "harness:\n  guardrails:\n    acceptance:\n      enabled: true\n",
                Unjudged::Closed,
            ),
            (
                "harness:\n  guardrails:\n    acceptance: {enabled: true, on_unjudged: 'allow'}\n",
                Unjudged::Open,
            ),
            (
                "harness:\n  guardrails:\n    protected-paths:\n      enabled: false\n",
                Unjudged::Open,
            ),
            (
                "harness:\n  guardrails:\n    protected-paths: {enabled: 'yes'}\n",
                Unjudged::Closed,
            ),
            ("harness: [\n", Unjudged::Closed),
            ("schema_version: 2\n", Unjudged::Open),
        ] {
            std::fs::write(dir.join(".urzua/config.yaml"), text).unwrap();
            assert_eq!(unjudged_at(&sub), want, "{text}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            unjudged_at(Path::new("/nonexistent-urzua/x")),
            Unjudged::NotGoverned
        );
    }
}
