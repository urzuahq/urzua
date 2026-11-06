#!/usr/bin/env python3
"""Claude Code translator for the urzua harness (ADR-89 section 6).

Installed by `urzua harness install --adapter claude-code`; do not edit it, the
install keeps its hash in .urzua/harness.lock and `urzua harness status` reports a
change. Customise the harness in .urzua/config.yaml instead.

    python3 -I urzua-claude-code.py EVENT -- urzua harness run EVENT

It turns Claude Code's hook payload into urzua's neutral request, runs the engine
command given after `--`, and turns the neutral verdict into Claude Code's hook
output. It holds no rule knowledge, and it takes the project directory from the
payload's `cwd`, never from CLAUDE_PROJECT_DIR: an unset variable must not become
an opaque blocking exit 2.

It always exits 0 after printing a stated message when the engine cannot be run:
the edit proceeds, and the message says it was not judged.
"""
import json
import os
import subprocess
import sys

EDIT_TOOLS = ("Write", "Edit", "MultiEdit")


def out(obj):
    json.dump(obj, sys.stdout)
    sys.stdout.write("\n")


def stated(message, event):
    """The engine could not give a verdict: say so, loudly, and let the call proceed."""
    sys.stderr.write(message + "\n")
    hook = {"session-start": "SessionStart", "post-command": "PostToolUse"}.get(event, "PreToolUse")
    out({"systemMessage": message, "hookSpecificOutput": {"hookEventName": hook, "additionalContext": message}})
    return 0


def proposed(tool, inp, path):
    if tool == "Write":
        return inp.get("content", "")
    edits = inp.get("edits") if tool == "MultiEdit" else [inp]
    try:
        with open(path, encoding="utf-8", newline="") as f:
            text = f.read()
    except OSError as e:
        raise ValueError("cannot read %s: %s" % (path, e))
    for e in edits:
        old, new = e.get("old_string", ""), e.get("new_string", "")
        n = text.count(old) if old else 0
        if n == 0:
            raise ValueError("an old_string is not in the file")
        if n > 1 and not e.get("replace_all"):
            raise ValueError("an old_string is not unique in the file")
        text = text.replace(old, new) if e.get("replace_all") else text.replace(old, new, 1)
    return text


def engine(argv, request, cwd):
    """Runs the engine command with the request on stdin; returns the verdict object."""
    argv = list(argv)
    # A binary that is not on PATH (a build directory, a pinned install) is named here.
    if argv and argv[0] == "urzua" and os.environ.get("URZUA_BIN"):
        argv[0] = os.environ["URZUA_BIN"]
    p = subprocess.run(argv, input=json.dumps(request), capture_output=True, text=True, cwd=cwd or None)
    return json.loads(p.stdout)


def main():
    args = sys.argv[1:]
    if not args or "--" not in args[1:]:
        return stated("urzua harness: the translator was called without an event and an engine command; run `urzua harness status`", "edit")
    event = args[0]
    argv = args[args.index("--") + 1:]
    if event not in ("edit", "session-start", "post-command") or not argv:
        return stated("urzua harness: the translator does not know the event '%s'; run `urzua harness update`" % event, "edit")
    try:
        payload = json.load(sys.stdin)
    except ValueError:
        return stated("urzua harness: the hook payload was unreadable, so this call was NOT judged", event)
    cwd = payload.get("cwd") or None
    if event == "session-start":
        request = {"event": "session-start", "tool": None, "paths": [], "content": None, "actor": "agent", "cwd": cwd}
    elif event == "post-command":
        # Any Bash command. The engine reads git, never the command.
        if payload.get("tool_name") != "Bash":
            return 0
        request = {"event": "post-command", "tool": "Bash", "paths": [], "content": None, "actor": "agent", "cwd": cwd}
    else:
        tool = payload.get("tool_name")
        inp = payload.get("tool_input") or {}
        path = inp.get("file_path")
        if tool not in EDIT_TOOLS or not path:
            return 0
        try:
            content = proposed(tool, inp, path)
        except ValueError as e:
            return stated("urzua harness: urzua did NOT judge this edit (%s of %s: %s). It is allowed; run 'urzua check' before committing." % (tool, path, e), event)
        request = {"event": "edit", "tool": tool, "paths": [path], "content": content, "actor": "agent", "cwd": cwd}
    try:
        verdict = engine(argv, request, cwd)
    except (OSError, ValueError):
        return stated("urzua harness: urzua did NOT judge this call (the engine command `%s` could not be run or answered with something that is not a verdict). It is allowed; run `urzua harness status`." % " ".join(argv), event)
    kind, message = verdict.get("verdict"), verdict.get("message", "")
    if event == "session-start":
        if kind != "allow" and message:
            out({"systemMessage": message, "hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": message}})
        return 0
    if event == "post-command":
        # Context for the agent, never a decision: the command has already run.
        if kind != "allow" and message:
            out({"systemMessage": message, "hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": message}})
        return 0
    if kind == "block":
        out({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny", "permissionDecisionReason": message}})
    elif kind in ("allow-unjudged", "allow-override", "allow-with-warnings"):
        out({"systemMessage": message, "hookSpecificOutput": {"hookEventName": "PreToolUse", "additionalContext": message}})
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # never a bare traceback and a blocking exit
        sys.exit(stated("urzua harness: the translator failed (%s), so this call was NOT judged" % e, "edit"))
