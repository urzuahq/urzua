//! Where each key of a configuration file is declared: a light second pass
//! over the YAML text that maps key paths (`record_types.bug.statuses.require[0]`)
//! to 1-based line numbers. The typed load keeps no source positions, so this
//! reads block-style YAML line by line, with the first level of a flow mapping
//! on the line that opens it. A key it cannot place (inside a multi-line flow
//! collection) is answered with the nearest enclosing key's line.

use std::collections::HashMap;

/// One segment appended to a key path: a plain key as `.key`, a key with any
/// other character as `["key"]`, so a rule id or a status with a space stays
/// one segment.
pub(crate) fn key(parent: &str, segment: &str) -> String {
    let plain = !segment.is_empty()
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    match (plain, parent.is_empty()) {
        (true, true) => segment.to_string(),
        (true, false) => format!("{parent}.{segment}"),
        (false, _) => format!("{parent}[{}]", serde_json::Value::from(segment)),
    }
}

pub(crate) fn index(parent: &str, i: usize) -> String {
    format!("{parent}[{i}]")
}

/// A flow sequence (`[a, b, ...]`) still open at the end of a line: each item
/// is given the line it starts on.
struct FlowSeq {
    path: String,
    depth: usize,
    next: usize,
    in_item: bool,
    quote: Option<char>,
}

impl FlowSeq {
    /// Reads `text` (from `line`), recording each top-level item's line;
    /// returns whether the sequence is still open.
    fn feed(&mut self, text: &str, line: usize, lines: &mut HashMap<String, usize>) -> bool {
        for c in text.chars() {
            if let Some(q) = self.quote {
                if c == q {
                    self.quote = None;
                }
                continue;
            }
            if self.depth == 1 && !self.in_item && !c.is_whitespace() && c != ',' && c != ']' {
                if c == '#' {
                    break;
                }
                lines.entry(index(&self.path, self.next)).or_insert(line);
                self.next += 1;
                self.in_item = true;
            }
            match c {
                '"' | '\'' => self.quote = Some(c),
                '[' | '{' => self.depth += 1,
                ']' | '}' => {
                    self.depth -= 1;
                    if self.depth == 0 {
                        return false;
                    }
                }
                ',' if self.depth == 1 => self.in_item = false,
                _ => {}
            }
        }
        true
    }
}

struct Frame {
    indent: usize,
    path: String,
    /// A key whose value starts on a later line: a sequence may sit at its
    /// own indent.
    open_key: bool,
}

#[derive(Default)]
pub(crate) struct ConfigLines {
    lines: HashMap<String, usize>,
    anchors: HashMap<String, String>,
    /// `(path of the key holding the alias, anchor name)`, in file order.
    aliases: Vec<(String, String)>,
}

impl ConfigLines {
    pub(crate) fn scan(text: &str) -> Self {
        let mut out = ConfigLines::default();
        let mut stack: Vec<Frame> = Vec::new();
        let mut counters: HashMap<String, usize> = HashMap::new();
        let mut block_scalar_at: Option<usize> = None;
        let mut flow: Option<FlowSeq> = None;
        for (n, raw) in text.lines().enumerate() {
            let line_no = n + 1;
            if let Some(seq) = flow.as_mut() {
                if !seq.feed(raw, line_no, &mut out.lines) {
                    flow = None;
                }
                continue;
            }
            let trimmed = raw.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("---") {
                continue;
            }
            let indent = raw.len() - trimmed.len();
            if let Some(at) = block_scalar_at {
                if indent > at {
                    continue;
                }
                block_scalar_at = None;
            }
            let (indent, rest, item_path) = if let Some(after) = trimmed
                .strip_prefix("- ")
                .or(if trimmed == "-" { Some("") } else { None })
            {
                while stack
                    .last()
                    .is_some_and(|f| f.indent > indent || (f.indent == indent && !f.open_key))
                {
                    stack.pop();
                }
                let parent = stack.last().map(|f| f.path.clone()).unwrap_or_default();
                let counter = counters.entry(parent.clone()).or_insert(0);
                let path = index(&parent, *counter);
                *counter += 1;
                out.lines.entry(path.clone()).or_insert(line_no);
                let inner = indent + 2 + (after.len() - after.trim_start().len());
                stack.push(Frame {
                    indent,
                    path: path.clone(),
                    open_key: false,
                });
                let after = after.trim_start();
                if after.starts_with('{') {
                    out.flow_keys(&path, after, line_no);
                }
                (inner, after, Some(path))
            } else {
                (indent, trimmed, None)
            };
            let Some((name, value)) = split_key(rest) else {
                continue;
            };
            while stack.last().is_some_and(|f| {
                f.indent >= indent && item_path.as_deref() != Some(f.path.as_str())
            }) {
                stack.pop();
            }
            let parent = stack.last().map(|f| f.path.clone()).unwrap_or_default();
            let path = key(&parent, &name);
            out.lines.entry(path.clone()).or_insert(line_no);
            counters.remove(&path);
            let mut value = value.trim();
            if let Some(rest) = value.strip_prefix('&') {
                let (name, after) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
                out.anchors.insert(name.to_string(), path.clone());
                value = after.trim();
            } else if let Some(name) = value.strip_prefix('*') {
                out.aliases.push((path.clone(), name.trim().to_string()));
            }
            if value.starts_with('{') {
                out.flow_keys(&path, value, line_no);
            }
            if value.starts_with('[') {
                let mut seq = FlowSeq {
                    path: path.clone(),
                    depth: 0,
                    next: 0,
                    in_item: false,
                    quote: None,
                };
                if seq.feed(value, line_no, &mut out.lines) {
                    flow = Some(seq);
                }
            }
            if value.starts_with('|') || value.starts_with('>') {
                block_scalar_at = Some(indent);
            }
            stack.push(Frame {
                indent,
                path,
                open_key: value.is_empty() || value.starts_with('#'),
            });
        }
        out
    }

    /// The first-level keys of a flow mapping that opens on `line`.
    fn flow_keys(&mut self, parent: &str, flow: &str, line: usize) {
        let mut depth = 0usize;
        let mut quote: Option<char> = None;
        let mut start = 1usize;
        for (i, c) in flow.char_indices() {
            match (quote, c) {
                (Some(q), c) if c == q => quote = None,
                (Some(_), _) => {}
                (None, '"' | '\'') => quote = Some(c),
                (None, '{' | '[') => depth += 1,
                (None, '}' | ']') => {
                    if depth == 1 {
                        self.flow_entry(parent, &flow[start..i], line);
                    }
                    depth = depth.saturating_sub(1);
                }
                (None, ',') if depth == 1 => {
                    self.flow_entry(parent, &flow[start..i], line);
                    start = i + 1;
                }
                _ => {}
            }
        }
    }

    fn flow_entry(&mut self, parent: &str, entry: &str, line: usize) {
        if let Some((name, _)) = split_key(entry.trim()) {
            self.lines.entry(key(parent, &name)).or_insert(line);
        }
    }

    /// The line `path` is declared on, else its nearest enclosing key's.
    pub(crate) fn line(&self, path: &str) -> Option<usize> {
        let mut p = path;
        loop {
            if let Some(n) = self.lines.get(p) {
                return Some(*n);
            }
            p = parent_of(p)?;
        }
    }

    /// Every alias at or under `prefix`, as `(alias at, anchor at)`.
    pub(crate) fn aliases_under(&self, prefix: &str) -> Vec<(String, String)> {
        self.aliases
            .iter()
            .filter(|(p, _)| {
                p == prefix
                    || p.starts_with(&format!("{prefix}."))
                    || p.starts_with(&format!("{prefix}["))
            })
            .filter_map(|(p, name)| Some((self.at(p), self.at(self.anchors.get(name)?))))
            .collect()
    }

    /// `path:line`, or the bare path when no line is known.
    pub(crate) fn at(&self, path: &str) -> String {
        match self.line(path) {
            Some(n) => format!("{path}:{n}"),
            None => path.to_string(),
        }
    }
}

fn parent_of(path: &str) -> Option<&str> {
    if let Some(stripped) = path.strip_suffix(']') {
        let open = if stripped.ends_with('"') {
            stripped.rfind("[\"")?
        } else {
            stripped.rfind('[')?
        };
        return Some(&path[..open]);
    }
    path.rfind('.').map(|i| &path[..i])
}

/// `key: value` with the key unquoted; `None` for a line that is not a key.
fn split_key(text: &str) -> Option<(String, &str)> {
    let (name, rest) = if let Some(q) = text.chars().next().filter(|c| *c == '"' || *c == '\'') {
        let close = text[1..].find(q)? + 1;
        (text[1..close].to_string(), &text[close + 1..])
    } else {
        let colon = text
            .char_indices()
            .find(|(i, c)| {
                *c == ':' && text[i + 1..].chars().next().is_none_or(char::is_whitespace)
            })?
            .0;
        let name = text[..colon].trim();
        if name.is_empty() || name.contains(['{', '[', '#']) {
            return None;
        }
        (name.to_string(), &text[colon..])
    };
    let rest = rest.trim_start().strip_prefix(':')?;
    Some((name, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_block_flow_and_sequence_keys() {
        let text = "a:\n  b: 1\n  list:\n  - x\n  - {name: y, targets: [t]}\n  \"k.v\": 2\nrules:\n  status.x: error\n";
        let l = ConfigLines::scan(text);
        assert_eq!(l.line("a.b"), Some(2));
        assert_eq!(l.line("a.list[0]"), Some(4));
        assert_eq!(l.line("a.list[1].targets"), Some(5));
        assert_eq!(l.line("a[\"k.v\"]"), Some(6));
        assert_eq!(l.line("rules[\"status.x\"]"), Some(8));
        assert_eq!(l.line("a.list[1].targets[0]"), Some(5));
        assert_eq!(l.line("rules[\"status.x\"][0]"), Some(8));
    }

    #[test]
    fn an_alias_names_where_its_anchor_is_defined() {
        let text = "t:\n  a:\n    statuses: &st\n      values: [X]\n  b:\n    statuses: *st\n";
        let l = ConfigLines::scan(text);
        assert_eq!(l.line("t.a.statuses.values"), Some(4));
        assert_eq!(
            l.aliases_under("t.b"),
            [("t.b.statuses:6".to_string(), "t.a.statuses:3".to_string())]
        );
    }

    #[test]
    fn each_item_of_a_multi_line_flow_list_has_its_own_line() {
        let text = "t:\n  known: [a,\n    \"b, c\",\n    {x: 1},\n    d]\n  after: 1\n";
        let l = ConfigLines::scan(text);
        assert_eq!(l.line("t.known[0]"), Some(2));
        assert_eq!(l.line("t.known[1]"), Some(3));
        assert_eq!(l.line("t.known[2]"), Some(4));
        assert_eq!(l.line("t.known[3]"), Some(5));
        assert_eq!(l.line("t.after"), Some(6));
    }

    #[test]
    fn nested_items_restart_their_count() {
        let text = "t:\n  x:\n    req:\n      - {when: [A]}\n  y:\n    req:\n      - a: 1\n        b: 2\n      - a: 3\n";
        let l = ConfigLines::scan(text);
        assert_eq!(l.line("t.x.req[0].when"), Some(4));
        assert_eq!(l.line("t.y.req[0].b"), Some(8));
        assert_eq!(l.line("t.y.req[1].a"), Some(9));
    }
}
