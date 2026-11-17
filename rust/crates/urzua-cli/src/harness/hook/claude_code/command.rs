//! The hook commands this adapter registers, in the repository's settings file
//! (`urzua harness install`) and in the plugin's hooks file alike: both are
//! rendered from the one settings fragment, so they cannot drift.

/// The configuration that governs the session, found without the binary: the
/// nearest `.urzua/config.yaml` at or above `CLAUDE_PROJECT_DIR`, else the
/// working directory, in `$cfg` (empty when there is none). The walk ends at
/// the root whatever `dirname` returns there (busybox gives `//` for `//`), so
/// it always terminates; nothing it reads comes from the payload.
const FIND_CONFIG: &str = r#"cfg=""; d="${CLAUDE_PROJECT_DIR:-$PWD}"; while [ -n "$d" ]; do if [ -e "$d/.urzua/config.yaml" ]; then cfg="$d/.urzua/config.yaml"; break; fi; n=$(dirname "$d"); [ "$n" = "$d" ] && break; d=$n; done; "#;

/// Prints `allow` only when it reads `$cfg` with certainty as letting an
/// unjudged edit through, by the rule the binary uses: neither `acceptance` nor
/// `protected-paths` enabled without `on_unjudged: allow` (their default being
/// deny), and `editor-hook` not enabled with `on_unjudged: deny`. It prints
/// nothing otherwise, and the command then refuses. Certain means: comments
/// stripped, no anchor, alias, tag, merge key or second document anywhere, and
/// every line that names a guardrail is exactly `<name>:` followed by
/// more-indented lines that are each exactly `enabled: true|false` or
/// `on_unjudged: allow|deny`, or exactly `<name>: {...}` holding only those
/// items. Any other shape (a quoted key, `True`, a flow map that spans lines,
/// the name anywhere else) is not certain. Plain POSIX awk: no `match` arrays,
/// no interval expressions, so mawk and gawk read it the same.
const CERTAIN_ALLOW: &str = r#"awk 'function trim(s){sub(/^[ \t]+/,"",s);sub(/[ \t]+$/,"",s);return s} function item(x){if(x=="enabled: true")en=1;else if(x=="enabled: false")en=0;else if(x=="on_unjudged: allow"){al=1;as=1}else if(x=="on_unjudged: deny"){al=0;as=1}else u=1} function closes(){return en&&(nm=="editor-hook"?(as&&!al):!al)} function shut(){if(blk&&closes())c=1;blk=0} {l=$0;sub(/\r$/,"",l);if(l~/^[ \t]*#/)l="";p=index(l," #");q=index(l,"\t#");if(q&&(!p||q<p))p=q;if(p)l=substr(l,1,p-1);if(l~/^[ \t]*$/)next;if(l~/(^|[ \t,:{[])[&*!]/||index(l,"<<")||(NR>1&&l~/^(---|[.][.][.])/))u=1;i=match(l,/[^ \t]/)-1;t=trim(l);if(blk){if(i>bi){item(t);next}shut()}if(l~/acceptance|protected-paths|editor-hook/){en=0;al=0;as=0;if(t=="acceptance:"||t=="protected-paths:"||t=="editor-hook:"){nm=substr(t,1,length(t)-1);blk=1;bi=i;next}if(t~/^(acceptance|protected-paths|editor-hook): [{][^{}]*[}]$/){nm=t;sub(/:.*/,"",nm);b=t;sub(/^[^{]*[{]/,"",b);sub(/[}]$/,"",b);n=split(b,parts,",");for(k=1;k<=n;k++)item(trim(parts[k]));if(closes())c=1;next}u=1}} END{shut();if(!u&&!c)print "allow"}' "$cfg" 2>/dev/null"#;

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
    let run = format!(
        "if o=$(urzua harness hook {} {event}); then [ -z \"$o\" ] || printf '%s\\n' \"$o\"; exit 0; else c=$?; fi; ",
        super::NAME
    );
    let why = r#"if command -v urzua >/dev/null 2>&1; then w="urzua exited $c: it may be a release older than urzua harness hook"; else w="urzua is not on PATH"; fi; "#;
    let tail = match event {
        "edit" => format!(
            r#"{FIND_CONFIG}[ -n "$cfg" ] || exit 0; v=""; if [ -r "$cfg" ]; then v=$({CERTAIN_ALLOW}); fi; if [ "$v" = allow ]; then printf '%s\n' '{{"systemMessage":"urzua harness: the editor hook could NOT run ('"$w"'), so this edit was not judged; .urzua/config.yaml lets an unjudged edit through. {FIX}"}}'; exit 0; fi; printf '%s\n' '{{"hookSpecificOutput":{{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"urzua harness: the editor hook could NOT run ('"$w"'). Without it no edit in this repository can be judged, and .urzua/config.yaml does not let an unjudged edit through, so every edit is refused until urzua is installed or upgraded. A person can also edit outside the agent harness. {FIX}"}}}}'; exit 0"#
        ),
        "post-command" => format!(
            r#"{FIND_CONFIG}[ -n "$cfg" ] || exit 0; printf '%s\n' '{{"systemMessage":"urzua harness: the post-command check could NOT run ('"$w"'), so the effect of this command on records was NOT judged. {FIX}","hookSpecificOutput":{{"hookEventName":"PostToolUse","additionalContext":"urzua harness (post-command): NOT judged; run urzua check before committing."}}}}'; exit 0"#
        ),
        "session-start" => format!(
            r#"{FIND_CONFIG}[ -n "$cfg" ] || exit 0; printf '%s\n' '{{"systemMessage":"urzua harness: the session-start check could NOT run ('"$w"'), so the harness was not checked and no guardrail can judge an edit in this session. {FIX}","hookSpecificOutput":{{"hookEventName":"SessionStart","additionalContext":"urzua harness: urzua could not run ('"$w"'); no edit is judged in this session. {FIX}"}}}}'; exit 0"#
        ),
        _ => return None,
    };
    Some(format!("{run}{why}{tail}"))
}
