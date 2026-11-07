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

When it cannot build a request or the engine cannot be run, it exits 0 after
printing a stated message and the edit proceeds, with one exception: where the
configuration turns on the acceptance or protected-paths guardrail and leaves
`on_unjudged: deny`, an edit that was not judged is refused. The engine decides
that when it can be run (the translator sends it the failure); when it cannot,
the translator looks for the same two settings in .urzua/config.yaml as text.
An adapter failure never blocks when the configuration allows.

The editor hook covers Write, Edit, MultiEdit and NotebookEdit, and any MCP tool
(`mcp__*`). For an MCP call it reads every path-like field, at any depth and in
any list (`path`, `file`, `from`, `source`, `destination`, `uri` and the like, a
`file://` URI included), and judges each one. An object with one path field and
a content field is judged as a write of that content to that path. A path that
belongs to a call that looks like an edit, a move or a delete (an `edits` list,
`old_string`, a tool name such as `edit_file`, `move_file` or `delete_file`) or
that sits where the translator cannot place it cannot be turned into the file's
new text, so it is an edit that was not judged: refused only where the target is
a gated record or a protected path and the configuration says deny. A call to a
tool named for reading (`read_file`, `list_directory`) is left alone, and so is
a call with no string that names a file.
"""
import json
import os
import re
import subprocess
import sys
from urllib.parse import unquote, urlparse

EDIT_TOOLS = ("Write", "Edit", "MultiEdit", "NotebookEdit")

# How an MCP tool names the file it writes and the text it writes. None of this
# is rule knowledge: it only finds the paths and the content in a payload.
MCP_CONTENT_KEYS = ("content", "contents", "file_text", "fileText", "text", "new_source")
MCP_EDIT_KEYS = ("edits", "old_string", "new_string", "old_str", "new_str", "old_text", "new_text", "oldText", "newText", "patch", "diff", "changes", "operations", "insert_text", "replacement", "append")
# A key names a path when one of its words is one of these (`filePath`, `from`,
# `source_path`, `destination`, `uri`).
MCP_PATH_WORDS = {"path", "paths", "file", "files", "filepath", "filename", "source", "src", "dest", "destination", "target", "from", "to", "uri", "url", "dir", "directory", "folder", "old", "new"}
MCP_PARTIAL_WORDS = {"append", "insert", "patch", "edit", "replace", "modify", "update", "move", "rename", "delete", "remove", "copy"}
MCP_WRITE_WORDS = MCP_PARTIAL_WORDS | {"write", "create", "save", "put", "overwrite", "mv", "rm", "touch", "mkdir", "set", "upload"}
MCP_READ_WORDS = {"read", "get", "list", "search", "find", "stat", "view", "show", "cat", "head", "tail", "fetch", "query", "describe", "info", "tree", "grep", "glob", "exists", "diff", "status", "log", "count", "check", "lookup", "browse", "inspect"}
MCP_MAX_DEPTH = 24
MCP_MAX_LOOSE = 64


def out(obj):
    json.dump(obj, sys.stdout)
    sys.stdout.write("\n")


def stated(message, event):
    """The engine could not give a verdict: say so, loudly, and let the call proceed."""
    sys.stderr.write(message + "\n")
    hook = {"session-start": "SessionStart", "post-command": "PostToolUse"}.get(event, "PreToolUse")
    out({"systemMessage": message, "hookSpecificOutput": {"hookEventName": hook, "additionalContext": message}})
    return 0


def deny(message):
    out({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny", "permissionDecisionReason": message}})
    return 0


def closed_by_config_text(cwd):
    """Last resort, only when the engine cannot be run: whether .urzua/config.yaml
    turns on acceptance or protected-paths without `on_unjudged: allow`. A text
    scan, so a configuration it cannot read is read as not closed."""
    d = os.path.abspath(cwd or os.getcwd())
    while True:
        try:
            with open(os.path.join(d, ".urzua", "config.yaml"), encoding="utf-8") as f:
                lines = f.read().splitlines()
            break
        except OSError:
            parent = os.path.dirname(d)
            if parent == d:
                return False
            d = parent
    for i, line in enumerate(lines):
        m = re.match(r"^(\s*)(acceptance|protected-paths)\s*:\s*(.*)$", line)
        if not m:
            continue
        indent, body = len(m.group(1)), m.group(3)
        if body == "" or body.startswith("#"):
            body = ""
            for nxt in lines[i + 1:]:
                if nxt.strip() and len(nxt) - len(nxt.lstrip()) <= indent:
                    break
                body += " " + nxt
        if re.search(r"\benabled\s*:\s*true\b", body) and not re.search(r"\bon_unjudged\s*:\s*[\"']?allow\b", body):
            return True
    return False


def unjudged_result(message, argv, cwd, path, engine_failed, loud=True):
    """An edit the translator could not judge, as (kind, message): the engine
    says whether the configuration refuses it (acceptance or protected-paths
    with on_unjudged: deny); the engine being unrunnable falls back to the
    config text. A call that is not `loud` (one that may not be an edit at all)
    says nothing unless it is refused."""
    paths = path if isinstance(path, list) else ([path] if path else [])
    if not engine_failed and argv:
        request = {"event": "edit", "tool": None, "paths": paths, "content": None, "actor": "agent", "cwd": cwd, "adapter_error": message}
        try:
            verdict = engine(argv, request, cwd)
            if verdict.get("verdict") == "block":
                return ("block", verdict.get("message", message))
            return ("stated", verdict.get("message") or message) if loud else ("allow", "")
        except (OSError, ValueError, AttributeError):
            pass
    if loud and closed_by_config_text(cwd):
        return ("block", message + " The configuration sets on_unjudged: deny for acceptance or protected-paths, so an agent's edit that was not judged is refused. A person edits outside the agent harness, or sets on_unjudged: allow in .urzua/config.yaml.")
    return ("stated", message) if loud else ("allow", "")


def decide(results):
    """Prints one answer for the edit judgements in `results`, (kind, message)
    pairs: any block refuses the call, the other messages are shown."""
    blocks = [m for k, m in results if k == "block"]
    if blocks:
        return deny("\n".join(blocks))
    stated_notes = [m for k, m in results if k == "stated" and m]
    notes = [m for k, m in results if k == "note" and m]
    if stated_notes:
        return stated("\n".join(stated_notes + notes), "edit")
    if notes:
        message = "\n".join(notes)
        out({"systemMessage": message, "hookSpecificOutput": {"hookEventName": "PreToolUse", "additionalContext": message}})
    return 0


def unjudged_edit(message, argv, cwd, path, engine_failed):
    return decide([unjudged_result(message, argv, cwd, path, engine_failed)])


def notebook_proposed(inp, path):
    """The notebook as it will be once NotebookEdit has run."""
    try:
        with open(path, encoding="utf-8", newline="") as f:
            nb = json.load(f)
    except (OSError, ValueError) as e:
        raise ValueError("cannot read notebook %s: %s" % (path, e))
    cells = nb.get("cells") if isinstance(nb, dict) else None
    if not isinstance(cells, list):
        raise ValueError("%s is not a notebook with cells" % path)
    mode, cell_id, source = inp.get("edit_mode") or "replace", inp.get("cell_id"), inp.get("new_source", "")
    index = None
    if cell_id is not None:
        for i, c in enumerate(cells):
            if isinstance(c, dict) and c.get("id") == cell_id:
                index = i
                break
        if index is None and str(cell_id).isdigit() and int(cell_id) < len(cells):
            index = int(cell_id)
        if index is None:
            raise ValueError("the cell %s is not in the notebook" % cell_id)
    if mode == "insert":
        kind = inp.get("cell_type") or "code"
        cell = {"cell_type": kind, "metadata": {}, "source": source}
        if kind == "code":
            cell.update({"outputs": [], "execution_count": None})
        cells.insert(0 if index is None else index + 1, cell)
    elif mode in ("replace", "delete"):
        if index is None:
            raise ValueError("the edit names no cell")
        if mode == "delete":
            del cells[index]
        else:
            cells[index]["source"] = source
            if inp.get("cell_type"):
                cells[index]["cell_type"] = inp["cell_type"]
    else:
        raise ValueError("an unknown edit_mode '%s'" % mode)
    return json.dumps(nb, indent=1, ensure_ascii=False) + "\n"


def words_of(text):
    return [w.lower() for w in re.findall(r"[A-Z]?[a-z]+|[A-Z]+(?![a-z])|[0-9]+", text)]


def as_path(value):
    """The file a string names, or None when it does not name one: a `file://`
    URI is the path it carries; any other URL, a string with a newline and one
    of an absurd length are not paths."""
    if not isinstance(value, str) or not value.strip() or len(value) > 4096 or "\n" in value or "\0" in value:
        return None
    if value.startswith("file://"):
        return unquote(urlparse(value).path) or None
    if re.match(r"^[A-Za-z][A-Za-z0-9+.-]*://", value):
        return None
    return value


def mcp_scan(inp):
    """Every path-like string in an MCP tool's input, at any depth. Returns
    (records, loose, deep): one record per object, (paths, content, partial)
    with the path fields of that object, its content text if any and whether it
    carries an edit-shaped field; `loose` is every other short string, which may
    still name a file; `deep` is whether the input nests beyond what is read."""
    records, loose, state = [], [], {"deep": False}

    def add(target, value):
        p = as_path(value)
        if p is not None and p not in target:
            target.append(p)

    def walk(node, depth, keyed):
        if depth > MCP_MAX_DEPTH:
            state["deep"] = True
            return
        if isinstance(node, dict):
            paths, content, partial = [], None, False
            for k, v in node.items():
                if not isinstance(k, str):
                    continue
                if k in MCP_CONTENT_KEYS:
                    if isinstance(v, str) and content is None:
                        content = v
                    continue
                if k in MCP_EDIT_KEYS:
                    partial = True
                    if isinstance(v, (dict, list)):
                        walk(v, depth + 1, False)
                    continue
                if set(words_of(k)) & MCP_PATH_WORDS:
                    if isinstance(v, str):
                        add(paths, v)
                    elif isinstance(v, list):
                        for item in v:
                            if isinstance(item, str):
                                add(paths, item)
                            else:
                                walk(item, depth + 1, False)
                    else:
                        walk(v, depth + 1, False)
                elif isinstance(v, str):
                    add(loose, v)
                else:
                    walk(v, depth + 1, False)
            records.append((paths, content, partial))
        elif isinstance(node, list):
            for item in node:
                if isinstance(item, str):
                    add(loose, item)
                else:
                    walk(item, depth + 1, False)

    walk(inp, 0, False)
    return records, loose[:MCP_MAX_LOOSE], state["deep"]


def mcp_write(tool, inp):
    """For an MCP tool call: None when it is not a file edit, else a dict with
    `items` (path, content) pairs that are whole-file writes urzua can judge as
    they are, `rest` the paths of the call that cannot be turned into a file's
    new text, `loose` other strings to look at only for a refusal, `loud`
    whether not judging `rest` is worth saying, and `deep`."""
    name = tool.rsplit("__", 1)[-1]
    words = set(words_of(name))
    write_like = bool(words & MCP_WRITE_WORDS)
    if words & MCP_READ_WORDS and not write_like:
        return None
    records, loose, deep = mcp_scan(inp)
    tool_partial = bool(words & MCP_PARTIAL_WORDS)
    items, rest = [], []
    for paths, content, partial in records:
        if len(paths) == 1 and content is not None and not partial and not tool_partial:
            items.append((paths[0], content))
        else:
            rest.extend(p for p in paths if p not in rest)
    keyed = bool(items or rest)
    if not (keyed or loose or deep):
        return None
    writes = write_like or tool_partial or any(c is not None or p for _, c, p in records)
    return {"items": items, "rest": rest, "loose": [p for p in loose if p not in rest], "loud": keyed and writes, "deep": deep}


def proposed(tool, inp, path):
    if tool == "NotebookEdit":
        return notebook_proposed(inp, path)
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
    verdict = json.loads(p.stdout)
    if not isinstance(verdict, dict) or "verdict" not in verdict:
        raise ValueError("the engine did not answer with a verdict")
    return verdict


def judge_mcp(tool, target, argv, cwd):
    results = []
    for path, content in target["items"]:
        request = {"event": "edit", "tool": tool, "paths": [path], "content": content, "actor": "agent", "cwd": cwd}
        try:
            verdict = engine(argv, request, cwd)
        except (OSError, ValueError, AttributeError):
            message = "urzua harness: urzua did NOT judge this call (the engine command `%s` could not be run or answered with something that is not a verdict). It is allowed unless the configuration refuses an edit that was not judged; run `urzua harness status`." % " ".join(argv)
            results.append(unjudged_result(message, argv, cwd, path, True))
            continue
        kind, message = verdict.get("verdict"), verdict.get("message", "")
        if kind == "block":
            results.append(("block", message))
        elif kind in ("allow-unjudged", "allow-override", "allow-with-warnings"):
            results.append(("note", message))
    if target["deep"]:
        results.append(unjudged_result("urzua harness: urzua did NOT judge this edit (%s: its input nests deeper than the translator reads). It is allowed unless the configuration refuses an edit that was not judged; run 'urzua check' before committing." % tool, argv, cwd, [], False))
    candidates = target["rest"] + target["loose"]
    if candidates:
        shown = ", ".join(target["rest"]) or "an unparsed path"
        message = "urzua harness: urzua did NOT judge this edit (%s on %s: the call is not a whole-file write urzua can read). It is allowed unless the configuration refuses an edit that was not judged; run 'urzua check' before committing." % (tool, shown)
        results.append(unjudged_result(message, argv, cwd, candidates, False, loud=target["loud"] and bool(target["rest"])))
    return decide(results)


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
        if not isinstance(payload, dict):
            raise ValueError("not an object")
    except ValueError:
        message = "urzua harness: the hook payload was unreadable, so this call was NOT judged"
        if event == "edit":
            return unjudged_edit(message, argv, None, None, False)
        return stated(message, event)
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
        if not isinstance(tool, str) or not isinstance(inp, dict):
            return 0
        if tool.startswith("mcp__"):
            target = mcp_write(tool, inp)
            if target is None:
                return 0
            return judge_mcp(tool, target, argv, cwd)
        else:
            path = inp.get("notebook_path") if tool == "NotebookEdit" else inp.get("file_path")
            if tool not in EDIT_TOOLS or not path:
                return 0
            try:
                content = proposed(tool, inp, path)
            except ValueError as e:
                return unjudged_edit("urzua harness: urzua did NOT judge this edit (%s of %s: %s). It is allowed unless the configuration refuses an edit that was not judged; run 'urzua check' before committing." % (tool, path, e), argv, cwd, path, False)
        request = {"event": "edit", "tool": tool, "paths": [path], "content": content, "actor": "agent", "cwd": cwd}
    try:
        verdict = engine(argv, request, cwd)
    except (OSError, ValueError, AttributeError):
        message = "urzua harness: urzua did NOT judge this call (the engine command `%s` could not be run or answered with something that is not a verdict). It is allowed unless the configuration refuses an edit that was not judged; run `urzua harness status`." % " ".join(argv)
        if event == "edit":
            return unjudged_edit(message, argv, cwd, request["paths"][0], True)
        return stated(message, event)
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
        message = "urzua harness: the translator failed (%s), so this call was NOT judged" % e
        try:
            sys.exit(unjudged_edit(message, None, None, None, True) if "edit" in sys.argv[1:2] else stated(message, "edit"))
        except Exception:
            sys.exit(stated(message, "edit"))
