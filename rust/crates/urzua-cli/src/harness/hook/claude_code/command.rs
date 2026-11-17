//! The hook commands this adapter registers, in the repository's settings file
//! (`urzua harness install`) and in the plugin's hooks file alike: both are
//! rendered from the one settings fragment, so they cannot drift.

/// The project's configuration when the binary cannot be run: the session's
/// project, else the nearest git repository at or above the working directory
/// that holds `.urzua/config.yaml`, else the working directory.
const FIND_PROJECT: &str = r#"r="${CLAUDE_PROJECT_DIR:-}"; if [ -z "$r" ]; then d="$PWD"; while [ -n "$d" ]; do top=$(git -C "$d" rev-parse --show-toplevel 2>/dev/null) || break; if [ -f "$top/.urzua/config.yaml" ]; then r="$top"; break; fi; d="${top%/*}"; done; [ -n "$r" ] || r="$PWD"; fi; "#;

/// Exit 0 when `.urzua/config.yaml` turns `acceptance` or `protected-paths` on
/// (`enabled: true`) without `on_unjudged: allow`, in the block form or the
/// inline `{...}` form, the two forms `urzua init` and the reference write.
const CONFIG_CLOSES: &str = r#"awk '{L[NR]=$0}END{for(i=1;i<=NR;i++){if(L[i]~/^[ \t]*(acceptance|protected-paths)[ \t]*:/){s=L[i];match(s,/[^ \t]/);n=RSTART-1;sub(/^[ \t]*(acceptance|protected-paths)[ \t]*:/,"",s);b=s;if(s~/^[ \t]*(#.*)?$/){b="";for(j=i+1;j<=NR;j++){q=L[j];if(q~/^[ \t]*(#.*)?$/)continue;match(q,/[^ \t]/);if(RSTART-1<=n)break;b=b" "q}}if(b~/enabled[ \t]*:[ \t]*true/&&b!~/on_unjudged[ \t]*:[ \t]*.?allow/)exit 0}}exit 1}' "$r/.urzua/config.yaml" 2>/dev/null"#;

const FIX: &str = "Install urzua (https://github.com/urzuahq/urzua/blob/main/docs/reference/install.md), then run: urzua harness status";

/// The hook command for `event`, or `None` for an event this adapter has no
/// hook for.
///
/// It runs `urzua harness hook claude-code <event>`, which reads the event on
/// stdin and always exits 0 with its answer on stdout. What follows `&& exit 0`
/// runs only when that did not happen: urzua is not on PATH (127), it is a
/// release without `harness hook` (exit 2 from its argument parser), or it was
/// killed. Then nothing judged the call. Without the binary nothing can load the
/// configuration, so the command reads `.urzua/config.yaml` as text (awk, which
/// every POSIX system has) for the one choice that matters here: when the
/// configuration turns `acceptance` or `protected-paths` on and leaves
/// `on_unjudged` at deny (its default for those two), an edit is refused, so a
/// missing binary closes the gate it would have enforced; otherwise the edit
/// proceeds with a message saying it was not judged. The post-command and
/// session-start hooks only ever report, so they only say what is wrong and how
/// to fix it. `$w` holds no quote or backslash, so it is safe inside the JSON.
pub fn hook_command(event: &str) -> Option<String> {
    let run = format!(
        "urzua harness hook {} {event} && exit 0; c=$?; ",
        super::NAME
    );
    let why = r#"if command -v urzua >/dev/null 2>&1; then w="urzua exited $c: it may be a release older than urzua harness hook"; else w="urzua is not on PATH"; fi; "#;
    let tail = match event {
        "edit" => format!(
            r#"{FIND_PROJECT}if {CONFIG_CLOSES}; then printf '%s\n' '{{"hookSpecificOutput":{{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"urzua harness: the editor hook could NOT run ('"$w"'), so this edit was not judged, and .urzua/config.yaml leaves acceptance or protected-paths at on_unjudged: deny, so it is refused. A person edits outside the agent harness (a plain editor or a terminal), or sets on_unjudged: allow in .urzua/config.yaml. {FIX}"}}}}'; exit 0; fi; printf '%s\n' '{{"systemMessage":"urzua harness: the editor hook could NOT run ('"$w"'), so this edit was not judged. {FIX}"}}'; exit 0"#
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
