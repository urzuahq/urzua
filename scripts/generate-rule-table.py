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
    if START_MARKER not in content or END_MARKER not in content:
        raise SystemExit(f"{SPEC_PATH} is missing {START_MARKER}/{END_MARKER}")

    before, rest = content.split(START_MARKER, 1)
    _, after = rest.split(END_MARKER, 1)
    new_content = before + render_table(rules) + after

    if args.check:
        if new_content != content:
            print(f"{SPEC_PATH}'s rule table is stale -- run `make rule-table`", file=sys.stderr)
            return 1
        return 0

    spec_path.write_text(new_content, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
