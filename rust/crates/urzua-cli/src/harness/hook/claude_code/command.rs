//! The hook commands this adapter registers, in the repository's settings file
//! (`urzua harness install`) and in the plugin's hooks file alike: both are
//! rendered from the one settings fragment, so they cannot drift.

/// The session's project when the binary cannot be run: `CLAUDE_PROJECT_DIR`,
/// else the nearest git repository at or above the working directory that
/// holds `.urzua/config.yaml`, else the working directory. Then the
/// configuration that governs it: the nearest `.urzua/config.yaml` at or above
/// that directory, as the engine finds it.
const FIND_CONFIG: &str = r#"r="${CLAUDE_PROJECT_DIR:-}"; if [ -z "$r" ]; then d="$PWD"; while [ -n "$d" ]; do top=$(git -C "$d" rev-parse --show-toplevel 2>/dev/null) || break; if [ -f "$top/.urzua/config.yaml" ]; then r="$top"; break; fi; d="${top%/*}"; done; [ -n "$r" ] || r="$PWD"; fi; cfg=""; d="$r"; while [ -n "$d" ]; do if [ -e "$d/.urzua/config.yaml" ]; then cfg="$d/.urzua/config.yaml"; break; fi; [ "$d" = / ] && break; d=$(dirname "$d"); done; "#;

/// Prints `allow` only when it reads `$k` with certainty as letting an
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
const CERTAIN_ALLOW: &str = r#"awk 'function trim(s){sub(/^[ \t]+/,"",s);sub(/[ \t]+$/,"",s);return s} function item(x){if(x=="enabled: true")en=1;else if(x=="enabled: false")en=0;else if(x=="on_unjudged: allow"){al=1;as=1}else if(x=="on_unjudged: deny"){al=0;as=1}else u=1} function closes(){return en&&(nm=="editor-hook"?(as&&!al):!al)} function shut(){if(blk&&closes())c=1;blk=0} {l=$0;sub(/\r$/,"",l);if(l~/^[ \t]*#/)l="";p=index(l," #");q=index(l,"\t#");if(q&&(!p||q<p))p=q;if(p)l=substr(l,1,p-1);if(l~/^[ \t]*$/)next;if(l~/(^|[ \t,:{[])[&*!]/||index(l,"<<")||(NR>1&&l~/^(---|[.][.][.])/))u=1;i=match(l,/[^ \t]/)-1;t=trim(l);if(blk){if(i>bi){item(t);next}shut()}if(l~/acceptance|protected-paths|editor-hook/){en=0;al=0;as=0;if(t=="acceptance:"||t=="protected-paths:"||t=="editor-hook:"){nm=substr(t,1,length(t)-1);blk=1;bi=i;next}if(t~/^(acceptance|protected-paths|editor-hook): [{][^{}]*[}]$/){nm=t;sub(/:.*/,"",nm);b=t;sub(/^[^{]*[{]/,"",b);sub(/[}]$/,"",b);n=split(b,parts,",");for(k=1;k<=n;k++)item(trim(parts[k]));if(closes())c=1;next}u=1}} END{shut();if(!u&&!c)print "allow"}' "$k" 2>/dev/null"#;

/// Every configuration that governs the edit, as far as the shell can place
/// it: the session's (`$cfg`), and every `.urzua/config.yaml` at or above each
/// `file_path`, `notebook_path` or `path` string in the payload, walked both as
/// written and through the physical path of its nearest existing directory. A
/// path the shell cannot place (an escape inside the string, or a final
/// component that is a symlink) sets `bad`, a refusal. Paths under any other
/// key, as an MCP call may use, are not read: the documented limit of a
/// fallback that has no JSON parser.
const PAYLOAD_CONFIGS: &str = r#"nl=$(printf '\n_'); nl=${nl%_}; cs="$cfg"; bad=""; printf '%s' "$p" | grep -qE '"(file_path|notebook_path|path)"[[:space:]]*:[[:space:]]*"[^"]*\\' && bad=1; fs=$(printf '%s' "$p" | grep -oE '"(file_path|notebook_path|path)"[[:space:]]*:[[:space:]]*"[^"\\]*"' | sed -e 's/^[^:]*:[[:space:]]*"//' -e 's/"$//'); set -f; o_ifs=$IFS; IFS=$nl; for f in $fs; do case "$f" in /*) ;; *) f="$PWD/$f";; esac; [ -L "$f" ] && bad=1; d=$(dirname "$f"); while [ ! -d "$d" ]; do d=$(dirname "$d"); done; for e in "$d" "$(cd -P "$d" 2>/dev/null && pwd -P)"; do while [ -n "$e" ]; do [ -e "$e/.urzua/config.yaml" ] && cs="$cs$nl$e/.urzua/config.yaml"; [ "$e" = / ] && break; e=$(dirname "$e"); done; done; done; IFS=$o_ifs; set +f; "#;

/// `allow` in `$v` when every configuration in `$cs` reads with certainty as
/// open and no path was unplaceable.
const ALL_CERTAIN: &str = r#"v=allow; [ -z "$bad" ] || v=""; set -f; IFS=$nl; for k in $cs; do if [ -r "$k" ]; then [ "$(CERTAIN)" = allow ] || v=""; else v=""; fi; done; IFS=$o_ifs; set +f; "#;

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
/// call, and nothing can load the configuration. The payload is read once into
/// `$p` so the fallback can look at it too. With no `.urzua/config.yaml` at or
/// above the project nor any edited path the shell can read, nothing is
/// governed and every hook exits quietly: a plugin installed for every
/// repository must not stop edits where nothing is governed. Otherwise, for an
/// edit the command fails closed: it lets the edit through, with a message,
/// only when every configuration found reads with certainty as open
/// (`CERTAIN_ALLOW`); any other, an unreadable one, or a path it cannot place
/// is a refusal that tells the person to install or upgrade urzua. The
/// post-command and session-start hooks only ever report. `$w` holds no quote
/// or backslash, so it is safe inside the JSON.
pub fn hook_command(event: &str) -> Option<String> {
    let run = format!(
        "p=$(cat); if o=$(printf '%s' \"$p\" | urzua harness hook {} {event}); then [ -z \"$o\" ] || printf '%s\\n' \"$o\"; exit 0; else c=$?; fi; ",
        super::NAME
    );
    let why = r#"if command -v urzua >/dev/null 2>&1; then w="urzua exited $c: it may be a release older than urzua harness hook"; else w="urzua is not on PATH"; fi; "#;
    let tail = match event {
        "edit" => format!(
            r#"{FIND_CONFIG}{PAYLOAD_CONFIGS}[ -n "$cs$bad" ] || exit 0; {all}if [ "$v" = allow ]; then printf '%s\n' '{{"systemMessage":"urzua harness: the editor hook could NOT run ('"$w"'), so this edit was not judged; every .urzua/config.yaml that governs it lets an unjudged edit through. {FIX}"}}'; exit 0; fi; printf '%s\n' '{{"hookSpecificOutput":{{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"urzua harness: the editor hook could NOT run ('"$w"'), so this edit was not judged, and the command cannot read with certainty from the .urzua/config.yaml that governs it that an unjudged edit may go through, so it is refused. Install or upgrade urzua; a person can also edit outside the agent harness. {FIX}"}}}}'; exit 0"#,
            all = ALL_CERTAIN.replace("$(CERTAIN)", &format!("$({CERTAIN_ALLOW})"))
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
