//! `.urzua/harness.lock` (`ADR-89` section 1): the bundle version, the adapters,
//! and a hash of every installed file, including the hook entries and the
//! instructions block the install merged into files a person also edits. It is
//! what `status` compares the repository with, so drift is detectable.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const LOCK_PATH: &str = ".urzua/harness.lock";

/// A hook entry merged into a shared settings file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryHash {
    pub file: String,
    /// The hook event, such as `PreToolUse`.
    pub event: String,
    /// The matcher of the group the entry sits in; absent for a group with none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    /// The word after `urzua harness run`: how the entry is found again.
    pub handler: String,
    pub hash: String,
}

/// The instructions block in a shared file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHash {
    pub file: String,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lock {
    pub schema: u32,
    /// The engine version that wrote the lock (the bundle is versioned with it).
    pub bundle_version: String,
    pub adapters: Vec<String>,
    /// Whole files the install owns, by path.
    pub files: BTreeMap<String, String>,
    #[serde(default)]
    pub entries: Vec<EntryHash>,
    #[serde(default)]
    pub blocks: Vec<BlockHash>,
}

impl EntryHash {
    /// `event|matcher|handler`, for display only.
    pub fn key(&self) -> String {
        format!(
            "{}|{}|{}",
            self.event,
            self.matcher.as_deref().unwrap_or(""),
            self.handler
        )
    }
}

impl Lock {
    pub fn new(adapters: Vec<String>) -> Self {
        Lock {
            schema: 1,
            bundle_version: env!("CARGO_PKG_VERSION").to_string(),
            adapters,
            files: BTreeMap::new(),
            entries: Vec::new(),
            blocks: Vec::new(),
        }
    }

    pub fn to_text(&self) -> String {
        let mut me = self.clone();
        me.entries.sort_by(|a, b| {
            (&a.file, &a.event, &a.matcher, &a.handler)
                .cmp(&(&b.file, &b.event, &b.matcher, &b.handler))
        });
        me.blocks.sort_by(|a, b| a.file.cmp(&b.file));
        format!(
            "{}\n",
            serde_json::to_string_pretty(&me).expect("a lock serializes")
        )
    }

    pub fn parse(text: &str) -> Result<Lock, String> {
        let lock: Lock = serde_json::from_str(text)
            .map_err(|e| format!("{LOCK_PATH} is not a harness lock: {e}"))?;
        if lock.schema != 1 {
            return Err(format!(
                "{LOCK_PATH} has schema {}, which this build does not read (it reads 1)",
                lock.schema
            ));
        }
        Ok(lock)
    }
}

pub fn hash(text: &str) -> String {
    format!("sha256:{}", urzua_io::sha256_hex_of_str(text))
}

const WORDS: &str = "words-sha256:";

/// What a generated instruction or command file says, with only the
/// differences a formatter makes left out. Prose tolerates a closed list:
/// whitespace and line wrapping, blank lines, leading indentation, the list
/// marker character (`-`, `*`, `+`) and ordered-list numbering, the emphasis
/// marker character (`*` or `_`), table padding, and trailing whitespace. Any
/// other character added, removed or changed is a change: a heading's level,
/// a comment or a processing instruction, a rule or an underline, a
/// strikethrough, a leading `|`, a `;` for a `.`. Lines inside a code fence
/// compare exactly apart from indentation and trailing whitespace. Indentation
/// alone is never a change, even where it nests a step or turns a line into a
/// code block: the text stays visible to the reader either way.
fn words(text: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut prose: Vec<&str> = Vec::new();
    let mut fence = false;
    let mut all = text.lines().peekable();
    // Front matter is YAML the host reads: compared as the value it parses to,
    // so a formatter that folds a long value over lines changes nothing and a
    // changed key, value, nesting or delimiter does. One that does not parse is
    // compared line for line.
    if all.peek() == Some(&"---") {
        all.next();
        let mut body: Vec<&str> = Vec::new();
        let mut closed = false;
        for line in all.by_ref() {
            if line.trim_end() == "---" {
                closed = true;
                break;
            }
            body.push(line.trim_end());
        }
        let text = body.join("\n");
        let parsed = yaml_serde::from_str::<yaml_serde::Value>(&text)
            .ok()
            .and_then(|v| serde_json::to_string(&v).ok());
        out.push(match (closed, parsed) {
            (true, Some(v)) => format!("\nfm:{v}"),
            _ => format!("\nfm-text:{closed}:{text}"),
        });
    }
    for line in all {
        let t = line.trim();
        if t.starts_with("```") || t.starts_with("~~~") {
            out.push(prose_text(&std::mem::take(&mut prose)));
            fence = !fence;
            out.push(format!("\n{t}"));
            continue;
        }
        if fence {
            out.push(format!("\n{t}"));
            continue;
        }
        prose.push(line.trim_end());
    }
    out.push(prose_text(&prose));
    out.join(" ")
}

/// A list item's marker and the space after it, when `t` is a list item: a
/// bullet always; a number only where a list can start or continue (`in_list`:
/// after a blank line, a heading, a list item or a line indented under one),
/// so a wrapped line that begins with a number keeps it.
fn list_marker(t: &str, in_list: bool) -> usize {
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    let marker = match t[digits..].chars().next() {
        Some('.' | ')') if digits > 0 => digits + 1,
        Some('-' | '*' | '+') if digits == 0 => 1,
        _ => return 0,
    };
    if !t[marker..].starts_with([' ', '\t']) || (digits > 0 && !in_list) {
        return 0;
    }
    marker
}

/// A table's separator row (cells of dashes and colons between pipes) as its
/// cells without padding: a token no other line can equal.
fn table_separator(t: &str) -> Option<String> {
    if !t.contains('|')
        || !t.contains('-')
        || !t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
    {
        return None;
    }
    let cells: Vec<String> = t
        .trim_matches('|')
        .split('|')
        .map(|c| {
            let c = c.trim();
            format!(
                "{}-{}",
                if c.starts_with(':') { ":" } else { "" },
                if c.ends_with(':') { ":" } else { "" }
            )
        })
        .collect();
    Some(format!("\u{1}table:{}", cells.join("|")))
}

/// An emphasis marker `_` read as `*`: only one that opens emphasis (after the
/// start, whitespace or an opening bracket or quote, before a letter or digit)
/// or closes it (after a letter or digit, before the end, whitespace or
/// sentence punctuation). Never inside a word, a path or a glob.
fn emphasis_as_star(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    for (i, ch) in c.iter().enumerate() {
        if *ch != '_' {
            out.push(*ch);
            continue;
        }
        let (mut a, mut b) = (i, i + 1);
        while a > 0 && c[a - 1] == '_' {
            a -= 1;
        }
        while b < c.len() && c[b] == '_' {
            b += 1;
        }
        let before = a.checked_sub(1).map(|j| c[j]);
        let after = c.get(b).copied();
        let opens = before.is_none_or(|x| x.is_whitespace() || "([{\"'".contains(x))
            && after.is_some_and(char::is_alphanumeric);
        let closes = before.is_some_and(char::is_alphanumeric)
            && after.is_none_or(|x| x.is_whitespace() || ".,;:!?)]}\"'".contains(x));
        out.push(if opens || closes { '*' } else { '_' });
    }
    out
}

/// Prose lines (indentation kept) as one string, with the tolerated
/// differences taken out.
fn prose_text(lines: &[&str]) -> String {
    let mut joined = String::new();
    let mut in_list = true;
    for line in lines {
        let t = line.trim_start();
        let indented = t.len() < line.len();
        let marker = list_marker(t, in_list);
        let body = match table_separator(t) {
            Some(row) => row,
            None => t[marker..].to_string(),
        };
        in_list =
            t.is_empty() || t.starts_with('#') || list_marker(t, true) > 0 || (in_list && indented);
        joined.push(' ');
        joined.push_str(&body);
    }
    let joined = joined.split_whitespace().collect::<Vec<_>>().join(" ");
    // Inline code is compared as written; emphasis outside it.
    joined
        .split('`')
        .enumerate()
        .map(|(i, part)| {
            if i % 2 == 1 {
                part.to_string()
            } else {
                emphasis_as_star(part)
            }
        })
        .collect::<Vec<_>>()
        .join("`")
}

/// The hash a lock keeps for a generated text file: of its words only.
pub fn text_hash(text: &str) -> String {
    format!("{WORDS}{}", urzua_io::sha256_hex_of_str(&words(text)))
}

/// Whether `text` is the text a lock hash was taken of. A hash a lock written
/// before words were compared (`sha256:`) still matches the exact text.
pub fn text_matches(locked: &str, text: &str) -> bool {
    if locked.starts_with(WORDS) {
        text_hash(text) == locked
    } else {
        hash(text) == locked
    }
}

/// Whether two generated texts say the same words.
pub fn same_text(a: &str, b: &str) -> bool {
    words(a) == words(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lock_round_trips_and_is_written_in_a_stable_order() {
        let mut lock = Lock::new(vec!["a".into()]);
        lock.files.insert("z".into(), hash("z"));
        lock.files.insert("a".into(), hash("a"));
        let entry = |handler: &str| EntryHash {
            file: "s".into(),
            event: "E".into(),
            matcher: None,
            handler: handler.into(),
            hash: hash(handler),
        };
        lock.entries.push(entry("k2"));
        lock.entries.push(entry("k1"));
        let text = lock.to_text();
        assert!(text.ends_with("}\n"));
        let back = Lock::parse(&text).unwrap();
        assert_eq!(back.to_text(), text);
        assert_eq!(back.entries[0].handler, "k1");
    }

    #[test]
    fn a_reformatted_text_matches_and_changed_words_do_not() {
        let text = "# /done\n\n1. Run this:\n   - `urzua harness record done \"$RECORD\"`  \n";
        let reflowed =
            "# /done\n\n3)  Run\n    this:\n\n    * `urzua harness record done \"$RECORD\"`\n";
        assert!(text_matches(&text_hash(text), reflowed));
        assert!(same_text(text, reflowed));
        assert!(!text_matches(
            &text_hash(text),
            &text.replace("done", "amend")
        ));
        assert!(text_matches(&hash(text), text));
        assert!(!text_matches(&hash(text), reflowed));
    }

    const STEP: &str = "# /done\n\nRun:\n\n   ```bash step\n   urzua harness record done \"$RECORD\" && echo ok\n   ```\n\nThen use `urzua check --json` on it.\n";

    #[test]
    fn a_step_commented_out_inside_a_code_fence_is_drift() {
        let commented = STEP.replace("   urzua harness record", "   # urzua harness record");
        assert!(!same_text(STEP, &commented));
        assert!(!text_matches(&text_hash(STEP), &commented));
    }

    #[test]
    fn changed_punctuation_in_a_command_is_drift_and_reindenting_it_is_not() {
        assert!(!same_text(STEP, &STEP.replace("&&", "||")));
        assert!(!same_text(STEP, &STEP.replace("--json", "json")));
        assert!(same_text(
            STEP,
            &STEP.replace("   urzua harness", "      urzua harness")
        ));
        assert!(same_text(STEP, &STEP.replace("Then use", "Then\nuse")));
        assert!(same_text(
            STEP,
            &STEP.replace("check --json", "check\n   --json")
        ));
    }

    #[test]
    fn a_line_of_markup_a_heading_level_or_sentence_punctuation_changed_is_drift() {
        let doc = "---\ndescription: x\n---\n\n# /done\n\nRun it.\n\n## Steps\n\n1. Do this.\n";
        assert!(!same_text(doc, &doc.replace("## Steps", "<!--\n## Steps")));
        assert!(!same_text(doc, &doc.replace("## Steps", "-->\n## Steps")));
        assert!(!same_text(doc, &doc.replace("# /done", "## /done")));
        assert!(!same_text(doc, &doc.replace("Run it.", "Run it.\n\n---")));
        assert!(!same_text(doc, &doc.replace("Run it.", "Run it;")));
        assert!(same_text(doc, &doc.replace("Run it.", "Run\n   it.")));
    }

    #[test]
    fn only_the_differences_a_formatter_makes_are_tolerated_in_prose() {
        let doc = "---\ndescription: x\n---\n\n# /done\n\nRun *it* now.\n\n- one\n- two\n\n1. first\n2. second\n\n## Steps\n";
        for (name, changed) in [
            (
                "a link-reference comment",
                doc.replace("## Steps", "[//]: # (hidden)\n## Steps"),
            ),
            (
                "a processing instruction on one line",
                doc.replace("## Steps", "<? hidden ?>\n## Steps"),
            ),
            (
                "a processing instruction on several lines",
                doc.replace("## Steps", "<?\nhidden\n?>\n## Steps"),
            ),
            (
                "a setext underline",
                doc.replace("Run *it* now.", "Run *it* now.\n==="),
            ),
            (
                "a dash underline",
                doc.replace("Run *it* now.", "Run *it* now.\n---"),
            ),
            (
                "a strikethrough",
                doc.replace("Run *it* now.", "Run ~~it~~ now."),
            ),
            (
                "a leading pipe",
                doc.replace("Run *it* now.", "| Run *it* now."),
            ),
        ] {
            assert!(!same_text(doc, &changed), "{name}");
        }
        for (name, reformatted) in [
            (
                "list markers",
                doc.replace("- one", "* one").replace("- two", "+ two"),
            ),
            ("numbering", doc.replace("2. second", "1. second")),
            ("emphasis marker", doc.replace("*it*", "_it_")),
            (
                "wrapping and indentation",
                doc.replace("Run *it* now.", "Run\n      *it*\n   now.   "),
            ),
            ("blank lines", doc.replace("\n\n", "\n\n\n")),
        ] {
            assert!(same_text(doc, &reformatted), "{name}");
        }
    }

    #[test]
    fn the_emphasis_marker_is_tolerated_only_where_it_marks_emphasis() {
        let doc = "Set `on_unjudged` and read `docs/*.md`; on_unjudged stays.\n";
        assert!(!same_text(
            doc,
            &doc.replace("`on_unjudged`", "`on*unjudged`")
        ));
        assert!(!same_text(doc, &doc.replace("`docs/*.md`", "`docs/_.md`")));
        assert!(!same_text(
            doc,
            &doc.replace("on_unjudged stays", "on*unjudged stays")
        ));
        let em = "Read *this* now.\n";
        assert!(same_text(em, &em.replace("*this*", "_this_")));
    }

    #[test]
    fn front_matter_compares_as_the_yaml_value_it_parses_to() {
        let doc = "---\ndescription: d\nargument-hint: <t>\n---\n\n# x\n";
        for changed in [
            doc.replace(
                "description: d\nargument-hint",
                "description: d argument-hint",
            ),
            doc.replace("\nargument-hint", "\n  argument-hint"),
            doc.replacen("---", "-", 1),
            doc.replacen("---", "- - -", 1),
            doc.replacen("---", "-----", 1),
        ] {
            assert!(!same_text(doc, &changed), "{changed}");
        }
        assert!(same_text(
            doc,
            &doc.replace("description: d", "description: d   ")
        ));
    }

    #[test]
    fn a_table_separator_tolerates_padding_and_never_equals_another_line() {
        let doc = "| a | b |\n|---|:--|\n| 1 | 2 |\n";
        assert!(same_text(
            doc,
            &doc.replace("|---|:--|", "| ----- | :---- |")
        ));
        assert!(!same_text("x\n\n---\n", "x\n\n|-|\n"));
        assert!(!same_text(doc, &doc.replace("|---|:--|", "|---|---|")));
    }

    #[test]
    fn a_number_starting_a_wrapped_line_is_kept() {
        let doc = "Wait\n30. seconds.\n";
        assert!(!same_text(doc, "Wait\n5. seconds.\n"));
        let list = "Steps:\n\n1. one\n2. two\n";
        assert!(same_text(list, "Steps:\n\n1. one\n1. two\n"));
    }

    #[test]
    fn a_lock_rewritten_by_a_json_formatter_reads_the_same() {
        let mut lock = Lock::new(vec!["a".into()]);
        lock.files.insert("f.md".into(), text_hash("x"));
        let text = lock.to_text();
        let reformatted = text
            .replace("\n  ", "\n    ")
            .replace(": ", ":")
            .replace('\n', "\r\n");
        assert_eq!(Lock::parse(&reformatted).unwrap(), lock);
    }

    #[test]
    fn a_lock_of_another_schema_or_no_lock_at_all_is_refused() {
        assert!(Lock::parse("{}").is_err());
        assert!(Lock::parse("not json").is_err());
        let other = Lock::new(vec![])
            .to_text()
            .replace("\"schema\": 1", "\"schema\": 9");
        assert!(Lock::parse(&other).unwrap_err().contains("schema 9"));
    }
}
