//! `urzua explain --types [--full]` and `urzua explain --type <name>`: what the configuration declares for its record types, read
//! from the configuration file alone. Every entry read from it carries `at`,
//! its key path and line (`path:line`). Four entries the configuration does
//! not declare are printed, marked `"default": true` with a `source` and no
//! `at`: an undeclared initial state, a rule on by declaration, an undeclared
//! identifier prefix, and the conventional template file when it exists.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Serialize;
use serde_json::{json, Map, Value};
use urzua_core::config::{Config, RecordTypeConfig, RuleLevel};
use urzua_core::report::{ActionInput, Failure, NextAction, Notice, Report, ReportStatus};
use urzua_core::rules::{
    effective_level, Declaration, RuleDef, Scope, DEFAULT_ON_RULES, RULE_EMBODIMENT_CLAIM_EXPIRED,
    RULE_POINTER_TARGET_TYPE, RULE_STATUS_FORBIDDEN_FIELD_PRESENT,
    RULE_STATUS_REQUIRED_FIELD_MISSING, RULE_TABLE,
};
use urzua_core::status_model::{StatusDecl, StatusFieldRule};

use super::config_lines::{index, key, ConfigLines};
use crate::emit;
use crate::session::{emit_or_not_run, Workspace};

/// Per-type keys with a rendering of their own; every other declared key is
/// printed raw under its own name, so no declaration can go unreported.
const TYPE_RENDERED: &[&str] = &[
    "dir",
    "extra_dirs",
    "prefix",
    "required_fields",
    "known_fields",
    "field_values",
    "pointer_fields",
    "narrative_fields",
    "relation_fields",
    "statuses",
];

/// `statuses` keys with a rendering of their own; the rest go under `states` raw.
const STATUS_RENDERED: &[&str] = &[
    "values",
    "initial",
    "success",
    "failure",
    "implementable",
    "claim_retained",
    "blocked",
    "progress",
    "transitions",
    "require",
    "forbid",
];

const REPOSITORY_EXCLUDED: &[&str] = &["record_types", "rules"];

pub enum Mode {
    Summary,
    Full,
    One(String),
}

#[derive(Serialize)]
pub(crate) struct ExplainTypesReport {
    status: ReportStatus,
    config: String,
    #[serde(flatten)]
    body: Map<String, Value>,
    #[serde(skip)]
    next: Option<NextAction>,
}

impl Report for ExplainTypesReport {
    fn notices(&self) -> &[Notice] {
        &[]
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::SUCCESS
    }
    fn next_action(&self) -> Option<NextAction> {
        self.next.clone()
    }
}

pub fn run(config_path: Option<PathBuf>, mode: Mode) -> ExitCode {
    emit_or_not_run(build(config_path, mode).map(|r| emit(&r)))
}

fn detail_action() -> NextAction {
    NextAction {
        command: "urzua explain --type <name>".to_string(),
        reason: "one type's full declaration".to_string(),
        inputs: vec![ActionInput {
            placeholder: "<name>".to_string(),
            flag: "--type".to_string(),
            means: "a `name` from `types`".to_string(),
        }],
    }
}

pub(crate) fn build(
    config_path: Option<PathBuf>,
    mode: Mode,
) -> Result<ExplainTypesReport, Failure> {
    let only = match &mode {
        Mode::One(n) => Some(n.clone()),
        _ => None,
    };
    let ws = Workspace::open(&[], config_path)?;
    let text = urzua_io::read_to_string(&ws.config_path)
        .map_err(|e| format!("could not read {}: {e}", ws.config_path.display()))?;
    let lines = ConfigLines::scan(urzua_core::config::strip_bom(&text));
    let names: Vec<&String> = ws.config.record_types.keys().collect();
    if let Some(name) = &only {
        if !ws.config.record_types.contains_key(name) {
            let declared: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            return Err(if declared.is_empty() {
                format!("'{name}' is not a declared record type; the configuration declares no record type")
            } else {
                format!(
                    "'{name}' is not a declared record type; the configuration declares: {}",
                    declared.join(", ")
                )
            }
            .into());
        }
    }
    let raw: yaml_serde::Value = yaml_serde::from_str(urzua_core::config::strip_bom(&text))
        .map_err(|e| format!("could not read {}: {e}", ws.config_path.display()))?;
    let ctx = Ctx {
        config: &ws.config,
        lines: &lines,
        raw: &raw,
        repo_root: &ws.repo_root,
        only: only.as_deref(),
    };
    let selected = ws
        .config
        .record_types
        .iter()
        .filter(|(n, _)| only.as_ref().is_none_or(|o| o == *n));
    let next = matches!(mode, Mode::Summary).then(detail_action);
    let body = match mode {
        Mode::Summary => obj(vec![
            ("types", selected.map(|(n, t)| ctx.summary(n, t)).collect()),
            ("rules", ctx.rules_summary()),
        ]),
        Mode::Full => obj(vec![
            (
                "types",
                selected.map(|(n, t)| ctx.record_type(n, t)).collect(),
            ),
            ("rules", ctx.rules()),
            ("repository", ctx.repository()),
        ]),
        Mode::One(_) => obj(vec![
            (
                "types",
                selected.map(|(n, t)| ctx.record_type(n, t)).collect(),
            ),
            ("rules", ctx.rules()),
        ]),
    };
    let mut body = body;
    if let Value::Object(m) = &mut body {
        m.entry("types").or_insert_with(|| json!([]));
    }
    let config = ws
        .config_path
        .strip_prefix(&ws.repo_root)
        .unwrap_or(&ws.config_path)
        .to_string_lossy()
        .replace('\\', "/");
    Ok(ExplainTypesReport {
        status: ReportStatus::Ok,
        config,
        body: match body {
            Value::Object(m) => m,
            _ => Map::new(),
        },
        next,
    })
}

struct Ctx<'a> {
    config: &'a Config,
    lines: &'a ConfigLines,
    raw: &'a yaml_serde::Value,
    repo_root: &'a Path,
    only: Option<&'a str>,
}

fn base(name: &str) -> String {
    key(&key("", "record_types"), name)
}

fn obj(pairs: Vec<(&str, Value)>) -> Value {
    let mut m = Map::new();
    for (k, v) in pairs {
        let empty = v.is_null()
            || v.as_array().is_some_and(Vec::is_empty)
            || v.as_object().is_some_and(Map::is_empty);
        if !empty {
            m.insert(k.to_string(), v);
        }
    }
    Value::Object(m)
}

fn yaml_json(v: &yaml_serde::Value) -> Value {
    serde_json::to_value(v).unwrap_or(Value::Null)
}

fn raw_get<'v>(root: &'v yaml_serde::Value, segs: &[&str]) -> Option<&'v yaml_serde::Value> {
    segs.iter()
        .try_fold(root, |v, s| v.get(*s))
        .filter(|v| !v.is_null())
}

fn non_empty(v: &yaml_serde::Value) -> bool {
    match v {
        yaml_serde::Value::Sequence(s) => !s.is_empty(),
        yaml_serde::Value::Mapping(m) => !m.is_empty(),
        yaml_serde::Value::Null => false,
        _ => true,
    }
}

fn paths_overlap(a: &str, b: &str) -> bool {
    let a = a.trim_end_matches('/');
    let b = b.trim_end_matches('/');
    a == b || a.starts_with(&format!("{b}/")) || b.starts_with(&format!("{a}/"))
}

/// `when Status is A or B and Kind is bug, X and Y are required`, from the
/// entry's own names and values.
pub(crate) fn sentence(status_field: &str, rule: &StatusFieldRule, verb: &str) -> String {
    let mut conds = Vec::new();
    if !rule.when.is_empty() {
        conds.push(format!("{status_field} is {}", rule.when.join(" or ")));
    }
    for (field, values) in &rule.when_field {
        conds.push(format!("{field} is {}", values.join(" or ")));
    }
    let fields = rule.fields.join(" and ");
    let be = if rule.fields.len() == 1 { "is" } else { "are" };
    if conds.is_empty() {
        format!("{fields} {be} {verb}")
    } else {
        format!("when {}, {fields} {be} {verb}", conds.join(" and "))
    }
}

impl Ctx<'_> {
    fn at(&self, path: &str) -> Value {
        Value::from(self.lines.at(path))
    }

    fn declared(&self, path: &str, value: Value) -> Value {
        json!({"value": value, "at": self.at(path)})
    }

    fn raw_type(&self, name: &str, segs: &[&str]) -> Option<&yaml_serde::Value> {
        let t = raw_get(self.raw, &["record_types", name])?;
        raw_get(t, segs)
    }

    /// Every key of the mapping at `path` not in `rendered`, as `{value, at}`
    /// under its own name, in declaration order.
    fn raw_rest<'k>(
        &'k self,
        mapping: Option<&'k yaml_serde::Value>,
        path: &str,
        rendered: &[&str],
    ) -> Vec<(&'k str, Value)> {
        let Some(m) = mapping.and_then(|m| m.as_mapping()) else {
            return Vec::new();
        };
        m.iter()
            .filter_map(|(k, v)| Some((k.as_str()?, v)))
            .filter(|(k, v)| !rendered.contains(k) && !v.is_null())
            .map(|(k, v)| (k, self.declared(&key(path, k), yaml_json(v))))
            .collect()
    }

    fn prefix(&self, name: &str, t: &RecordTypeConfig) -> Value {
        match &t.prefix {
            Some(p) => self.declared(&key(&base(name), "prefix"), p.clone().into()),
            None => {
                json!({"value": name.to_ascii_uppercase(), "default": true, "source": "type_name_uppercased"})
            }
        }
    }

    fn record_type(&self, name: &str, t: &RecordTypeConfig) -> Value {
        let b = base(name);
        let decl = t.status_decl();
        let mut dirs = vec![self.declared(&key(&b, "dir"), t.dir.clone().into())];
        dirs.extend(
            t.extra_dirs
                .iter()
                .enumerate()
                .map(|(i, d)| self.declared(&index(&key(&b, "extra_dirs"), i), d.clone().into())),
        );
        let mut out = vec![
            ("name", Value::from(name)),
            ("at", self.at(&b)),
            (
                "create",
                Value::from(format!("urzua new {name} \"<title>\"")),
            ),
            ("prefix", self.prefix(name, t)),
            ("dirs", Value::from(dirs)),
            ("template", self.template(name)),
            ("header_fields", Value::from(self.fields(name, t))),
            ("relation_pairs", Value::from(self.pairs(name, t))),
        ];
        if let Some(d) = &decl {
            out.push(("states", self.states(name, d)));
            out.push(("conditions", Value::from(self.conditions(name, t, d))));
        }
        out.push(("gates", Value::from(self.gates(name, t))));
        let aliases: Vec<Value> = self
            .lines
            .aliases_under(&b)
            .into_iter()
            .map(|(at, defined_at)| json!({"at": at, "defined_at": defined_at}))
            .collect();
        out.push(("aliases", Value::from(aliases)));
        let raw = raw_get(self.raw, &["record_types", name]);
        out.extend(self.raw_rest(raw, &b, TYPE_RENDERED));
        obj(out)
    }

    fn summary(&self, name: &str, t: &RecordTypeConfig) -> Value {
        let required: Vec<&String> = t.required_fields.iter().collect();
        let optional: Vec<&String> = t
            .known_fields
            .iter()
            .flatten()
            .filter(|f| !t.required_fields.contains(f))
            .collect();
        let references: Map<String, Value> = t
            .pointer_fields
            .iter()
            .flatten()
            .map(|f| (f.clone(), self.effective_targets(name, f)))
            .collect();
        let decl = t.status_decl();
        let states = decl.as_ref().map(|d| {
            let initial = self.initial(name, d);
            obj(vec![
                ("values", json!(d.values)),
                (
                    "initial",
                    if initial.get("default").is_some() {
                        initial
                    } else {
                        initial["value"].clone()
                    },
                ),
            ])
        });
        let count = |kind: &str, n: usize| -> (usize, usize) {
            let on = self.condition_rule(kind)["level"] != RuleLevel::Off.as_str();
            if on {
                (n, 0)
            } else {
                (0, n)
            }
        };
        let (on_r, off_r) = decl
            .as_ref()
            .map_or((0, 0), |d| count("require", d.require.len()));
        let (on_f, off_f) = decl
            .as_ref()
            .map_or((0, 0), |d| count("forbid", d.forbid.len()));
        let positive = |n: usize| if n > 0 { json!(n) } else { Value::Null };
        let prefix = self.prefix(name, t);
        let mut dirs = vec![t.dir.clone()];
        dirs.extend(t.extra_dirs.iter().cloned());
        obj(vec![
            ("name", Value::from(name)),
            ("at", self.at(&base(name))),
            (
                "prefix",
                if prefix.get("default").is_some() {
                    prefix
                } else {
                    prefix["value"].clone()
                },
            ),
            ("dirs", json!(dirs)),
            (
                "create",
                Value::from(format!("urzua new {name} \"<title>\"")),
            ),
            ("required", json!(required)),
            ("optional", json!(optional)),
            ("states", states.unwrap_or(Value::Null)),
            ("references", Value::Object(references)),
            ("conditions", positive(on_r + on_f)),
            ("conditions_not_enabled", positive(off_r + off_f)),
        ])
    }

    fn repository(&self) -> Value {
        let rest = self.raw_rest(Some(self.raw), "", REPOSITORY_EXCLUDED);
        obj(rest)
    }

    fn template(&self, name: &str) -> Value {
        let rel = format!(".urzua/templates/{name}.md");
        if self.repo_root.join(&rel).is_file() {
            json!({"value": rel, "default": true, "source": "conventional_template_path"})
        } else {
            Value::Null
        }
    }

    /// Every field the type names: required, then known, then any other a
    /// declaration names, each with what the configuration says of it.
    fn fields(&self, name: &str, t: &RecordTypeConfig) -> Vec<Value> {
        let b = base(name);
        let mut order: Vec<(String, String, bool)> = Vec::new();
        let mut push = |f: &str, path: String, required: bool| {
            if !order.iter().any(|(n, _, _)| n == f) {
                order.push((f.to_string(), path, required));
            }
        };
        for (i, f) in t.required_fields.iter().enumerate() {
            push(f, index(&key(&b, "required_fields"), i), true);
        }
        for (i, f) in t.known_fields.iter().flatten().enumerate() {
            push(f, index(&key(&b, "known_fields"), i), false);
        }
        for f in t.field_values.iter().flat_map(|m| m.keys()) {
            push(f, key(&key(&b, "field_values"), f), false);
        }
        let roles = t
            .relation_fields
            .as_ref()
            .map(|r| r.declared())
            .unwrap_or_default();
        for (role, f) in &roles {
            push(f, key(&key(&b, "relation_fields"), role), false);
        }
        order
            .into_iter()
            .map(|(f, path, required)| {
                let role: Vec<Value> = roles
                    .iter()
                    .filter(|(_, n)| *n == f)
                    .map(|(r, _)| json!({"role": r, "at": self.at(&key(&key(&b, "relation_fields"), r))}))
                    .collect();
                obj(vec![
                    ("name", Value::from(f.clone())),
                    ("at", self.at(&path)),
                    ("required", if required { Value::Bool(true) } else { Value::Null }),
                    ("values", self.field_values(name, t, &f)),
                    ("pointer", self.pointer(name, t, &f)),
                    ("narrative", self.narrative(name, t, &f)),
                    ("roles", Value::from(role)),
                ])
            })
            .collect()
    }

    fn field_values(&self, name: &str, t: &RecordTypeConfig, f: &str) -> Value {
        match t.field_values.as_ref().and_then(|m| m.get(f)) {
            Some(v) => self.declared(&key(&key(&base(name), "field_values"), f), json!(v)),
            None => Value::Null,
        }
    }

    /// What each declaration says of a reference field's targets, and the
    /// result: `any` when nothing constrains it.
    fn pointer(&self, name: &str, t: &RecordTypeConfig, f: &str) -> Value {
        let Some(i) = t.pointer_fields.iter().flatten().position(|p| p == f) else {
            return Value::Null;
        };
        let path = index(&key(&base(name), "pointer_fields"), i);
        let own = self
            .config
            .pointer_targets
            .get(name)
            .and_then(|m| m.get(f))
            .map(|v| self.declared(&key(&path, "targets"), json!(v)));
        let pairs: Vec<Value> = self
            .carried_pairs(name, t)
            .filter(|(_, p)| p.forward == f)
            .filter_map(|(i, p)| {
                let tt = p.target_types.as_ref()?;
                Some(self.declared(&key(&index("relation_pairs", i), "target_types"), json!(tt)))
            })
            .collect();
        obj(vec![
            ("at", self.at(&path)),
            ("targets", own.unwrap_or(Value::Null)),
            ("pair_target_types", Value::from(pairs)),
            ("effective_targets", self.effective_targets(name, f)),
        ])
    }

    /// The types a reference field may name once every declaration is
    /// applied, or `"any"`.
    fn effective_targets(&self, name: &str, f: &str) -> Value {
        match urzua_core::pointer_targets::constraints(self.config)
            .get(name)
            .and_then(|m| m.get(f))
        {
            Some(v) => json!(v),
            None => json!("any"),
        }
    }

    fn narrative(&self, name: &str, t: &RecordTypeConfig, f: &str) -> Value {
        match t.narrative_fields.iter().flatten().position(|p| p == f) {
            Some(i) => json!({"at": self.at(&index(&key(&base(name), "narrative_fields"), i))}),
            None => Value::Null,
        }
    }

    fn carried_pairs<'p>(
        &'p self,
        name: &'p str,
        t: &'p RecordTypeConfig,
    ) -> impl Iterator<Item = (usize, &'p urzua_core::config::RelationPair)> + 'p {
        let fields = t.declared_fields();
        self.config
            .relation_pairs
            .iter()
            .enumerate()
            .filter(move |(_, p)| {
                p.types
                    .as_ref()
                    .is_none_or(|ts| ts.iter().any(|x| x == name))
            })
            .filter(move |(_, p)| {
                [&p.forward, &p.inverse]
                    .iter()
                    .any(|f| fields.contains(&urzua_core::values::FieldName::new(f.as_str())))
            })
    }

    fn pairs(&self, name: &str, t: &RecordTypeConfig) -> Vec<Value> {
        let raw = raw_get(self.raw, &["relation_pairs"]).and_then(|v| v.as_sequence());
        self.carried_pairs(name, t)
            .map(|(i, _)| {
                let mut v = raw
                    .and_then(|r| r.get(i))
                    .map(yaml_json)
                    .unwrap_or(Value::Null);
                if let Some(m) = v.as_object_mut() {
                    m.insert("at".into(), self.at(&index("relation_pairs", i)));
                }
                v
            })
            .collect()
    }

    /// The declared initial statuses, or, undeclared, that a record may start
    /// at any status and the status `urzua new` writes.
    fn initial(&self, name: &str, d: &StatusDecl) -> Value {
        if !d.initial.is_empty() {
            return self.declared(
                &key(&key(&base(name), "statuses"), "initial"),
                json!(d.initial),
            );
        }
        match d.values.first() {
            Some(first) => json!({
                "required_of_records": false,
                "new_writes": first,
                "default": true,
                "source": "first_status_value"
            }),
            None => Value::Null,
        }
    }

    fn states(&self, name: &str, d: &StatusDecl) -> Value {
        let s = key(&base(name), "statuses");
        let list = |k: &str, v: &Vec<String>| -> Value {
            if v.is_empty() {
                Value::Null
            } else {
                self.declared(&key(&s, k), json!(v))
            }
        };
        let raw = self.raw_type(name, &["statuses"]);
        let transitions: Vec<Value> = raw
            .and_then(|r| r.get("transitions"))
            .and_then(|t| t.as_mapping())
            .into_iter()
            .flatten()
            .filter_map(|(from, to)| {
                let from = from.as_str()?;
                Some(json!({"from": from, "to": yaml_json(to), "at": self.at(&key(&key(&s, "transitions"), from))}))
            })
            .collect();
        let mut out = vec![
            ("values", list("values", &d.values)),
            ("initial", self.initial(name, d)),
            ("success", list("success", &d.success)),
            ("failure", list("failure", &d.failure)),
            ("implementable", list("implementable", &d.implementable)),
            ("claim_retained", list("claim_retained", &d.claim_retained)),
            ("blocked", list("blocked", &d.blocked)),
            ("progress", list("progress", &d.progress)),
            ("transitions", Value::from(transitions)),
            ("expiry", Value::from(self.expiry(name))),
        ];
        out.extend(self.raw_rest(raw, &s, STATUS_RENDERED));
        obj(out)
    }

    /// The claim periods the claim-expiry rule declares for this type, when the
    /// rule is in effect.
    fn expiry(&self, name: &str) -> Vec<Value> {
        const RULE: &str = RULE_EMBODIMENT_CLAIM_EXPIRED;
        if effective_level(self.config, RULE) == RuleLevel::Off {
            return Vec::new();
        }
        let Some(periods) =
            raw_get(self.raw, &["rules", RULE, "periods"]).and_then(|p| p.as_sequence())
        else {
            return Vec::new();
        };
        let path = key(&key("rules", RULE), "periods");
        periods
            .iter()
            .enumerate()
            .filter(|(_, p)| p.get("type").and_then(|t| t.as_str()) == Some(name))
            .map(|(i, p)| {
                let mut v = yaml_json(p);
                if let Some(m) = v.as_object_mut() {
                    m.remove("type");
                    m.insert("at".into(), self.at(&index(&path, i)));
                }
                v
            })
            .collect()
    }

    fn condition_rule(&self, kind: &str) -> Value {
        let id = if kind == "require" {
            RULE_STATUS_REQUIRED_FIELD_MISSING
        } else {
            RULE_STATUS_FORBIDDEN_FIELD_PRESENT
        };
        json!({"id": id, "level": effective_level(self.config, id).as_str()})
    }

    fn conditions(&self, name: &str, t: &RecordTypeConfig, d: &StatusDecl) -> Vec<Value> {
        let s = key(&base(name), "statuses");
        let status_field = t.relation_field(urzua_core::config::RelationRole::Status);
        // A pending or placeholder value meets no requirement, so the sentence
        // alone would overstate what satisfies it.
        let missing = obj(["pending", "placeholders"]
            .into_iter()
            .map(|k| {
                let v = match self.raw_type(name, &["statuses", k]) {
                    Some(v) => self.declared(&key(&s, k), yaml_json(v)),
                    None => json!({"default": true, "source": "built_in_list"}),
                };
                (k, v)
            })
            .collect());
        [
            ("require", &d.require, "required"),
            ("forbid", &d.forbid, "forbidden"),
        ]
        .into_iter()
        .flat_map(|(kind, rules, verb)| {
            rules
                .iter()
                .enumerate()
                .map(move |(i, r)| (kind, verb, i, r))
        })
        .map(|(kind, verb, i, r)| {
            obj(vec![
                ("kind", Value::from(kind)),
                ("rule", self.condition_rule(kind)),
                ("when", json!(r.when)),
                ("when_field", json!(r.when_field)),
                ("fields", json!(r.fields)),
                ("sentence", Value::from(sentence(status_field, r, verb))),
                (
                    "counts_as_missing",
                    if kind == "require" {
                        missing.clone()
                    } else {
                        Value::Null
                    },
                ),
                ("at", self.at(&index(&key(&s, kind), i))),
            ])
        })
        .collect()
    }

    fn type_gates<'g>(
        &'g self,
        name: &'g str,
        t: &'g RecordTypeConfig,
    ) -> impl Iterator<Item = (usize, &'g urzua_core::authority::Gate)> + 'g {
        self.config
            .authority
            .iter()
            .flat_map(|a| a.gates.iter().enumerate())
            .filter(move |(_, g)| {
                g.record_type.as_deref() == Some(name)
                    || g.paths
                        .iter()
                        .any(|p| t.all_dirs().any(|d| paths_overlap(p, d)))
            })
    }

    fn gates(&self, name: &str, t: &RecordTypeConfig) -> Vec<Value> {
        let Some(auth) = &self.config.authority else {
            return Vec::new();
        };
        self.type_gates(name, t)
            .map(|(i, g)| {
                let logins = auth.deciders.get(&g.by).map(|d| d.logins.clone());
                obj(vec![
                    ("to", json!(g.to)),
                    ("paths", json!(g.paths)),
                    ("by", Value::from(g.by.clone())),
                    ("logins", json!(logins)),
                    ("at", self.at(&index(&key("authority", "gates"), i))),
                ])
            })
            .collect()
    }

    /// The `at` of each key under the type that carries `d`; empty when the
    /// type does not declare it.
    fn declaration_paths(&self, name: &str, t: &RecordTypeConfig, d: Declaration) -> Vec<String> {
        let b = base(name);
        let present = |segs: &[&str]| -> Option<String> {
            self.raw_type(name, segs)
                .filter(|v| non_empty(v) && v.as_bool() != Some(false))
                .map(|_| {
                    self.lines
                        .at(&segs.iter().fold(b.clone(), |p, s| key(&p, s)))
                })
        };
        let field = |f: &str| -> Option<String> {
            let at = |k: &str, list: &[String]| {
                list.iter()
                    .position(|x| x == f)
                    .map(|i| self.lines.at(&index(&key(&b, k), i)))
            };
            at("required_fields", &t.required_fields).or_else(|| {
                at(
                    "known_fields",
                    t.known_fields.as_deref().unwrap_or_default(),
                )
            })
        };
        use urzua_core::config::RelationRole;
        let found: Vec<Option<String>> = match d {
            Declaration::RequiredFields => vec![present(&["required_fields"])],
            Declaration::KnownFields => vec![present(&["known_fields"])],
            Declaration::PointerFields => vec![present(&["pointer_fields"])],
            Declaration::VersionBump => vec![present(&["revision_log", "version_bump"])],
            Declaration::BlockedOrProgress => vec![
                present(&["statuses", "blocked"]),
                present(&["statuses", "progress"]),
            ],
            Declaration::DeliveryItems => vec![present(&["statuses", "delivery", "items"])],
            Declaration::UniqueHeadings => vec![present(&["sections", "unique_headings"])],
            Declaration::NoSkippedLevels => vec![present(&["sections", "no_skipped_levels"])],
            Declaration::SectionOrder => vec![present(&["sections", "order"])],
            Declaration::SectionsRequired => vec![present(&["sections", "required"])],
            Declaration::DeclaredFields => {
                vec![present(&["required_fields"]), present(&["known_fields"])]
            }
            Declaration::PointerOrNarrativeFields => {
                vec![present(&["pointer_fields"]), present(&["narrative_fields"])]
            }
            Declaration::SupersessionField => {
                vec![field(t.relation_field(RelationRole::Supersession))]
            }
            Declaration::LocatorField => {
                vec![field(t.relation_field(RelationRole::EmbodimentLocator))]
            }
            Declaration::RevisionLog => vec![present(&["revision_log"])],
            Declaration::StatusValues => vec![present(&["statuses", "values"])],
            Declaration::FieldValues => vec![present(&["field_values"])],
            Declaration::StatusRequire => vec![present(&["statuses", "require"])],
            Declaration::StatusForbid => vec![present(&["statuses", "forbid"])],
            Declaration::StatusTransitions => vec![present(&["statuses", "transitions"])],
            Declaration::StatusDelivery => vec![present(&["statuses", "delivery"])],
            Declaration::WaiverType => {
                vec![t.role.is_some().then(|| self.lines.at(&key(&b, "role")))]
            }
        };
        found.into_iter().flatten().collect()
    }

    /// Whether `def` reads something type `name` takes part in, and the `at`
    /// of each declaration that makes it so (empty for a rule that reads every
    /// record). Read from what the rule table says the rule needs.
    fn rule_for_type(
        &self,
        name: &str,
        t: &RecordTypeConfig,
        def: &RuleDef,
    ) -> Option<Vec<String>> {
        let mut by = Vec::new();
        if let Some(d) = def.reads_declaration {
            let paths = self.declaration_paths(name, t, d);
            if paths.is_empty() {
                return None;
            }
            by.extend(paths);
        }
        if def.requires_relation_pairs || def.requires_acyclic_pair {
            let pairs: Vec<String> = self
                .carried_pairs(name, t)
                .filter(|(_, p)| !def.requires_acyclic_pair || p.acyclic)
                .map(|(i, _)| self.lines.at(&index("relation_pairs", i)))
                .collect();
            if pairs.is_empty() {
                return None;
            }
            by.extend(pairs);
        }
        if def.requires_authority {
            let gates = self
                .type_gates(name, t)
                .map(|(i, _)| self.lines.at(&index(&key("authority", "gates"), i)));
            let gates: Vec<String> = gates.collect();
            if gates.is_empty() {
                return None;
            }
            by.extend(gates);
        }
        if def.id == RULE_POINTER_TARGET_TYPE {
            by.extend(self.target_declarations(name, t));
        }
        let reads_records = def.scope == Scope::Records;
        (reads_records || !by.is_empty()).then_some(by)
    }

    fn target_declarations(&self, name: &str, t: &RecordTypeConfig) -> Vec<String> {
        let b = base(name);
        let own = t
            .pointer_fields
            .iter()
            .flatten()
            .enumerate()
            .filter(|(_, f)| {
                self.config
                    .pointer_targets
                    .get(name)
                    .is_some_and(|m| m.contains_key(*f))
            })
            .map(|(i, _)| {
                self.lines
                    .at(&key(&index(&key(&b, "pointer_fields"), i), "targets"))
            });
        let pairs = self
            .carried_pairs(name, t)
            .filter(|(_, p)| p.target_types.is_some())
            .map(|(i, _)| {
                self.lines
                    .at(&key(&index("relation_pairs", i), "target_types"))
            });
        own.chain(pairs).collect()
    }

    /// Every rule that reads something a selected type takes part in, with the
    /// declarations that make it so, merged across the selected types.
    fn rules_by_type(&self) -> Vec<(&'static RuleDef, Vec<String>)> {
        RULE_TABLE
            .iter()
            .filter_map(|def| {
                let mut any = false;
                let mut by: Vec<String> = Vec::new();
                for (name, t) in &self.config.record_types {
                    if self.only.is_some_and(|o| o != name) {
                        continue;
                    }
                    if let Some(paths) = self.rule_for_type(name, t, def) {
                        any = true;
                        by.extend(paths);
                    }
                }
                by.dedup();
                any.then_some((def, by))
            })
            .collect()
    }

    /// `(set, on_by_declaration, declared_but_not_enabled)` as
    /// `(id, level, at or declared_by)` rows. For one type, only the rules
    /// that read something it takes part in; otherwise every rule the
    /// configuration sets.
    fn rule_rows(&self) -> [Vec<(String, &'static str, Value)>; 3] {
        let typed = self.rules_by_type();
        let by_of = |id: &str| -> Vec<String> {
            typed
                .iter()
                .find(|(d, _)| d.id == id)
                .map(|(_, b)| b.clone())
                .unwrap_or_default()
        };
        let set = self
            .config
            .rules
            .iter()
            .filter(|(id, _)| self.only.is_none() || typed.iter().any(|(d, _)| d.id == id.as_str()))
            .filter(|(_, s)| self.only.is_none() || s.level != RuleLevel::Off)
            .map(|(id, s)| {
                let by = by_of(id);
                let mut row = json!({"at": self.at(&key("rules", id))});
                if self.only.is_some() && !by.is_empty() {
                    row["declared_by"] = json!(by);
                }
                (id.clone(), s.level.as_str(), row)
            })
            .collect();
        let on = DEFAULT_ON_RULES
            .iter()
            .filter(|d| d.applies.is_some_and(|f| f(self.config)))
            .filter(|d| !self.config.rules.contains_key(d.id))
            .filter(|d| {
                typed
                    .iter()
                    .any(|(def, by)| def.id == d.id && !by.is_empty())
            })
            .map(|d| (d.id.to_string(), d.level.as_str(), json!(by_of(d.id))))
            .collect();
        let off = typed
            .iter()
            .filter(|(d, by)| {
                !by.is_empty() && effective_level(self.config, d.id) == RuleLevel::Off
            })
            .map(|(d, by)| (d.id.to_string(), RuleLevel::Off.as_str(), json!(by)))
            .collect();
        [set, on, off]
    }

    fn rules(&self) -> Value {
        let [set, on, off] = self.rule_rows();
        let set: Vec<Value> = set
            .into_iter()
            .map(|(id, level, mut row)| {
                row["id"] = json!(id);
                row["level"] = json!(level);
                row
            })
            .collect();
        let on: Vec<Value> = on
            .into_iter()
            .map(|(id, level, by)| json!({"id": id, "level": level, "default": true, "source": "default_on", "declared_by": by}))
            .collect();
        let off: Vec<Value> = off
            .into_iter()
            .map(|(id, _, by)| json!({"id": id, "declared_by": by}))
            .collect();
        obj(vec![
            ("set", Value::from(set)),
            ("on_by_declaration", Value::from(on)),
            ("declared_but_not_enabled", Value::from(off)),
        ])
    }

    /// The rules on because of a declaration and the rules a declaration needs
    /// but that are off, as `id: level`.
    fn rules_summary(&self) -> Value {
        let [_, on, off] = self.rule_rows();
        let map = |rows: Vec<(String, &'static str, Value)>| -> Value {
            Value::Object(
                rows.into_iter()
                    .map(|(id, l, _)| (id, Value::from(l)))
                    .collect(),
            )
        };
        obj(vec![
            ("on_by_declaration", map(on)),
            ("declared_but_not_enabled", map(off)),
        ])
    }
}
