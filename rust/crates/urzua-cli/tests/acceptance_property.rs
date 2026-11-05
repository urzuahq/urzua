//! SPEC-4's generative property (MILE-170): no input is ever silently ignored.
//!
//! For any generated field name, body wrapping or relation value, the checker's
//! output is either a recognised-and-accepted result or a report. Never neither.
//!
//! The generator is a fixed-seed PRNG, so a run is reproducible and a failure
//! names the input that caused it. Each property is one throwaway git
//! repository checked in one `urzua check`, which keeps the cost to a few runs.
//!
//! A property that has never been seen failing proves nothing, so each one also
//! runs with the rule it depends on switched off (or, for the invariance
//! property, against an input that really does change the verdict) and must then
//! report violations.

mod common;

use serde_json::Value;
use std::process::Command;

/// xorshift64*: deterministic, no dependency, good enough to vary inputs.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }
}

struct Repo {
    dir: common::TempDir,
}

impl Repo {
    fn new(name: &str, config: &str, files: &[(String, String)]) -> Repo {
        let dir = common::TempDir::new(&format!("urzua-property-{name}"));
        std::fs::create_dir_all(dir.join(".urzua")).unwrap();
        std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
        for (path, text) in files {
            let to = dir.join(path);
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::write(to, text).unwrap();
        }
        for args in [
            &["init", "-q"][..],
            &["config", "user.email", "property@example.com"],
            &["config", "user.name", "property"],
            &["add", "-A"],
            &["commit", "-q", "-m", "fixture"],
        ] {
            let ok = Command::new("git")
                .args(args)
                .current_dir(&dir)
                .status()
                .unwrap()
                .success();
            assert!(ok, "git {args:?} failed");
        }
        Repo { dir }
    }

    fn check(&self) -> Value {
        let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
            .arg("check")
            .current_dir(&self.dir)
            .output()
            .unwrap();
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
            panic!(
                "check printed no JSON ({e}): {}",
                String::from_utf8_lossy(&out.stdout)
            )
        })
    }
}

fn findings(report: &Value) -> Vec<&Value> {
    report["findings"]
        .as_array()
        .map(|f| f.iter().collect())
        .unwrap_or_default()
}

fn field<'a>(finding: &'a Value, key: &str) -> &'a str {
    finding[key].as_str().unwrap_or("")
}

/// A config for one `rec` type with the given rules, each switched on unless
/// it is `off`.
fn config(extra_known: &str, pointer: &str, rules: &[&str], off: Option<&str>) -> String {
    let mut text = format!(
        "schema_version: 2\nrecord_types:\n  rec:\n    dir: records\n    prefix: REC\n    \
         required_fields: [Status]\n    known_fields: [Date{extra_known}]\n    \
         header_shape: yaml-frontmatter\n    pointer_fields: [{pointer}]\n    \
         narrative_fields: []\nrules:\n"
    );
    for r in rules {
        let level = if Some(*r) == off { "off" } else { "error" };
        text.push_str(&format!("  {r}: {level}\n"));
    }
    text
}

fn record(n: usize, header: &str, body: &str) -> (String, String) {
    (
        format!("records/REC-{n:04}-x.md"),
        format!("---\nStatus: Proposed\n{header}---\n# REC-{n} -- x\n\n{body}"),
    )
}

// ---------------------------------------------------------------- field names

const NAME_ALPHABET: &[&str] = &[
    "a", "b", "z", "Q", "X", "0", "7", "_", "-", ".", "/", " ", "#", "*", "&", "!", "?", ":", "'",
    "\"", "\\", "@", "%", "|", ">", "{", "[", "~", "$", "^", "(", ")", "=", "+", ",", ";",
    "e\u{301}", "\u{e9}", "\u{df}", "\u{65e5}", "\u{3a9}", "\u{44f}",
];

fn generated_name(rng: &mut Rng) -> String {
    loop {
        let len = 1 + rng.below(10);
        let name: String = (0..len).map(|_| rng.pick(NAME_ALPHABET)).collect();
        // A name with edge whitespace is a different property (the untrimmed
        // value rule); the declared names are exercised separately.
        if name.trim() == name && name != "Status" && name != "Date" {
            return name;
        }
    }
}

fn yaml_key(name: &str) -> String {
    format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Each record carries one generated header field. It must be accepted as
/// declared (no finding about it) or reported as undeclared, by name.
fn field_name_violations(off: Option<&str>) -> Vec<String> {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut files = Vec::new();
    let mut names = Vec::new();
    for n in 1..=80 {
        let declared = n % 10 == 0;
        let name = if declared {
            "Date".to_string()
        } else {
            generated_name(&mut rng)
        };
        files.push(record(n, &format!("{}: x\n", yaml_key(&name)), ""));
        names.push((files[n - 1].0.clone(), name, declared));
    }
    let cfg = config(
        "",
        "",
        &["header.required-fields", "header.field-set-consistency"],
        off,
    );
    let report = Repo::new("field-names", &cfg, &files).check();
    let all = findings(&report);
    let mut violations = Vec::new();
    for (path, name, declared) in names {
        let about: Vec<&&Value> = all
            .iter()
            .filter(|f| {
                field(f, "rule") == "header.field-set-consistency"
                    && field(f, "file") == path
                    && field(f, "message").contains(&format!("'{name}'"))
            })
            .collect();
        let reported_otherwise = all
            .iter()
            .any(|f| field(f, "file") == path && field(f, "rule") == "header.required-fields");
        if declared && !about.is_empty() {
            violations.push(format!(
                "{path}: declared field {name:?} was reported as undeclared"
            ));
        }
        if !declared && about.is_empty() && !reported_otherwise {
            violations.push(format!(
                "{path}: field {name:?} was neither accepted as declared nor reported"
            ));
        }
    }
    violations
}

#[test]
fn a_generated_field_name_is_accepted_as_declared_or_reported() {
    let clean = field_name_violations(None);
    assert!(clean.is_empty(), "{}", clean.join("\n"));
    // Mutation: with the rule off, an undeclared name produces no output at all,
    // which is exactly the failure the property exists to catch.
    let mutated = field_name_violations(Some("header.field-set-consistency"));
    assert!(
        !mutated.is_empty(),
        "the property did not fail with header.field-set-consistency off, so it proves nothing"
    );
}

// ------------------------------------------------------------ body invariance

/// A fake header, as a body would carry it inside a code sample.
const FAKE: &str = "---\nStatus: Bogus\nMood: grim\nImplements: REC-7777\n---\n";

fn wrappings() -> Vec<(&'static str, String)> {
    let indent = |s: &str, pad: &str| s.lines().map(|l| format!("{pad}{l}\n")).collect::<String>();
    vec![
        ("none", String::new()),
        ("backtick fence", format!("```\n{FAKE}```\n")),
        ("tilde fence", format!("~~~\n{FAKE}~~~\n")),
        ("four-backtick fence", format!("````yaml\n{FAKE}````\n")),
        ("long tilde fence", format!("~~~~~~\n{FAKE}~~~~~~\n")),
        (
            "indented fence",
            format!("   ```\n{}   ```\n", indent(FAKE, "   ")),
        ),
        (
            "fence nested in a fence",
            format!("~~~~\n```\n{FAKE}```\n~~~~\n"),
        ),
        ("indented code block", format!("{}\n", indent(FAKE, "    "))),
        ("html comment", format!("<!--\n{FAKE}-->\n")),
        ("block quote", indent(FAKE, "> ")),
        (
            "fence in a list item",
            format!("- an example:\n\n  ```\n{}  ```\n", indent(FAKE, "  ")),
        ),
    ]
}

/// (archetype name, extra header lines): a clean record and one with an
/// undeclared field.
fn archetypes() -> Vec<(&'static str, &'static str)> {
    vec![("clean", ""), ("undeclared", "Flavor: mint\n")]
}

type Verdict = std::collections::BTreeSet<(String, String)>;

fn verdicts(report: &Value, files: &[(usize, String)]) -> Vec<Verdict> {
    files
        .iter()
        .map(|(_, path)| {
            findings(report)
                .into_iter()
                .filter(|f| field(f, "file") == path)
                .map(|f| {
                    (
                        field(f, "rule").to_string(),
                        field(f, "message").to_string(),
                    )
                })
                .collect()
        })
        .collect()
}

/// For each archetype, every wrapping of fake-header content in the body must
/// leave the verdict identical to the unwrapped record's. `control` swaps one
/// wrapping for a real change (the true header hidden inside a fence), which
/// the comparison must notice.
fn invariance_violations(control: bool) -> Vec<String> {
    let mut files = Vec::new();
    let mut index = Vec::new();
    let mut n = 0;
    for (arch, header) in archetypes() {
        for (wrap, text) in wrappings() {
            n += 1;
            let (path, mut content) = record(n, header, &format!("## Context\n\n{text}\nafter\n"));
            if control && wrap == "backtick fence" {
                // The real header now sits inside a fence: the verdict must change.
                content = format!(
                    "```\n{}```\n# REC-{n} -- x\n",
                    &content[..content.find("---\n#").unwrap() + 4]
                );
            }
            files.push((path.clone(), content));
            index.push((n, path, arch, wrap));
        }
    }
    let cfg = config(
        "",
        "",
        &[
            "header.required-fields",
            "header.field-set-consistency",
            "field.quality",
        ],
        None,
    );
    let report = Repo::new(
        if control {
            "invariance-control"
        } else {
            "invariance"
        },
        &cfg,
        &files,
    )
    .check();
    let per_file: Vec<(usize, String)> = index.iter().map(|(n, p, _, _)| (*n, p.clone())).collect();
    let verdicts = verdicts(&report, &per_file);
    let mut violations = Vec::new();
    for (arch, _) in archetypes() {
        let base = index
            .iter()
            .position(|(_, _, a, w)| *a == arch && *w == "none")
            .unwrap();
        for (i, (_, path, a, w)) in index.iter().enumerate() {
            if *a == arch && verdicts[i] != verdicts[base] {
                violations.push(format!(
                    "{path}: wrapping the body in a {w} changed the {arch} verdict: {:?} vs {:?}",
                    verdicts[i], verdicts[base]
                ));
            }
        }
    }
    violations
}

#[test]
fn wrapping_the_body_in_any_fence_or_comment_leaves_the_verdict_unchanged() {
    let clean = invariance_violations(false);
    assert!(clean.is_empty(), "{}", clean.join("\n"));
    // Mutation: an input that does change the verdict must be seen to.
    assert!(
        !invariance_violations(true).is_empty(),
        "the comparison did not notice a real change, so it proves nothing"
    );
}

// ------------------------------------------------------------ relation values

const ANNOTATIONS: &[&str] = &[
    "(see REC-8001, REC-8002)",
    "(cf. REC-8003; REC-8004)",
    "[REC-8005, REC-8006] and more",
    "(REC-8007)",
    "- REC-8008 explains it",
    "(parent: REC-8009)",
    "(see REC-8010 and REC-8011 for the argument)",
];

/// Generated Implements values: each entry is a reference that begins it,
/// optionally followed by an annotation that itself cites references. The
/// claims must be the entries' leading references; no annotation reference may
/// appear. A value YAML cannot read must be reported instead.
fn relation_violations(off: Option<&str>) -> Vec<String> {
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    let mut files = vec![record(1, "", "")];
    let mut expected = Vec::new();
    for n in 2..=40 {
        let mut entries = Vec::new();
        let mut leads = Vec::new();
        for _ in 0..1 + rng.below(3) {
            let lead = if rng.below(3) == 0 {
                "REC-0001".to_string()
            } else {
                format!("REC-{}", 9000 + rng.below(900))
            };
            let mut entry = lead.clone();
            if rng.below(2) == 0 {
                entry.push(' ');
                let annotation = rng.pick(ANNOTATIONS);
                entry.push_str(annotation);
            }
            if lead != "REC-0001" {
                leads.push(lead);
            }
            entries.push(entry);
        }
        let value = entries.join(", ");
        // Quoted, so that a ": " or "#" in an annotation is data, not YAML syntax.
        let header = format!("Implements: \"{}\"\n", value.replace('"', "\\\""));
        let (path, text) = record(n, &header, "");
        expected.push((path.clone(), leads));
        files.push((path, text));
    }
    let cfg = config(
        ", Implements",
        "Implements",
        &["header.required-fields", "pointer.resolution"],
        off,
    );
    let report = Repo::new("relations", &cfg, &files).check();
    let all = findings(&report);
    let mut violations = Vec::new();
    for f in &all {
        if field(f, "rule") == "pointer.resolution" && field(f, "message").contains("REC-8") {
            violations.push(format!(
                "{}: an annotation reference was read as a claim: {}",
                field(f, "file"),
                field(f, "message")
            ));
        }
    }
    for (path, leads) in expected {
        let reported_otherwise = all
            .iter()
            .any(|f| field(f, "file") == path && field(f, "rule") == "header.required-fields");
        for lead in leads {
            let found = all.iter().any(|f| {
                field(f, "rule") == "pointer.resolution"
                    && field(f, "file") == path
                    && field(f, "message").contains(&format!("{lead} "))
            });
            if !found && !reported_otherwise {
                violations.push(format!(
                    "{path}: claim {lead} produced no finding and no report"
                ));
            }
        }
    }
    violations
}

#[test]
fn relation_claims_round_trip_to_the_references_that_begin_each_entry() {
    let clean = relation_violations(None);
    assert!(clean.is_empty(), "{}", clean.join("\n"));
    let mutated = relation_violations(Some("pointer.resolution"));
    assert!(
        !mutated.is_empty(),
        "the property did not fail with pointer.resolution off, so it proves nothing"
    );
}
