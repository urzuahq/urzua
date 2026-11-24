//! `diff.pin-changed-unattested`: in a record the base holds, every pin the
//! head carries is either unchanged at the same occurrence or accounted for by
//! rows the pinning tools write.

use super::*;
use crate::fix::pin::Replaced;
use crate::report::{census_records, Outcome};

/// One evidence citation: where it sits (the evidence field, or a Done-means
/// item clause), the locator as `display()` shows it (path and scope) and the
/// hash its pin holds (`None` when unpinned), in evidence order.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Pin {
    in_item: bool,
    locator: String,
    hash: Option<String>,
}

fn pins_of(text: &str, in_item: bool, out: &mut Vec<Pin>) {
    for entry in crate::locator::realized_by_locators(text) {
        let pin = match entry.parsed {
            Ok(l) => Pin {
                in_item,
                locator: l.display(),
                hash: match &l.anchor {
                    Some(crate::locator::Anchor::Pin(h)) => Some(h.clone()),
                    _ => None,
                },
            },
            Err(_) => Pin {
                in_item,
                locator: entry.raw,
                hash: None,
            },
        };
        out.push(pin);
    }
}

/// Every evidence citation of a record, field first, then the item clauses.
fn citations(config: &Config, record: &Record) -> Vec<Pin> {
    let field = relation_field_name(
        config,
        &record.record_type,
        crate::config::RelationRole::EmbodimentLocator,
    );
    let mut out = Vec::new();
    if let Some(v) = record.header.get(field) {
        pins_of(v, false, &mut out);
    }
    if let Some(text) = record
        .items
        .as_ref()
        .and_then(crate::done_items::DoneItems::locator_text)
    {
        pins_of(&text, true, &mut out);
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

fn base_has(base: &[Pin], locator: &str, hash: &str) -> bool {
    base.iter()
        .any(|p| p.locator == locator && p.hash.as_deref().is_some_and(|h| same(h, hash)))
}

/// A first pin, or the new side of a replacement whose old side the base held.
fn added_ok(events: &[Event], base: &[Pin], locator: &str, head: &str) -> bool {
    // The first pin or the replacement's new side, then any re-pins after it.
    events.iter().any(|e| match e {
        Event::First { locator: l, to } => l == locator && moved(events, locator, to, head),
        Event::Replace(r) => {
            r.to_locator == locator
                && moved(events, locator, &r.to_hash, head)
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

/// Pairs head pins with base pins: first an unused base pin at the same kind
/// of site with the same locator and hash, then one with the same locator and
/// hash anywhere (an unchanged pin is unchanged wherever it sits). Returns the
/// pinned citations left unpaired on each side, by index.
fn unpaired(base: &[Pin], head: &[Pin]) -> (Vec<usize>, Vec<usize>) {
    let mut b_used: Vec<bool> = base.iter().map(|p| p.hash.is_none()).collect();
    let mut h_left = Vec::new();
    let mut h_rest: Vec<usize> = (0..head.len())
        .filter(|&i| head[i].hash.is_some())
        .collect();
    for same_site in [true, false] {
        let mut still = Vec::new();
        for i in h_rest {
            let h = &head[i];
            let found = (0..base.len()).find(|&j| {
                !b_used[j]
                    && base[j].locator == h.locator
                    && base[j].hash == h.hash
                    && (!same_site || base[j].in_item == h.in_item)
            });
            match found {
                Some(j) => b_used[j] = true,
                None => still.push(i),
            }
        }
        h_rest = still;
    }
    h_left.extend(h_rest);
    let b_left = (0..base.len()).filter(|&j| !b_used[j]).collect();
    (b_left, h_left)
}

/// Every pin the head carries that is neither unchanged nor accounted for, and
/// every base pin that went without a row. After pairing unchanged pins, the
/// pins left at one locator are compared in order: a base pin and a head pin
/// left at the same locator are a change (so a current hash copied over a stale
/// one is reported), a head pin left alone was added and a base pin left alone
/// was removed. A base pin left alone whose locator and hash the head still
/// pins was a duplicate the head collapsed, so the evidence is unchanged. A
/// citation added or removed without a pin is not judged.
fn unattested(base: &[Pin], head: &[Pin], events: &[Event]) -> Vec<Problem> {
    let (b_left, h_left) = unpaired(base, head);
    let mut out = Vec::new();
    let mut b_pool: Vec<usize> = b_left;
    for i in h_left {
        let locator = &head[i].locator;
        let h = head[i].hash.as_deref().unwrap_or_default();
        match b_pool.iter().position(|&j| base[j].locator == *locator) {
            Some(k) => {
                let j = b_pool.remove(k);
                let b = base[j].hash.as_deref().unwrap_or_default();
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
            None => {
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
    for j in b_pool {
        let locator = &base[j].locator;
        let b = base[j].hash.as_deref().unwrap_or_default();
        if removed_ok(events, locator, b)
            || head
                .iter()
                .any(|p| p.locator == *locator && p.hash == base[j].hash)
        {
            continue;
        }
        let what = if head.iter().any(|p| p.locator == *locator) {
            format!(
                "the pin on '{locator}' was removed (it was {}): un-pinning weakens the evidence",
                short(b)
            )
        } else {
            format!(
                "the pinned locator '{locator}' ({}) was removed with no row",
                short(b)
            )
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
    let tail = "--actor agent --reason \"<why the claim still holds>\"` (`--actor human` when a person runs it)";
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

/// The `Stable-Id` the base copy carried and the head changed or dropped: a
/// record's identity does not change, and a rename that changes it would be a
/// way to look like a new record.
fn stable_id_change(before: &Record, now: &Record) -> Option<String> {
    let sid = |r: &Record| {
        r.header
            .get_reserved("Stable-Id")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let was = sid(before)?;
    match sid(now) {
        Some(is) if is == was => None,
        Some(is) => Some(format!("its Stable-Id changed from {was} to {is} against the merge base's copy (paired by record id or by git's rename detection): a record keeps its Stable-Id; put {was} back")),
        None => Some(format!("its Stable-Id {was} was removed against the merge base's copy (paired by record id or by git's rename detection): a record keeps its Stable-Id; put it back")),
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
    let (population, _) = census_records(
        PopulationUnit::Record,
        records.iter().collect(),
        |record| record.path.clone(),
        |record| {
            if !changed.contains(&record.path) {
                return Outcome::Absent;
            }
            let (Some(now), Some(then)) =
                (full_text.get(&record.path), base_text.get(&record.path))
            else {
                return Outcome::Absent;
            };
            let mut before = crate::record::Record::parse_declared(
                config,
                std::path::PathBuf::new(),
                &record.record_type,
                then,
            );
            delivery_items::attach_done_items_to(&mut before, config, then);
            let events = added_events(config, record, now, then);
            let shown = record.path.display().to_string();
            if let Some(message) = stable_id_change(&before, record) {
                findings.push(Finding::new(
                    RULE_ID,
                    FindingSeverity::Error,
                    record.path.clone(),
                    ActionSource::new("stable-id-changed"),
                    message,
                ));
            }
            for p in unattested(
                &citations(config, &before),
                &citations(config, record),
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
        },
    );
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

    fn occ(entries: &[(&str, Option<&str>)]) -> Vec<Pin> {
        entries
            .iter()
            .map(|(l, h)| Pin {
                in_item: false,
                locator: l.to_string(),
                hash: h.map(str::to_string),
            })
            .collect()
    }

    #[test]
    fn a_first_pin_re_pinned_later_in_the_same_change_is_accounted_for() {
        let base = occ(&[("src/u.rs", None)]);
        let head = occ(&[("src/u.rs", Some(B))]);
        let events = [
            Event::First {
                locator: "src/u.rs".into(),
                to: A[..12].into(),
            },
            mv("src/u.rs", &[&A[..12]], &B[..12]),
        ];
        assert!(unattested(&base, &head, &events).is_empty());
        assert_eq!(unattested(&base, &head, &events[..1]).len(), 1);
    }

    #[test]
    fn a_pin_moved_between_the_field_and_an_item_or_reordered_is_unchanged() {
        let base = occ(&[("src/a.rs", Some(A)), ("src/a.rs", Some(B))]);
        let mut head = occ(&[
            ("src/a.rs", None),
            ("src/a.rs", Some(B)),
            ("src/a.rs", Some(A)),
        ]);
        head[2].in_item = true;
        assert!(unattested(&base, &head, &[]).is_empty());
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
    fn a_collapsed_duplicate_pin_is_not_a_removal_observed_failing() {
        let base = occ(&[("src/a.rs", Some(A)), ("src/a.rs", Some(A))]);
        let collapsed = occ(&[("src/a.rs", Some(A))]);
        assert!(unattested(&base, &collapsed, &[]).is_empty());
        let mut moved_site = collapsed.clone();
        moved_site[0].in_item = true;
        assert!(unattested(&base, &moved_site, &[]).is_empty());
        assert_eq!(
            variants(unattested(&base, &occ(&[("src/u.rs", None)]), &[])),
            ["pin-removed", "pin-removed"]
        );
        assert_eq!(
            variants(unattested(&base, &occ(&[("src/a.rs", Some(B))]), &[])),
            ["pin-changed-unattested", "pin-removed"]
        );
        assert_eq!(
            variants(unattested(&base, &occ(&[("src/a.rs#f", Some(A))]), &[])),
            ["pin-added-unattested", "pin-removed", "pin-removed"]
        );
        let two = occ(&[("src/a.rs", Some(A)), ("src/a.rs", Some(B))]);
        assert_eq!(variants(unattested(&two, &collapsed, &[])), ["pin-removed"]);
        assert_eq!(
            variants(unattested(&collapsed, &base, &[])),
            ["pin-added-unattested"]
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
