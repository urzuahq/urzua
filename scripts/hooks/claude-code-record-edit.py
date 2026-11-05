#!/usr/bin/env python3
"""Claude Code PreToolUse adapter for the record-edit hook (ADR-69, MILE-178).

Thin: it turns a Write / Edit / MultiEdit payload into the proposed full content
of the file, hands that to scripts/hooks/record-edit-hook.sh (the harness-neutral
core, which relays `urzua check --stdin-path`), and turns the core's verdict into Claude
Code's hook output. It holds no derivation and no record knowledge.

Run with `python3 -I`. A payload it cannot turn into proposed content (an Edit
whose old_string is not in the file) is allowed with a loud message; the
harness's own tool will report the failed edit.
"""
import json
import os
import subprocess
import sys

CORE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "record-edit-hook.sh")
NOT_JUDGED = os.path.join(os.path.dirname(os.path.abspath(__file__)), "record-edit-not-judged.sh")


def proposed(tool, inp, path):
    if tool == "Write":
        return inp.get("content", "")
    edits = inp.get("edits") if tool == "MultiEdit" else [inp]
    try:
        with open(path, encoding="utf-8", newline="") as f:
            text = f.read()
    except OSError as e:
        raise ValueError(f"cannot read {path}: {e}")
    for e in edits:
        old, new = e.get("old_string", ""), e.get("new_string", "")
        n = text.count(old) if old else 0
        if n == 0:
            raise ValueError("an old_string is not in the file")
        if n > 1 and not e.get("replace_all"):
            raise ValueError("an old_string is not unique in the file")
        text = text.replace(old, new) if e.get("replace_all") else text.replace(old, new, 1)
    return text


def not_judged(why):
    """Fail open, but visibly: the shared helper prints a stderr notice and logs a row."""
    try:
        p = subprocess.run(["bash", NOT_JUDGED, why], capture_output=True, text=True)
        sys.stderr.write(p.stderr)
        return p.stdout.strip()
    except OSError:
        msg = f"record-edit-hook: urzua did NOT judge this edit ({why}). It is allowed."
        sys.stderr.write(msg + "\n")
        return msg


def out(obj):
    json.dump(obj, sys.stdout)
    sys.stdout.write("\n")


def main():
    try:
        payload = json.load(sys.stdin)
    except ValueError:
        out({"systemMessage": not_judged("unreadable hook payload")})
        return 0
    tool = payload.get("tool_name")
    inp = payload.get("tool_input") or {}
    path = inp.get("file_path")
    if tool not in ("Write", "Edit", "MultiEdit") or not path:
        return 0
    try:
        content = proposed(tool, inp, path)
    except ValueError as e:
        msg = not_judged(f"{tool} of {path}: {e}")
        out({"systemMessage": msg, "hookSpecificOutput": {"hookEventName": "PreToolUse", "additionalContext": msg}})
        return 0
    cwd = payload.get("cwd") or None
    try:
        p = subprocess.run(["bash", CORE, path], input=content, capture_output=True, text=True, cwd=cwd)
        v = json.loads(p.stdout)
    except (OSError, ValueError):
        msg = not_judged(f"the core hook failed for {path}")
        out({"systemMessage": msg, "hookSpecificOutput": {"hookEventName": "PreToolUse", "additionalContext": msg}})
        return 0
    verdict, msg = v.get("verdict"), v.get("message", "")
    if verdict == "block":
        out({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny", "permissionDecisionReason": msg}})
    elif verdict in ("allow-unjudged", "allow-override", "allow-with-warnings"):
        out({"systemMessage": msg, "hookSpecificOutput": {"hookEventName": "PreToolUse", "additionalContext": msg}})
    return 0


sys.exit(main())
