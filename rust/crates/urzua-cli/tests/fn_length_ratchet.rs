//! `SPEC-22`'s function-size bound, enforced over the whole workspace with a
//! shrink-only ratchet.
//!
//! A function is at most 100 lines, counted from its signature to its closing
//! brace. This test measures every non-test function in every crate's `src` and
//! compares it with `fn-length-ceilings.tsv` (next to the workspace
//! `Cargo.toml`), the functions that were already over the bound when the gate
//! was introduced, each with its length then. It fails when
//!
//! - a function that is not listed is over the bound (split it; do not list it),
//! - a listed function is longer than its ceiling (it grew), or
//! - a listed function is now at or under the bound, or shorter than its ceiling
//!   (lower the entry to its length, or delete it once the function is at most 100
//!   lines), so the list can only shrink as functions are split.
//!
//! Entries are keyed by file and by the function's path (enclosing `mod`, `impl`,
//! `trait` and `fn` names), never by line number, so an unrelated edit above a
//! function does not change the file. To print today's offenders in the file's
//! format: `cargo test -p urzua-cli --test fn_length_ratchet -- --ignored --nocapture`.
//!
//! The assertion is a pure function over already-measured lengths (`judge`), so
//! the tests at the bottom prove each failure with a planted case.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MAX_FUNCTION_LINES: usize = 100;

/// One non-test function: its key (`file::path::to::fn`) and its length in lines.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Measured {
    key: String,
    lines: usize,
}

// ---------------------------------------------------------------------------
// Measuring
// ---------------------------------------------------------------------------

/// The source with comments, string and character literals blanked to spaces
/// (newlines kept), so braces and keywords inside them are not seen and every
/// byte keeps its line.
fn blank_literals(src: &str) -> Vec<u8> {
    let b = src.as_bytes();
    let n = b.len();
    let mut out = b.to_vec();
    let blank = |out: &mut Vec<u8>, from: usize, to: usize| {
        for byte in &mut out[from..to.min(n)] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    };
    let mut i = 0;
    while i < n {
        let c = b[i];
        if b[i..].starts_with(b"//") {
            let j = b[i..].iter().position(|&x| x == b'\n').map_or(n, |p| i + p);
            blank(&mut out, i, j);
            i = j;
        } else if b[i..].starts_with(b"/*") {
            // Block comments nest in Rust.
            let mut depth = 0;
            let mut j = i;
            while j < n {
                if b[j..].starts_with(b"/*") {
                    depth += 1;
                    j += 2;
                } else if b[j..].starts_with(b"*/") {
                    depth -= 1;
                    j += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    j += 1;
                }
            }
            blank(&mut out, i, j);
            i = j;
        } else if c == b'r' && starts_raw_string_prefix(b, i) {
            // A raw string `r"..."` / `r#"..."#` or a byte raw string `br"..."` / `br#"..."#`;
            // otherwise an ordinary identifier.
            let mut h = i + 1;
            while h < n && b[h] == b'#' {
                h += 1;
            }
            if h < n && b[h] == b'"' {
                let mut close = vec![b'"'];
                close.extend(std::iter::repeat_n(b'#', h - i - 1));
                let body = h + 1;
                let j = find(&b[body..], &close).map_or(n, |p| body + p + close.len());
                blank(&mut out, i, j);
                i = j;
            } else {
                i += 1;
            }
        } else if c == b'"' {
            let mut j = i + 1;
            while j < n && b[j] != b'"' {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            blank(&mut out, i, j + 1);
            i = j + 1;
        } else if c == b'\'' {
            // A character literal (`'x'`, `'\n'`, `'\u{1F}'`), not a lifetime (`'a`).
            let end = if b.get(i + 1) == Some(&b'\\') {
                b[i + 2..]
                    .iter()
                    .position(|&x| x == b'\'')
                    .map(|p| i + 2 + p)
            } else if b.get(i + 2) == Some(&b'\'') && b.get(i + 1) != Some(&b'\'') {
                Some(i + 2)
            } else {
                None
            };
            match end {
                Some(e) => {
                    blank(&mut out, i, e + 1);
                    i = e + 1;
                }
                None => i += 1,
            }
        } else {
            i += 1;
        }
    }
    out
}

/// Whether the `r` at `i` begins a raw string prefix: not part of a longer identifier,
/// except for the `b` of a byte raw string (`br"..."`).
fn starts_raw_string_prefix(b: &[u8], i: usize) -> bool {
    let ident = |x: u8| x.is_ascii_alphanumeric() || x == b'_';
    match i.checked_sub(1).map(|p| b[p]) {
        None => true,
        Some(prev) if !ident(prev) => true,
        Some(b'b') => i == 1 || !ident(b[i - 2]),
        Some(_) => false,
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// What a `{` opened, from the item header in front of it.
#[derive(Debug, Clone)]
enum ScopeKind {
    Fn { name: String, start_line: usize },
    Named(String),
    Other,
}

#[derive(Debug, Clone)]
struct Scope {
    kind: ScopeKind,
    test: bool,
}

const QUALIFIERS: &[&str] = &["pub", "const", "async", "unsafe", "extern", "default"];

/// The words of a header with attributes, `pub(...)` restrictions and
/// qualifiers dropped: what is left starts with the item keyword.
fn header_words(header: &str) -> Vec<String> {
    let mut text = String::new();
    let mut depth = 0usize;
    // Drop `#[...]` / `#![...]` attributes.
    let chars: Vec<char> = header.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '#' && matches!(chars.get(i + 1), Some('[') | Some('!')) {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '[' {
                j += 1;
            }
            for &ch in &chars[j..] {
                j += 1;
                if ch == '[' {
                    depth += 1;
                } else if ch == ']' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            i = j;
            continue;
        }
        text.push(chars[i]);
        i += 1;
    }
    // Drop the parenthesised restriction after `pub`.
    let text = text.replace("pub (", "pub(");
    let mut words: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut paren = 0usize;
    for ch in text.chars() {
        if ch == '(' && cur == "pub" {
            paren += 1;
            continue;
        }
        if paren > 0 {
            if ch == '(' {
                paren += 1;
            } else if ch == ')' {
                paren -= 1;
            }
            continue;
        }
        if ch.is_alphanumeric() || ch == '_' {
            cur.push(ch);
        } else {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            if !ch.is_whitespace() {
                words.push(ch.to_string());
            }
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    let skip = words
        .iter()
        .take_while(|w| QUALIFIERS.contains(&w.as_str()))
        .count();
    words.split_off(skip)
}

/// Whether the header's attributes make the item test-only.
fn is_test_attribute(header: &str) -> bool {
    let compact: String = header.chars().filter(|c| !c.is_whitespace()).collect();
    compact.contains("#[cfg(test)]")
        || compact.contains("#[cfg(all(test")
        || compact.contains(",test)") && compact.contains("#[cfg(all(")
        || compact.contains("#[test]")
}

/// `impl<T> Trait<T> for Type<T> where ...` -> `Type as Trait`; `impl<T> Type<T>` -> `Type`.
fn impl_name(words: &[String]) -> String {
    // Drop generic argument lists: everything between balanced `<` `>`.
    let mut flat: Vec<&str> = Vec::new();
    let mut depth = 0usize;
    for w in words.iter().skip(1) {
        match w.as_str() {
            "<" => depth += 1,
            ">" => depth = depth.saturating_sub(1),
            "where" if depth == 0 => break,
            other if depth == 0 => flat.push(other),
            _ => {}
        }
    }
    match flat.iter().position(|w| *w == "for") {
        Some(at) => format!("{} as {}", flat[at + 1..].join(""), flat[..at].join("")),
        None => flat.join(""),
    }
}

/// Every non-test function in one source file with its length, plus the names of
/// out-of-line `#[cfg(test)] mod name;` declarations (their files are test code).
fn measure_source(file: &str, src: &str) -> (Vec<Measured>, Vec<String>) {
    let s = blank_literals(src);
    let text = String::from_utf8_lossy(&s).into_owned();
    let mut stack: Vec<Scope> = Vec::new();
    let mut found: Vec<Measured> = Vec::new();
    let mut test_mods: Vec<String> = Vec::new();
    let mut header = String::new();
    let mut nest = 0usize; // ( and [ depth inside the current header
    let mut line = 1usize;
    for ch in text.chars() {
        match ch {
            '\n' => {
                line += 1;
                header.push(ch);
            }
            '(' | '[' => {
                nest += 1;
                header.push(ch);
            }
            ')' | ']' => {
                nest = nest.saturating_sub(1);
                header.push(ch);
            }
            ';' if nest == 0 => {
                let words = header_words(&header);
                if words.first().map(String::as_str) == Some("mod")
                    && is_test_attribute(&header)
                    && stack.iter().all(|sc| !sc.test)
                {
                    if let Some(name) = words.get(1) {
                        test_mods.push(name.clone());
                    }
                }
                header.clear();
            }
            '{' if nest == 0 => {
                let words = header_words(&header);
                let in_test = stack.last().is_some_and(|sc| sc.test);
                let test = in_test || is_test_attribute(&header);
                let kind = match words.first().map(String::as_str) {
                    Some("fn") => {
                        let name = words.get(1).cloned().unwrap_or_default();
                        ScopeKind::Fn {
                            name,
                            start_line: line - lines_after_fn_keyword(&header),
                        }
                    }
                    Some("impl") => ScopeKind::Named(impl_name(&words)),
                    Some("mod") | Some("trait") => {
                        ScopeKind::Named(words.get(1).cloned().unwrap_or_default())
                    }
                    _ => ScopeKind::Other,
                };
                stack.push(Scope { kind, test });
                header.clear();
            }
            '}' if nest == 0 => {
                if let Some(scope) = stack.pop() {
                    if let (ScopeKind::Fn { name, start_line }, false) = (&scope.kind, scope.test) {
                        let mut path: Vec<&str> = stack
                            .iter()
                            .filter_map(|sc| match &sc.kind {
                                ScopeKind::Fn { name, .. } | ScopeKind::Named(name) => {
                                    Some(name.as_str())
                                }
                                ScopeKind::Other => None,
                            })
                            .collect();
                        path.push(name);
                        found.push(Measured {
                            key: format!("{file}::{}", path.join("::")),
                            lines: line - start_line + 1,
                        });
                    }
                }
                header.clear();
            }
            _ => header.push(ch),
        }
    }
    (disambiguate(found), test_mods)
}

/// Newlines between the header's `fn` keyword and its end: the signature's line
/// count beyond the first.
fn lines_after_fn_keyword(header: &str) -> usize {
    let b = header.as_bytes();
    let at = (0..b.len().saturating_sub(2)).find(|&i| {
        &b[i..i + 2] == b"fn"
            && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_'))
            && b[i + 2].is_ascii_whitespace()
    });
    at.map_or(0, |p| header[p..].matches('\n').count())
}

/// Two functions with the same key (e.g. the same method in two trait impls of one
/// type with the same name) get `#2`, `#3`, ... in source order.
fn disambiguate(found: Vec<Measured>) -> Vec<Measured> {
    // `found` is in closing-brace order; sort by nothing, count per key in that order.
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    found
        .into_iter()
        .map(|m| {
            let n = seen.entry(m.key.clone()).or_insert(0);
            *n += 1;
            if *n == 1 {
                m
            } else {
                Measured {
                    key: format!("{}#{}", m.key, n),
                    lines: m.lines,
                }
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// The workspace
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root")
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// The Rust files `SPEC-22` bounds in one crate: its `src`, `benches` and `examples`
/// directories and its `build.rs`, where present.
fn crate_rust_files(krate: &Path, out: &mut Vec<PathBuf>) {
    for dir in ["src", "benches", "examples"] {
        rust_files(&krate.join(dir), out);
    }
    let build = krate.join("build.rs");
    if build.is_file() {
        out.push(build);
    }
}

/// Every non-test function in each crate's `src`, `benches`, `examples` and `build.rs`,
/// keyed by its path from the workspace root.
fn measure_workspace(root: &Path) -> Vec<Measured> {
    let mut files = Vec::new();
    let crates = root.join("crates");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&crates)
        .expect("the crates directory")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("src").is_dir())
        .collect();
    dirs.sort();
    for d in &dirs {
        crate_rust_files(d, &mut files);
    }
    files.sort();
    let mut excluded: Vec<PathBuf> = Vec::new();
    let mut measured: Vec<(PathBuf, Vec<Measured>)> = Vec::new();
    for f in &files {
        let rel = f
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let src = std::fs::read_to_string(f).expect("a readable source file");
        let (fns, test_mods) = measure_source(&rel, &src);
        let stem = f.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let base = if matches!(stem, "lib" | "main" | "mod") {
            f.parent().unwrap().to_path_buf()
        } else {
            f.parent().unwrap().join(stem)
        };
        for m in test_mods {
            excluded.push(base.join(format!("{m}.rs")));
            excluded.push(base.join(&m));
        }
        measured.push((f.clone(), fns));
    }
    measured
        .into_iter()
        .filter(|(f, _)| !excluded.iter().any(|x| f == x || f.starts_with(x)))
        .flat_map(|(_, fns)| fns)
        .collect()
}

// ---------------------------------------------------------------------------
// The ceiling file
// ---------------------------------------------------------------------------

/// Parse `key<TAB>ceiling` lines; blank lines and `#` comments are skipped.
fn parse_ceilings(text: &str) -> Result<Vec<(String, usize)>, Vec<String>> {
    let mut entries = Vec::new();
    let mut problems = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let Some((key, len)) = t.rsplit_once('\t') else {
            problems.push(format!("line {}: expected `<key><TAB><ceiling>`", n + 1));
            continue;
        };
        match len.trim().parse::<usize>() {
            Ok(v) => entries.push((key.to_string(), v)),
            Err(_) => problems.push(format!("line {}: `{}` is not a number", n + 1, len.trim())),
        }
    }
    if problems.is_empty() {
        Ok(entries)
    } else {
        Err(problems)
    }
}

/// Every way the measured functions and the ceilings disagree; empty when the gate holds.
fn judge(measured: &[Measured], ceilings: &[(String, usize)]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut listed: BTreeMap<&str, usize> = BTreeMap::new();
    for (key, ceiling) in ceilings {
        if listed.insert(key.as_str(), *ceiling).is_some() {
            problems.push(format!(
                "{key}: listed twice in the ceiling file; delete one entry"
            ));
        }
        if *ceiling <= MAX_FUNCTION_LINES {
            problems.push(format!(
                "{key}: ceiling {ceiling} is not above the {MAX_FUNCTION_LINES}-line bound; delete the entry"
            ));
        }
    }
    let by_key: BTreeMap<&str, usize> =
        measured.iter().map(|m| (m.key.as_str(), m.lines)).collect();
    for m in measured {
        match listed.get(m.key.as_str()) {
            None if m.lines > MAX_FUNCTION_LINES => problems.push(format!(
                "{}: {} lines, over the {MAX_FUNCTION_LINES}-line bound (SPEC-22); split it into helpers. Do not add it to the ceiling file, which may only shrink",
                m.key, m.lines
            )),
            None => {}
            Some(&ceiling) if m.lines > ceiling => problems.push(format!(
                "{}: grew to {} lines, over its ceiling of {ceiling}; split it, do not raise the ceiling",
                m.key, m.lines
            )),
            Some(_) if m.lines <= MAX_FUNCTION_LINES => problems.push(format!(
                "{}: now {} lines, within the {MAX_FUNCTION_LINES}-line bound; delete its entry from the ceiling file",
                m.key, m.lines
            )),
            Some(&ceiling) if m.lines < ceiling => problems.push(format!(
                "{}: now {} lines, below its ceiling of {ceiling}; lower the entry to {}",
                m.key, m.lines, m.lines
            )),
            Some(_) => {}
        }
    }
    for (key, _) in ceilings {
        if !by_key.contains_key(key.as_str()) {
            problems.push(format!(
                "{key}: no such non-test function; the entry is stale (renamed, moved or removed), so delete it or update its key"
            ));
        }
    }
    problems
}

fn ceilings_path(root: &Path) -> PathBuf {
    root.join("fn-length-ceilings.tsv")
}

#[test]
fn every_function_is_within_the_bound_or_ratcheted() {
    let root = workspace_root();
    let measured = measure_workspace(&root);
    assert!(
        measured.len() > 500,
        "the scan should see the workspace's functions, saw {}",
        measured.len()
    );
    let text = std::fs::read_to_string(ceilings_path(&root)).expect("fn-length-ceilings.tsv");
    let ceilings =
        parse_ceilings(&text).unwrap_or_else(|p| panic!("fn-length-ceilings.tsv: {p:?}"));
    let problems = judge(&measured, &ceilings);
    assert!(
        problems.is_empty(),
        "SPEC-22 function-size ratchet (fn-length-ceilings.tsv may only shrink):\n{}",
        problems.join("\n")
    );
}

/// Prints today's offenders in the ceiling file's format.
#[test]
#[ignore = "prints the current offenders; run it by hand"]
fn print_offenders() {
    let mut over: Vec<Measured> = measure_workspace(&workspace_root())
        .into_iter()
        .filter(|m| m.lines > MAX_FUNCTION_LINES)
        .collect();
    over.sort_by(|a, b| a.key.cmp(&b.key));
    for m in over {
        println!("{}\t{}", m.key, m.lines);
    }
}

// ---------------------------------------------------------------------------
// The gate itself
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn long_fn(name: &str, body_lines: usize) -> String {
        let mut s = format!("fn {name}() {{\n");
        for _ in 0..body_lines {
            s.push_str("    let _ = 1;\n");
        }
        s.push_str("}\n");
        s
    }

    fn m(key: &str, lines: usize) -> Measured {
        Measured {
            key: key.to_string(),
            lines,
        }
    }

    #[test]
    fn counts_from_signature_to_closing_brace() {
        let (fns, _) = measure_source("a.rs", &long_fn("f", 98));
        assert_eq!(fns, vec![m("a.rs::f", 100)]);
        let (fns, _) = measure_source("a.rs", &long_fn("f", 99));
        assert_eq!(fns, vec![m("a.rs::f", 101)]);
    }

    #[test]
    fn a_planted_long_function_that_is_not_listed_fails() {
        let (fns, _) = measure_source("a.rs", &long_fn("planted", 120));
        let problems = judge(&fns, &[]);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(
            problems[0].contains("a.rs::planted")
                && problems[0].contains("over the 100-line bound")
        );
    }

    #[test]
    fn a_function_at_the_bound_passes() {
        let (fns, _) = measure_source("a.rs", &long_fn("ok", 98));
        assert!(judge(&fns, &[]).is_empty());
    }

    #[test]
    fn a_listed_function_that_grew_fails() {
        let problems = judge(&[m("a.rs::f", 131)], &[("a.rs::f".into(), 130)]);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("grew to 131") && problems[0].contains("ceiling of 130"));
    }

    #[test]
    fn a_listed_function_at_its_ceiling_passes() {
        assert!(judge(&[m("a.rs::f", 130)], &[("a.rs::f".into(), 130)]).is_empty());
    }

    #[test]
    fn a_listed_function_that_shrank_below_the_bound_must_be_removed() {
        let problems = judge(&[m("a.rs::f", 90)], &[("a.rs::f".into(), 130)]);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("delete its entry"));
        // Exactly 100 is within the bound too.
        assert_eq!(
            judge(&[m("a.rs::f", 100)], &[("a.rs::f".into(), 130)]).len(),
            1
        );
    }

    #[test]
    fn a_listed_function_that_shrank_but_is_still_long_must_lower_its_ceiling() {
        let problems = judge(&[m("a.rs::f", 110)], &[("a.rs::f".into(), 130)]);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("lower the entry to 110"));
    }

    #[test]
    fn an_entry_for_a_function_that_no_longer_exists_fails() {
        let problems = judge(&[], &[("a.rs::gone".into(), 130)]);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("stale"));
    }

    #[test]
    fn a_ceiling_at_or_under_the_bound_and_a_duplicate_entry_are_rejected() {
        let measured = [m("a.rs::f", 120)];
        let low = judge(&measured, &[("a.rs::f".into(), 100)]);
        assert!(low.iter().any(|p| p.contains("not above")), "{low:?}");
        let dup = judge(
            &measured,
            &[("a.rs::f".into(), 120), ("a.rs::f".into(), 120)],
        );
        assert!(dup.iter().any(|p| p.contains("listed twice")), "{dup:?}");
    }

    #[test]
    fn ceiling_file_parsing_skips_comments_and_reports_bad_lines() {
        let ok = parse_ceilings("# note\n\na.rs::f\t120\n").unwrap();
        assert_eq!(ok, vec![("a.rs::f".to_string(), 120)]);
        assert_eq!(parse_ceilings("a.rs::f 120\n").unwrap_err().len(), 1);
        assert_eq!(parse_ceilings("a.rs::f\tmany\n").unwrap_err().len(), 1);
    }

    #[test]
    fn test_code_is_not_measured() {
        let src = format!(
            "{}\n#[cfg(test)]\nmod tests {{\n{}}}\n\n#[cfg(test)]\nmod elsewhere;\n",
            long_fn("real", 5),
            long_fn("big_test", 150)
        );
        let (fns, test_mods) = measure_source("a.rs", &src);
        assert_eq!(fns, vec![m("a.rs::real", 7)]);
        assert_eq!(test_mods, vec!["elsewhere".to_string()]);
    }

    #[test]
    fn braces_in_strings_comments_and_chars_do_not_confuse_the_count() {
        let src = "fn f() {\n    let a = \"}\";\n    // }\n    let b = '}';\n    let c = r#\"}\"#;\n    let d: &'static str = \"{\";\n}\nfn g() {}\n";
        let (fns, _) = measure_source("a.rs", src);
        assert_eq!(fns, vec![m("a.rs::f", 7), m("a.rs::g", 1)]);
    }

    #[test]
    fn byte_raw_strings_are_blanked_like_raw_strings() {
        let src = "fn f() {\n    let a = br#\"}\"#;\n    let b = br\"{\";\n    let r = 1;\n    let br = 2;\n}\nfn g() {}\n";
        let (fns, _) = measure_source("a.rs", src);
        assert_eq!(fns, vec![m("a.rs::f", 6), m("a.rs::g", 1)]);
    }

    #[test]
    fn an_identifier_ending_in_br_is_not_a_raw_string() {
        let src = "fn f() {\n    let abr = 1;\n    let x = abr;\n}\nfn g() {}\n";
        let (fns, _) = measure_source("a.rs", src);
        assert_eq!(fns, vec![m("a.rs::f", 4), m("a.rs::g", 1)]);
    }

    #[test]
    fn the_workspace_scan_includes_build_scripts() {
        let measured = measure_workspace(&workspace_root());
        assert!(
            measured
                .iter()
                .any(|f| f.key.starts_with("crates/urzua-cli/build.rs::")),
            "build.rs functions are not measured"
        );
        let dir = std::env::temp_dir().join(format!("urzua-ratchet-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("benches")).unwrap();
        std::fs::create_dir_all(dir.join("examples")).unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        for f in ["build.rs", "benches/b.rs", "examples/e.rs", "src/lib.rs"] {
            std::fs::write(dir.join(f), "fn f() {}\n").unwrap();
        }
        let mut files = Vec::new();
        crate_rust_files(&dir, &mut files);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(files.len(), 4, "{files:?}");
    }

    #[test]
    fn keys_carry_the_enclosing_modules_impls_and_functions() {
        let src = "mod inner {\n    pub struct S;\n    impl<T> S<T> {\n        pub(crate) fn run(&self) {\n            fn helper() {}\n        }\n    }\n    impl std::fmt::Display for S {\n        fn fmt(&self) {}\n    }\n}\n";
        let (fns, _) = measure_source("a.rs", src);
        let keys: Vec<&str> = fns.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(
            keys,
            vec![
                "a.rs::inner::S::run::helper",
                "a.rs::inner::S::run",
                "a.rs::inner::S as std::fmt::Display::fmt"
            ]
        );
    }

    #[test]
    fn a_multiline_signature_counts_from_the_fn_keyword() {
        let src = "pub fn f(\n    a: [u8; 4],\n    b: u8,\n) -> u8 {\n    a[0] + b\n}\n";
        let (fns, _) = measure_source("a.rs", src);
        assert_eq!(fns, vec![m("a.rs::f", 6)]);
    }

    #[test]
    fn two_functions_with_one_key_stay_distinct() {
        let src = "impl A for X { fn f() {} }\nimpl A for X { fn f() {} }\n";
        let (fns, _) = measure_source("a.rs", src);
        assert_eq!(fns.len(), 2);
        assert_ne!(fns[0].key, fns[1].key);
    }
}
