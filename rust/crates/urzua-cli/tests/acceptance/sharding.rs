//! Splitting the acceptance suite across CI jobs without losing its
//! suite-wide coverage gate.
//!
//! The runner is one process, so a test partitioner cannot split it. Instead a
//! run may take shard `i/n` (`URZUA_ACCEPTANCE_SHARD`): the cases, in discovery
//! order, are dealt out round-robin and the shard runs its own. A shard cannot
//! judge the coverage gate (a rule's failing case may live in another shard),
//! so it writes what it proved to a file instead, and a merge run
//! (`URZUA_ACCEPTANCE_MERGE=<dir>`) reads every shard's file, refuses a set that
//! is not exactly shards 1..n of one discovery, and judges the gate over the
//! union. Missing, duplicated or stale shard files fail the merge, so a shard
//! that never ran cannot read as a pass.
//!
//! Shared by the `acceptance` runner and tested in `acceptance_sharding.rs`.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shard {
    /// 1-based.
    pub index: usize,
    pub count: usize,
}

/// Parse `i/n` with `1 <= i <= n`.
pub fn parse_shard(text: &str) -> Result<Shard, String> {
    let bad = || format!("shard {text:?} is not `i/n` with 1 <= i <= n");
    let (i, n) = text.trim().split_once('/').ok_or_else(bad)?;
    let index: usize = i.parse().map_err(|_| bad())?;
    let count: usize = n.parse().map_err(|_| bad())?;
    if index == 0 || index > count {
        return Err(bad());
    }
    Ok(Shard { index, count })
}

/// Whether the case at `position` (discovery order) belongs to `shard`.
pub fn owns(shard: Shard, position: usize) -> bool {
    position % shard.count == shard.index - 1
}

/// What one shard proved: the rules whose mutation proof held, `true` when a
/// case also expects the rule's finding.
pub struct ShardReport {
    pub shard: Shard,
    /// Cases discovered in the whole suite, before sharding.
    pub discovered: usize,
    /// Cases this shard ran.
    pub ran: usize,
    pub proven: BTreeMap<String, bool>,
}

pub fn encode(report: &ShardReport) -> String {
    let mut out = format!(
        "shard {}/{} discovered {} ran {}\n",
        report.shard.index, report.shard.count, report.discovered, report.ran
    );
    for (rule, detects) in &report.proven {
        out.push_str(&format!(
            "{rule}\t{}\n",
            if *detects { "detects" } else { "ran-only" }
        ));
    }
    out
}

pub fn decode(text: &str) -> Result<ShardReport, String> {
    let mut lines = text.lines();
    let header = lines.next().ok_or("empty shard file")?;
    let words: Vec<&str> = header.split_whitespace().collect();
    let ["shard", spec, "discovered", d, "ran", r] = words.as_slice() else {
        return Err(format!("unreadable shard header {header:?}"));
    };
    let num = |s: &str| {
        s.parse::<usize>()
            .map_err(|_| format!("unreadable shard header {header:?}"))
    };
    let mut proven = BTreeMap::new();
    for line in lines.filter(|l| !l.trim().is_empty()) {
        let (rule, tier) = line
            .split_once('\t')
            .ok_or_else(|| format!("unreadable shard line {line:?}"))?;
        let detects = match tier {
            "detects" => true,
            "ran-only" => false,
            other => return Err(format!("unknown tier {other:?} for {rule}")),
        };
        proven.insert(rule.to_string(), detects);
    }
    Ok(ShardReport {
        shard: parse_shard(spec)?,
        discovered: num(d)?,
        ran: num(r)?,
        proven,
    })
}

/// Combine every shard's report into the suite-wide proof, or say why the set
/// is not a whole run. A rule with a failing case in any shard stays detected.
pub fn merge(reports: &[ShardReport]) -> Result<BTreeMap<String, bool>, String> {
    let first = reports.first().ok_or("no shard files to merge")?;
    let count = first.shard.count;
    let mut seen = vec![false; count];
    let mut ran = 0;
    for r in reports {
        if r.shard.count != count {
            return Err(format!(
                "shard files disagree on the shard count: {count} and {}",
                r.shard.count
            ));
        }
        if r.discovered != first.discovered {
            return Err(format!(
                "shard files disagree on the cases discovered: {} and {}",
                first.discovered, r.discovered
            ));
        }
        if std::mem::replace(&mut seen[r.shard.index - 1], true) {
            return Err(format!(
                "shard {}/{count} appears more than once",
                r.shard.index
            ));
        }
        ran += r.ran;
    }
    if let Some(missing) = seen.iter().position(|s| !s) {
        return Err(format!("shard {}/{count} is missing", missing + 1));
    }
    if ran != first.discovered {
        return Err(format!(
            "the shards ran {ran} cases but {} were discovered",
            first.discovered
        ));
    }
    let mut merged: BTreeMap<String, bool> = BTreeMap::new();
    for r in reports {
        for (rule, detects) in &r.proven {
            let e = merged.entry(rule.clone()).or_insert(*detects);
            *e = *e || *detects;
        }
    }
    Ok(merged)
}
