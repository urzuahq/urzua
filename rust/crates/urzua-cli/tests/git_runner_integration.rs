//! `git` is started in one place: `urzua-io`'s runner (`urzua_io::git`). A second
//! production site that builds its own `Command::new("git")` would word spawn
//! failures and non-zero exits its own way and wire its own pipes, which is the
//! duplication the runner removed.
//!
//! The scan covers every production source file (`crates/*/src` and each crate's
//! `build.rs`), skipping `#[cfg(test)]` and `#[test]` items and the files of
//! out-of-line test modules. Outside the runner it flags a `Command::new(` (also
//! `std::process::Command::new(`) whose argument is the literal `"git"`, and one
//! whose argument is not a string literal at all, since that could be `git` in
//! disguise. A legitimate non-literal spawn goes in `ALLOWED_NON_LITERAL`.

use std::path::{Path, PathBuf};

/// The one file allowed to start `git`.
const RUNNER: &str = "urzua-io/src/git.rs";

/// Non-literal `Command::new(<expr>)` sites that are legitimate, as
/// (file relative to the crates directory, whitespace-free argument expression).
/// Each entry needs a comment saying what it runs. There are none today: the
/// only non-literal spawn (running the urzua binary) is inside a test module.
const ALLOWED_NON_LITERAL: &[(&str, &str)] = &[];

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// The source with comments blanked and the contents of string and character
/// literals blanked, to spaces (newlines kept, every byte keeps its offset). A
/// string keeps its opening and closing `"`, so `"   "` in the result marks a
/// string literal whose text is at the same offsets in the source.
fn blank(src: &str) -> Vec<u8> {
    let b = src.as_bytes();
    let n = b.len();
    let mut out = b.to_vec();
    let wipe = |out: &mut Vec<u8>, from: usize, to: usize| {
        for byte in &mut out[from..to.min(n)] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    };
    let mut i = 0;
    while i < n {
        if b[i..].starts_with(b"//") {
            let j = b[i..].iter().position(|&x| x == b'\n').map_or(n, |p| i + p);
            wipe(&mut out, i, j);
            i = j;
        } else if b[i..].starts_with(b"/*") {
            let (mut depth, mut j) = (0, i);
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
            wipe(&mut out, i, j);
            i = j;
        } else if b[i] == b'r'
            && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_'))
        {
            let mut h = i + 1;
            while h < n && b[h] == b'#' {
                h += 1;
            }
            if h < n && b[h] == b'"' {
                let mut close = vec![b'"'];
                close.extend(std::iter::repeat_n(b'#', h - i - 1));
                let body = h + 1;
                let j = find(&b[body..], &close).map_or(n, |p| body + p + close.len());
                wipe(&mut out, i, j);
                out[i] = b'"';
                out[j - 1] = b'"';
                i = j;
            } else {
                i += 1;
            }
        } else if b[i] == b'"' {
            let mut j = i + 1;
            while j < n && b[j] != b'"' {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            let end = (j + 1).min(n);
            wipe(&mut out, i + 1, j);
            i = end;
        } else if b[i] == b'\'' {
            // A character literal, not a lifetime.
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
                    wipe(&mut out, i, e + 1);
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

/// Index of the byte closing the bracket opened at `open` (`{`, `(` or `[`).
fn matching(b: &[u8], open: usize) -> usize {
    let mut depth = 0usize;
    for (k, &c) in b.iter().enumerate().skip(open) {
        match c {
            b'{' | b'(' | b'[' => depth += 1,
            b'}' | b')' | b']' => {
                depth -= 1;
                if depth == 0 {
                    return k;
                }
            }
            _ => {}
        }
    }
    b.len() - 1
}

fn is_test_attribute(attr: &str) -> bool {
    let c: String = attr.chars().filter(|c| !c.is_whitespace()).collect();
    c == "#[cfg(test)]"
        || c == "#[test]"
        || c.starts_with("#[cfg(all(test")
        || (c.starts_with("#[cfg(all(") && c.contains(",test"))
}

/// Blank every test-only item (attribute, header and body) in `b` in place.
/// Returns the names of out-of-line test modules (`#[cfg(test)] mod name;`).
fn mask_tests(b: &mut [u8]) -> Vec<String> {
    let mut mods = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'#' && b.get(i + 1) == Some(&b'[') {
            let close = matching(b, i + 1);
            let attr = String::from_utf8_lossy(&b[i..=close]).into_owned();
            if is_test_attribute(&attr) {
                // The item runs to its `;` or through its braced body.
                let mut k = close + 1;
                let mut nest = 0usize;
                while k < b.len() {
                    match b[k] {
                        b'(' | b'[' => nest += 1,
                        b')' | b']' => nest = nest.saturating_sub(1),
                        b';' | b'{' if nest == 0 => break,
                        _ => {}
                    }
                    k += 1;
                }
                let header = String::from_utf8_lossy(&b[close + 1..k.min(b.len())]).into_owned();
                let words: Vec<&str> = header
                    .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .filter(|w| !w.is_empty())
                    .collect();
                let end = if k < b.len() && b[k] == b'{' {
                    matching(b, k)
                } else {
                    k.min(b.len() - 1)
                };
                if b.get(k) == Some(&b';') {
                    if let Some(at) = words.iter().position(|w| *w == "mod") {
                        if let Some(name) = words.get(at + 1) {
                            mods.push((*name).to_string());
                        }
                    }
                }
                for byte in &mut b[i..=end] {
                    if *byte != b'\n' {
                        *byte = b' ';
                    }
                }
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
    mods
}

/// Violations in one file's production code, and its out-of-line test modules.
fn scan_source(rel: &str, src: &str) -> (Vec<String>, Vec<String>) {
    let mut masked = blank(src);
    let test_mods = mask_tests(&mut masked);
    let mut found = Vec::new();
    let needle = b"Command::new(";
    let mut from = 0;
    while let Some(p) = find(&masked[from..], needle) {
        let open = from + p + needle.len() - 1;
        let close = matching(&masked, open);
        from = open + 1;
        let arg = &masked[open + 1..close];
        let trimmed = String::from_utf8_lossy(arg).trim().to_string();
        let line = masked[..open].iter().filter(|&&c| c == b'\n').count() + 1;
        let is_literal = trimmed.len() >= 2
            && trimmed.starts_with('"')
            && trimmed.ends_with('"')
            && trimmed[1..trimmed.len() - 1].trim().is_empty();
        let expr: String = src[open + 1..close]
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        if rel == RUNNER {
            continue;
        }
        if is_literal {
            if expr.trim_matches('"') == "git" {
                found.push(format!("{rel}:{line}: Command::new(\"git\")"));
            }
        } else if !ALLOWED_NON_LITERAL.contains(&(rel, expr.as_str())) {
            found.push(format!(
                "{rel}:{line}: Command::new({expr}) with a non-literal program"
            ));
        }
    }
    (found, test_mods)
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Every place production code starts a process in a way that could be `git`.
fn violations() -> Vec<String> {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for krate in std::fs::read_dir(&crates).unwrap() {
        let krate = krate.unwrap().path();
        let src = krate.join("src");
        if src.is_dir() {
            rust_files(&src, &mut files);
        }
        let build = krate.join("build.rs");
        if build.is_file() {
            files.push(build);
        }
    }
    files.sort();
    let mut excluded: Vec<PathBuf> = Vec::new();
    let mut scanned = Vec::new();
    for f in &files {
        let rel = f
            .strip_prefix(&crates)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let (found, test_mods) = scan_source(&rel, &std::fs::read_to_string(f).unwrap());
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
        scanned.push((f.clone(), found));
    }
    scanned
        .into_iter()
        .filter(|(f, _)| !excluded.iter().any(|x| f == x || f.starts_with(x)))
        .flat_map(|(_, found)| found)
        .collect()
}

#[test]
fn git_is_spawned_in_one_place() {
    let hits = violations();
    assert!(
        hits.is_empty(),
        "production code that starts git itself, or starts a process whose program is not a \
         literal; use urzua_io::git::GitRun, or allow-list a legitimate spawn by file and \
         expression: {hits:#?}"
    );
}

#[test]
fn the_runner_itself_is_found_and_exempt() {
    let src = "fn run() { let mut c = Command::new(\"git\"); }";
    assert!(scan_source(RUNNER, src).0.is_empty());
    assert_eq!(scan_source("a/src/x.rs", src).0.len(), 1);
}

#[test]
fn planted_spawns_are_flagged() {
    let planted = [
        "fn f() { Command::new(\"git\"); }",
        "fn f() { std::process::Command::new(\"git\").arg(\"x\"); }",
        "fn f() { std::process::Command::new(\n    \"git\",\n); }",
        "fn f() { Command::new(program); }",
        "fn f() { Command::new(&exe); }",
        "fn f() { Command::new(GIT); }",
        "fn f() { Command::new(format!(\"g{}\", \"it\")); }",
        // A `//` inside a string must not hide the call that follows it.
        "fn f() { let u = \"http://x\"; Command::new(\"git\"); }",
        "fn f() { let _ = (\"//\", Command::new(prog)); }",
        // Test code before production code does not end the scan.
        "#[cfg(test)]\nmod tests { fn t() {} }\nfn f() { Command::new(\"git\"); }",
        // A `#[cfg(test)]` that is not a module only masks its own item.
        "#[cfg(test)]\nfn helper() {}\nfn f() { Command::new(\"git\"); }",
    ];
    for src in planted {
        assert_eq!(
            scan_source("a/src/x.rs", src).0.len(),
            1,
            "not flagged: {src}"
        );
    }
}

#[test]
fn legitimate_and_test_only_spawns_pass() {
    let fine = [
        "fn f() { Command::new(\"curl\"); }",
        "fn f() { Command::new(r#\"gh\"#); }",
        "fn f() { /* Command::new(\"git\") */ }",
        "fn f() { // Command::new(\"git\")\n}",
        "fn f() { let s = \"Command::new(\\\"git\\\")\"; }",
        "#[cfg(test)]\nmod tests {\n    fn t() { Command::new(\"git\"); Command::new(&exe); }\n}",
        "#[test]\nfn t() { Command::new(\"git\"); }",
    ];
    for src in fine {
        assert!(
            scan_source("a/src/x.rs", src).0.is_empty(),
            "flagged: {src}"
        );
    }
    let (found, mods) = scan_source("a/src/x.rs", "#[cfg(test)]\nmod tests;\nfn f() {}");
    assert!(found.is_empty());
    assert_eq!(mods, vec!["tests".to_string()]);
}
