//! The hook commands this adapter registers, in the repository's settings file
//! (`urzua harness install`) and in the plugin's hooks file alike: both are
//! rendered from the one settings fragment, so they cannot drift.

/// The session's project when the binary cannot be run: `CLAUDE_PROJECT_DIR`,
/// else the nearest git repository at or above the working directory that
/// holds `.urzua/config.yaml`, else the working directory. Then the
/// configuration that governs it: the nearest `.urzua/config.yaml` at or above
/// that directory, as the engine finds it.
const FIND_CONFIG: &str = r#"r="${CLAUDE_PROJECT_DIR:-}"; if [ -z "$r" ]; then d="$PWD"; while [ -n "$d" ]; do top=$(git -C "$d" rev-parse --show-toplevel 2>/dev/null) || break; if [ -f "$top/.urzua/config.yaml" ]; then r="$top"; break; fi; d="${top%/*}"; done; [ -n "$r" ] || r="$PWD"; fi; cfg=""; d="$r"; while [ -n "$d" ]; do if [ -e "$d/.urzua/config.yaml" ]; then cfg="$d/.urzua/config.yaml"; break; fi; [ "$d" = / ] && break; d=$(dirname "$d"); done; "#;

/// Prints `allow` only when it can read with certainty that neither
/// `acceptance` nor `protected-paths` refuses an unjudged edit; prints nothing
/// otherwise, and the command then refuses. Certain means: comments stripped,
/// no anchor, alias, tag, merge key or second document anywhere, and every line
/// that names either guardrail is exactly `<name>:` followed by more-indented
/// lines that are each exactly `enabled: true|false` or
/// `on_unjudged: allow|deny`, or exactly `<name>: {...}` holding only those
/// items. Any other shape (a quoted key, `True`, a flow map that spans lines,
/// the name anywhere else) is not certain. A guardrail is refusing when it is
/// enabled without `on_unjudged: allow`, the default for these two being deny.
/// Plain POSIX awk: no `match` arrays, no interval expressions, so mawk and
/// gawk read it the same.
const CERTAIN_ALLOW: &str = r#"awk 'function trim(s){sub(/^[ \t]+/,"",s);sub(/[ \t]+$/,"",s);return s} function item(x){if(x=="enabled: true")en=1;else if(x=="enabled: false")en=0;else if(x=="on_unjudged: allow")al=1;else if(x=="on_unjudged: deny")al=0;else u=1} function shut(){if(blk&&en&&!al)c=1;blk=0} {l=$0;sub(/\r$/,"",l);if(l~/^[ \t]*#/)l="";p=index(l," #");q=index(l,"\t#");if(q&&(!p||q<p))p=q;if(p)l=substr(l,1,p-1);if(l~/^[ \t]*$/)next;if(l~/(^|[ \t,:{[])[&*!]/||index(l,"<<")||(NR>1&&l~/^(---|[.][.][.])/))u=1;i=match(l,/[^ \t]/)-1;t=trim(l);if(blk){if(i>bi){item(t);next}shut()}if(l~/acceptance|protected-paths/){if(t=="acceptance:"||t=="protected-paths:"){blk=1;bi=i;en=0;al=0;next}if(t~/^(acceptance|protected-paths): [{][^{}]*[}]$/){b=t;sub(/^[^{]*[{]/,"",b);sub(/[}]$/,"",b);en=0;al=0;n=split(b,parts,",");for(k=1;k<=n;k++)item(trim(parts[k]));if(en&&!al)c=1;next}u=1}} END{shut();if(!u&&!c)print "allow"}' "$cfg" 2>/dev/null"#;

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
/// call, and nothing can load the configuration. For an edit the command fails
/// closed: it lets the edit through, with a message, only when awk reads with
/// certainty that no guardrail refuses an unjudged edit (`CERTAIN_ALLOW`); a
/// configuration it cannot read that way, an unreadable one, or none found is
/// a refusal that tells the person to install or upgrade urzua. The
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
            r#"{FIND_CONFIG}v=""; if [ -n "$cfg" ] && [ -r "$cfg" ]; then v=$({CERTAIN_ALLOW}); fi; if [ "$v" = allow ]; then printf '%s\n' '{{"systemMessage":"urzua harness: the editor hook could NOT run ('"$w"'), so this edit was not judged; .urzua/config.yaml lets an unjudged edit through. {FIX}"}}'; exit 0; fi; printf '%s\n' '{{"hookSpecificOutput":{{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"urzua harness: the editor hook could NOT run ('"$w"'), so this edit was not judged, and the command cannot read with certainty from .urzua/config.yaml that an unjudged edit may go through, so it is refused. Install or upgrade urzua; a person can also edit outside the agent harness. {FIX}"}}}}'; exit 0"#
        ),
        "post-command" => format!(
            r#"printf '%s\n' '{{"systemMessage":"urzua harness: the post-command check could NOT run ('"$w"'), so the effect of this command on records was NOT judged. {FIX}","hookSpecificOutput":{{"hookEventName":"PostToolUse","additionalContext":"urzua harness (post-command): NOT judged; run urzua check before committing."}}}}'; exit 0"#
        ),
        "session-start" => format!(
            r#"printf '%s\n' '{{"systemMessage":"urzua harness: the session-start check could NOT run ('"$w"'), so the harness was not checked and no guardrail can judge an edit in this session. {FIX}","hookSpecificOutput":{{"hookEventName":"SessionStart","additionalContext":"urzua harness: urzua could not run ('"$w"'); no edit is judged in this session. {FIX}"}}}}'; exit 0"#
        ),
        _ => return None,
    };
    Some(format!("{run}{why}{tail}"))
}
