---
default: minor
closes: [BUG-196]
---

**Upgrading: `embodiment.pin-stale` is on by default, at `warn`.** `check` now reports a stale
`@sha256` evidence pin on any record, whatever the record states. Until now only a record that
states an `Embodiment`, an ADR for example, drew a finding when a pinned file changed; a bug or a
milestone that pins code or a test drifted unseen. A repository whose pins went stale unseen sees
those warnings on its first run after upgrading, and **a repository with `fail_on: warning` sees CI
fail on them.** There are two ways out:

- Re-pin each record after reading the change: `urzua fix --rebaseline` lists them (it runs whether
  or not `embodiment.consistency` or `status.delivery-agreement` is declared), and `urzua fix
  --rebaseline --apply --ids <record> --by <you> --actor <agent|human> --reason "<why the evidence
  still holds>"` clears each. A record you choose to leave can be waived.
- Or set the rule's level under `rules:`: `embodiment.pin-stale: info` keeps the report without
  failing a `fail_on: warning` run, and `embodiment.pin-stale: off` turns it off.

**New refusal: a short `--reason`.** `urzua fix --rebaseline --apply` and `urzua reverify --apply`
refuse a `--reason` of fewer than 3 words or 12 characters (`--reason r` used to be accepted). The
minimum is configuration: `attestation: {reason: {min_words: 3, min_chars: 12}}` are the defaults,
and a repository may lower or raise either.

Also fixed: `fix` counts only the `Realized-by` locators that resolve, so a locator naming a missing
file no longer lifts a computed tier or is written as `Verified` or `Done`; the renewal check
`embodiment.claim-renewal-unattested` reads the record type's own header layout, so it sees an audit
date typed into a prefix-lines record; a `drift_audit_field` date after today is reported and
ignored; a heading inside an HTML comment is not a section, however many comments open and close on
one line; two records carrying one `Stable-Id` are reported by `identity.collision`; `urzua waive
--from-report` drafts omit the optional `Category` and `Justified-by` keys; `urzua migrate filenames`
rewrites `./` citations from files in other directories and reports how many records it examined;
`urzua reverify --apply` accepts a record that starts with a byte-order mark; a source file that is
not UTF-8 is named in a warning when `code.dangling-bug-reference` could not read it; and a usage
error of `urzua mcp` is printed on stderr, never on stdout.
