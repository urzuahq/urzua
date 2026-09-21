---
default: major
---

**Breaking: header field names are compared exactly against a type's declared fields.** They were
previously compared case-insensitively, so a record writing `status` matched a config declaring
`Status`. A corpus relying on that now gets a finding, and the finding names the declared spelling:
`field 'status' is not declared for record type 'adr' -- the type declares 'Status', differing only in
case`.

Fixes BUG-97, where the comparison was ASCII-only and reported a declared non-ASCII field name as
undeclared with no spelling an adopter could use to fix it. ADR-57 records why the comparison was
removed rather than corrected: every correct case-insensitive comparison makes `Maße` and `Masse` one
field, and accepting an undeclared name is a missing finding.
