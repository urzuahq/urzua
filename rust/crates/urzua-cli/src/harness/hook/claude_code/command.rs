//! The hook commands this adapter registers, in the repository's settings file
//! (`urzua harness install`) and in the plugin's hooks file alike: both are
//! rendered from the one settings fragment, so they cannot drift.

/// Prints `allow` only when it reads the configuration on stdin with
/// certainty as letting an unjudged edit through, by the rule the binary uses:
/// neither `acceptance` nor `protected-paths` enabled without
/// `on_unjudged: allow` (their default being deny), and `editor-hook` not
/// enabled with `on_unjudged: deny`. A guardrail is read only at a key position
/// under `harness.guardrails`, in the block form or the one-line `{...}` form,
/// with only the keys it may carry (`enabled: true|false`,
/// `on_unjudged: allow|deny`, and for `editor-hook` `pending_pair:
/// allow|refuse`), a value plain or quoted; any other key or value is not
/// certain.
///
/// Each line is scanned as YAML reads it, in linear time: a quote opens a
/// quoted scalar only where a value starts (so `it's` stays plain), `#` after a
/// blank starts a comment only outside quotes, and brackets count flow depth
/// only where a value starts or inside a flow. Quote and flow state carry
/// across lines; those continuation lines, and the lines of a block scalar
/// (`|`, `>`), are content, never keys. The root indentation is the first
/// content line's, and a key is trimmed before it is compared.
///
/// Not certain, so refused, anywhere in the file: a line longer than 64 KiB;
/// a byte that is not valid UTF-8, a control character other than tab, line
/// feed and a line-final carriage return, a C1 control, U+2028, U+2029,
/// U+FFFE, U+FFFF or a byte order mark after the first three bytes; a tab in
/// leading whitespace, on any line; a second document, content after `...`, or
/// a directive other than one `%YAML` line before `---`; a quote or flow left
/// open at the end of the file, flow nested more than 32 deep or blocks more
/// than 64, a quoted scalar continued at or left of its key's indentation; a
/// plain value holding `: ` or opening with `- `; a line left of the root
/// indentation. A flow's continuation lines may sit at any indentation. At the root: a duplicate key, a merge
/// key, a complex, flow, anchored, aliased or tagged key, a key that cannot be
/// read, a list item not under a key with an empty value, a quoted `harness`
/// key, and a `harness` value other than a block mapping or null. In the
/// `harness` subtree: an anchor, alias, tag or merge key, a quoted, complex,
/// flow or list-item key, a line under a key that already has a value, and the
/// guardrails written inline. In a guardrail: a repeated key, a child indented
/// unlike its siblings, or a value on the next line. Anchors, aliases, tags,
/// quoted scalars and flow collections elsewhere are ordinary YAML. The reader
/// does not validate the file: one the engine cannot load but whose
/// guardrails read as open is allowed with the notice, and installing urzua
/// closes that.
///
/// Plain POSIX awk under `LC_ALL=C` (bytes, no `match` arrays, no interval
/// expressions, `\047` for a quote) fed on stdin so no path is read as an
/// assignment; written over lines here and joined into one for the command.
/// mawk, gawk and busybox awk give the same answer on every shape the tests
/// feed.
const CERTAIN_ALLOW: &str = r##"LC_ALL=C awk 'function trim(s){sub(/^[ \t]+/,"",s);sub(/[ \t]+$/,"",s);return s}
function unq(v){if(v~/^"[^"\\]*"$/||v~/^\047[^\047]*\047$/)return substr(v,2,length(v)-2);return v}
function no(){u=1;exit}
function item(k,v,  w){if(k in seen)u=1;seen[k]=1;w=unq(v);if(k=="enabled"&&v=="true")en=1;else if(k=="enabled"&&v=="false")en=0;else if(k=="on_unjudged"&&w=="allow"){al=1;as=1}else if(k=="on_unjudged"&&w=="deny"){al=0;as=1}else if(k=="pending_pair"&&nm=="editor-hook"&&(w=="allow"||w=="refuse"))x=1;else u=1}
function shut(){if(blk&&en&&(nm=="editor-hook"?(as&&!al):!al))c=1;blk=0}
function start(n){shut();nm=n;blk=1;bi="";en=0;al=0;as=0;split("",seen)}
function scan(l,want,  n,j,ch,lt,gap,st,sg){n=length(l);cut=n+1;lt=q?"x":"";gap=1;bl="";sg=1;for(j=1;j<=n;j++){ch=substr(l,j,1);if(q==1){if(ch=="\047"){if(substr(l,j+1,1)=="\047"){j++;continue}q=0;sg=j;gap=0}continue}if(q==2){if(ch=="\\"){j++;continue}if(ch=="\""){q=0;sg=j;gap=0}continue}if(ch==" "||ch=="\t"){gap=1;continue}if(ch=="#"&&gap){cut=j;break}st=(lt==""||index("[{,",lt)>0||(gap&&index(":-?",lt)>0));if((ch=="\047"||ch=="\"")&&st){q=(ch=="\047")?1:2;if(want)bl=bl substr(l,sg,j-sg+1);continue}if((ch=="["||ch=="{")&&(st||dp>0)){if(++dp>32)u=1}else if((ch=="]"||ch=="}")&&dp>0)dp--;lt=ch;gap=0}if(want&&!q)bl=bl substr(l,sg,cut-sg);return bl}
function hz(s){if(s~/:[ \t]+[&*!]/||s~/^(-[ \t]+)*[&*!]/||s~/[[{,][ \t]*[&*!]/||s~/<<[ \t]*:/)no()}
BEGIN{A="[\011\040-\176]|\302[\240-\277]|[\303-\337][\200-\277]|\340[\240-\277][\200-\277]|[\341\343-\354\356][\200-\277][\200-\277]|\342([\201-\277][\200-\277]|\200[\200-\247\252-\277])|\355[\200-\237][\200-\277]|\357([\200-\272\274-\276][\200-\277]|\273[\200-\276]|\277[\200-\275])|\360[\220-\277][\200-\277][\200-\277]|[\361-\363][\200-\277][\200-\277][\200-\277]|\364[\200-\217][\200-\277][\200-\277]";V="^(" A ")*$"}
{l=$0;if(length(l)>65536)no();if(NR==1)sub(/^\357\273\277/,"",l);sub(/\r$/,"",l);if(l!~V)no();ws=l;sub(/[^ \t].*/,"",ws);t0=substr(l,length(ws)+1);sp=l;sub(/[^ ].*/,"",sp);sp=length(sp);
if(bs){if(t0==""){if(index(substr(ws,1,bci==""?bk+1:bci),"\t"))no();next}if(bci==""&&sp>bk&&!index(substr(ws,1,sp+1),"\t"))bci=sp;if(bci!=""&&sp>=bci)next;if(sp>bk||index(ws,"\t"))no();bs=0}
if(index(ws,"\t"))no();if(t0=="")next;i=length(ws);
if(q){if(i<=qi)no();scan(t0,0);next}
if(dp>0){scan(t0,inh);if(inh)hz(bl);if(u)no();next}
if(t0~/^#/)next;
if(ended)no();
if(length(ws)==0&&t0~/^(---|[.][.][.])([ \t]|$)/){if(t0~/^[.]/){ended=1;next}if(seen0||trim(t0)!="---")no();seen0=1;r0="";next}
if(t0~/^%/){if(seen0||t0!~/^%YAML[ \t]/)no();dv=1;next}
if(dv&&!seen0)no();seen0=1;
s=trim(scan(t0,1));if(u)no();if(q)qi=i;t=trim(substr(t0,1,cut-1));if(match(s,/^[^:]*:([ \t]|$)/)&&substr(s,1,RLENGTH)!~/[[{]/){vs=substr(s,RLENGTH+1);if(vs!~/^[ \t]*[[{]/&&(vs~/:([ \t]|$)/||vs~/^[ \t]*-([ \t]|$)/))no()}
if(r0=="")r0=i;if(i<r0)no();i-=r0;while(ns&&st_[ns]>=i)ns--;st_[++ns]=i;if(ns>64)no();
if(s~/(^|:|-)[ \t]+[|>][-+0-9]*$/||s~/^[|>][-+0-9]*$/){tt=t;d=0;while(match(tt,/^-[ \t]+/)){d+=RLENGTH;tt=substr(tt,RLENGTH+1)}bs=1;bci="";bk=(tt~/^[|>]/)?i+r0:i+r0+d}
k="";v="";if(match(t,/^[^:]*:([ \t]|$)/)){k=substr(t,1,RLENGTH);sub(/:[ \t]*$/,"",k);k=trim(k);v=trim(substr(t,RLENGTH+1))}
if(i==0){shut();ing=0;inh=0;hi="";if(t~/^[?{[&*!]/)no();if(t~/^-/){if(lk==""||lk=="harness"||lv!="")no();next}if(k~/^["\047]/){w=unq(k);if(w==k||w=="harness")no();k=w}if(k==""||k=="<<"||(k in rk))no();rk[k]=1;lk=k;lv=v;if(k=="harness"){if(v=="")inh=1;else if(v!="~"&&v!="null"&&v!="Null"&&v!="NULL")no()}next}
if(!inh)next;hz(s);if(hi=="")hi=i;if(i<hi)no();
if(i==hi){shut();ing=0;if(t~/^[-?{[]/||k~/^["\047]/||k=="")no();hv=(v!="");if(k=="guardrails"){if(v!="")no();ing=1;gi=""}next}
if(!ing){if(hv)no();next}
if(gi=="")gi=i;if(i<gi)no();
if(i==gi){shut();if(t~/^[-?{[]/||k~/^["\047]/||k==""||(k in gk))no();gk[k]=1;if(v~/acceptance|protected-paths|editor-hook/||gsub(/\{/,"{",v)>1)no();gv=(v!="");if(k=="acceptance"||k=="protected-paths"||k=="editor-hook"){start(k);if(v~/^[{].*[}]$/){b=v;sub(/^[{]/,"",b);sub(/[}]$/,"",b);n=split(b,parts,",");for(j=1;j<=n;j++){e=trim(parts[j]);if(match(e,/^[^:]*:[ \t]+/)){kk=substr(e,1,RLENGTH);sub(/:[ \t]*$/,"",kk);item(trim(kk),trim(substr(e,RLENGTH+1)))}else u=1};shut()}else if(v!="")no()}next}
if(blk){if(bi=="")bi=i;if(i!=bi||v==""||k=="")no();item(k,v);next}
if(gv)no()}
END{if(dp>0||q)u=1;shut();if(!u&&!c)print "allow"}' < "$k" 2>/dev/null"##;

/// The configurations that govern the session, found without the binary: from
/// `CLAUDE_PROJECT_DIR` and from the working directory, each resolved to its
/// physical path first (a symlinked session directory is the directory it
/// points to), the nearest `.urzua/config.yaml` at or above it. `found` is set
/// when either finds one; `bad` when one is not a regular, readable, NUL-free
/// file that `CERTAIN_ALLOW` reads as open. The walk ends at the root whatever
/// `dirname` returns there (busybox gives `//` for `//`), so it always
/// terminates; nothing it reads comes from the payload.
const FIND_CONFIG: &str = r##"found=""; bad=""; for s in "${CLAUDE_PROJECT_DIR:-}" "$PWD"; do [ -n "$s" ] || continue; d=$(cd "$s" 2>/dev/null && pwd -P); [ -n "$d" ] || d=$s; while [ -n "$d" ]; do if [ -e "$d/.urzua/config.yaml" ]; then k="$d/.urzua/config.yaml"; found=1; if [ -f "$k" ] && [ -r "$k" ] && [ "$(tr -d '\000' < "$k" | wc -c)" -eq "$(wc -c < "$k")" ] && [ "$(CERTAIN)" = allow ]; then :; else bad=1; fi; break; fi; n=$(dirname "$d"); [ "$n" = "$d" ] && break; d=$n; done; done; "##;

const FIX: &str = "Install urzua (https://github.com/urzuahq/urzua/blob/main/docs/reference/install.md), then run: urzua harness status";

/// The hook command for `event`, or `None` for an event this adapter has no
/// hook for.
///
/// It runs `urzua harness hook claude-code <event>`, which reads the event on
/// stdin and exits 0 with its whole answer on stdout. Its stdout is captured
/// and printed only when it exits 0, so a failed run can never leave half an
/// answer in front of the fallback's. The fallback runs when the binary did not
/// answer: urzua is not on PATH (127), it is a release without `harness hook`
/// (exit 2 from its argument parser), or it was killed. Then nothing judged the
/// call, and nothing can load the configuration or read the payload, so the
/// fallback decides from the session alone. With no `.urzua/config.yaml` at or
/// above the session's directory nothing is governed and every hook exits
/// quietly: a plugin installed for every repository must not stop edits where
/// nothing is governed. With one that awk reads with certainty as letting an
/// unjudged edit through (`CERTAIN_ALLOW`), an edit goes through with a
/// notice. With any other, every edit is refused until urzua is installed or
/// upgraded: without the binary the fallback cannot tell a protected file from
/// another, or see which repository an edited file is in. The
/// post-command and session-start hooks only ever report. `$w` holds no quote
/// or backslash, so it is safe inside the JSON.
pub fn hook_command(event: &str) -> Option<String> {
    let certain = CERTAIN_ALLOW.replace('\n', " ");
    let find = FIND_CONFIG.replace("$(CERTAIN)", &format!("$({certain})"));
    // `command -v` finds the binary without running it, so a missing one
    // writes nothing to stderr. The fallback reads the payload only to drain
    // it, so the host's write never meets a closed pipe.
    let run = format!(
        "if command -v urzua >/dev/null 2>&1 && o=$(urzua harness hook {} {event}); then [ -z \"$o\" ] || printf '%s\\n' \"$o\"; exit 0; else c=$?; fi; cat >/dev/null 2>&1; ",
        super::NAME
    );
    let why = r#"if command -v urzua >/dev/null 2>&1; then w="urzua exited $c: it may be a release older than urzua harness hook"; else w="urzua is not on PATH"; fi; "#;
    let tail = match event {
        "edit" => format!(
            r#"{find}[ -n "$found" ] || exit 0; if [ -z "$bad" ]; then printf '%s\n' '{{"systemMessage":"urzua harness: the editor hook could NOT run ('"$w"'), so this edit was not judged; .urzua/config.yaml lets an unjudged edit through. {FIX}"}}'; exit 0; fi; printf '%s\n' '{{"hookSpecificOutput":{{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"urzua harness: the editor hook could NOT run ('"$w"'). Without it no edit in this repository can be judged, and .urzua/config.yaml does not let an unjudged edit through, so every edit is refused until urzua is installed or upgraded. A person can also edit outside the agent harness. {FIX}"}}}}'; exit 0"#
        ),
        "post-command" => format!(
            r#"{find}[ -n "$found" ] || exit 0; printf '%s\n' '{{"systemMessage":"urzua harness: the post-command check could NOT run ('"$w"'). {FIX}"}}'; exit 0"#
        ),
        "session-start" => format!(
            r#"{find}[ -n "$found" ] || exit 0; printf '%s\n' '{{"systemMessage":"urzua harness: the session-start check could NOT run ('"$w"'), so no guardrail can judge an edit in this session. {FIX}","hookSpecificOutput":{{"hookEventName":"SessionStart","additionalContext":"urzua harness: urzua could not run ('"$w"'); no edit is judged in this session. {FIX}"}}}}'; exit 0"#
        ),
        _ => return None,
    };
    Some(format!("{run}{why}{tail}"))
}
