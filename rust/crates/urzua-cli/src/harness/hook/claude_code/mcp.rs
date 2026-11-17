//! An MCP tool call as file writes. Every path-like field is read, at any depth
//! and in any list (`path`, `file`, `from`, `source`, `destination`, `uri` and
//! the like, a `file:` URI included, whatever the case of its scheme and with
//! one slash or three; a `~` or `$HOME` at its start is the home directory),
//! and a key that is itself a path (`{"files": {"<path>": "<text>"}}`). An
//! object with one path field and a content field is a write of that content to
//! that path. A path of a call that looks like an edit, a move or a delete, or
//! that sits where it cannot be placed, cannot be turned into the file's new
//! text, so it is an edit that was not judged: refused only where the target is
//! a gated record or a protected path and the configuration says deny. A tool
//! named for reading is left alone unless the call names an output or carries
//! content.
//!
//! A call is judged cheapest first, and the first block ends it: every file
//! against the gates (one engine process for all of them), then the strings
//! that could not be placed, then the files whose judgement checks a record.
//! Beyond a cap of `harness.limits` a call is an edit that was not judged,
//! never silently cut short.

use super::super::super::json::J;
use super::super::{engine, text as vtext, truthy};
use super::text::{as_paths, words_of};
use super::{Kind, Result1, Run, ALLOWED_NOTES};

const CONTENT_KEYS: [&str; 10] = [
    "content",
    "contents",
    "file_text",
    "fileText",
    "text",
    "new_source",
    "body",
    "data",
    "payload",
    "value",
];
const EDIT_KEYS: [&str; 16] = [
    "edits",
    "old_string",
    "new_string",
    "old_str",
    "new_str",
    "old_text",
    "new_text",
    "oldText",
    "newText",
    "patch",
    "diff",
    "changes",
    "operations",
    "insert_text",
    "replacement",
    "append",
];
const PATH_WORDS: [&str; 20] = [
    "path",
    "paths",
    "file",
    "files",
    "filepath",
    "filename",
    "source",
    "src",
    "dest",
    "destination",
    "target",
    "from",
    "to",
    "uri",
    "url",
    "dir",
    "directory",
    "folder",
    "old",
    "new",
];
const PARTIAL_WORDS: [&str; 12] = [
    "append", "insert", "patch", "edit", "replace", "modify", "update", "move", "rename", "delete",
    "remove", "copy",
];
const WRITE_EXTRA: [&str; 11] = [
    "write",
    "create",
    "save",
    "put",
    "overwrite",
    "mv",
    "rm",
    "touch",
    "mkdir",
    "set",
    "upload",
];
const READ_WORDS: [&str; 27] = [
    "read", "get", "list", "search", "find", "stat", "view", "show", "cat", "head", "tail",
    "fetch", "query", "describe", "info", "tree", "grep", "glob", "exists", "diff", "status",
    "log", "count", "check", "lookup", "browse", "inspect",
];
const DEST_WORDS: [&str; 11] = [
    "output",
    "out",
    "outfile",
    "outpath",
    "dest",
    "destination",
    "target",
    "save",
    "saveas",
    "into",
    "sink",
];

/// A verdict word this build knows.
fn known(kind: &str) -> bool {
    kind == "allow" || kind == "block" || ALLOWED_NOTES.contains(&kind)
}

fn has(list: &[&str], w: &str) -> bool {
    !w.is_empty() && list.contains(&w)
}

/// The caps of one call: the built-in floors, raised to the configured
/// `harness.limits`, never lowered.
pub struct Caps {
    depth: usize,
    path: usize,
    string: usize,
    loose: usize,
    records: usize,
    items: usize,
}

impl Caps {
    pub fn configured(dir: Option<&str>) -> Caps {
        use urzua_core::harness::limits as l;
        let limits = super::super::super::limits::of(None, dir.map(std::path::Path::new));
        let cap = |k: &str| {
            let floor = l::limit_info(k).map_or(0, |i| i.default);
            usize::try_from(limits.get(k).max(floor)).unwrap_or(usize::MAX)
        };
        Caps {
            depth: cap(l::INPUT_DEPTH),
            path: cap(l::PATH_LENGTH),
            string: cap(l::STRING_LENGTH),
            loose: cap(l::LOOSE_STRINGS_PER_CALL),
            records: cap(l::RECORDS_PER_CALL),
            items: cap(l::PATHS_PER_CALL),
        }
    }
}

/// One object of the input: its path fields, its content text, whether it
/// carries an edit-shaped field, and which path fields name an output.
#[derive(Clone)]
struct Rec {
    paths: Vec<String>,
    content: Option<String>,
    partial: bool,
    dest: Vec<String>,
}

struct Scan<'a> {
    caps: &'a Caps,
    records: Vec<Rec>,
    loose: Vec<String>,
    deep: bool,
}

impl Scan<'_> {
    /// Adds what `value` names to `target` (`None`: the loose strings), and its
    /// other readings to the loose strings.
    fn add(&mut self, target: Option<&mut Vec<String>>, value: &str) {
        let (p, extras) = as_paths(value, self.caps.path);
        let into_loose = target.is_none();
        let mut target = target;
        if let Some(p) = p {
            match target.as_deref_mut() {
                Some(t) => {
                    if !t.contains(&p) {
                        t.push(p);
                    }
                }
                None if !self.loose.contains(&p) => {
                    if self.loose.len() >= self.caps.loose {
                        self.deep = true;
                    } else {
                        self.loose.push(p);
                    }
                }
                None => {}
            }
        }
        for e in extras {
            let in_target = !into_loose && target.as_deref().is_some_and(|t| t.contains(&e));
            if !self.loose.contains(&e) && !in_target && e.chars().count() <= self.caps.string {
                if self.loose.len() >= self.caps.loose {
                    self.deep = true;
                } else {
                    self.loose.push(e);
                }
            }
        }
    }

    fn walk_list(&mut self, items: &[J], depth: usize) {
        for item in items {
            match item {
                J::Str(t) => self.add(None, t),
                other => self.walk(other, depth + 1),
            }
        }
    }

    fn walk(&mut self, node: &J, depth: usize) {
        if depth > self.caps.depth {
            self.deep = true;
            return;
        }
        match node {
            J::Obj(members) => self.walk_obj(members, depth),
            J::Arr(items) => self.walk_list(items, depth),
            _ => {}
        }
    }

    fn walk_obj(&mut self, members: &[(String, J)], depth: usize) {
        let mut rec = Rec {
            paths: Vec::new(),
            content: None,
            partial: false,
            dest: Vec::new(),
        };
        for (k, v) in members {
            if k.contains(['/', '.', '\\']) {
                let mut keyed = Vec::new();
                self.add(Some(&mut keyed), k);
                if let (Some(first), J::Str(text)) = (keyed.first(), v) {
                    self.records.push(Rec {
                        paths: vec![first.clone()],
                        content: Some(text.clone()),
                        partial: false,
                        dest: Vec::new(),
                    });
                    continue;
                }
                for p in keyed {
                    if !rec.paths.contains(&p) {
                        rec.paths.push(p);
                    }
                }
            }
            if CONTENT_KEYS.contains(&k.as_str()) {
                match v {
                    J::Str(t) => match &rec.content {
                        None => rec.content = Some(t.clone()),
                        Some(c) if c != t => rec.partial = true,
                        Some(_) => {}
                    },
                    J::Obj(_) | J::Arr(_) => self.walk(v, depth + 1),
                    _ => {}
                }
                continue;
            }
            if EDIT_KEYS.contains(&k.as_str()) {
                rec.partial = true;
                if matches!(v, J::Obj(_) | J::Arr(_)) {
                    self.walk(v, depth + 1);
                }
                continue;
            }
            let kw = words_of(k);
            let is_dest = kw.iter().any(|w| has(&DEST_WORDS, w));
            if is_dest || kw.iter().any(|w| has(&PATH_WORDS, w)) {
                self.keyed_value(&mut rec, v, is_dest, depth);
            } else if let J::Str(t) = v {
                self.add(None, t);
            } else {
                self.walk(v, depth + 1);
            }
        }
        self.records.push(rec);
    }

    fn keyed_value(&mut self, rec: &mut Rec, v: &J, is_dest: bool, depth: usize) {
        let mut put = |scan: &mut Self, t: &str| {
            scan.add(Some(&mut rec.paths), t);
            if is_dest {
                scan.add(Some(&mut rec.dest), t);
            }
        };
        match v {
            J::Str(t) => put(self, t),
            J::Arr(items) => {
                for item in items {
                    match item {
                        J::Str(t) => put(self, t),
                        other => self.walk(other, depth + 1),
                    }
                }
            }
            other => self.walk(other, depth + 1),
        }
    }
}

/// What an MCP call writes, when it writes files.
pub struct Target {
    /// Whole-file writes urzua can judge as they are.
    items: Vec<(String, String)>,
    /// Paths that cannot be turned into a file's new text.
    rest: Vec<String>,
    /// Other strings, looked at only for a refusal.
    loose: Vec<String>,
    /// Whether not judging `rest` is worth saying.
    loud: bool,
    deep: bool,
}

pub fn write(tool: &str, inp: &J, caps: &Caps) -> Option<Target> {
    let name = tool.rsplit("__").next().unwrap_or(tool);
    let words = words_of(name);
    let any = |list: &[&str]| words.iter().any(|w| has(list, w));
    let write_like = any(&PARTIAL_WORDS) || any(&WRITE_EXTRA);
    let mut scan = Scan {
        caps,
        records: Vec::new(),
        loose: Vec::new(),
        deep: false,
    };
    scan.walk(inp, 0);
    let (mut records, mut loose, mut deep) = (scan.records, scan.loose, scan.deep);
    if any(&READ_WORDS) && !write_like {
        // The name is the server's, the arguments are the call's.
        let kept: Vec<Rec> = records
            .iter()
            .filter_map(|r| {
                if r.content.is_some() {
                    Some(r.clone())
                } else if !r.dest.is_empty() {
                    Some(Rec {
                        paths: r.dest.clone(),
                        content: None,
                        partial: true,
                        dest: r.dest.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();
        if kept.is_empty() {
            return None;
        }
        (records, loose, deep) = (kept, Vec::new(), false);
    }
    let tool_partial = any(&PARTIAL_WORDS);
    let (mut items, mut rest) = (Vec::new(), Vec::<String>::new());
    for r in &records {
        match (&r.paths[..], &r.content) {
            ([one], Some(c)) if !r.partial && !tool_partial => items.push((one.clone(), c.clone())),
            _ => {
                for p in &r.paths {
                    if !rest.contains(p) {
                        rest.push(p.clone());
                    }
                }
            }
        }
    }
    let keyed = !items.is_empty() || !rest.is_empty();
    if !(keyed || !loose.is_empty() || deep) {
        return None;
    }
    let writes = write_like
        || records
            .iter()
            .any(|r| r.content.is_some() || r.partial || !r.dest.is_empty());
    loose.retain(|p| !rest.contains(p));
    Some(Target {
        items,
        rest,
        loose,
        loud: keyed && writes,
        deep,
    })
}

fn unjudged_tail() -> &'static str {
    "It is allowed unless the configuration refuses an edit that was not judged; run 'urzua check' before committing."
}

/// The gates for every whole-file write at once; the files whose judgement
/// checks a record, or the block that ends the call.
fn gates(
    run: &mut Run,
    tool: &str,
    items: &[(String, String)],
    cwd: Option<&str>,
    results: &mut Vec<Result1>,
) -> Result<Vec<(String, String)>, String> {
    let mut records = Vec::new();
    let mut request = run.request("edit", Some(tool), cwd);
    request["paths"] = serde_json::json!(items.iter().map(|(p, _)| p).collect::<Vec<_>>());
    request["content"] = match items {
        [(_, c)] => serde_json::json!(c),
        _ => serde_json::Value::Null,
    };
    request["contents"] = serde_json::json!(items.iter().map(|(_, c)| c).collect::<Vec<_>>());
    request["gates_only"] = serde_json::json!(true);
    let Ok(verdict) = engine(&run.event, &request, cwd) else {
        results.push(run.engine_unrunnable(cwd, &items[0].0));
        return Ok(records);
    };
    if vtext(&verdict, "verdict") == "block" {
        return Err(vtext(&verdict, "message").to_string());
    }
    if !known(vtext(&verdict, "verdict")) {
        let message = super::unknown_verdict(vtext(&verdict, "verdict"));
        let paths = items.iter().map(|(p, _)| p.clone()).collect();
        results.push(run.unjudged_result(&message, cwd, paths, true, true));
        return Ok(records);
    }
    if let Some(flags) = verdict.get("records").and_then(|f| f.as_array()) {
        if flags.len() == items.len() {
            for (item, flag) in items.iter().zip(flags) {
                if truthy(Some(flag)) {
                    records.push(item.clone());
                }
            }
            return Ok(records);
        }
    }
    // An engine that does not read a batch: one file at a time.
    for (path, content) in items {
        let mut request = run.request("edit", Some(tool), cwd);
        request["paths"] = serde_json::json!([path]);
        request["content"] = serde_json::json!(content);
        request["gates_only"] = serde_json::json!(true);
        let Ok(verdict) = engine(&run.event, &request, cwd) else {
            results.push(run.engine_unrunnable(cwd, path));
            continue;
        };
        let (kind, message) = (vtext(&verdict, "verdict"), vtext(&verdict, "message"));
        if kind == "block" {
            return Err(message.to_string());
        }
        match verdict.get("record") {
            None if ALLOWED_NOTES.contains(&kind) => {
                results.push((Kind::Note, message.to_string()))
            }
            None => {}
            Some(flag) if truthy(Some(flag)) => records.push((path.clone(), content.clone())),
            Some(_) => {}
        }
    }
    Ok(records)
}

/// The parts of the call that are not judged as whole-file writes.
fn unplaced(
    run: &mut Run,
    tool: &str,
    target: &Target,
    cwd: Option<&str>,
    caps: &Caps,
    results: &mut Vec<Result1>,
) {
    let overflow: Vec<String> = target
        .items
        .iter()
        .skip(caps.items)
        .map(|(p, _)| p.clone())
        .collect();
    if !overflow.is_empty() {
        let message = format!("urzua harness: urzua did NOT judge this edit ({tool} writes {} files, more than the {} it reads in one call; the first {} were judged against the gates). {}", target.items.len(), caps.items, caps.items, unjudged_tail());
        let r = run.unjudged_result(&message, cwd, overflow, false, true);
        results.push(r);
    }
    if target.deep {
        let message = format!("urzua harness: urzua did NOT judge this edit ({tool}: its input has more strings or nests deeper than the translator reads). {}", unjudged_tail());
        let r = run.unjudged_result(&message, cwd, Vec::new(), false, true);
        results.push(r);
    }
    let candidates: Vec<String> = target.rest.iter().chain(&target.loose).cloned().collect();
    if !candidates.is_empty() {
        let shown = if target.rest.is_empty() {
            "an unparsed path".to_string()
        } else {
            target.rest.join(", ")
        };
        let message = format!("urzua harness: urzua did NOT judge this edit ({tool} on {shown}: the call is not a whole-file write urzua can read). {}", unjudged_tail());
        let loud = target.loud && !target.rest.is_empty();
        let r = run.unjudged_result(&message, cwd, candidates, false, loud);
        results.push(r);
    }
}

pub fn judge(run: &mut Run, tool: &str, target: Target, cwd: Option<&str>, caps: &Caps) {
    let mut results: Vec<Result1> = Vec::new();
    let items: Vec<(String, String)> = target.items.iter().take(caps.items).cloned().collect();
    let mut records = Vec::new();
    if !items.is_empty() {
        match gates(run, tool, &items, cwd, &mut results) {
            Ok(r) => records = r,
            Err(block) => return run.decide(&[(Kind::Block, block)]),
        }
    }
    unplaced(run, tool, &target, cwd, caps, &mut results);
    if results.iter().any(|(k, _)| *k == Kind::Block) {
        return run.decide(&results);
    }
    if records.len() > caps.records {
        let shown: Vec<&str> = records
            .iter()
            .take(caps.records)
            .map(|(p, _)| p.as_str())
            .collect();
        let message = format!("urzua harness: urzua did NOT judge this edit ({tool} writes {} files that are checked as records, more than the {} it judges in one call: {}). Write them in separate calls. {}", records.len(), caps.records, shown.join(", ") + ", ...", unjudged_tail());
        let paths = records.iter().map(|(p, _)| p.clone()).collect();
        let r = run.unjudged_result(&message, cwd, paths, false, true);
        results.push(r);
        return run.decide(&results);
    }
    for (path, content) in &records {
        let mut request = run.request("edit", Some(tool), cwd);
        request["paths"] = serde_json::json!([path]);
        request["content"] = serde_json::json!(content);
        let Ok(verdict) = engine(&run.event, &request, cwd) else {
            results.push(run.engine_unrunnable(cwd, path));
            continue;
        };
        let (kind, message) = (vtext(&verdict, "verdict"), vtext(&verdict, "message"));
        if kind == "block" {
            return run.decide(&[(Kind::Block, message.to_string())]);
        }
        if ALLOWED_NOTES.contains(&kind) {
            results.push((Kind::Note, message.to_string()));
        } else if !known(kind) {
            let message = super::unknown_verdict(kind);
            results.push(run.unjudged_result(&message, cwd, vec![path.clone()], true, true));
        }
    }
    run.decide(&results);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_no_configuration_the_caps_are_the_floors() {
        let caps = Caps::configured(Some("/nonexistent-urzua-dir"));
        assert_eq!(
            (
                caps.depth,
                caps.path,
                caps.string,
                caps.loose,
                caps.records,
                caps.items
            ),
            (24, 4096, 65536, 512, 8, 512)
        );
    }
}
