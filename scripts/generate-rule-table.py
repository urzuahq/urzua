#!/usr/bin/env python3
"""Regenerate SPEC-2's rule table from the real, compiled rule set.

Not a second maintained source: every row comes from `urzua rules`, which
reads `urzua_core::rules::RULE_METADATA` directly. Re-run this any time a
rule is added, removed, or its description changes; nothing in the table
is hand-updated (BUG-52).

Usage:
    python3 scripts/generate-rule-table.py           # write the file
    python3 scripts/generate-rule-table.py --check   # exit 1 if stale, write nothing
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path

START_MARKER = "<!-- rule-table:start -->"
END_MARKER = "<!-- rule-table:end -->"
ACTION_START_MARKER = "<!-- action-table:start -->"
ACTION_END_MARKER = "<!-- action-table:end -->"
SPEC_PATH = "docs/specs/SPEC-2-urzua-check.md"


def repo_root() -> Path:
    here = Path(__file__).resolve().parent
    while not (here / ".git").exists():
        if here == here.parent:
            raise SystemExit("could not find repo root (no .git above scripts/)")
        here = here.parent
    return here


def urzua_binary(root: Path) -> Path:
    for candidate in ("target/release/urzua", "target/debug/urzua"):
        path = root / "rust" / candidate
        if path.exists():
            return path
    raise SystemExit("no urzua binary found -- run `make build` first")


def render_table(rules: list[dict]) -> str:
    lines = [START_MARKER, "| Rule id | What it checks |", "|---|---|"]
    for rule in rules:
        lines.append(f"| `{rule['id']}` | {rule['description']} |")
    lines.append(END_MARKER)
    return "\n".join(lines)


def render_action_table(rules: list[dict]) -> str:
    """ADR-73's per-variant declaration, one row per finding variant: the
    action it offers, or the reason it offers none. Read from the same
    `urzua rules` output as the rule table, so the two cannot disagree."""
    lines = [
        ACTION_START_MARKER,
        "| Rule id | Variant | Declared action |",
        "|---|---|---|",
    ]
    for rule in rules:
        for action in rule["actions"]:
            if "exempt" in action:
                declared = f"exempt: {action['exempt']}"
            elif "op" in action:
                declared = f"`{action['kind']}` `{action['op']}`"
            else:
                declared = f"`{action['kind']}`"
            # ADR-96: a variant may carry a severity floor no setting lowers and
            # may be unwaivable by a rule-level waiver.
            if "severity_floor" in action:
                declared += f"; severity floor `{action['severity_floor']}`"
            if action.get("waivable") is False:
                declared += "; not waivable by a rule-level waiver"
            lines.append(f"| `{rule['id']}` | `{action['variant']}` | {declared} |")
    lines.append(ACTION_END_MARKER)
    return "\n".join(lines)


def splice(content: str, start: str, end: str, table: str) -> str:
    if start not in content or end not in content:
        raise SystemExit(f"{SPEC_PATH} is missing {start}/{end}")
    before, rest = content.split(start, 1)
    _, after = rest.split(end, 1)
    return before + table + after


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = repo_root()
    binary = urzua_binary(root)
    result = subprocess.run(
        [str(binary), "rules"], capture_output=True, text=True, encoding="utf-8", check=True
    )
    rules = json.loads(result.stdout)["rules"]

    spec_path = root / SPEC_PATH
    content = spec_path.read_text(encoding="utf-8")
    new_content = splice(content, START_MARKER, END_MARKER, render_table(rules))
    new_content = splice(
        new_content, ACTION_START_MARKER, ACTION_END_MARKER, render_action_table(rules)
    )

    if args.check:
        if new_content != content:
            print(f"{SPEC_PATH}'s rule table is stale -- run `make rule-table`", file=sys.stderr)
            return 1
        return 0

    spec_path.write_text(new_content, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
