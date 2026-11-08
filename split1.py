import re
p='config.rs'
s=open(p).read()
start=s.index('    // Values that reach filenames and discovery: validated here, once, so')
endmark='    Ok((config, models))\n}\n'
end=s.index(endmark, start)
body=s[start:end]
def cut(text, markers):
    idx=[text.index(m) for m in markers]
    assert idx==sorted(idx), idx
    idx.append(len(text))
    return [text[idx[i]:idx[i+1]] for i in range(len(markers))]
M=['    // Values that reach filenames and discovery: validated here, once, so',
'    // A misspelled rule name would otherwise',
'    // `indexes` is the one structured option',
'    // `issue_refs` is structured too',
'    // A list of blanks is an empty list',
'    // Published paths: reject what cannot name',
'    // Declared boundaries (`ADR-80`)',
'    // `external_ids` entries must parse',
'    // `closes` names a frontmatter key',
'    // `non_reference_field` must name a field',
'    // A test path outside every back-pointer path',
'    // `fields` must name something the repository']
secs=cut(body,M)
names=['validate_record_types','validate_rule_names','validate_index_options','validate_issue_refs_options','validate_rule_settings','normalize_rule_paths','validate_boundaries','validate_external_ids','validate_claim_fields','validate_non_reference_fields','validate_test_paths','validate_rule_fields']
docs=['Record types: the values that reach filenames and discovery (directories,\n/// prefixes, `requires`, `permanent`, excluded files, the revision-log declaration).',
'Every rule name under `rules:` is one this build ships.',
'The `indexes` option: owner rule only, declared when the rule runs, each spec valid.',
'The `issue_refs` option: owner rule only, declared when the rule runs, each field known.',
'',
'Published paths: refused when they cannot name a path in the repository, otherwise\n/// stored in their normalised form.',
'Declared boundaries: structure, then the one spelling of each path.',
'`external_ids` entries parse and sit under a declared record prefix.',
'`closes`, `strict` and `verb_claims`: a key the one-line reader can find, and the\n/// options that need it.',
'`non_reference_field` names a declared field no rule resolves or stale-checks.',
'A test path lies inside a back-pointer path, or is a glob.',
'`fields` names a declared pointer or narrative field.']
mut={'normalize_rule_paths','validate_boundaries'}

def dedent(t,n=4):
    out=[]
    for l in t.split('\n'):
        out.append(l[n:] if l.startswith(' '*n) else l)
    return '\n'.join(out)

funcs=[]
for n,d,t in zip(names,docs,secs):
    t=re.sub(r'&config\b(?!\.)','config',t)
    if n=='validate_rule_settings':
        continue
    if n=='validate_external_ids':
        # drop the extra block braces
        t=t.replace('    {\n        let prefixes','    let prefixes',1)
        lines=t.rstrip().split('\n')
        assert lines[-1]=='    }', lines[-1]
        lines=lines[:-1]
        # dedent everything after the first 'let prefixes' line by 4
        i=[k for k,l in enumerate(lines) if l.startswith('    let prefixes')][0]
        lines=lines[:i+1]+[l[4:] if l.startswith('    ') else l for l in lines[i+1:]]
        t='\n'.join(lines)+'\n'
    ty='&mut Config' if n in mut else '&Config'
    funcs.append(f"/// {d}\nfn {n}(config: {ty}) -> Result<(), ConfigError> {{\n{t.rstrip()}\n    Ok(())\n}}\n")

# rule settings
t=secs[4]
t=re.sub(r'&config\b(?!\.)','config',t)
pre_loop_marker='    // A list of blanks is an empty list wearing a value.\n    let declared = |v: Vec<String>| !v.is_empty() && v.iter().all(|s| !s.trim().is_empty());\n\n    for (name, setting) in &config.rules {\n'
assert t.startswith(pre_loop_marker)
loopbody=t[len(pre_loop_marker):]
assert loopbody.rstrip().endswith('    }')
loopbody=loopbody.rstrip()[:-len('    }')]
LM=['        for opt in RULE_OPTIONS {\n            let present',
'        if let (Some(fields), true) = (&setting.date_fields',
'        if let Some(levels) = &setting.field_levels {',
'        for (option, present) in [\n            ("decision_types"',
'        // A pin on a kind the rule rejects',
'        // A sticky state equal',
'        // The audit field only moves',
'        for (option, present) in [\n            ("drift_grace_days"',
'        // An option-taking rule given no option']
parts=cut(loopbody,LM)
assert len(parts)==9
# parts[8] = comment + Off continue + required loop
last=parts[8]
cm=last.index('        if setting.level == RuleLevel::Off {\n            continue;\n        }\n')
comment=last[:cm]
cont='        if setting.level == RuleLevel::Off {\n            continue;\n        }\n'
req=last[cm+len(cont):]
helpers=[
('rule_option_applicability',"Which options a rule accepts, and the value checks of the plain ones.",parts[0]),
('rule_date_options',"`date_fields` and `date_order` name declared fields, for a rule that runs.",parts[1]),
('rule_field_levels',"`field_levels`: accepted by the rule, non-empty, no `off`, declared fields.",parts[2]),
('rule_decision_gate',"The decision-gate options: accepted by the rule and a coherent gate.",parts[3]),
('rule_pinnable_kinds',"`pinnable` only narrows `locator_kinds`.",parts[4]),
('rule_sticky_states',"A sticky state is never one the evidence computes.",parts[5]),
('rule_drift_audit_field',"`drift_audit_field` needs a window and a declared field.",parts[6]),
('rule_expiry_options',"The drift and claim-expiry options, checked together once every record type is known.",parts[7]),
]
rs=[]
for n,d,pt in helpers:
    uses_config = 'config' in re.sub(r'config\.|\bconfig\b','CONFIG',pt) and 'CONFIG' in re.sub(r'config\b','CONFIG',pt)
    pt2=dedent(pt)
    cfg = 'config: &Config' if re.search(r'\bconfig\b',pt2) else '_config: &Config'
    rs.append(f"/// {d}\nfn check_{n}(name: &String, setting: &RuleSetting, {cfg}) -> Result<(), ConfigError> {{\n{pt2.rstrip()}\n    Ok(())\n}}\n")
rs=[x for x in rs]
reqbody=comment_text=None
req_d=dedent(req)
req_fn=f"""/// An option-taking rule given no option does not fall back to a sane default
/// (`BUG-44`): each option the rule accepts must be declared, with no blank entry.
fn check_rule_required_options(
    name: &String,
    setting: &RuleSetting,
    config: &Config,
) -> Result<(), ConfigError> {{
    // A list of blanks is an empty list wearing a value.
    let declared = |v: Vec<String>| !v.is_empty() && v.iter().all(|s| !s.trim().is_empty());
{req_d.rstrip()}
    Ok(())
}}
"""
settings_fn=f"""/// The per-rule options: acceptance by the rule, the value checks of each option,
/// then the options a running rule must declare.
fn validate_rule_settings(config: &Config) -> Result<(), ConfigError> {{
    for (name, setting) in &config.rules {{
        check_rule_option_applicability(name, setting, config)?;
        check_rule_date_options(name, setting, config)?;
        check_rule_field_levels(name, setting, config)?;
        check_rule_decision_gate(name, setting, config)?;
        check_rule_pinnable_kinds(name, setting)?;
        check_rule_sticky_states(name, setting, config)?;
        check_rule_drift_audit_field(name, setting, config)?;
        check_rule_expiry_options(name, setting, config)?;
        // A declined rule never runs, so `gated` never reads its options --
        // requiring them anyway would mean a rule could not be turned off
        // without supplying values it will not use.
        if setting.level == RuleLevel::Off {{
            continue;
        }}
        check_rule_required_options(name, setting, config)?;
    }}
    Ok(())
}}
"""
# the comment block originally above the Off continue is long; keep it on the helper
print(comment)
newcalls="""    validate_record_types(&config)?;
    validate_rule_names(&config)?;
    validate_index_options(&config)?;
    validate_issue_refs_options(&config)?;
    validate_rule_settings(&config)?;
    normalize_rule_paths(&mut config)?;
    validate_boundaries(&mut config)?;
    validate_external_ids(&config)?;
    validate_claim_fields(&config)?;
    validate_non_reference_fields(&config)?;
    validate_test_paths(&config)?;
    validate_rule_fields(&config)?;
"""
# order of helper functions: in pass order
order=[]
for n in names:
    if n=='validate_rule_settings':
        order.append(settings_fn); order.extend(rs); order.append(req_fn)
    else:
        order.append([f for f in funcs if f.split('\nfn ')[1].startswith(n+'(')][0])
helpers_text='\n'.join(order)
s=s[:start]+newcalls+s[end:]
# insert helpers after the end of parse_with_document_models
fe=s.index(endmark,start)+len(endmark)
s=s[:fe]+'\n'+helpers_text+s[fe:]
open(p,'w').write(s)
