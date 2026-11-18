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
/// allow|refuse`), a value plain or quoted.
///
/// Each line is scanned as YAML reads it: a quote opens a quoted scalar only
/// where a value starts (so `it's` stays plain), `#` after a blank starts a
/// comment only outside quotes, and brackets count flow depth only where a
/// value starts or inside a flow, carried across lines. The lines of a block
/// scalar (`|`, `>`) and of a multi-line flow collection are content, never
/// keys, so quoted scalars and flow collections anywhere outside the guardrails
/// are ordinary YAML. Not certain, so refused, wherever it appears: an anchor,
/// alias, tag or merge key outside quotes, a second document, a quote or flow
/// left open at the end of a line or of the file, a flow continuation line not
/// indented past its opening line, a tab in indentation, and a document whose
/// root is not a block mapping. Not certain at a key position that can name
/// `harness`, `guardrails` or a guardrail: a quoted key, a complex or flow
/// key, or a list item. In a guardrail block: a repeated key, a child indented
/// unlike its siblings, or a value on the next line. The guardrails written
/// inline at the `harness:` level are refused too. Plain POSIX awk (no `match`
/// arrays, no interval expressions, `\047` for a quote) fed on stdin so no
/// path is read as an assignment; written over lines here and joined into one
/// for the command. mawk, gawk and busybox awk give the same answer on every
/// shape the tests feed.
const CERTAIN_ALLOW: &str = r##"awk 'function trim(s){sub(/^[ \t]+/,"",s);sub(/[ \t]+$/,"",s);return s}
function unq(v){if(v~/^"[^"\\]*"$/||v~/^\047[^\047]*\047$/)return substr(v,2,length(v)-2);return v}
function item(k,v,  w){if(k in seen)u=1;seen[k]=1;w=unq(v);if(k=="enabled"&&v=="true")en=1;else if(k=="enabled"&&v=="false")en=0;else if(k=="on_unjudged"&&w=="allow"){al=1;as=1}else if(k=="on_unjudged"&&w=="deny"){al=0;as=1}else if(k=="pending_pair"&&nm=="editor-hook"&&(w=="allow"||w=="refuse"))x=1;else u=1}
function shut(){if(blk&&en&&(nm=="editor-hook"?(as&&!al):!al))c=1;blk=0}
function start(n){shut();nm=n;blk=1;bi="";en=0;al=0;as=0;split("",seen)}
function scan(l,  n,j,ch,lt,gap,st){n=length(l);bl="";cut=l;lt="";gap=1;q=0;for(j=1;j<=n;j++){ch=substr(l,j,1);if(q==1){if(ch=="\047"){if(substr(l,j+1,1)=="\047"){j++;bl=bl "xx";continue}q=0;bl=bl ch}else bl=bl "x";continue}if(q==2){if(ch=="\\"){j++;bl=bl "xx";continue}if(ch=="\""){q=0;bl=bl ch}else bl=bl "x";continue}if(ch==" "||ch=="\t"){gap=1;bl=bl ch;continue}if(ch=="#"&&gap){cut=substr(l,1,j-1);break}st=(lt==""||index("[{,",lt)>0||(gap&&index(":-?",lt)>0));if((ch=="\047"||ch=="\"")&&st)q=(ch=="\047")?1:2;else if((ch=="["||ch=="{")&&(st||dp>0))dp++;else if((ch=="]"||ch=="}")&&dp>0)dp--;bl=bl ch;lt=ch;gap=0}return bl}
function hz(s){if(s~/:[ \t]+[&*!]/||s~/^(-[ \t]+)+[&*!]/||s~/[[{,][ \t]*[&*!][^ \t,]/||s~/^[&*!]/||s~/<<[ \t]*:/)u=1}
{l=$0;sub(/\r$/,"",l);if(NR==1)sub(/^\357\273\277/,"",l);ws=l;sub(/[^ \t].*/,"",ws);i=length(ws);t0=substr(l,i+1);
if(bs){if(t0==""||i>bk)next;bs=0}
if(t0==""||t0~/^#/)next;
if(index(ws,"\t"))u=1;
if(dp>0){if(i<=fi)u=1;s=trim(scan(t0));if(q)u=1;hz(s);next}
s=trim(scan(t0));if(q)u=1;t=trim(cut);if(t=="")next;hz(s);if(dp>0)fi=i;
if(i==0&&t~/^(---|[.][.][.])([ \t]|$)/){if(seen0||t!="---")u=1;seen0=1;next}seen0=1;
if(s~/(^|:|-)[ \t]+[|>][-+0-9]*$/||s~/^[|>][-+0-9]*$/){tt=t;d=0;while(match(tt,/^-[ \t]+/)){d+=RLENGTH;tt=substr(tt,RLENGTH+1)}bs=1;bk=(tt~/^[|>]/)?i:i+d}
k="";v="";if(match(t,/^[^:]*:([ \t]|$)/)){k=substr(t,1,RLENGTH);sub(/:[ \t]*$/,"",k);v=trim(substr(t,RLENGTH+1))};qk=(k~/^["\047]/||t~/^[?{[]/);
if(i==0){shut();ing=0;inh=0;hi="";if(qk)u=1;if(t~/^-/){if(lk==""||lk=="harness"||lv!="")u=1;next}lk=k;lv=v;if(k=="harness"){if(v!="")u=1;else inh=1};next};
if(!inh)next;if(hi=="")hi=i;if(i<hi){u=1;next};
if(i==hi){shut();ing=0;if(qk||t~/^-/)u=1;if(k=="guardrails"){if(v!="")u=1;else{ing=1;gi=""}};next};
if(!ing)next;if(gi=="")gi=i;if(i<gi){u=1;next};
if(i==gi){shut();if(qk||t~/^-/)u=1;if(v~/acceptance|protected-paths|editor-hook/||gsub(/\{/,"{",v)>1)u=1;if(k=="acceptance"||k=="protected-paths"||k=="editor-hook"){start(k);if(v~/^[{].*[}]$/){b=v;sub(/^[{]/,"",b);sub(/[}]$/,"",b);n=split(b,parts,",");for(j=1;j<=n;j++){e=trim(parts[j]);if(match(e,/^[^:]*:[ \t]+/)){kk=substr(e,1,RLENGTH);sub(/:[ \t]*$/,"",kk);item(kk,trim(substr(e,RLENGTH+1)))}else u=1};shut()}else if(v!="")u=1};next};
if(!blk)next;if(bi=="")bi=i;if(i!=bi){u=1;next};if(v=="")u=1;item(k,v)}
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
