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

/// What a generated instruction or command file says, in order, with what a
/// formatter changes left out. Prose is compared by its words (indentation,
/// line breaks, list markers and their numbers, emphasis marks and trailing
/// spaces do not count). Code is compared exactly: every line inside a code
/// fence, apart from its indentation and trailing spaces, and every inline code
/// span, apart from runs of whitespace inside it. A changed command (`&&` for
/// `||`, a `#` that comments a step out, `--json` for `json`) is therefore a
/// change, and reflowed prose is not.
fn words(text: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut prose = String::new();
    let mut fence = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("```") || t.starts_with("~~~") {
            prose_words(&std::mem::take(&mut prose), &mut out);
            fence = !fence;
            out.push(format!("\n{t}"));
            continue;
        }
        if fence {
            out.push(format!("\n{t}"));
            continue;
        }
        // A list marker a formatter may change or renumber (`-`, `*`, `+`, `1.`,
        // `1)`), followed by a space; `--flag` at a line start is not one.
        let digits = t.chars().take_while(char::is_ascii_digit).count();
        let marker = match t[digits..].chars().next() {
            Some('.' | ')') if digits > 0 => digits + 1,
            Some('-' | '*' | '+') if digits == 0 => 1,
            _ => 0,
        };
        let body = if marker > 0 && t[marker..].starts_with([' ', '\t']) {
            &t[marker..]
        } else {
            t
        };
        prose.push(' ');
        prose.push_str(body);
    }
    prose_words(&prose, &mut out);
    out.join(" ")
}

/// The words of prose, and its inline code spans exactly.
fn prose_words(prose: &str, out: &mut Vec<String>) {
    for (i, part) in prose.split('`').enumerate() {
        if i % 2 == 1 {
            out.push(format!(
                "`{}`",
                part.split_whitespace().collect::<Vec<_>>().join(" ")
            ));
        } else {
            out.extend(
                part.split(|c: char| !c.is_alphanumeric())
                    .filter(|w| !w.is_empty())
                    .map(str::to_string),
            );
        }
    }
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
