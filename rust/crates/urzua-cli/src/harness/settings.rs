//! Merging hook entries into an agent's project settings file (`ADR-89`
//! section 2). The file is a person's too, so the merge adds the bundle's
//! entries, keyed by event, matcher and handler, and leaves every other key and
//! entry as written. An entry is urzua's when its command runs
//! `urzua harness run <handler>`, which is how `update` and `uninstall` find
//! it. An entry that has the key of one of ours and was edited is a conflict:
//! the merge reports it and never guesses.

use super::json::J;

const RUNS: &str = "urzua harness run ";

/// One hook entry of the adapter's settings fragment.
#[derive(Debug, Clone)]
pub struct Entry {
    pub event: String,
    pub matcher: Option<String>,
    /// `edit`, `session-start`: the word after `urzua harness run`.
    pub handler: String,
    /// The hook object, as the fragment writes it.
    pub hook: J,
}

impl Entry {
    /// `event|matcher|handler`.
    pub fn key(&self) -> String {
        format!(
            "{}|{}|{}",
            self.event,
            self.matcher.as_deref().unwrap_or(""),
            self.handler
        )
    }

    pub fn hash(&self) -> String {
        super::lock::hash(&self.hook.compact())
    }
}

/// The handler a hook command runs, when it is one of urzua's.
pub fn handler_of(hook: &J) -> Option<String> {
    let command = hook.get("command")?.as_str()?;
    let at = command.find(RUNS)?;
    let handler: String = command[at + RUNS.len()..]
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '-')
        .collect();
    (!handler.is_empty()).then_some(handler)
}

/// The entries a fragment holds, or why it is not a fragment.
pub fn entries(fragment: &J) -> Result<Vec<Entry>, String> {
    let J::Obj(events) = fragment.get("hooks").ok_or("the fragment has no `hooks`")? else {
        return Err("the fragment's `hooks` is not an object".into());
    };
    let mut out = Vec::new();
    for (event, groups) in events {
        let J::Arr(groups) = groups else {
            return Err(format!("the fragment's `hooks.{event}` is not a list"));
        };
        for group in groups {
            let matcher = group.get("matcher").and_then(J::as_str).map(str::to_string);
            let Some(J::Arr(hooks)) = group.get("hooks") else {
                return Err(format!("a `hooks.{event}` group has no `hooks` list"));
            };
            for hook in hooks {
                let handler = handler_of(hook).ok_or_else(|| {
                    format!("a `hooks.{event}` entry does not run `{}`", RUNS.trim_end())
                })?;
                out.push(Entry {
                    event: event.clone(),
                    matcher: matcher.clone(),
                    handler,
                    hook: hook.clone(),
                });
            }
        }
    }
    Ok(out)
}

fn groups_mut<'a>(settings: &'a mut J, event: &str) -> Option<&'a mut Vec<J>> {
    match settings.get_mut("hooks")?.get_mut(event)? {
        J::Arr(groups) => Some(groups),
        _ => None,
    }
}

fn matches(group: &J, matcher: &Option<String>) -> bool {
    group.get("matcher").and_then(J::as_str).map(str::to_string) == *matcher
}

/// The hook of `entry`'s key as it stands in `settings`.
pub fn find<'a>(
    settings: &'a J,
    event: &str,
    matcher: &Option<String>,
    handler: &str,
) -> Option<&'a J> {
    let J::Arr(groups) = settings.get("hooks")?.get(event)? else {
        return None;
    };
    groups
        .iter()
        .filter(|g| matches(g, matcher))
        .filter_map(|g| match g.get("hooks") {
            Some(J::Arr(hooks)) => Some(hooks),
            _ => None,
        })
        .flatten()
        .find(|h| handler_of(h).as_deref() == Some(handler))
}

#[derive(Debug, PartialEq, Eq)]
pub enum Merged {
    Added,
    Unchanged,
    /// An entry with this key exists and differs from ours.
    Edited,
}

/// What merging `entry` would do, without doing it.
pub fn plan(settings: &J, entry: &Entry) -> Result<Merged, String> {
    check_shape(settings, &entry.event)?;
    Ok(
        match find(settings, &entry.event, &entry.matcher, &entry.handler) {
            None => Merged::Added,
            Some(existing) if *existing == entry.hook => Merged::Unchanged,
            Some(_) => Merged::Edited,
        },
    )
}

fn check_shape(settings: &J, event: &str) -> Result<(), String> {
    if !matches!(settings, J::Obj(_)) {
        return Err("the settings file is not a JSON object".into());
    }
    match settings.get("hooks") {
        None => Ok(()),
        Some(J::Obj(_)) => match settings.get("hooks").and_then(|h| h.get(event)) {
            None | Some(J::Arr(_)) => Ok(()),
            Some(_) => Err(format!(
                "`hooks.{event}` in the settings file is not a list"
            )),
        },
        Some(_) => Err("`hooks` in the settings file is not an object".into()),
    }
}

/// Adds the entry when no entry has its key. An existing entry is left as it is
/// (`Unchanged` when identical, `Edited` when not).
pub fn add(settings: &mut J, entry: &Entry) -> Result<Merged, String> {
    let planned = plan(settings, entry)?;
    if planned != Merged::Added {
        return Ok(planned);
    }
    if settings.get("hooks").is_none() {
        settings.set("hooks", J::Obj(Vec::new()));
    }
    if settings
        .get("hooks")
        .and_then(|h| h.get(&entry.event))
        .is_none()
    {
        settings
            .get_mut("hooks")
            .expect("hooks was just set")
            .set(&entry.event, J::Arr(Vec::new()));
    }
    let groups = groups_mut(settings, &entry.event).expect("the event list exists");
    match groups.iter_mut().find(|g| matches(g, &entry.matcher)) {
        Some(group) => match group.get_mut("hooks") {
            Some(J::Arr(hooks)) => hooks.push(entry.hook.clone()),
            _ => group.set("hooks", J::Arr(vec![entry.hook.clone()])),
        },
        None => {
            let mut group = J::Obj(Vec::new());
            if let Some(m) = &entry.matcher {
                group.set("matcher", J::Str(m.clone()));
            }
            group.set("hooks", J::Arr(vec![entry.hook.clone()]));
            groups.push(group);
        }
    }
    Ok(Merged::Added)
}

/// Overwrites the entry with `entry`'s key by ours (an update), adding it when absent.
pub fn replace(settings: &mut J, entry: &Entry) -> Result<(), String> {
    check_shape(settings, &entry.event)?;
    if let Some(groups) = groups_mut(settings, &entry.event) {
        for group in groups.iter_mut().filter(|g| matches(g, &entry.matcher)) {
            if let Some(J::Arr(hooks)) = group.get_mut("hooks") {
                if let Some(h) = hooks
                    .iter_mut()
                    .find(|h| handler_of(h).as_deref() == Some(entry.handler.as_str()))
                {
                    *h = entry.hook.clone();
                    return Ok(());
                }
            }
        }
    }
    add(settings, entry).map(|_| ())
}

/// Removes the entry with this key, and the group, event and `hooks` object it
/// leaves empty. Nothing else is touched.
pub fn remove(settings: &mut J, event: &str, matcher: &Option<String>, handler: &str) {
    let Some(groups) = groups_mut(settings, event) else {
        return;
    };
    for group in groups.iter_mut().filter(|g| matches(g, matcher)) {
        if let Some(J::Arr(hooks)) = group.get_mut("hooks") {
            hooks.retain(|h| handler_of(h).as_deref() != Some(handler));
        }
    }
    groups.retain(|g| {
        !(matches(g, matcher)
            && matches!(g.get("hooks"), Some(J::Arr(h)) if h.is_empty())
            && only_matcher_and_hooks(g))
    });
    if groups.is_empty() {
        settings
            .get_mut("hooks")
            .expect("hooks exists")
            .remove(event);
    }
    if settings.get("hooks").is_some_and(J::is_empty_container) {
        settings.remove("hooks");
    }
}

/// A group is ours to delete only when it holds nothing but `matcher` and `hooks`.
fn only_matcher_and_hooks(group: &J) -> bool {
    match group {
        J::Obj(members) => members.iter().all(|(k, _)| k == "matcher" || k == "hooks"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::json::parse;

    const FRAGMENT: &str = r#"{"hooks":{"PreToolUse":[{"matcher":"Write|Edit","hooks":[{"type":"command","command":"x urzua harness run edit"}]}],"SessionStart":[{"hooks":[{"type":"command","command":"y urzua harness run session-start"}]}]}}"#;

    fn entries_of(f: &str) -> Vec<Entry> {
        entries(&parse(f).unwrap()).unwrap()
    }

    #[test]
    fn a_fragment_yields_one_keyed_entry_per_hook() {
        let e = entries_of(FRAGMENT);
        let keys: Vec<String> = e.iter().map(Entry::key).collect();
        assert_eq!(
            keys,
            ["PreToolUse|Write|Edit|edit", "SessionStart||session-start"]
        );
        assert!(
            entries(&parse(r#"{"hooks":{"E":[{"hooks":[{"command":"other"}]}]}}"#).unwrap())
                .is_err()
        );
    }

    #[test]
    fn merging_into_nothing_builds_the_structure_and_a_second_merge_changes_nothing() {
        let mut s = parse("{}").unwrap();
        for e in entries_of(FRAGMENT) {
            assert_eq!(add(&mut s, &e).unwrap(), Merged::Added);
        }
        let once = s.pretty();
        for e in entries_of(FRAGMENT) {
            assert_eq!(add(&mut s, &e).unwrap(), Merged::Unchanged);
        }
        assert_eq!(s.pretty(), once);
    }

    #[test]
    fn every_other_key_and_entry_is_left_as_written() {
        let person = "{\n  \"model\": \"x\",\n  \"hooks\": {\n    \"PreToolUse\": [\n      {\n        \"matcher\": \"Write|Edit\",\n        \"hooks\": [\n          {\n            \"type\": \"command\",\n            \"command\": \"mine\"\n          }\n        ]\n      }\n    ],\n    \"Stop\": []\n  },\n  \"env\": {\n    \"A\": \"1\"\n  }\n}";
        let mut s = parse(person).unwrap();
        assert_eq!(s.pretty(), person, "the fixture is canonical");
        for e in entries_of(FRAGMENT) {
            add(&mut s, &e).unwrap();
        }
        // Ours joins the group with the same matcher; the person's hook is first.
        let merged = s.pretty();
        assert!(merged.contains("\"command\": \"mine\""));
        assert!(
            merged.contains("\"Stop\": []") && merged.contains("\"env\": {\n    \"A\": \"1\"\n  }")
        );
        assert!(merged.find("\"mine\"").unwrap() < merged.find("urzua harness run edit").unwrap());
        // Removing ours restores the file byte for byte.
        for e in entries_of(FRAGMENT) {
            remove(&mut s, &e.event, &e.matcher, &e.handler);
        }
        assert_eq!(s.pretty(), person);
    }

    #[test]
    fn an_edited_entry_with_our_key_is_a_conflict_not_overwritten() {
        let mut s = parse(
            r#"{"hooks":{"PreToolUse":[{"matcher":"Write|Edit","hooks":[{"type":"command","command":"edited urzua harness run edit"}]}]}}"#,
        )
        .unwrap();
        let before = s.pretty();
        let e = &entries_of(FRAGMENT)[0];
        assert_eq!(add(&mut s, e).unwrap(), Merged::Edited);
        assert_eq!(s.pretty(), before);
        replace(&mut s, e).unwrap();
        assert!(s.pretty().contains("x urzua harness run edit"));
    }

    #[test]
    fn a_settings_file_of_the_wrong_shape_is_refused() {
        let e = &entries_of(FRAGMENT)[0];
        for bad in [r#"[]"#, r#"{"hooks":[]}"#, r#"{"hooks":{"PreToolUse":{}}}"#] {
            assert!(plan(&parse(bad).unwrap(), e).is_err(), "{bad}");
        }
    }
}
