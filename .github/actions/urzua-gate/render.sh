#!/usr/bin/env bash
# Renders urzua's JSON reports for a pull request. A consumer of the JSON
# contract, outside the engine: stdout of `urzua check` stays JSON and
# this reads it.
#
#   render.sh OUT_MD [LABEL REPORT_JSON EXIT_CODE]...
#
# Prints GitHub workflow commands (::error / ::warning, one inline annotation
# per distinct finding that has a file) on stdout, and writes the summary
# comment body to OUT_MD. A run that did not produce a report (exit 2, status
# not-run, a missing or non-JSON file) is rendered as NOT RUN with its reason,
# never as clean. Always exits 0: the caller owns the gate's verdict.
set -uo pipefail

marker='<!-- urzua-gate -->'
max_rows=50
out="$1"
shift

# A report can be megabytes (every waived finding is listed), and one argv entry
# is capped near 128 KB, so reports reach jq through files and stdin, never argv.
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
runs='[]'
while [ "$#" -ge 3 ]; do
  label="$1" file="$2" code="$3"
  shift 3
  if [ -s "$file" ] && jq -e 'type == "object"' "$file" >/dev/null 2>&1; then
    cp "$file" "$work/report.json"
  else
    echo null >"$work/report.json"
  fi
  runs="$(jq -c --arg label "$label" --argjson code "$code" --slurpfile report "$work/report.json" '
    . + [{label: $label, code: $code, report: $report[0]}]' <<<"$runs")"
done

jq -r --arg marker "$marker" --argjson max "$max_rows" '
  def esc: gsub("%"; "%25") | gsub("\r"; "%0D") | gsub("\n"; "%0A");
  def escp: esc | gsub(":"; "%3A") | gsub(","; "%2C");
  def cell: gsub("\\|"; "\\|") | gsub("\n"; " ");
  # A waived finding is a recorded decision, not an open finding: it is
  # counted, never annotated or listed.
  def open_findings: [(.report.findings // [])[] | select(.waived == null)];
  def waived_count: [(.report.findings // [])[] | select(.waived != null)] | length;
  def notrun: .code == 2 or .report == null or (.report.status == "not-run");
  def reason:
    if .report == null then "no JSON report was produced (exit code \(.code))"
    else (.report.error // "the run reported status not-run") end;

  . as $runs
  | ([ $runs[] | select(notrun | not) | open_findings[] ]
      | unique_by([.rule, .file, .line, .message])) as $findings
  | ($runs | map(select(notrun))) as $notrun
  | ($runs | map(select((notrun | not) and .code == 1)) | length > 0) as $blocking
  | {
      annotations: [
        $findings[] | select(.file != null)
        | "::\(if .severity == "error" then "error" elif .severity == "info" then "notice" else "warning" end) file=\(.file | escp)\(if .line != null then ",line=\(.line)" else "" end),title=\(.rule | escp)::\(.message | esc)"
      ][:$max],
      body: (
        [$marker, "## urzua gate", ""]
        + [ $runs[] | if notrun then "- `\(.label)`: **NOT RUN** -- \(reason)"
            else "- `\(.label)`: \(open_findings | length) finding(s)\(if waived_count > 0 then ", \(waived_count) waived" else "" end)\(if .code == 1 then ", blocking" else "" end)" end ]
        + [""]
        + [ if ($notrun | length) > 0 then "**The gate did not run to completion.** A run that did not run is not a pass."
            elif $blocking then "**Blocking findings -- this pull request fails the gate.**"
            elif ($findings | length) > 0 then "Passed. The findings below do not block."
            else "Clean: no findings." end ]
        + (if ($findings | length) > 0 then
            ["", "| Severity | Rule | Location | Message |", "|---|---|---|---|"]
            + [ $findings[:$max][]
                | "| \(.severity) | `\(.rule)` | \(if .file != null then "`\(.file)\(if .line != null then ":\(.line)" else "" end)`" else "" end) | \(.message | cell) |" ]
            + (if ($findings | length) > $max then ["", "_\(($findings | length) - $max) more finding(s) not shown; see the job log._"] else [] end)
          else [] end)
      ) | join("\n")
    }
  | (.annotations[]), "::urzua-body::" , .body
' <<<"$runs" >"$out.tmp"

# Split annotations (stdout) from the body (file).
sep="$(grep -n '^::urzua-body::$' "$out.tmp" | head -1 | cut -d: -f1)"
if [ -n "$sep" ]; then
  [ "$sep" -gt 1 ] && head -n "$((sep - 1))" "$out.tmp"
  tail -n "+$((sep + 1))" "$out.tmp" >"$out"
else
  echo "${marker}
## urzua gate

**NOT RUN** -- the renderer could not read the reports." >"$out"
fi
rm -f "$out.tmp"
exit 0
