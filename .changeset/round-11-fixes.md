---
default: patch
---

Fixes BUG-94: a rule reporting that it was handed nothing no longer certifies the corpus `ok`. The
population added this release said `eligible: 0` while the same report said `status: ok`, because the
gate read only the older `scope` field.

Fixes BUG-95: `field.quality` and `field.pending` report `records_examined` as a record count again.
They had changed its unit to field slots in the same release that documents the field as
record-shaped, so one entry contradicted itself — `6` beside `scope: records` on a two-record corpus.

Fixes BUG-96: the release invariant guards run in CI. They were wired into `make ci` only, which fires
when a human runs it — not the unattended path both defects they guard against actually took. An
undeterminable tag is now a failure rather than being read as agreement.
