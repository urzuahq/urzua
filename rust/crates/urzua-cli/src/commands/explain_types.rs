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
use urzua_core::config::{Config, RecordTypeConfig};
use urzua_core::report::{ActionInput, Failure, NextAction, Notice, Report, ReportStatus};
use urzua_core::rules::RULE_EMBODIMENT_CLAIM_EXPIRED;
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

const ENFORCEMENT_NOTE: &str = "Whether a declared condition or field rule is enforced is answered by the rules set in the configuration (`urzua explain --types --full`) and by `urzua check`.";

const RULES_SET_NOTE: &str = "Rules set in the configuration, as written under `rules:`. Rules on by default or by a declaration do not appear here; `urzua check` reports every rule that ran and how many records each examined. Whether a declared condition or field rule is enforced is answered by this section and by `urzua check`.";

const PATH_GATE_NOTE: &str = "A path gate declares who may change paths matching these patterns, whatever the type; `urzua new`, the edit hook and the pull request check act on it where they are enabled.";

const NO_TARGET_NOTE: &str = "no type satisfies both declarations";

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

/// `--full` with `--type`: the one-type report is already the full one.
pub fn refuse_full_with_type(name: &str) -> ExitCode {
    let command = urzua_core::report::shell_command(&["urzua", "explain", "--type", name]);
    emit_or_not_run(Err(Failure::with_next(
        format!("--full goes with --types; `{command}` already prints the full detail of one type"),
        NextAction::runs_as_printed(command, "the full detail of one type"),
    )))
}

/// `--full` with neither `--types` nor `--type`.
pub fn refuse_full_alone() -> ExitCode {
    let command = "urzua explain --types --full";
    emit_or_not_run(Err(Failure::with_next(
        format!("--full goes with --types: `{command}`"),
        NextAction::runs_as_printed(command, "every declaration of every type"),
    )))
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
    };
    let selected = ws
        .config
        .record_types
        .iter()
        .filter(|(n, _)| only.as_ref().is_none_or(|o| o == *n));
    let next = (matches!(mode, Mode::Summary) && !names.is_empty()).then(detail_action);
    let body = match mode {
        Mode::Summary => obj(vec![
            ("types", selected.map(|(n, t)| ctx.summary(n, t)).collect()),
            ("path_gates", ctx.path_gates()["gates"].clone()),
        ]),
        Mode::Full => obj(vec![
            (
                "types",
                selected.map(|(n, t)| ctx.record_type(n, t)).collect(),
            ),
            ("rules_set_in_configuration", ctx.rules_set()),
            ("path_gates", ctx.path_gates()),
            ("repository", ctx.repository()),
        ]),
        Mode::One(_) => obj(vec![
            (
                "types",
                selected.map(|(n, t)| ctx.record_type(n, t)).collect(),
            ),
            ("note", Value::from(ENFORCEMENT_NOTE)),
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
                json!({"value": crate::commands::new::type_prefix(name, t), "default": true, "source": "type_name_uppercased"})
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
            ("create_command", self.create(name, t)),
            ("cannot_create", self.cannot_create(name, t)),
            ("prefix", self.prefix(name, t)),
            ("dirs", Value::from(dirs)),
            ("template", self.template(name)),
            ("header_fields", Value::from(self.fields(name, t))),
        ];
        if let Some(d) = &decl {
            out.push(("states", self.states(name, t, d)));
            out.push(("conditions", Value::from(self.conditions(name, t, d))));
        }
        out.push(("gates", Value::from(self.gates(name))));
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
        let references: Map<String, Value> = self
            .reference_fields(name, t)
            .into_iter()
            .map(|f| {
                let v = self.effective_targets(name, &f);
                (f, v)
            })
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
        let conditions = decl
            .as_ref()
            .map_or(0, |d| d.require.len() + d.forbid.len());
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
            ("cannot_create", self.cannot_create(name, t)),
            ("required", json!(required)),
            ("optional", json!(optional)),
            ("states", states.unwrap_or(Value::Null)),
            ("references", Value::Object(references)),
            (
                "conditions",
                if conditions > 0 {
                    json!(conditions)
                } else {
                    Value::Null
                },
            ),
        ])
    }

    fn create(&self, name: &str, t: &RecordTypeConfig) -> Value {
        match crate::commands::new::scaffold_refusal(name, t) {
            Some(_) => Value::Null,
            None => Value::from(urzua_core::report::shell_command(&[
                "urzua", "new", name, "<title>",
            ])),
        }
    }

    /// The reason `urzua new` gives for refusing the type, from its own check.
    fn cannot_create(&self, name: &str, t: &RecordTypeConfig) -> Value {
        crate::commands::new::scaffold_refusal(name, t).map_or(Value::Null, Value::from)
    }

    /// The type's reference fields: its `pointer_fields`, then each field the
    /// engine's target constraints name for the type (a pair's forward field).
    fn reference_fields(&self, name: &str, t: &RecordTypeConfig) -> Vec<String> {
        let mut out: Vec<String> = t.pointer_fields.iter().flatten().cloned().collect();
        if let Some(m) = urzua_core::pointer_targets::constraints(self.config).get(name) {
            for f in m.keys() {
                if !out.contains(f) {
                    out.push(f.clone());
                }
            }
        }
        out
    }

    fn repository(&self) -> Value {
        let rest = self.raw_rest(Some(self.raw), "", REPOSITORY_EXCLUDED);
        obj(rest)
    }

    fn template(&self, name: &str) -> Value {
        let rel = crate::commands::new::template_path(name)
            .to_string_lossy()
            .replace('\\', "/");
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
                    ("relation_pairs", Value::from(self.field_pairs(name, &f))),
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
        if !self.reference_fields(name, t).iter().any(|r| r == f) {
            return Value::Null;
        }
        let path = t
            .pointer_fields
            .iter()
            .flatten()
            .position(|p| p == f)
            .map(|i| index(&key(&base(name), "pointer_fields"), i));
        let own = path.as_ref().and_then(|path| {
            self.config
                .pointer_targets
                .get(name)
                .and_then(|m| m.get(f))
                .map(|v| self.declared(&key(path, "targets"), json!(v)))
        });
        let pairs: Vec<Value> = self
            .config
            .relation_pairs
            .iter()
            .enumerate()
            .filter(|(_, p)| p.forward == f)
            .filter(|(_, p)| {
                p.types
                    .as_ref()
                    .is_none_or(|ts| ts.iter().any(|x| x == name))
            })
            .filter_map(|(i, p)| {
                let tt = p.target_types.as_ref()?;
                let mut v =
                    self.declared(&key(&index("relation_pairs", i), "target_types"), json!(tt));
                v["pair"] = json!(p.name);
                Some(v)
            })
            .collect();
        obj(vec![
            ("at", path.map_or(Value::Null, |p| self.at(&p))),
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
            Some(v) if v.is_empty() => json!({"value": "none", "note": NO_TARGET_NOTE}),
            Some(v) => json!(v),
            None => json!("any"),
        }
    }

    /// Each relation pair that declares this field as its forward or inverse
    /// side and whose `types`, when given, name the type, as declared.
    fn field_pairs(&self, name: &str, f: &str) -> Vec<Value> {
        self.config
            .relation_pairs
            .iter()
            .enumerate()
            .filter(|(_, p)| p.forward == f || p.inverse == f)
            .filter(|(_, p)| {
                p.types
                    .as_ref()
                    .is_none_or(|ts| ts.iter().any(|x| x == name))
            })
            .map(|(i, p)| {
                obj(vec![
                    ("name", json!(p.name)),
                    ("forward", json!(p.forward)),
                    ("inverse", json!(p.inverse)),
                    ("types", json!(p.types)),
                    ("target_types", json!(p.target_types)),
                    ("at", self.at(&index("relation_pairs", i))),
                ])
            })
            .collect()
    }

    fn narrative(&self, name: &str, t: &RecordTypeConfig, f: &str) -> Value {
        match t.narrative_fields.iter().flatten().position(|p| p == f) {
            Some(i) => json!({"at": self.at(&index(&key(&base(name), "narrative_fields"), i))}),
            None => Value::Null,
        }
    }

    fn initial(&self, name: &str, d: &StatusDecl) -> Value {
        if !d.initial.is_empty() {
            return self.declared(
                &key(&key(&base(name), "statuses"), "initial"),
                json!(d.initial),
            );
        }
        Value::Null
    }

    /// The status `urzua new` writes, from the function that writes it; null
    /// when it writes none.
    fn new_writes(&self, name: &str, t: &RecordTypeConfig) -> Value {
        if crate::commands::new::scaffold_refusal(name, t).is_some() {
            return Value::Null;
        }
        match urzua_core::new_record::scaffold_status(t) {
            Some(v) => json!({"value": v, "default": true, "source": "urzua_new_scaffold"}),
            None => Value::Null,
        }
    }

    fn states(&self, name: &str, t: &RecordTypeConfig, d: &StatusDecl) -> Value {
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
            ("new_writes", self.new_writes(name, t)),
            ("success", list("success", &d.success)),
            ("failure", list("failure", &d.failure)),
            ("implementable", list("implementable", &d.implementable)),
            ("claim_retained", list("claim_retained", &d.claim_retained)),
            ("blocked", list("blocked", &d.blocked)),
            ("progress", list("progress", &d.progress)),
            ("transitions", Value::from(transitions)),
            ("expiry", self.expiry(name)),
        ];
        out.extend(self.raw_rest(raw, &s, STATUS_RENDERED));
        obj(out)
    }

    /// The claim periods the claim-expiry rule's setting declares for this
    /// type, with that setting's level as written. Whether the rule judges
    /// these records is not claimed here.
    fn expiry(&self, name: &str) -> Value {
        const RULE: &str = RULE_EMBODIMENT_CLAIM_EXPIRED;
        let Some(periods) =
            raw_get(self.raw, &["rules", RULE, "periods"]).and_then(|p| p.as_sequence())
        else {
            return Value::Null;
        };
        let path = key(&key("rules", RULE), "periods");
        let mine: Vec<Value> = periods
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
            .collect();
        if mine.is_empty() {
            return Value::Null;
        }
        let level = self
            .config
            .rules
            .get(RULE)
            .map(|r| self.declared(&key("rules", RULE), json!(r.level.as_str())));
        obj(vec![
            ("rule", Value::from(RULE)),
            ("level", level.unwrap_or(Value::Null)),
            ("periods", Value::from(mine)),
        ])
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
                ("when", json!(r.when)),
                ("when_field", json!(r.when_field)),
                ("fields", json!(r.fields)),
                (
                    "sentence",
                    Value::from(format!("declared: {}", sentence(status_field, r, verb))),
                ),
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

    /// The gates declared for this type by name, as declared.
    fn gates(&self, name: &str) -> Vec<Value> {
        let Some(auth) = &self.config.authority else {
            return Vec::new();
        };
        auth.gates
            .iter()
            .enumerate()
            .filter(|(_, g)| g.record_type.as_deref() == Some(name))
            .map(|(i, g)| {
                let logins = auth.deciders.get(&g.by).map(|d| d.logins.clone());
                obj(vec![
                    ("to", json!(g.to)),
                    ("by", Value::from(g.by.clone())),
                    ("logins", json!(logins)),
                    ("at", self.at(&index(&key("authority", "gates"), i))),
                ])
            })
            .collect()
    }

    /// Every path gate, as declared, once for the repository.
    fn path_gates(&self) -> Value {
        let Some(auth) = &self.config.authority else {
            return Value::Null;
        };
        let gates: Vec<Value> = auth
            .gates
            .iter()
            .enumerate()
            .filter(|(_, g)| !g.paths.is_empty())
            .map(|(i, g)| {
                json!({"paths": g.paths, "by": g.by, "at": self.at(&index(&key("authority", "gates"), i))})
            })
            .collect();
        if gates.is_empty() {
            return Value::Null;
        }
        json!({"gates": gates, "note": PATH_GATE_NOTE})
    }

    /// The rules as written under `rules:`.
    fn rules_set(&self) -> Value {
        let written = raw_get(self.raw, &["rules"]).and_then(|r| r.as_mapping());
        let set: Vec<Value> = written
            .into_iter()
            .flatten()
            .filter_map(|(k, _)| {
                let id = k.as_str()?;
                let r = self.config.rules.get(id)?;
                Some(json!({"id": id, "level": r.level.as_str(), "at": self.at(&key("rules", id))}))
            })
            .collect();
        if set.is_empty() {
            return Value::Null;
        }
        json!({"rules": set, "note": RULES_SET_NOTE})
    }
}
