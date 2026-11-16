//! `diff.pin-changed-unattested`: a pin a pull request changed carries the row
//! the tool writes when it changes a pin.

use super::*;
use crate::fix::RowPin;
use crate::report::{census, Outcome};
use std::collections::BTreeMap;

/// Each locator's pin hashes in a record, by the locator as `display()` shows
/// it (path and scope); an unpinned locator has an empty list.
fn pins_by_locator(config: &Config, record: &Record) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (locator, pin) in claim_expiry::current_locator_pins(config, record) {
        let hashes = out.entry(locator).or_default();
        if let Some(h) = pin {
            if !hashes.contains(&h) {
                hashes.push(h);
            }
        }
    }
    out
}

/// The re-pins the rows the head's log gained since the base name: re-baseline
/// and merge rows with their old and new hash, renewal rows with the hash each
/// locator was renewed at (no old hash: a renewal re-hashes every locator).
/// Parsed by the functions that sit beside the writers of those rows.
fn added_row_pins(config: &Config, record: &Record, now: &str, then: &str) -> Vec<RowPin> {
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
        if let Some(pins) = crate::fix::rebaseline_row_pins(&row) {
            out.extend(pins);
        } else if let Some(pins) = crate::conflicts::merge_row_pins(&row) {
            out.extend(pins);
        } else if let Some(renewed) = crate::reverify::parse_reverified_row(&row) {
            out.extend(renewed.pins.into_iter().map(|(locator, to)| RowPin {
                locator,
                from: Vec::new(),
                to,
            }));
        }
    }
    out
}

fn short(hash: &str) -> &str {
    hash.get(..12).unwrap_or(hash)
}

/// Whether a row accounts for `locator` moving from one of `base` to `head`:
/// the row's hashes equal the base's and the head's exactly, so a row is not
/// reused for another change. A renewal row carries no old hash and accounts
/// for the hash it renewed at.
fn accounted(rows: &[RowPin], locator: &str, base: &[String], head: &str) -> bool {
    rows.iter().any(|r| {
        r.locator == locator
            && r.to == short(head)
            && (r.from.is_empty() || r.from.iter().any(|f| base.iter().any(|b| f == short(b))))
    })
}

/// What a record's pins did between the base and the head that no added row
/// accounts for: one message per locator.
fn unattested(
    base: &BTreeMap<String, Vec<String>>,
    head: &BTreeMap<String, Vec<String>>,
    rows: &[RowPin],
) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    for (locator, then) in base.iter().filter(|(_, h)| !h.is_empty()) {
        let Some(now) = head.get(locator) else {
            continue;
        };
        let was = then.iter().map(|h| short(h)).collect::<Vec<_>>().join(", ");
        if now.is_empty() {
            out.push((
                "pin-removed",
                format!(
                    "the pin on '{locator}' was removed (it was {was}): un-pinning weakens the evidence; restore the pin, or remove the locator if the record no longer claims it"
                ),
            ));
            continue;
        }
        for h in now.iter().filter(|h| !then.contains(h)) {
            if !accounted(rows, locator, then, h) {
                out.push((
                    "pin-changed-unattested",
                    format!(
                        "the pin on '{locator}' changed from {was} to {} with no revision-log row the re-pin tooling writes for that change",
                        short(h)
                    ),
                ));
            }
        }
    }
    out
}

/// `diff.pin-changed-unattested`: on a `--base` run, a changed record whose
/// evidence pin for a locator differs from the merge base's copy, when the head
/// gained no tool-written row for that change (`urzua fix --rebaseline`,
/// `urzua reverify`, `urzua fix --conflicts`). A pin added to a locator that had
/// none, a locator added or removed, a scope change (another locator) and a
/// record new in the diff are not reported; a pin removed from a locator that
/// stays is.
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
        let rows = added_row_pins(config, record, now, then);
        let problems = unattested(
            &pins_by_locator(config, &before),
            &pins_by_locator(config, record),
            &rows,
        );
        for (variant, message) in problems {
            findings.push(Finding::new(
                RULE_ID,
                FindingSeverity::Error,
                record.path.clone(),
                ActionSource::new(variant),
                format!(
                    "{message}; put the merge base's pin back, then re-pin with `urzua fix --rebaseline --apply --ids {} --actor agent --reason \"<why the claim still holds>\"` (`--actor human` when a person runs it), which writes the row",
                    record.path.display()
                ),
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

    fn map(entries: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        entries
            .iter()
            .map(|(l, h)| (l.to_string(), h.iter().map(|s| s.to_string()).collect()))
            .collect()
    }

    const A: &str = "aaaaaaaaaaaa1111";
    const B: &str = "bbbbbbbbbbbb2222";
    const C: &str = "cccccccccccc3333";

    fn row(locator: &str, from: &[&str], to: &str) -> RowPin {
        RowPin {
            locator: locator.into(),
            from: from.iter().map(|s| s.to_string()).collect(),
            to: to.into(),
        }
    }

    #[test]
    fn a_hand_changed_pin_is_reported_and_an_exact_row_accounts_for_it() {
        let (base, head) = (map(&[("src/a.rs", &[A])]), map(&[("src/a.rs", &[B])]));
        let found = unattested(&base, &head, &[]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "pin-changed-unattested");
        assert!(found[0].1.contains("src/a.rs") && found[0].1.contains("aaaaaaaaaaaa"));
        assert!(unattested(
            &base,
            &head,
            &[row("src/a.rs", &["aaaaaaaaaaaa"], "bbbbbbbbbbbb")]
        )
        .is_empty());
        assert!(unattested(
            &base,
            &head,
            &[row(
                "src/a.rs",
                &["cccccccccccc", "aaaaaaaaaaaa"],
                "bbbbbbbbbbbb"
            )]
        )
        .is_empty());
        assert!(unattested(&base, &head, &[row("src/a.rs", &[], "bbbbbbbbbbbb")]).is_empty());
        for reused in [
            row("src/a.rs", &["cccccccccccc"], "bbbbbbbbbbbb"),
            row("src/a.rs", &["aaaaaaaaaaaa"], "cccccccccccc"),
            row("src/b.rs", &["aaaaaaaaaaaa"], "bbbbbbbbbbbb"),
            row("src/a.rs", &[], "cccccccccccc"),
        ] {
            assert_eq!(
                unattested(&base, &head, &[reused.clone()]).len(),
                1,
                "{reused:?}"
            );
        }
    }

    #[test]
    fn edge_cases_follow_the_spec() {
        let none: &[&str] = &[];
        let base = map(&[
            ("src/a.rs", &[A]),
            ("src/gone.rs", &[A]),
            ("src/bare.rs", none),
        ]);
        let head = map(&[
            ("src/a.rs", none),
            ("src/new.rs", &[C]),
            ("src/bare.rs", &[C]),
            ("src/a.rs#f", &[B]),
        ]);
        let found = unattested(&base, &head, &[]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].0, "pin-removed");
    }

    #[test]
    fn the_rows_the_tools_write_are_read_back() {
        let rebaseline = "2026-11-16 | Re-baselined (attested by t, identity: asserted, actor: agent): re-pinned `src/a.rs` (lines 1-4) aaaaaaaaaaaa -> bbbbbbbbbbbb, `src/b.rs` cccccccccccc -> dddddddddddd. Reason: r **Why:** w | **evidence**";
        assert_eq!(
            crate::fix::rebaseline_row_pins(rebaseline).unwrap(),
            vec![
                row("src/a.rs", &["aaaaaaaaaaaa"], "bbbbbbbbbbbb"),
                row("src/b.rs", &["cccccccccccc"], "dddddddddddd")
            ]
        );
        let merge = format!("2026-11-16 | {}t, identity: asserted, actor: agent): both merged sides re-pinned, `src/a.rs` aaa and bbb -> ccc; the new hash is the merged file's. | **evidence**", crate::conflicts::MERGE_ROW_PREFIX);
        assert_eq!(
            crate::conflicts::merge_row_pins(&merge).unwrap(),
            vec![row("src/a.rs", &["aaa", "bbb"], "ccc")]
        );
        assert!(crate::fix::rebaseline_row_pins("2026-11-16 | Re-baselined by hand | x").is_none());
    }
}
