#!/usr/bin/env python3
"""Generate a self-contained HTML dashboard from this repo's own corpus.

Not a second maintained source: every number and record on the page comes
from parsing docs/ directly and from running the real `urzua` binary
(check/explain/graph), the same way `check` itself reads the corpus. Re-run
this any time the corpus changes; nothing here is hand-updated.

Usage:
    python3 scripts/generate-dashboard.py > dashboard.html
    python3 scripts/generate-dashboard.py --output dashboard.html
"""

import argparse
import glob
import json
import re
import subprocess
import sys
from pathlib import Path

import yaml


def repo_root() -> Path:
    here = Path(__file__).resolve().parent
    while not (here / ".git").exists():
        if here == here.parent:
            raise SystemExit("could not find repo root (no .git above scripts/)")
        here = here.parent
    return here


def urzua_binary(root: Path) -> Path:
    for candidate in ("rust/target/release/urzua", "rust/target/debug/urzua"):
        p = root / candidate
        if p.exists():
            return p
    raise SystemExit("no urzua binary found -- run `make build` first")


def extract_body(content: str) -> str:
    lines = content.split("\n")
    idx = 0
    while idx < len(lines) and not lines[idx].startswith("# "):
        idx += 1
    idx += 1
    while idx < len(lines) and lines[idx].strip() == "":
        idx += 1
    while idx < len(lines) and lines[idx].strip().startswith(">") and not lines[idx].strip().startswith("> **"):
        idx += 1
    return "\n".join(lines[idx:]).strip()


def _yaml_frontmatter(content: str):
    if not content.startswith("---\n"):
        return None
    end = content.find("\n---\n", 4)
    if end == -1:
        return None
    parsed = yaml.safe_load(content[4:end])
    return parsed if isinstance(parsed, dict) else None


def field(content: str, name: str, default: str = "—") -> str:
    frontmatter = _yaml_frontmatter(content)
    if frontmatter is not None:
        value = frontmatter.get(name)
        return default if value is None else str(value).strip()
    m = re.search(rf"^> {name}: (.+)$", content, re.M)
    return m.group(1).strip() if m else default


def section(content: str, name: str) -> str:
    m = re.search(
        rf"^## {name}\n\n(.+?)(?=\n## |\n> \*\*Revision log\*\*|\Z)", content, re.M | re.S
    )
    return m.group(1).strip().replace("\n", " ") if m else ""


def parse_milestones(root: Path):
    records = []
    files = sorted(
        glob.glob(str(root / "docs/milestones/MILE-*.md")),
        key=lambda p: int(re.search(r"MILE-(\d+)-", p).group(1)),
    )
    for f in files:
        content = Path(f).read_text()
        num = re.search(r"MILE-(\d+)-", f).group(1)
        title = re.search(r"^# \d+ — (.+)$", content, re.M).group(1)
        records.append(
            {
                "num": num,
                "title": title,
                "status": field(content, "Status"),
                "phase": field(content, "Phase"),
                "track": field(content, "Track"),
                "implements": field(content, "Implements"),
                "what": section(content, "What"),
                "why": section(content, "Why"),
                "blocked": field(content, "Blocked-on"),
                "file": str(Path(f).relative_to(root)),
            }
        )
    return records


def parse_bugs(root: Path):
    records = []
    files = sorted(
        glob.glob(str(root / "docs/bugs/BUG-*.md")),
        key=lambda p: int(re.search(r"BUG-(\d+)-", p).group(1)),
    )
    for f in files:
        content = Path(f).read_text()
        num = re.search(r"BUG-(\d+)-", f).group(1)
        title = re.search(r"^# \d+ — (.+)$", content, re.M).group(1)
        records.append(
            {
                "num": num,
                "title": title,
                "status": field(content, "Status"),
                "found_in": field(content, "Found-in"),
                "body": extract_body(content),
                "file": str(Path(f).relative_to(root)),
            }
        )
    return records


def parse_rfcs(root: Path):
    records = []
    files = sorted(
        glob.glob(str(root / "docs/rfc/RFC-*.md")),
        key=lambda p: int(re.search(r"RFC-(\d+)-", p).group(1)),
    )
    for f in files:
        content = Path(f).read_text()
        num = re.search(r"RFC-(\d+)-", f).group(1)
        title = re.search(r"^# \d+ — (.+)$", content, re.M).group(1)
        records.append(
            {
                "num": num,
                "title": title,
                "status": field(content, "Status"),
                "body": extract_body(content),
                "file": str(Path(f).relative_to(root)),
            }
        )
    return records


def parse_adrs(root: Path):
    records = []
    files = sorted(
        glob.glob(str(root / "docs/adr/ADR-*.md")),
        key=lambda p: int(re.search(r"ADR-(\d+)-", p).group(1)),
    )
    for f in files:
        content = Path(f).read_text()
        num = re.search(r"ADR-(\d+)-", f).group(1)
        title = re.search(r"^# \d+ — (.+)$", content, re.M).group(1)
        records.append(
            {
                "num": num,
                "title": title,
                "status": field(content, "Status"),
                "embodiment": field(content, "Embodiment"),
                "body": extract_body(content),
                "file": str(Path(f).relative_to(root)),
            }
        )
    return records


def parse_specs(root: Path):
    records = []
    files = sorted(
        glob.glob(str(root / "docs/specs/SPEC-*.md")),
        key=lambda p: int(re.search(r"SPEC-(\d+)-", p).group(1)),
    )
    for f in files:
        content = Path(f).read_text()
        num = re.search(r"SPEC-(\d+)-", f).group(1)
        title = re.search(r"^# SPEC-\d+ — (.+)$", content, re.M).group(1)
        records.append(
            {
                "num": num,
                "title": title,
                "status": field(content, "Status"),
                "version": field(content, "Version"),
                "body": extract_body(content),
                "file": str(Path(f).relative_to(root)),
            }
        )
    return records


def run_urzua(binary: Path, root: Path, *args):
    result = subprocess.run(
        [str(binary), *args], cwd=root, capture_output=True, text=True, check=False
    )
    return json.loads(result.stdout)


def build_data(root: Path) -> dict:
    binary = urzua_binary(root)
    check = run_urzua(binary, root, "check", "docs/")
    explain = run_urzua(binary, root, "explain", "rust/crates/urzua-core/src/new_record.rs")
    graph = run_urzua(binary, root, "graph")
    milestone_edges = [e for e in graph["edges"] if e.get("from", "").startswith("MILE-")]

    return {
        "milestones": parse_milestones(root),
        "bugs": parse_bugs(root),
        "rfcs": parse_rfcs(root),
        "adrs": parse_adrs(root),
        "specs": parse_specs(root),
        "check": {
            "status": check["status"],
            "files_examined": check["files_examined"],
            "blocking": check["blocking"],
            "rules_executed": check["rules_executed"],
            "findings_count": len(check["findings"]),
        },
        "explain": explain,
        "milestone_edges": milestone_edges,
    }


TEMPLATE = r'''<title>Urzua — Governance Ledger</title>
<style>
:root {
  --ink-900: #12151b;
  --ink-800: #1b1f28;
  --ink-700: #262b36;
  --paper-50: #f1ede4;
  --paper-100: #e7e1d3;
  --line: #3a4150;
  --line-light: #d9d1bd;
  --text: #e8e6df;
  --text-dim: #9aa3b3;
  --text-light: #262220;
  --text-light-dim: #6b6255;
  --accent: #c99a52;
  --accent-strong: #e0b56d;
  --done: #5fa383;
  --planned: #8492a8;
  --blocked: #c76b4e;
  --mono: ui-monospace, "SF Mono", "Menlo", "Consolas", monospace;
  --serif: Charter, "Iowan Old Style", Georgia, "Times New Roman", serif;
  --sans: -apple-system, "Segoe UI", ui-sans-serif, system-ui, sans-serif;
}
:root[data-theme="light"] {
  --bg: var(--paper-50); --panel: #ffffff; --panel-2: var(--paper-100);
  --border: var(--line-light); --fg: var(--text-light); --fg-dim: var(--text-light-dim);
  --term-bg: #1b1f28; --term-fg: #d7dbe4;
}
:root[data-theme="dark"] {
  --bg: var(--ink-900); --panel: var(--ink-800); --panel-2: var(--ink-700);
  --border: var(--line); --fg: var(--text); --fg-dim: var(--text-dim);
  --term-bg: #0d0f13; --term-fg: #c7ccd6;
}
@media (prefers-color-scheme: light) {
  :root:not([data-theme]) {
    --bg: var(--paper-50); --panel: #ffffff; --panel-2: var(--paper-100);
    --border: var(--line-light); --fg: var(--text-light); --fg-dim: var(--text-light-dim);
    --term-bg: #1b1f28; --term-fg: #d7dbe4;
  }
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme]) {
    --bg: var(--ink-900); --panel: var(--ink-800); --panel-2: var(--ink-700);
    --border: var(--line); --fg: var(--text); --fg-dim: var(--text-dim);
    --term-bg: #0d0f13; --term-fg: #c7ccd6;
  }
}

* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--fg); font-family: var(--sans); font-size: 15px; line-height: 1.5; }

.shell { display: grid; grid-template-columns: 240px 1fr; min-height: 100vh; }
@media (max-width: 900px) { .shell { grid-template-columns: 1fr; } }

.rail {
  border-right: 1px solid var(--border);
  padding: 28px 22px;
  display: flex; flex-direction: column; gap: 24px;
  position: sticky; top: 0; align-self: start; max-height: 100vh; overflow-y: auto;
}
.wordmark { font-family: var(--serif); font-size: 22px; font-weight: 600; color: var(--accent); letter-spacing: 0.01em; }
.wordmark small {
  display: block; font-family: var(--sans); font-size: 11px; font-weight: 500;
  letter-spacing: 0.08em; text-transform: uppercase; color: var(--fg-dim); margin-top: 4px;
}

.stat-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
.stat { background: var(--panel); border: 1px solid var(--border); border-radius: 3px; padding: 9px 10px; }
.stat .n { font-family: var(--mono); font-variant-numeric: tabular-nums; font-size: 19px; font-weight: 600; color: var(--fg); display: block; }
.stat .l { font-size: 10px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--fg-dim); }

.nav-group h4 { font-size: 10.5px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--fg-dim); margin: 0 0 8px; font-weight: 600; }
.nav-list { display: flex; flex-direction: column; gap: 2px; }
.nav-item {
  display: flex; align-items: center; justify-content: space-between; gap: 8px;
  font-family: var(--mono); font-size: 12.5px; padding: 7px 10px; border-radius: 4px;
  color: var(--fg-dim); cursor: pointer; border: 1px solid transparent; background: none; text-align: left; width: 100%;
}
.nav-item:hover { color: var(--fg); background: var(--panel-2); }
.nav-item.active { color: var(--accent); background: var(--panel); border-color: var(--border); font-weight: 600; }
.nav-item .count { font-size: 10.5px; color: var(--fg-dim); }
.nav-item.active .count { color: var(--accent); }

.main { padding: 30px 40px 60px; max-width: 1040px; }
h1 { font-family: var(--serif); font-size: 27px; margin: 0 0 6px; text-wrap: balance; font-weight: 600; }
.sub { color: var(--fg-dim); margin: 0 0 24px; max-width: 68ch; font-size: 13.5px; }
.sub code { font-family: var(--mono); background: var(--panel-2); padding: 1px 5px; border-radius: 3px; font-size: 0.92em; }

.tab-panel { display: none; }
.tab-panel.active { display: block; }

.explainer {
  background: var(--panel); border: 1px solid var(--border); border-left: 3px solid var(--accent);
  border-radius: 4px; padding: 13px 16px; margin-bottom: 22px; font-size: 13px; color: var(--fg-dim); max-width: 76ch;
}
.explainer strong { color: var(--fg); font-weight: 600; }
.explainer code { font-family: var(--mono); background: var(--panel-2); padding: 1px 5px; border-radius: 3px; font-size: 0.92em; color: var(--fg); }

.chip-row { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 16px; }
.chip {
  font-family: var(--mono); font-size: 11.5px; padding: 4px 9px; border-radius: 999px;
  border: 1px solid var(--border); background: var(--panel); color: var(--fg-dim); cursor: pointer;
  transition: border-color 0.12s, color 0.12s;
}
.chip:hover { border-color: var(--accent); color: var(--fg); }
.chip.active { border-color: var(--accent); color: var(--accent); background: color-mix(in srgb, var(--accent) 12%, transparent); }

.section-head { display: flex; align-items: baseline; justify-content: space-between; border-bottom: 1px solid var(--border); padding-bottom: 8px; margin-bottom: 14px; }
.section-head h2 { font-family: var(--serif); font-size: 16px; margin: 0; font-weight: 600; }
.section-head .count { font-family: var(--mono); font-size: 12px; color: var(--fg-dim); }

.phase-group { margin-bottom: 32px; }
.phase-head { display: flex; align-items: baseline; gap: 12px; margin-bottom: 4px; }
.phase-badge { font-family: var(--mono); font-size: 12px; font-weight: 600; color: var(--bg); background: var(--accent); padding: 3px 10px; border-radius: 3px; flex-shrink: 0; }
.phase-title { font-family: var(--serif); font-size: 18px; font-weight: 600; }
.phase-count { font-family: var(--mono); font-size: 11px; color: var(--fg-dim); margin-left: auto; }

.track-group { margin-bottom: 16px; padding-left: 4px; border-left: 2px solid var(--border); }
.track-label { font-family: var(--mono); font-size: 10.5px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--accent); margin-bottom: 7px; padding-left: 10px; }

.rec-list { display: flex; flex-direction: column; gap: 1px; background: var(--border); border: 1px solid var(--border); border-radius: 4px; overflow: hidden; margin-left: 10px; }
.rec-row { background: var(--panel); padding: 9px 13px; display: grid; grid-template-columns: 44px 1fr auto; align-items: center; gap: 12px; cursor: pointer; }
.rec-row:hover { background: var(--panel-2); }
.rec-num { font-family: var(--mono); font-size: 11px; color: var(--fg-dim); font-variant-numeric: tabular-nums; }
.rec-title { font-size: 13px; }
.rec-title .sub-line { display: block; font-family: var(--mono); font-size: 10.5px; color: var(--fg-dim); margin-top: 2px; }

.status-pill { font-family: var(--mono); font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; padding: 3px 8px; border-radius: 999px; white-space: nowrap; }
.status-Done, .status-Accepted, .status-Fixed { background: color-mix(in srgb, var(--done) 20%, transparent); color: var(--done); }
.status-Planned, .status-Proposed, .status-Draft, .status-Discussion { background: color-mix(in srgb, var(--planned) 20%, transparent); color: var(--planned); }
.status-Blocked, .status-Rejected, .status-WontFix, .status-WontDo, .status-Open { background: color-mix(in srgb, var(--blocked) 22%, transparent); color: var(--blocked); }
.status-InProgress { background: color-mix(in srgb, var(--accent) 22%, transparent); color: var(--accent); }
.status-Superseded, .status-Deprecated { background: var(--panel-2); color: var(--fg-dim); }

.detail { display: none; background: var(--panel); border: 1px solid var(--border); border-top: none; padding: 13px 15px; font-size: 12.5px; margin-left: 10px; }
.detail.open { display: block; }
.detail dl { margin: 0; display: grid; grid-template-columns: max-content 1fr; gap: 4px 14px; }
.detail dt { color: var(--fg-dim); font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; padding-top: 2px; }
.detail dd { margin: 0; }
.detail.body-detail { margin-left: 0; padding: 18px 20px; max-height: 520px; overflow-y: auto; }
.body-detail h3 { font-family: var(--serif); font-size: 15px; margin: 18px 0 8px; color: var(--accent); }
.body-detail h3:first-child { margin-top: 0; }
.body-detail p { margin: 0 0 10px; line-height: 1.6; }
.body-detail code { font-family: var(--mono); background: var(--panel-2); padding: 1px 5px; border-radius: 3px; font-size: 0.9em; }
.body-detail ul { margin: 0 0 10px; padding-left: 20px; }
.body-detail li { margin-bottom: 4px; line-height: 1.5; }
.body-detail blockquote { margin: 0 0 10px; padding-left: 12px; border-left: 2px solid var(--border); color: var(--fg-dim); }
.body-detail .file-path { font-family: var(--mono); font-size: 11px; color: var(--fg-dim); margin-bottom: 14px; padding-bottom: 10px; border-bottom: 1px solid var(--border); }

.flat-list { display: flex; flex-direction: column; gap: 1px; background: var(--border); border: 1px solid var(--border); border-radius: 4px; overflow: hidden; }

.term {
  background: var(--term-bg); color: var(--term-fg); border-radius: 6px; padding: 16px 18px;
  font-family: var(--mono); font-size: 12.5px; line-height: 1.6; overflow-x: auto; white-space: pre;
}
.term .comment { color: #7d8494; font-style: italic; }
.term .key { color: #e0b56d; }
.term .str { color: #a3c9a8; }
.term .num { color: #d38ac7; }

.query-tabs { display: flex; gap: 4px; margin-bottom: 10px; }
.qtab { font-family: var(--mono); font-size: 11.5px; padding: 6px 12px; background: var(--panel-2); border: 1px solid var(--border); border-bottom: none; border-radius: 4px 4px 0 0; color: var(--fg-dim); cursor: pointer; }
.qtab.active { color: var(--accent); background: var(--term-bg); border-color: var(--term-bg); }
.qpane { display: none; }
.qpane.active { display: block; }

footer { color: var(--fg-dim); font-size: 11.5px; margin-top: 50px; border-top: 1px solid var(--border); padding-top: 16px; }
footer code { font-family: var(--mono); }
</style>

<div class="shell">
  <aside class="rail">
    <div class="wordmark">Urzua<small>Governance Ledger</small></div>
    <div class="stat-grid" id="stat-grid"></div>
    <div class="nav-group">
      <h4>Dashboard</h4>
      <div class="nav-list" id="tab-nav"></div>
    </div>
  </aside>

  <main class="main">
    <div id="panel-roadmap" class="tab-panel">
      <h1>The roadmap, queried live</h1>
      <p class="sub">Every row below is a real record parsed from <code>docs/milestones/</code> by the actual <code>urzua</code> binary. Click a milestone to see its full record.</p>
      <div class="explainer">
        <strong>How this is organized.</strong> <code>Phase</code> and <code>Track</code> are plain text tags on each milestone record, not a separate table or an enforced enum. <code>Phase 0</code> is the bootstrap backlog (required for the tool to validate and build itself further, plus urgent bugs); phases 1&ndash;4 are worth-installing, agent-facing, differentiated, and scale work. Moving a milestone anywhere costs one edit to its own file.
      </div>
      <div class="section-head"><h2>Milestones by phase, then track</h2><span class="count" id="ms-visible-count"></span></div>
      <div class="chip-row" id="ms-phase-filters"></div>
      <div class="chip-row" id="ms-track-filters"></div>
      <div class="chip-row" id="ms-status-filters"></div>
      <div id="phase-groups"></div>
    </div>

    <div id="panel-rfcs" class="tab-panel">
      <h1>RFCs</h1>
      <p class="sub">Pre-decision proposals &mdash; the design record. An <code>Accepted</code> RFC feeds an ADR; a <code>Draft</code> one is still genuinely open.</p>
      <div class="chip-row" id="rfc-status-filters"></div>
      <div class="section-head"><h2>All RFCs</h2><span class="count" id="rfc-visible-count"></span></div>
      <div class="flat-list" id="rfc-list"></div>
    </div>

    <div id="panel-adrs" class="tab-panel">
      <h1>ADRs</h1>
      <p class="sub">Decisions made about Urzua itself, with the realization axis (<code>Embodiment</code>) tracked separately from the decision axis (<code>Status</code>).</p>
      <div class="chip-row" id="adr-status-filters"></div>
      <div class="section-head"><h2>All ADRs</h2><span class="count" id="adr-visible-count"></span></div>
      <div class="flat-list" id="adr-list"></div>
    </div>

    <div id="panel-specs" class="tab-panel">
      <h1>Specs</h1>
      <p class="sub">Build-level detail for what ships. A spec renders as its latest revision; the append-only log underneath is the audit trail (RFC-6).</p>
      <div class="section-head"><h2>All specs</h2><span class="count" id="spec-visible-count"></span></div>
      <div class="flat-list" id="spec-list"></div>
    </div>

    <div id="panel-bugs" class="tab-panel">
      <h1>Bugs</h1>
      <p class="sub">Defects found in Urzua itself, each requiring a named regression test before it counts as fixed (ADR-35).</p>
      <div class="section-head"><h2>All bugs</h2><span class="count" id="bug-visible-count"></span></div>
      <div class="flat-list" id="bug-list"></div>
    </div>

    <div id="panel-query" class="tab-panel">
      <h1>Live query output</h1>
      <p class="sub">Genuine <code>urzua explain</code> / <code>graph</code> / <code>check</code> JSON output from this corpus &mdash; stdout is always this shape (ADR-23/26), this page is the generated view.</p>
      <div class="query-tabs">
        <div class="qtab active" data-q="explain">explain new_record.rs</div>
        <div class="qtab" data-q="graph">graph &mdash;milestones</div>
        <div class="qtab" data-q="check">check docs/</div>
      </div>
      <div id="q-explain" class="qpane active"></div>
      <div id="q-graph" class="qpane"></div>
      <div id="q-check" class="qpane"></div>
    </div>

    <footer>
      Generated from a live run of <code>urzua explain</code>, <code>urzua graph</code>, and <code>urzua check docs/</code> against this repository's own corpus.
    </footer>
  </main>
</div>

<script id="data" type="application/json">__DATA__</script>

<script>
const DATA = JSON.parse(document.getElementById('data').textContent);
function esc(s) { const d = document.createElement('div'); d.textContent = s; return d.innerHTML; }
function syntaxHighlightJSON(obj) {
  const json = JSON.stringify(obj, null, 2);
  return esc(json)
    .replace(/(&quot;.*?&quot;)(:)/g, '<span class="key">$1</span>$2')
    .replace(/: (&quot;.*?&quot;)/g, ': <span class="str">$1</span>')
    .replace(/: (\d+|true|false|null)/g, ': <span class="num">$1</span>');
}
function inlineMd(s) {
  return esc(s)
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener" style="color:var(--accent);">$1</a>');
}
function renderBody(md) {
  const lines = md.split('\n');
  let html = '';
  let i = 0;
  let inList = false;
  while (i < lines.length) {
    const line = lines[i];
    if (/^## /.test(line)) {
      if (inList) { html += '</ul>'; inList = false; }
      html += `<h3>${inlineMd(line.replace(/^## /, ''))}</h3>`;
    } else if (/^> \*\*Revision log\*\*/.test(line)) {
      if (inList) { html += '</ul>'; inList = false; }
      html += `<h3>Revision log</h3>`;
    } else if (/^>\s*\|/.test(line)) {
      if (inList) { html += '</ul>'; inList = false; }
      const cells = line.replace(/^>\s*/, '').split('|').map(c => c.trim()).filter(Boolean);
      if (cells.every(c => /^-+$/.test(c))) { i++; continue; }
      html += `<div style="font-family:var(--mono);font-size:11px;color:var(--fg-dim);padding:2px 0;">${cells.map(inlineMd).join(' &middot; ')}</div>`;
    } else if (/^>/.test(line)) {
      if (inList) { html += '</ul>'; inList = false; }
      html += `<blockquote>${inlineMd(line.replace(/^>\s?/, ''))}</blockquote>`;
    } else if (/^-\s/.test(line)) {
      if (!inList) { html += '<ul>'; inList = true; }
      html += `<li>${inlineMd(line.replace(/^-\s/, ''))}</li>`;
    } else if (line.trim() === '') {
      if (inList) { html += '</ul>'; inList = false; }
    } else if (/^```/.test(line)) {
      if (inList) { html += '</ul>'; inList = false; }
      let code = [];
      i++;
      while (i < lines.length && !/^```/.test(lines[i])) { code.push(lines[i]); i++; }
      html += `<pre style="background:var(--term-bg);color:var(--term-fg);padding:10px 12px;border-radius:5px;overflow-x:auto;font-family:var(--mono);font-size:11.5px;">${esc(code.join('\n'))}</pre>`;
    } else {
      if (inList) { html += '</ul>'; inList = false; }
      html += `<p>${inlineMd(line)}</p>`;
    }
    i++;
  }
  if (inList) html += '</ul>';
  return html;
}
function chip(label, active, onClick) {
  const el = document.createElement('button');
  el.className = 'chip' + (active ? ' active' : '');
  el.textContent = label;
  el.onclick = onClick;
  return el;
}

const TABS = [
  { id: 'roadmap', label: 'Roadmap', count: () => DATA.milestones.length },
  { id: 'rfcs', label: 'RFCs', count: () => DATA.rfcs.length },
  { id: 'adrs', label: 'ADRs', count: () => DATA.adrs.length },
  { id: 'specs', label: 'Specs', count: () => DATA.specs.length },
  { id: 'bugs', label: 'Bugs', count: () => DATA.bugs.length },
  { id: 'query', label: 'Live query', count: () => null },
];
// ---- URL state: every filter/tab choice is a query param, so a reload or a
// shared link reproduces the exact view. Read once at load; write on every change.
const urlParams = new URLSearchParams(location.search);
let activeTab = urlParams.get('tab') || 'roadmap';
let msActivePhase = urlParams.get('phase');
let msActiveTrack = urlParams.get('track');
let msActiveStatus = urlParams.get('msStatus');
let rfcActiveStatus = urlParams.get('rfcStatus');
let adrActiveStatus = urlParams.get('adrStatus');

function syncUrl() {
  const p = new URLSearchParams();
  p.set('tab', activeTab);
  if (msActivePhase !== null) p.set('phase', msActivePhase);
  if (msActiveTrack !== null) p.set('track', msActiveTrack);
  if (msActiveStatus !== null) p.set('msStatus', msActiveStatus);
  if (rfcActiveStatus !== null) p.set('rfcStatus', rfcActiveStatus);
  if (adrActiveStatus !== null) p.set('adrStatus', adrActiveStatus);
  history.replaceState(null, '', '?' + p.toString());
}

function renderTabNav() {
  const nav = document.getElementById('tab-nav');
  nav.innerHTML = '';
  TABS.forEach(t => {
    const btn = document.createElement('button');
    btn.className = 'nav-item' + (activeTab === t.id ? ' active' : '');
    const c = t.count();
    btn.innerHTML = `<span>${t.label}</span>` + (c !== null ? `<span class="count">${c}</span>` : '');
    btn.onclick = () => { activeTab = t.id; renderTabNav(); renderPanels(); syncUrl(); };
    nav.appendChild(btn);
  });
}
function renderPanels() {
  TABS.forEach(t => {
    document.getElementById('panel-' + t.id).classList.toggle('active', activeTab === t.id);
  });
}

document.getElementById('stat-grid').innerHTML = `
  <div class="stat"><span class="n">${DATA.check.files_examined}</span><span class="l">Records checked</span></div>
  <div class="stat"><span class="n">${DATA.milestones.filter(m=>m.status==='Done').length}</span><span class="l">Milestones done</span></div>
  <div class="stat"><span class="n">${DATA.rfcs.filter(r=>r.status==='Draft').length}</span><span class="l">RFCs open</span></div>
  <div class="stat"><span class="n">${DATA.bugs.filter(b=>b.status==='Open').length}</span><span class="l">Bugs open</span></div>
`;

const ms = DATA.milestones;
const msTracks = [...new Set(ms.map(m => m.track))];
const msPhases = [...new Set(ms.map(m => m.phase))].sort();
const phaseLabels = {
  '0': 'Bootstrap — the tool validating and building itself',
  '1': 'Worth installing', '2': 'Agents can use it', '3': 'Differentiated bets', '4': 'Scale',
};
function msFilterChanged() { renderMsFilters(); renderMs(); syncUrl(); }
function renderMsFilters() {
  const pf = document.getElementById('ms-phase-filters'); pf.innerHTML = '';
  pf.appendChild(chip('all phases', msActivePhase===null, () => { msActivePhase=null; msFilterChanged(); }));
  msPhases.forEach(p => pf.appendChild(chip('phase ' + p, msActivePhase===p, () => { msActivePhase = msActivePhase===p?null:p; msFilterChanged(); })));

  const tf = document.getElementById('ms-track-filters'); tf.innerHTML = '';
  tf.appendChild(chip('all tracks', msActiveTrack===null, () => { msActiveTrack=null; msFilterChanged(); }));
  msTracks.forEach(t => tf.appendChild(chip(t, msActiveTrack===t, () => { msActiveTrack = msActiveTrack===t?null:t; msFilterChanged(); })));

  const sf = document.getElementById('ms-status-filters'); sf.innerHTML = '';
  sf.appendChild(chip('all statuses', msActiveStatus===null, () => { msActiveStatus=null; msFilterChanged(); }));
  ['Planned','InProgress','Blocked','Done','WontDo'].forEach(s => sf.appendChild(chip(s, msActiveStatus===s, () => { msActiveStatus = msActiveStatus===s?null:s; msFilterChanged(); })));
}

function renderMs() {
  const filtered = ms.filter(m =>
    (msActivePhase===null || m.phase===msActivePhase) &&
    (msActiveTrack===null || m.track===msActiveTrack) &&
    (msActiveStatus===null || m.status===msActiveStatus)
  );
  document.getElementById('ms-visible-count').textContent = filtered.length + ' shown';
  const byPhase = {};
  filtered.forEach(m => { (byPhase[m.phase] = byPhase[m.phase]||[]).push(m); });
  const container = document.getElementById('phase-groups');
  container.innerHTML = '';
  Object.keys(byPhase).sort().forEach(phase => {
    const phaseMs = byPhase[phase];
    const pgroup = document.createElement('div');
    pgroup.className = 'phase-group';
    pgroup.innerHTML = `
      <div class="phase-head">
        <span class="phase-badge">Phase ${esc(phase)}</span>
        <span class="phase-title">${esc(phaseLabels[phase] || 'Phase ' + phase)}</span>
        <span class="phase-count">${phaseMs.length} milestone${phaseMs.length===1?'':'s'}</span>
      </div>
    `;
    const byTrack = {};
    phaseMs.forEach(m => { (byTrack[m.track] = byTrack[m.track]||[]).push(m); });
    Object.keys(byTrack).forEach(track => {
      const group = document.createElement('div');
      group.className = 'track-group';
      group.innerHTML = `<div class="track-label">${esc(track)}</div>`;
      const list = document.createElement('div');
      list.className = 'rec-list';
      byTrack[track].forEach(m => {
        const row = document.createElement('div');
        row.className = 'rec-row';
        row.innerHTML = `
          <span class="rec-num">${m.num}</span>
          <span class="rec-title">${esc(m.title)}<span class="sub-line">Implements: ${esc(m.implements)}</span></span>
          <span class="status-pill status-${m.status}">${m.status}</span>
        `;
        const detail = document.createElement('div');
        detail.className = 'detail';
        detail.innerHTML = `<dl>
          <dt>What</dt><dd>${esc(m.what)}</dd>
          <dt>Why</dt><dd>${esc(m.why)}</dd>
          <dt>Blocked on</dt><dd>${esc(m.blocked)}</dd>
          <dt>File</dt><dd style="font-family:var(--mono);font-size:11px;">${esc(m.file)}</dd>
        </dl>`;
        row.onclick = () => detail.classList.toggle('open');
        list.appendChild(row);
        list.appendChild(detail);
      });
      group.appendChild(list);
      pgroup.appendChild(group);
    });
    container.appendChild(pgroup);
  });
}

function renderFlatList(containerId, records, columns, rowInnerHtml) {
  const container = document.getElementById(containerId);
  container.innerHTML = '';
  records.forEach(rec => {
    const row = document.createElement('div');
    row.className = 'rec-row';
    row.style.gridTemplateColumns = columns;
    row.innerHTML = rowInnerHtml(rec);
    const detail = document.createElement('div');
    detail.className = 'detail body-detail';
    detail.innerHTML = `<div class="file-path">${esc(rec.file)}</div>` + renderBody(rec.body || '');
    row.onclick = () => detail.classList.toggle('open');
    container.appendChild(row);
    container.appendChild(detail);
  });
}

function rfcFilterChanged() { renderRfcFilters(); renderRfcs(); syncUrl(); }
function renderRfcFilters() {
  const statuses = [...new Set(DATA.rfcs.map(r => r.status))];
  const sf = document.getElementById('rfc-status-filters'); sf.innerHTML = '';
  sf.appendChild(chip('all', rfcActiveStatus===null, () => { rfcActiveStatus=null; rfcFilterChanged(); }));
  statuses.forEach(s => sf.appendChild(chip(s, rfcActiveStatus===s, () => { rfcActiveStatus = rfcActiveStatus===s?null:s; rfcFilterChanged(); })));
}
function renderRfcs() {
  const filtered = DATA.rfcs.filter(r => rfcActiveStatus===null || r.status===rfcActiveStatus);
  document.getElementById('rfc-visible-count').textContent = filtered.length + ' shown';
  renderFlatList('rfc-list', filtered, '56px 1fr auto', r => `
    <span class="rec-num">RFC-${r.num}</span>
    <span class="rec-title">${esc(r.title)}</span>
    <span class="status-pill status-${r.status}">${r.status}</span>
  `);
}

function adrFilterChanged() { renderAdrFilters(); renderAdrs(); syncUrl(); }
function renderAdrFilters() {
  const statuses = [...new Set(DATA.adrs.map(a => a.status))];
  const sf = document.getElementById('adr-status-filters'); sf.innerHTML = '';
  sf.appendChild(chip('all', adrActiveStatus===null, () => { adrActiveStatus=null; adrFilterChanged(); }));
  statuses.forEach(s => sf.appendChild(chip(s, adrActiveStatus===s, () => { adrActiveStatus = adrActiveStatus===s?null:s; adrFilterChanged(); })));
}
function renderAdrs() {
  const filtered = DATA.adrs.filter(a => adrActiveStatus===null || a.status===adrActiveStatus);
  document.getElementById('adr-visible-count').textContent = filtered.length + ' shown';
  renderFlatList('adr-list', filtered, '56px 1fr auto', a => `
    <span class="rec-num">ADR-${a.num}</span>
    <span class="rec-title">${esc(a.title)}<span class="sub-line">Embodiment: ${esc(a.embodiment)}</span></span>
    <span class="status-pill status-${a.status}">${a.status}</span>
  `);
}

function renderSpecs() {
  document.getElementById('spec-visible-count').textContent = DATA.specs.length + ' shown';
  renderFlatList('spec-list', DATA.specs, '64px 1fr auto', s => `
    <span class="rec-num">SPEC-${s.num}</span>
    <span class="rec-title">${esc(s.title)}<span class="sub-line">v${esc(s.version)}</span></span>
    <span class="status-pill status-${s.status}">${s.status}</span>
  `);
}

function renderBugsTab() {
  document.getElementById('bug-visible-count').textContent = DATA.bugs.length + ' shown';
  renderFlatList('bug-list', DATA.bugs, '56px 1fr auto', b => `
    <span class="rec-num">BUG-${b.num}</span>
    <span class="rec-title">${esc(b.title)}<span class="sub-line">${esc(b.found_in)}</span></span>
    <span class="status-pill status-${b.status}">${b.status}</span>
  `);
}

renderTabNav();
renderPanels();
renderMsFilters(); renderMs();
renderRfcFilters(); renderRfcs();
renderAdrFilters(); renderAdrs();
renderSpecs();
renderBugsTab();

document.getElementById('q-explain').innerHTML = `<div class="term"><span class="comment"># urzua explain rust/crates/urzua-core/src/new_record.rs</span>\n${syntaxHighlightJSON(DATA.explain)}</div>`;
document.getElementById('q-graph').innerHTML = `<div class="term"><span class="comment"># urzua graph, filtered to MILE-* edges (${DATA.milestone_edges.length} of them)</span>\n${syntaxHighlightJSON(DATA.milestone_edges)}</div>`;
document.getElementById('q-check').innerHTML = `<div class="term"><span class="comment"># urzua check docs/  (full corpus, ${DATA.check.files_examined} files)</span>\n${syntaxHighlightJSON(DATA.check)}</div>`;
document.querySelectorAll('.qtab').forEach(tab => {
  tab.onclick = () => {
    document.querySelectorAll('.qtab').forEach(t => t.classList.remove('active'));
    document.querySelectorAll('.qpane').forEach(p => p.classList.remove('active'));
    tab.classList.add('active');
    document.getElementById('q-' + tab.dataset.q).classList.add('active');
  };
});
</script>
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", "-o", help="write to this path instead of stdout")
    args = parser.parse_args()

    root = repo_root()
    data = build_data(root)
    html = TEMPLATE.replace("__DATA__", json.dumps(data))

    if args.output:
        Path(args.output).write_text(html)
        print(f"wrote {len(html)} bytes to {args.output}", file=sys.stderr)
    else:
        sys.stdout.write(html)


if __name__ == "__main__":
    main()
