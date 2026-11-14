#!/usr/bin/env python3
"""Replay `header.scaffold-mismatch` over a repository's merged pull requests.

MILE-103's third Done-means item asks whether, over a window, any record was
merged whose header `urzua new` would have written differently and that `check`
rejected. The merged state always passes `check`, so "rejected" can only be seen
before the merge: at the commits a pull request had on its way. This script
replays that. For each pull request merged into the main branch in the window it
checks out every commit of the pull request's head that touches a markdown file,
runs

    urzua check --base <merge base with main> --rules <header rules>

there, with `header.scaffold-mismatch` declared at `warn` in that commit's own
configuration (the rule is opt-in, and a commit before it was introduced does not
declare it), and records, per added record, whether

  differs    the comparator reported the record's header as not the scaffold's,
  rejected   and, at the same commit, a header-category rule (`header.*`,
             `field.*`, `date.*`, `status.*` by default) reported the same file.

The headline is the number of distinct (pull request, record) pairs that were
`rejected`: the target is 0. It also replays the merged commits themselves, which
lists the merged records whose header differs from the scaffold (a deliberate
difference needs a waiver, or a person to look at it).

It never touches the checkout it is run from: every commit is checked out into a
throwaway `git worktree`.

Usage:
    python3 scripts/replay-header-comparator.py --since 2026-10-15 --until 2026-11-14
    python3 scripts/replay-header-comparator.py --since 2026-10-15 --pr-ref 'refs/urzua/pr/origin/{n}'

Pull request heads are read from `refs/pull/<n>/head` unless `--pr-ref` says
otherwise (`{n}` stands for the number), so fetch them first:

    git fetch --no-tags origin '+refs/pull/*/head:refs/urzua/pr/origin/*'

The merged pull request of a main-branch commit is the `(#N)` its squash subject
ends with (`(#N)` or `(MILE-N, #N)`). Prints one JSON object on stdout; progress and the summary go to
stderr.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

COMPARATOR = "header.scaffold-mismatch"
DEFAULT_HEADER_PREFIXES = ("header.", "field.", "date.", "status.")
PR_SUBJECT = re.compile(r"#(\d+)\)\s*$")
RULES_KEY = re.compile(r"^rules:\s*(#.*)?$")


def git(repo, *args, check=True):
    done = subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, text=True
    )
    if check and done.returncode != 0:
        raise SystemExit(f"git {' '.join(args)}: {done.stderr.strip()}")
    return done.stdout


def repo_root(start):
    out = subprocess.run(
        ["git", "-C", str(start), "rev-parse", "--show-toplevel"],
        capture_output=True,
        text=True,
    )
    if out.returncode != 0:
        raise SystemExit(f"{start} is not inside a git repository")
    return Path(out.stdout.strip())


def merged_pull_requests(repo, main_ref, since, until):
    """(sha, pull request number or None) for each first-parent commit of
    `main_ref` in the window, oldest first."""
    args = ["log", "--first-parent", "--reverse", "--format=%H%x09%s", main_ref]
    if since:
        args.append(f"--since={since}")
    if until:
        args.append(f"--until={until}")
    out = []
    for line in git(repo, *args).splitlines():
        sha, _, subject = line.partition("\t")
        found = PR_SUBJECT.search(subject)
        out.append((sha, int(found.group(1)) if found else None))
    return out


def touches_markdown(repo, sha):
    names = git(
        repo, "diff-tree", "--no-commit-id", "--name-only", "-r", "-m", "--root", sha
    )
    return any(n.lower().endswith(".md") for n in names.splitlines())


def record_dirs(repo):
    """The `dir` of each record type in the checkout's configuration, read as
    text (no YAML library is assumed): the places an added record can be."""
    path = Path(repo) / ".urzua" / "config.yaml"
    if not path.is_file():
        return []
    found = re.findall(r'^    dir:\s*["\']?([^"\'\s#]+)', path.read_text(), re.M)
    return sorted(set(d.rstrip("/") for d in found))


def adds_markdown(repo, base, sha, dirs):
    """Whether `base..sha` adds a markdown file under one of `dirs` (any, when
    `dirs` is empty): the comparator judges added records only, so a commit
    that adds none has nothing to replay."""
    names = git(
        repo, "diff", "--no-renames", "--name-only", "--diff-filter=A", base, sha
    )
    return any(
        n.lower().endswith(".md")
        and (not dirs or any(n.startswith(d + "/") for d in dirs))
        for n in names.splitlines()
    )


def pull_request_commits(repo, head, main_ref):
    """The commits of a pull request head that are not on main, oldest first,
    merges left out (a merge of main into the branch adds nothing of its own)."""
    fork = git(repo, "merge-base", head, main_ref, check=False).strip()
    if not fork:
        return []
    revs = git(repo, "rev-list", "--no-merges", "--reverse", f"{fork}..{head}")
    return revs.split()


def inject_rule(worktree):
    """Declare the comparator at warn in the worktree's configuration, unless the
    configuration already declares it. Returns False when there is no
    configuration to declare it in."""
    path = Path(worktree) / ".urzua" / "config.yaml"
    if not path.is_file():
        return False
    text = path.read_text()
    if re.search(rf"^\s+{re.escape(COMPARATOR)}\s*:", text, re.M):
        return True
    lines = text.splitlines(keepends=True)
    for i, line in enumerate(lines):
        if RULES_KEY.match(line.rstrip("\n")):
            lines.insert(i + 1, f"  {COMPARATOR}: warn\n")
            break
    else:
        if lines and not lines[-1].endswith("\n"):
            lines[-1] += "\n"
        lines.append(f"rules:\n  {COMPARATOR}: warn\n")
    path.write_text("".join(lines))
    return True


RULE_KEY_LINE = re.compile(r"^  ([a-z][a-z0-9-]*(?:\.[a-z0-9-]+)+)\s*:")


def strip_rule(path, rule=None, token=None):
    """Remove one rule's entry from the worktree's configuration: the entry for
    `rule`, or the one whose text holds `token` (a value or option name the
    parse error quotes). Returns the rule id
    removed, or None when no entry was found. A configuration written for an
    older build can name a rule, or an option, this build does not know; the
    replay then drops that entry and says so, instead of not replaying the commit."""
    lines = path.read_text().splitlines(keepends=True)
    start = None
    if rule is not None:
        for i, text in enumerate(lines):
            found = RULE_KEY_LINE.match(text)
            if found and found.group(1) == rule:
                start = i
                break
    elif token is not None:
        at = next((i for i, text in enumerate(lines) if token in text), None)
        for i in range(-1 if at is None else at, -1, -1):
            found = RULE_KEY_LINE.match(lines[i])
            if found:
                start, rule = i, found.group(1)
                break
    if start is None:
        return None
    end = start + 1
    # The entry's own continuation: deeper-indented or blank lines after it.
    while end < len(lines) and (
        lines[end].startswith("    ") or not lines[end].strip()
    ):
        end += 1
    # Comment lines directly above belong to the entry.
    while start > 0 and lines[start - 1].lstrip().startswith("#"):
        start -= 1
    del lines[start:end]
    path.write_text("".join(lines))
    return rule


class Worker:
    """One throwaway worktree that commits are checked out into in turn."""

    def __init__(self, repo, scratch, index, binary, rule_ids):
        self.repo = repo
        self.path = Path(scratch) / f"wt-{index}"
        self.binary = binary
        self.rule_ids = rule_ids
        git(repo, "worktree", "add", "--detach", "--force", str(self.path), "HEAD")

    def close(self):
        git(self.repo, "worktree", "remove", "--force", str(self.path), check=False)

    def check(self, sha, base):
        """The findings `check --base <base>` reports at `sha`, or an error."""
        git(self.path, "checkout", "--detach", "--force", "--quiet", sha)
        git(self.path, "clean", "-fdxq")
        if not inject_rule(self.path):
            return {"error": "no .urzua/config.yaml at this commit"}
        # `--rules` narrows what runs and enables nothing, so a rule this commit's
        # configuration does not declare is refused by name: drop it and run again.
        rule_ids = list(self.rule_ids)
        dropped = []
        while True:
            run = subprocess.run(
                [self.binary, "check", "--base", base, "--rules", ",".join(rule_ids)],
                cwd=self.path,
                capture_output=True,
                text=True,
            )
            try:
                report = json.loads(run.stdout)
            except json.JSONDecodeError:
                return {"error": (run.stderr or run.stdout).strip()[:300]}
            error = str(report.get("error", ""))
            undeclared = re.search(r"'([a-z.-]+)' is not declared in the configuration", error)
            if report.get("status") == "not-run" and undeclared and undeclared.group(1) in rule_ids:
                rule_ids.remove(undeclared.group(1))
                continue
            if report.get("status") == "not-run" and len(dropped) < 12:
                unknown = re.search(r"unknown rule '([^']+)' under `rules:`", error)
                quoted = re.search(r'invalid type: string "([^"]+)"', error) or re.search(
                    r"unknown field `([^`]+)`", error
                )
                config = Path(self.path) / ".urzua" / "config.yaml"
                gone = None
                if unknown:
                    gone = strip_rule(config, rule=unknown.group(1))
                elif quoted and "rules:" in error:
                    gone = strip_rule(config, token=quoted.group(1))
                if gone:
                    dropped.append(gone)
                    if gone in rule_ids:
                        rule_ids.remove(gone)
                    continue
            break
        if report.get("status") == "not-run":
            return {"error": error[:300], "dropped_rules": dropped}
        examined = 0
        for execution in report.get("rules_executed", []):
            if execution.get("rule") == COMPARATOR and execution.get("status") == "ran":
                examined = (execution.get("population") or {}).get("examined", 0)
        return {
            "findings": report.get("findings", []),
            "examined": examined,
            "dropped_rules": dropped,
        }


def header_rule(rule_id, prefixes):
    return rule_id != COMPARATOR and rule_id.startswith(tuple(prefixes))


def judge(report, prefixes):
    """Per file: the comparator's messages and the header findings."""
    files = {}
    for finding in report.get("findings", []):
        rule = finding.get("rule", "")
        if finding.get("waived"):
            continue
        entry = files.setdefault(
            finding["file"], {"differences": [], "header_findings": []}
        )
        if rule == COMPARATOR:
            entry["differences"].append(finding.get("message", ""))
        elif header_rule(rule, prefixes):
            entry["header_findings"].append(rule)
    return files


def list_header_rules(binary, prefixes):
    out = subprocess.run([binary, "rules"], capture_output=True, text=True)
    try:
        listed = json.loads(out.stdout)
    except json.JSONDecodeError:
        raise SystemExit(f"{binary} rules did not print JSON: {out.stderr.strip()}")
    ids = [r["id"] for r in listed.get("rules", [])]
    if COMPARATOR not in ids:
        raise SystemExit(f"{binary} has no {COMPARATOR} rule; build this branch first")
    return [COMPARATOR] + [i for i in ids if header_rule(i, prefixes)]


def key_text(key):
    return f"{key[0]}:{key[1]}"


def run_jobs(jobs, repo, scratch, binary, rule_ids, count, cache_path):
    """`jobs` is a list of (key, sha, base); returns {key: result}. One worktree
    per thread. With a cache file, each result is appended to it as it arrives
    and a rerun skips the jobs it already holds, so a long replay can be resumed."""
    results = {}
    if cache_path and Path(cache_path).is_file():
        for line in Path(cache_path).read_text().splitlines():
            row = json.loads(line)
            if "error" not in row["result"]:
                results[tuple(row["key"])] = row["result"]
    jobs = [j for j in jobs if j[0] not in results]
    if not jobs:
        return results
    pool = [Worker(repo, scratch, i, binary, rule_ids) for i in range(count)]
    free = list(pool)
    import threading

    lock = threading.Lock()

    def one(job):
        key, sha, base = job
        with lock:
            worker = free.pop()
        try:
            result = worker.check(sha, base)
            with lock:
                results[key] = result
                if cache_path and "error" not in result:
                    with open(cache_path, "a") as out:
                        out.write(json.dumps({"key": list(key), "result": result}) + "\n")
        finally:
            with lock:
                free.append(worker)

    try:
        with ThreadPoolExecutor(max_workers=count) as ex:
            for n, _ in enumerate(ex.map(one, jobs), 1):
                if n % 25 == 0:
                    print(f"  {n}/{len(jobs)} commits checked", file=sys.stderr)
    finally:
        for worker in pool:
            worker.close()
    return results


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--repo", default=".", help="the repository (default: here)")
    ap.add_argument("--bin", help="the urzua binary (default: rust/target/release/urzua)")
    ap.add_argument("--main-ref", default="origin/main")
    ap.add_argument("--since", help="first day of the window (git date, e.g. 2026-10-15)")
    ap.add_argument("--until", help="last day of the window (git date)")
    ap.add_argument(
        "--pr-ref",
        default="refs/pull/{n}/head",
        help="where a pull request's head is, `{n}` being its number",
    )
    ap.add_argument(
        "--header-prefix",
        action="append",
        help="a rule id prefix that counts as a header finding (default: "
        + ", ".join(DEFAULT_HEADER_PREFIXES)
        + ")",
    )
    ap.add_argument(
        "--record-dir",
        action="append",
        help="a directory records live in (repeatable); a commit that adds no "
        "markdown file under one is not replayed. Default: the `dir` of each "
        "record type in the checkout's configuration",
    )
    ap.add_argument("--jobs", type=int, default=2)
    ap.add_argument(
        "--cache",
        help="a JSON-lines file each commit's result is appended to; a rerun with "
        "the same file (and the same binary) resumes instead of starting over",
    )
    args = ap.parse_args()

    repo = repo_root(args.repo)
    binary = args.bin or str(repo / "rust" / "target" / "release" / "urzua")
    if not os.access(binary, os.X_OK):
        raise SystemExit(f"no urzua binary at {binary}; build it or pass --bin")
    prefixes = tuple(args.header_prefix or DEFAULT_HEADER_PREFIXES)
    rule_ids = list_header_rules(binary, prefixes)
    dirs = args.record_dir if args.record_dir is not None else record_dirs(repo)

    merged = merged_pull_requests(repo, args.main_ref, args.since, args.until)
    pulls = {n: sha for sha, n in merged if n is not None}
    print(
        f"{len(merged)} commits on {args.main_ref} in the window, {len(pulls)} merged pull requests",
        file=sys.stderr,
    )

    # Jobs: every markdown-touching commit of each merged pull request's head
    # (plus its final head), then each merged commit against its parent.
    jobs = []
    seen = set()
    pr_commits = {}
    pr_final = {}
    missing_heads = []
    for n in sorted(pulls):
        head = git(
            repo, "rev-parse", "--verify", "--quiet", args.pr_ref.format(n=n), check=False
        ).strip()
        if not head:
            missing_heads.append(n)
            continue
        commits = pull_request_commits(repo, head, args.main_ref)
        pr_final[n] = commits[-1] if commits else None
        wanted = []
        for sha in commits:
            # Every commit that touches markdown, and the final head whatever it
            # touches: a record added earlier is judged at the head that merged.
            if sha != pr_final[n] and not touches_markdown(repo, sha):
                continue
            base = git(repo, "merge-base", sha, args.main_ref).strip()
            if adds_markdown(repo, base, sha, dirs):
                wanted.append(sha)
                if sha not in seen:
                    seen.add(sha)
                    jobs.append((("pr", sha), sha, base))
        pr_commits[n] = wanted
    # Every merged commit, also one pushed with no pull request number.
    for sha, _ in merged:
        if adds_markdown(repo, f"{sha}^", sha, dirs):
            jobs.append((("main", sha), sha, f"{sha}^"))

    with tempfile.TemporaryDirectory(prefix="replay-header-") as scratch:
        results = run_jobs(
            jobs, repo, scratch, binary, rule_ids, max(1, args.jobs), args.cache
        )

    # Per (pull request, record): what was seen at any of its commits.
    pairs = {}
    could_not_run = []
    for n, commits in pr_commits.items():
        for sha in commits:
            result = results[("pr", sha)]
            if "error" in result:
                could_not_run.append({"pr": n, "commit": sha, "error": result["error"]})
                continue
            for path, seen_here in judge(result, prefixes).items():
                if not seen_here["differences"]:
                    continue
                pair = pairs.setdefault(
                    (n, path),
                    {
                        "pr": n,
                        "file": path,
                        "first_commit": sha,
                        "rejected": False,
                        "header_rules": set(),
                        "differences": seen_here["differences"][0],
                        "at_final_head": False,
                    },
                )
                if seen_here["header_findings"]:
                    pair["rejected"] = True
                    pair["header_rules"].update(seen_here["header_findings"])
                if sha == pr_final[n]:
                    pair["at_final_head"] = True

    merged_rows = []
    merged_failed = []
    merged_records_examined = 0
    for sha, n in merged:
        result = results.get(("main", sha))
        if result is None:
            continue
        if "error" in result:
            merged_failed.append({"pr": n, "commit": sha, "error": result["error"]})
            continue
        merged_records_examined += result.get("examined", 0)
        for path, seen_here in judge(result, prefixes).items():
            if seen_here["differences"]:
                merged_rows.append(
                    {
                        "pr": n,
                        "commit": sha,
                        "file": path,
                        "differences": seen_here["differences"][0],
                        "header_findings_on_merged_state": seen_here["header_findings"],
                    }
                )

    rejected = sorted(
        (p for p in pairs.values() if p["rejected"]), key=lambda p: (p["pr"], p["file"])
    )
    for p in pairs.values():
        p["header_rules"] = sorted(p["header_rules"])
    def kinds(rows):
        """How the merged records differ, by kind of difference and by type."""
        by_kind, by_type = {}, {}
        for row in rows:
            text = row["differences"]
            found = re.search(r"`urzua new ([^`]+)` writes", text)
            by_type[found.group(1) if found else "?"] = (
                by_type.get(found.group(1) if found else "?", 0) + 1
            )
            for kind, pattern in (
                ("field missing", r"is missing \(urzua new writes it\)"),
                ("field not written by new", r"is not a field urzua new writes"),
                ("another field order", r"another order"),
                ("scaffolded value changed", r"where urzua new writes"),
            ):
                if re.search(pattern, text):
                    by_kind[kind] = by_kind.get(kind, 0) + 1
        return by_kind, by_type

    by_kind, by_type = kinds(merged_rows)
    dropped = {}
    for result in results.values():
        for rule in result.get("dropped_rules", []):
            dropped[rule] = dropped.get(rule, 0) + 1
    summary = {
        "window": {"since": args.since, "until": args.until, "main_ref": args.main_ref},
        "binary": binary,
        "header_rule_prefixes": list(prefixes),
        "record_dirs": dirs,
        "merged_commits": len(merged),
        "merged_pull_requests": len(pulls),
        "pull_requests_without_a_head_ref": missing_heads,
        "pull_request_commits_checked": sum(len(c) for c in pr_commits.values()),
        "commits_that_could_not_run": could_not_run,
        "merged_commits_that_could_not_run": merged_failed,
        "pairs_that_differed_at_any_head": len(pairs),
        "pairs_rejected": len(rejected),
        "pairs_differing_at_final_head": sum(1 for p in pairs.values() if p["at_final_head"]),
        "merged_records_examined": merged_records_examined,
        "merged_records_differing_from_scaffold": len(merged_rows),
        "merged_differing_by_kind": by_kind,
        "merged_differing_by_type": by_type,
        "rules_dropped_from_old_configurations": dropped,
        "rejected": rejected,
        "merged_differing": merged_rows,
        "differed_but_not_rejected": sorted(
            (p for p in pairs.values() if not p["rejected"]),
            key=lambda p: (p["pr"], p["file"]),
        ),
    }
    print(json.dumps(summary, indent=2))
    print(
        f"records that differed at some head: {len(pairs)}; rejected by a header "
        f"finding: {len(rejected)} (target 0); merged records examined: "
        f"{merged_records_examined}, differing from the scaffold: {len(merged_rows)}; commits that could not run: "
        f"{len(could_not_run) + len(merged_failed)}",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
