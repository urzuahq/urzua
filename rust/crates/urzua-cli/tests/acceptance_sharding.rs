//! Tests of the acceptance suite's sharding and merge (see
//! `acceptance/sharding.rs`).

#[path = "acceptance/sharding.rs"]
mod sharding;

use sharding::{decode, encode, merge, owns, parse_shard, Shard, ShardReport};
use std::collections::BTreeMap;

fn report(i: usize, n: usize, discovered: usize, ran: usize, p: &[(&str, bool)]) -> ShardReport {
    ShardReport {
        shard: Shard { index: i, count: n },
        discovered,
        ran,
        proven: p.iter().map(|(r, d)| (r.to_string(), *d)).collect(),
    }
}

#[test]
fn a_shard_spec_is_one_based_and_bounded() {
    assert_eq!(parse_shard("2/4").unwrap(), Shard { index: 2, count: 4 });
    for bad in ["0/4", "5/4", "1", "a/b", "1/0", "", "1/2/3"] {
        assert!(parse_shard(bad).is_err(), "{bad:?} must be refused");
    }
}

#[test]
fn every_case_belongs_to_exactly_one_shard() {
    for n in 1..=5 {
        for position in 0..40 {
            let owners = (1..=n)
                .filter(|i| {
                    owns(
                        Shard {
                            index: *i,
                            count: n,
                        },
                        position,
                    )
                })
                .count();
            assert_eq!(owners, 1, "position {position} of {n} shards");
        }
    }
}

#[test]
fn a_report_survives_the_round_trip() {
    let r = report(2, 3, 9, 3, &[("a.b", true), ("c.d", false)]);
    let back = decode(&encode(&r)).unwrap();
    assert_eq!(back.shard, r.shard);
    assert_eq!((back.discovered, back.ran), (9, 3));
    assert_eq!(back.proven, r.proven);
    assert!(decode("garbage").is_err());
    assert!(decode("shard 1/2 discovered 1 ran 1\nrule\tmaybe\n").is_err());
}

#[test]
fn a_whole_set_merges_and_a_detecting_case_anywhere_wins() {
    let merged = merge(&[
        report(1, 2, 4, 2, &[("a", false), ("b", false)]),
        report(2, 2, 4, 2, &[("a", true)]),
    ])
    .unwrap();
    assert_eq!(
        merged,
        BTreeMap::from([("a".to_string(), true), ("b".to_string(), false)])
    );
}

#[test]
fn an_incomplete_or_inconsistent_set_never_merges() {
    let ok = |i| report(i, 3, 6, 2, &[]);
    assert!(merge(&[]).unwrap_err().contains("no shard"));
    assert!(merge(&[ok(1), ok(2)])
        .unwrap_err()
        .contains("3/3 is missing"));
    assert!(merge(&[ok(1), ok(2), ok(2)])
        .unwrap_err()
        .contains("more than once"));
    assert!(merge(&[ok(1), ok(2), report(3, 3, 7, 2, &[])])
        .unwrap_err()
        .contains("disagree on the cases"));
    assert!(merge(&[ok(1), ok(2), report(3, 4, 6, 2, &[])])
        .unwrap_err()
        .contains("shard count"));
    assert!(merge(&[ok(1), ok(2), report(3, 3, 6, 1, &[])])
        .unwrap_err()
        .contains("ran 5 cases"));
}
