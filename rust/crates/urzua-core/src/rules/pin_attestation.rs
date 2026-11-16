//! `diff.pin-changed-unattested`: in a record the base holds, every pin the
//! head carries is either unchanged at the same occurrence or accounted for by
//! rows the pinning tools write.

use super::*;
use crate::fix::pin::Replaced;
use crate::report::{census, Outcome};
use std::collections::BTreeMap;

/// A pin occurrence: the locator as `display()` shows it (path and scope) and
/// its position among the occurrences of that locator, in evidence order.
type Key = (String, usize);

/// Each evidence occurrence of a record with the hash its pin holds (`None`
/// when unpinned), keyed occurrence by occurrence, so a hash copied from one
/// occurrence to another is a change.
fn occurrences(config: &Config, record: &Record) -> BTreeMap<Key, Option<String>> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = BTreeMap::new();
    for (locator, pin) in claim_expiry::current_locator_pins(config, record) {
        let n = seen.entry(locator.clone()).or_default();
        out.insert((locator, *n), pin);
        *n += 1;
    }
    out
}

/// One change of evidence a tool-written row attests.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    /// A pin moved from one of `from` to `to` (re-baseline, merge).
    Move {
        locator: String,
        from: Vec<String>,
        to: String,
    },
    /// A renewal that acknowledged drift of `locator` and lists it at `to`.
    Renewed {
        locator: String,
        to: String,
    },
    /// A first pin.
    First {
        locator: String,
        to: String,
    },
    Replace(Replaced),
    Remove {
        locator: String,
        hash: String,
    },
}

/// The events of the rows the head's log gained since the base, each read by
/// the parser that sits beside its writer.
fn added_events(config: &Config, record: &Record, now: &str, then: &str) -> Vec<Event> {
    let (id, shape) = (
        config.revision_log_id(&record.record_type),
        config.revision_log_shape(&record.record_type),
    );
    let rows = |text: &str| -> Vec<String> {
        match find_revision_log_entries(text, &id, &shape) {
            Some(Ok(log)) => log.entries.into_iter().map(|e| e.row).collect(),
            _ => Vec::new(),
        }
    };
    let mut before = rows(then);
    let mut out = Vec::new();
    for row in rows(now) {
        if let Some(i) = before.iter().position(|r| *r == row) {
            before.swap_remove(i);
            continue;
        }
        out.extend(row_events(&row));
    }
    out
}

fn row_events(row: &str) -> Vec<Event> {
    let moves = |pins: Vec<crate::fix::RowPin>| {
        pins.into_iter()
            .map(|p| Event::Move {
                locator: p.locator,
                from: p.from,
                to: p.to,
            })
            .collect()
    };
    if let Some(pins) = crate::fix::rebaseline_row_pins(row) {
        return moves(pins);
    }
    if let Some(pins) = crate::conflicts::merge_row_pins(row) {
        return moves(pins);
    }
    if let Some(r) = crate::fix::pin::parse_replaced_row(row) {
        return vec![Event::Replace(r)];
    }
    if let Some((locator, hash)) = crate::fix::pin::parse_removed_row(row) {
        return vec![Event::Remove { locator, hash }];
    }
    if let Some(pins) = crate::reverify::parse_pinned_row(row) {
        return pins
            .into_iter()
            .map(|(locator, to)| Event::First { locator, to })
            .collect();
    }
    if let Some(renewed) = crate::reverify::parse_reverified_row(row) {
        let acknowledged = |l: &str| {
            renewed
                .acknowledged
                .iter()
                .any(|a| a.split_once(':').map_or(a.as_str(), |(_, rest)| rest) == l)
        };
        return renewed
            .pins
            .iter()
            .filter(|(l, _)| acknowledged(l))
            .map(|(locator, to)| Event::Renewed {
                locator: locator.clone(),
                to: to.clone(),
            })
            .collect();
    }
    Vec::new()
}

/// Whether two hash texts name the same hash: equal, or one is the
/// 12-character form the rows write of the other.
fn same(a: &str, b: &str) -> bool {
    a == b || (a.len() == 12 && b.starts_with(a)) || (b.len() == 12 && a.starts_with(b))
}

fn short(hash: &str) -> &str {
    hash.get(..12).unwrap_or(hash)
}

/// Whether the events lead `locator` from `base` to `head`: a chain of moves,
/// each from a hash the chain has reached to its own new hash (a branch that
/// re-pinned, merged and re-pinned again), or a renewal that acknowledged the
/// locator's drift and lists it at the head's hash.
fn moved(events: &[Event], locator: &str, base: &str, head: &str) -> bool {
    let mut reached: Vec<&str> = vec![base];
    loop {
        let before = reached.len();
        for e in events {
            match e {
                Event::Move {
                    locator: l,
                    from,
                    to,
                } if l == locator => {
                    let from_reached = from.iter().any(|f| reached.iter().any(|r| same(f, r)));
                    if from_reached && !reached.iter().any(|r| same(r, to)) {
                        reached.push(to);
                    }
                }
                Event::Renewed { locator: l, to } if l == locator && same(to, head) => {
                    return true;
                }
                _ => {}
            }
        }
        if reached.iter().any(|r| same(r, head)) {
            return true;
        }
        if reached.len() == before {
            return false;
        }
    }
}

fn base_has(base: &BTreeMap<Key, Option<String>>, locator: &str, hash: &str) -> bool {
    base.iter()
        .any(|((l, _), h)| l == locator && h.as_deref().is_some_and(|h| same(h, hash)))
}

/// A first pin, or the new side of a replacement whose old side the base held.
fn added_ok(
    events: &[Event],
    base: &BTreeMap<Key, Option<String>>,
    locator: &str,
    head: &str,
) -> bool {
    events.iter().any(|e| match e {
        Event::First { locator: l, to } => l == locator && same(to, head),
        Event::Replace(r) => {
            r.to_locator == locator
                && same(&r.to_hash, head)
                && base_has(base, &r.from_locator, &r.from_hash)
        }
        _ => false,
    })
}

/// A removal, or the old side of a replacement.
fn removed_ok(events: &[Event], locator: &str, base: &str) -> bool {
    events.iter().any(|e| match e {
        Event::Remove { locator: l, hash } => l == locator && same(hash, base),
        Event::Replace(r) => r.from_locator == locator && same(&r.from_hash, base),
        _ => false,
    })
}

/// One unaccounted change of evidence: the finding variant and what happened.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Problem {
    variant: &'static str,
    locator: String,
    what: String,
}

/// Every pin the head carries that is neither unchanged at its occurrence nor
/// accounted for, and every base pin that went without a row. A locator added
/// or removed without a pin is not judged.
fn unattested(
    base: &BTreeMap<Key, Option<String>>,
    head: &BTreeMap<Key, Option<String>>,
    events: &[Event],
) -> Vec<Problem> {
    let mut out = Vec::new();
    for ((locator, n), pin) in head {
        let Some(h) = pin else { continue };
        match base.get(&(locator.clone(), *n)) {
            Some(Some(b)) if b == h => {}
            Some(Some(b)) => {
                if !moved(events, locator, b, h) {
                    out.push(Problem {
                        variant: "pin-changed-unattested",
                        locator: locator.clone(),
                        what: format!(
                            "the pin on '{locator}' changed from {} to {} with no row the pinning tools write for that change",
                            short(b),
                            short(h)
                        ),
                    });
                }
            }
            _ => {
                if !added_ok(events, base, locator, h) {
                    out.push(Problem {
                        variant: "pin-added-unattested",
                        locator: locator.clone(),
                        what: format!(
                            "a pin on '{locator}' ({}) was added with no row the pinning tools write: a first pin, a new pinned locator, a changed scope and a pin moved to another file each need one",
                            short(h)
                        ),
                    });
                }
            }
        }
    }
    for ((locator, n), pin) in base {
        let Some(b) = pin else { continue };
        let now = head.get(&(locator.clone(), *n));
        if matches!(now, Some(Some(_))) || removed_ok(events, locator, b) {
            continue;
        }
        let what = match now {
            Some(_) => format!(
                "the pin on '{locator}' was removed (it was {}): un-pinning weakens the evidence",
                short(b)
            ),
            None => format!(
                "the pinned locator '{locator}' ({}) was removed with no row",
                short(b)
            ),
        };
        out.push(Problem {
            variant: "pin-removed",
            locator: locator.clone(),
            what,
        });
    }
    out
}

/// The command that writes the row a problem lacks, for its message.
fn remedy(p: &Problem, record: &str) -> String {
    let tail = format!("--actor agent --reason \"<why the claim still holds>\"` (`--actor human` when a person runs it)");
    match p.variant {
        "pin-changed-unattested" => format!(
            "put the merge base's pin back, then re-pin with `urzua fix --rebaseline --apply --ids {record} {tail}"
        ),
        "pin-added-unattested" => format!(
            "take the hand-typed `@sha256:` off '{}' and pin it with `urzua fix --pin --apply --ids {record} {tail}; to move a pin to another locator (a new scope, a renamed file), restore the old locator and run `urzua fix --pin --apply --ids {record} --replace '<old locator>=<new locator>' {tail}",
            p.locator
        ),
        _ => format!(
            "restore the pin; if the record no longer rests on it, run `urzua fix --pin --apply --ids {record} --remove '{}' {tail}, or `--replace '{}=<new locator>'` when other evidence carries the claim",
            p.locator, p.locator
        ),
    }
}

/// `diff.pin-changed-unattested`: on a `--base` run, every pin of a changed
/// record that the merge base holds (a renamed record is judged against its
/// copy under the old name) is unchanged at its occurrence or accounted for by
/// tool-written rows; a record new in the diff is not judged.
pub fn diff_pin_changed_unattested(
    records: &[Record],
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
    base_text: &HashMap<PathBuf, String>,
    changed: Option<&[PathBuf]>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_DIFF_PIN_CHANGED_UNATTESTED;
    let Some(changed) = changed else {
        return not_applicable(RULE_ID);
    };
    let changed: HashSet<&PathBuf> = changed.iter().collect();
    let mut findings = Vec::new();
    let population = census(PopulationUnit::Record, records.iter().collect(), |record| {
        if !changed.contains(&record.path) {
            return Outcome::Absent;
        }
        let (Some(now), Some(then)) = (full_text.get(&record.path), base_text.get(&record.path))
        else {
            return Outcome::Absent;
        };
        let mut before = claim_expiry::record_in_text(config, &record.record_type, then);
        delivery_items::attach_done_items_to(&mut before, config, then);
        let events = added_events(config, record, now, then);
        let shown = record.path.display().to_string();
        for p in unattested(
            &occurrences(config, &before),
            &occurrences(config, record),
            &events,
        ) {
            findings.push(Finding::new(
                RULE_ID,
                FindingSeverity::Error,
                record.path.clone(),
                ActionSource::new(p.variant),
                format!("{}; {}", p.what, remedy(&p, &shown)),
            ));
        }
        Outcome::Examined
    });
    (
        RuleExecution::ran(RULE_ID, population, Vec::new()),
        findings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaa1111111111111111111111111111111111111111111111111111";
    const B: &str = "bbbbbbbbbbbb2222222222222222222222222222222222222222222222222222";
    const C: &str = "cccccccccccc3333333333333333333333333333333333333333333333333333";

    fn occ(entries: &[(&str, Option<&str>)]) -> BTreeMap<Key, Option<String>> {
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        let mut out = BTreeMap::new();
        for (l, h) in entries {
            let n = seen.entry(l.to_string()).or_default();
            out.insert((l.to_string(), *n), h.map(str::to_string));
            *n += 1;
        }
        out
    }

    fn mv(l: &str, from: &[&str], to: &str) -> Event {
        Event::Move {
            locator: l.into(),
            from: from.iter().map(|s| s.to_string()).collect(),
            to: to.into(),
        }
    }

    fn variants(p: Vec<Problem>) -> Vec<&'static str> {
        p.into_iter().map(|p| p.variant).collect()
    }

    #[test]
    fn a_changed_pin_needs_a_chain_of_rows_with_short_or_full_hashes() {
        let (base, head) = (occ(&[("src/a.rs", Some(A))]), occ(&[("src/a.rs", Some(B))]));
        assert_eq!(
            variants(unattested(&base, &head, &[])),
            ["pin-changed-unattested"]
        );
        for ok in [
            vec![mv("src/a.rs", &[&A[..12]], &B[..12])],
            vec![mv("src/a.rs", &[A], B)],
            vec![
                mv("src/a.rs", &[&A[..12]], &C[..12]),
                mv("src/a.rs", &[&C[..12]], &B[..12]),
            ],
        ] {
            assert!(unattested(&base, &head, &ok).is_empty(), "{ok:?}");
        }
        for reused in [
            mv("src/a.rs", &[&C[..12]], &B[..12]),
            mv("src/b.rs", &[&A[..12]], &B[..12]),
            Event::Renewed {
                locator: "src/b.rs".into(),
                to: B[..12].into(),
            },
        ] {
            assert_eq!(
                unattested(&base, &head, &[reused.clone()]).len(),
                1,
                "{reused:?}"
            );
        }
        let renewed = Event::Renewed {
            locator: "src/a.rs".into(),
            to: B[..12].into(),
        };
        assert!(unattested(&base, &head, &[renewed]).is_empty());
    }

    #[test]
    fn a_hash_copied_from_one_occurrence_to_another_is_a_change() {
        let base = occ(&[("src/a.rs", Some(A)), ("src/a.rs", Some(B))]);
        let head = occ(&[("src/a.rs", Some(B)), ("src/a.rs", Some(B))]);
        assert_eq!(
            variants(unattested(&base, &head, &[])),
            ["pin-changed-unattested"]
        );
    }

    #[test]
    fn added_moved_and_removed_pins_need_their_rows() {
        let base = occ(&[("src/a.rs", Some(A)), ("src/u.rs", None)]);
        let scoped = occ(&[("src/a.rs#f", Some(B)), ("src/u.rs", None)]);
        assert_eq!(
            variants(unattested(&base, &scoped, &[])),
            ["pin-added-unattested", "pin-removed"]
        );
        let replace = Event::Replace(Replaced {
            from_locator: "src/a.rs".into(),
            from_hash: A.into(),
            to_locator: "src/a.rs#f".into(),
            to_hash: B[..12].into(),
        });
        assert!(unattested(&base, &scoped, &[replace]).is_empty());
        let first = occ(&[("src/a.rs", Some(A)), ("src/u.rs", Some(C))]);
        assert_eq!(
            variants(unattested(&base, &first, &[])),
            ["pin-added-unattested"]
        );
        let pinned = Event::First {
            locator: "src/u.rs".into(),
            to: C[..12].into(),
        };
        assert!(unattested(&base, &first, &[pinned]).is_empty());
        let unpinned = occ(&[("src/a.rs", None), ("src/u.rs", None)]);
        assert_eq!(variants(unattested(&base, &unpinned, &[])), ["pin-removed"]);
        let gone = occ(&[("src/u.rs", None)]);
        assert_eq!(variants(unattested(&base, &gone, &[])), ["pin-removed"]);
        let remove = Event::Remove {
            locator: "src/a.rs".into(),
            hash: A[..12].into(),
        };
        assert!(unattested(&base, &gone, &[remove]).is_empty());
        let plain = occ(&[("src/a.rs", Some(A)), ("src/new.rs", None)]);
        assert!(
            unattested(&base, &plain, &[]).is_empty(),
            "an unpinned locator is not judged"
        );
    }

    #[test]
    fn the_rows_the_tools_write_are_read_back() {
        let rebaseline = "2026-11-16 | Re-baselined (attested by t, identity: asserted, actor: agent): re-pinned `src/a.rs` (lines 1-4) aaaaaaaaaaaa -> bbbbbbbbbbbb. Reason: r **Why:** w | **evidence**";
        assert_eq!(
            row_events(rebaseline),
            [mv("src/a.rs", &["aaaaaaaaaaaa"], "bbbbbbbbbbbb")]
        );
        let merge = format!("2026-11-16 | {}t, identity: asserted, actor: agent): both merged sides re-pinned, `src/b.rs` (Realized-by, occurrence 2) ddd and eee -> fff; the new hash is the merged file's. | **evidence**", crate::conflicts::MERGE_ROW_PREFIX);
        assert_eq!(row_events(&merge), [mv("src/b.rs", &["ddd", "eee"], "fff")]);
        let pinned = "2026-11-16 | Pinned (attested by t, identity: asserted, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: src/a.rs@sha256:aaaaaaaaaaaa Reason: r **Why:** w | **evidence**";
        assert_eq!(
            row_events(pinned),
            [Event::First {
                locator: "src/a.rs".into(),
                to: "aaaaaaaaaaaa".into()
            }]
        );
        assert!(row_events("2026-11-16 | Re-baselined by hand | x").is_empty());
    }
}
