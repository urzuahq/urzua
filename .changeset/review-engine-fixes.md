---
default: minor
closes: []
---

`check` now reports a stale `@sha256` evidence pin on any record as the warning `embodiment.pin-stale`,
on by default for every configuration. Until now only a record that states an `Embodiment`, an ADR
for example, drew a finding when a pinned file changed; a bug or a milestone that pins code or a test
drifted unseen. Upgrading can therefore surface stale-evidence warnings in a repository whose pins
went stale without notice. List them with `urzua fix --rebaseline`, read the change, and clear each
with `urzua fix --rebaseline --apply --ids <record> --by <you> --reason "<why the evidence still
holds>"`, or waive a record you choose to leave. Set `embodiment.pin-stale: off` under `rules:` to
turn the rule off, or another level to change it.

Also fixed: `fix` counts only the `Realized-by` locators that resolve, so a locator naming a missing
file no longer lifts a computed tier or is written as `Verified` or `Done`; the renewal check
`embodiment.claim-renewal-unattested` reads the record type's own header layout, so it sees an audit
date typed into a prefix-lines record; a `drift_audit_field` date after today is reported and
ignored; a heading inside an HTML comment is not a section; two records carrying one `Stable-Id` are
reported by `identity.collision`; `urzua waive --from-report` drafts omit the optional `Category`
and `Justified-by` keys; `urzua migrate filenames` rewrites `./` citations from files in other
directories and reports how many records it examined; `urzua reverify --apply` accepts a record that
starts with a byte-order mark; `urzua fix --rebaseline --apply` refuses a `--reason` of fewer than 3
words or 12 characters; a source file that is not UTF-8 is named in a warning when
`code.dangling-bug-reference` could not read it; and a usage error of `urzua mcp` is printed on
stderr, never on stdout.
