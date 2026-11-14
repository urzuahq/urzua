#!/usr/bin/env python3
"""Time the two ways of writing a record: `urzua new` and authoring by hand.

MILE-103's first Done-means item asks for a recorded comparison of steps and time
between creating a record with `urzua new` and hand-authoring it. This runs both
procedures for each record type, several times, in a throwaway `git worktree` of
this repository each, and prints one JSON object on stdout (a summary table goes
to stderr).

A step is one action an agent takes with a tool: one command run, or one file
read or written. Each arm is a fixed list of steps, and every command in it is
really run, so the wall-clock figure is the time of the commands themselves
(model time is not measured and favours `urzua new` further, see below).

  new   scaffold   bash .claude/hooks/urzua-record.sh new <type> <title>
        read       the scaffolded record
        read       .urzua/config.yaml
        edit       fill the fields the scaffold left null, and the body
        finish     bash .claude/hooks/urzua-record.sh finish <record>
                   (guard, stage, `urzua index --apply`, `urzua check`)

  hand  read       .urzua/config.yaml        (required fields, statuses)
        read       .urzua/templates/<type>.md
        list       the type's directory      (the next free number)
        read       the newest record of the type (the header's shape)
        mint       a Stable-Id
        write      the whole record
        stage      git add
        index      urzua index --apply, then stage what it changed
        check      urzua check <record>

The hand arm writes the same final text the new arm ends with (with its own
number and id), so both arms end at a record `urzua check` accepts; what differs
is the procedure. That is a cost model, not a model run: it counts the steps the
hand path needs when nothing goes wrong, so it is a floor for hand-authoring. The
real hand path also pays for each mistake (one more edit and one more check), the
mistake `header.scaffold-mismatch` exists to catch; the new path cannot make the
header mistakes because it does not type the header. The figure that carries the
difference an author feels is `authored_bytes`: the text the author has to
produce (the whole record by hand, only the fill with `urzua new`).

Usage:
    python3 scripts/time-new-vs-hand.py --runs 3
    python3 scripts/time-new-vs-hand.py --types adr,bug --runs 1 --bin target/release/urzua
"""

import argparse
import json
import os
import re
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

TYPES = ["adr", "bug", "milestone", "rfc", "spec"]

# What an author supplies after the scaffold: values for the fields `urzua new`
# leaves null, and body text. Chosen so each type ends at a record `urzua check`
# accepts; the same fill is used by both arms.
FILL = {
    "adr": {"header": {"Deciders": "team"}, "body": []},
    "bug": {"header": {"Found-in": "SPEC-2"}, "body": []},
    "milestone": {
        "header": {"Phase": '"1"', "Track": "governance-process", "Blocked-on": "—"},
        "body_done_means": "- A probe record exists and `urzua check` examines it.\n",
    },
    "rfc": {"header": {}, "body": []},
    "spec": {"header": {}, "body": []},
}


def run(cmd, cwd, env=None, timed=None):
    start = time.perf_counter()
    done = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, env=env)
    if timed is not None:
        timed.append(time.perf_counter() - start)
    return done


def repo_root(start):
    out = subprocess.run(
        ["git", "-C", str(start), "rev-parse", "--show-toplevel"],
        capture_output=True,
        text=True,
    )
    if out.returncode != 0:
        raise SystemExit(f"{start} is not inside a git repository")
    return Path(out.stdout.strip())


def apply_fill(text, record_type):
    """Fill the null header fields and the body; returns (text, authored_bytes),
    the bytes of the lines the author had to produce."""
    fill = FILL[record_type]
    authored = 0
    for key, value in fill["header"].items():
        new = re.sub(rf"^{re.escape(key)}: null$", f"{key}: {value}", text, flags=re.M)
        if new != text:
            authored += len(f"{key}: {value}\n".encode())
        text = new
    if "body_done_means" in fill:
        start = text.index("## Done means")
        end = text.index("> **Revision log**")
        body = "## Done means\n\n" + fill["body_done_means"] + "\n"
        text = text[:start] + body + text[end:]
        authored += len(body.encode())
    return text, authored


def clean_worktree(repo, path, commit):
    subprocess.run(
        ["git", "-C", str(path), "checkout", "--detach", "--force", "--quiet", commit],
        check=True,
    )
    subprocess.run(["git", "-C", str(path), "clean", "-fdxq"], check=True)


def first_json(text):
    start = text.find("{")
    return json.loads(text[start:]) if start >= 0 else {}


def new_arm(wt, binary, record_type, title, env):
    times, steps = [], []
    scaffold = run(
        ["bash", ".claude/hooks/urzua-record.sh", "new", record_type, title],
        wt,
        env,
        times,
    )
    steps.append("scaffold")
    path = first_json(scaffold.stdout).get("path")
    if scaffold.returncode != 0 or not path:
        return {"converged": False, "error": scaffold.stderr.strip()[:300]}
    text = (wt / path).read_text()
    steps.append("read the scaffold")
    (wt / ".urzua" / "config.yaml").read_text()
    steps.append("read the config")
    filled, authored = apply_fill(text, record_type)
    (wt / path).write_text(filled)
    steps.append("fill")
    finish = run(
        ["bash", ".claude/hooks/urzua-record.sh", "finish", path], wt, env, times
    )
    steps.append("finish")
    return {
        "converged": finish.returncode == 0,
        "steps": len(steps),
        "step_list": steps,
        "seconds": round(sum(times), 3),
        "authored_bytes": authored,
        "path": path,
        "final_text": filled,
    }


def hand_arm(wt, binary, record_type, title, reference, directory, env):
    times, steps = [], []
    (wt / ".urzua" / "config.yaml").read_text()
    steps.append("read the config")
    (wt / ".urzua" / "templates" / f"{record_type}.md").read_text()
    steps.append("read the template")
    run(["ls", directory], wt, env, times)
    steps.append("list the directory")
    siblings = sorted((wt / directory).glob("*.md"))
    siblings[-1].read_text() if siblings else None
    steps.append("read the newest record")
    run(
        [
            sys.executable,
            "-c",
            "import os,time;print('01M'+''.join('0123456789ABCDEFGHJKMNPQRSTVWXYZ'[b%32] for b in os.urandom(23)))",
        ],
        wt,
        env,
        times,
    )
    steps.append("mint an id")
    # The record the author writes is the new arm's final text under its own id.
    stable = "01M" + "0" * 23
    text = re.sub(r"^Stable-Id: .*$", f"Stable-Id: {stable}", reference["final_text"], count=1, flags=re.M)
    path = reference["path"]
    (wt / path).parent.mkdir(parents=True, exist_ok=True)
    (wt / path).write_text(text)
    steps.append("write the record")
    run(["git", "add", "--", path], wt, env, times)
    steps.append("stage")
    index = run([binary, "index", "--apply"], wt, env, times)
    for changed in re.findall(r'"path": *"([^"]*)"', index.stdout):
        run(["git", "add", "--", changed], wt, env, times)
    steps.append("index and stage")
    check = run([binary, "check", path], wt, env, times)
    steps.append("check")
    return {
        "converged": check.returncode == 0,
        "steps": len(steps),
        "step_list": steps,
        "seconds": round(sum(times), 3),
        "authored_bytes": len(text.encode()),
        "path": path,
    }


def comparator_clean(wt, binary, path, env):
    """Whether the record is also what `urzua new` would have written: the
    comparator rule, enabled for this run on the throwaway worktree only."""
    config = wt / ".urzua" / "config.yaml"
    text = config.read_text()
    if "header.scaffold-mismatch" not in text:
        text = re.sub(r"^rules:\s*$", "rules:\n  header.scaffold-mismatch: warn", text, count=1, flags=re.M)
        config.write_text(text)
    out = run([binary, "check", "--base", "HEAD", "--rules", "header.scaffold-mismatch"], wt, env)
    report = first_json(out.stdout)
    return not any(
        f.get("rule") == "header.scaffold-mismatch" and f.get("file") == path
        for f in report.get("findings", [])
    )


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--repo", default=".")
    ap.add_argument("--bin", help="the urzua binary (default: rust/target/release/urzua)")
    ap.add_argument("--types", default=",".join(TYPES))
    ap.add_argument("--runs", type=int, default=3)
    args = ap.parse_args()

    repo = repo_root(args.repo)
    binary = args.bin or str(repo / "rust" / "target" / "release" / "urzua")
    if not os.access(binary, os.X_OK):
        raise SystemExit(f"no urzua binary at {binary}; build it or pass --bin")
    binary = str(Path(binary).resolve())
    commit = subprocess.run(
        ["git", "-C", str(repo), "rev-parse", "HEAD"], capture_output=True, text=True
    ).stdout.strip()
    env = dict(os.environ, URZUA_BIN=binary)
    types = [t for t in args.types.split(",") if t]

    rows = []
    with tempfile.TemporaryDirectory(prefix="time-new-hand-") as scratch:
        wt = Path(scratch) / "wt"
        subprocess.run(
            ["git", "-C", str(repo), "worktree", "add", "--detach", "--force", str(wt), commit],
            check=True,
            capture_output=True,
        )
        try:
            for record_type in types:
                directory = None
                for n in range(1, args.runs + 1):
                    title = f"Timing probe {record_type} {n}"
                    clean_worktree(repo, wt, commit)
                    new = new_arm(wt, binary, record_type, title, env)
                    if not new.get("converged"):
                        rows.append({"type": record_type, "run": n, "arm": "new", **new})
                        continue
                    directory = str(Path(new["path"]).parent)
                    new["header_matches_scaffold"] = comparator_clean(wt, binary, new["path"], env)
                    clean_worktree(repo, wt, commit)
                    hand = hand_arm(wt, binary, record_type, title, new, directory, env)
                    hand["header_matches_scaffold"] = comparator_clean(wt, binary, hand["path"], env)
                    for arm, result in (("new", new), ("hand", hand)):
                        result.pop("final_text", None)
                        rows.append({"type": record_type, "run": n, "arm": arm, **result})
                    print(
                        f"{record_type} run {n}: new {new['steps']} steps {new['seconds']}s, "
                        f"hand {hand['steps']} steps {hand['seconds']}s",
                        file=sys.stderr,
                    )
        finally:
            subprocess.run(
                ["git", "-C", str(repo), "worktree", "remove", "--force", str(wt)],
                capture_output=True,
            )

    summary = []
    for record_type in types:
        for arm in ("new", "hand"):
            mine = [r for r in rows if r["type"] == record_type and r["arm"] == arm]
            ok = [r for r in mine if r.get("converged")]
            if not ok:
                summary.append({"type": record_type, "arm": arm, "runs": len(mine), "converged": 0})
                continue
            summary.append(
                {
                    "type": record_type,
                    "arm": arm,
                    "runs": len(mine),
                    "converged": len(ok),
                    "steps": ok[0]["steps"],
                    "seconds_mean": round(statistics.mean(r["seconds"] for r in ok), 3),
                    "seconds_min": min(r["seconds"] for r in ok),
                    "seconds_max": max(r["seconds"] for r in ok),
                    "authored_bytes_mean": round(statistics.mean(r["authored_bytes"] for r in ok)),
                    "header_matches_scaffold": all(r.get("header_matches_scaffold") for r in ok),
                }
            )
    print(json.dumps({"commit": commit, "binary": binary, "summary": summary, "runs": rows}, indent=2))
    print(f"{'type':10} {'arm':5} {'steps':>5} {'seconds':>8} {'authored bytes':>15}", file=sys.stderr)
    for s in summary:
        if s.get("converged"):
            print(
                f"{s['type']:10} {s['arm']:5} {s['steps']:>5} {s['seconds_mean']:>8} {s['authored_bytes_mean']:>15}",
                file=sys.stderr,
            )
        else:
            print(f"{s['type']:10} {s['arm']:5} did not converge", file=sys.stderr)


if __name__ == "__main__":
    main()
