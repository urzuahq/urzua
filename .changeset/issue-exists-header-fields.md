---
default: minor
closes: [MILE-184]
---

`citation.issue-exists` can now also check named header fields. Declare `issue_refs.fields: ["Found-in"]`
and a `#N` or repository link in that field of any record is held to the committed snapshot like one under a
reference section; a finding names the field and its line. A name no record type declares is refused when the
config loads. Off unless declared, so existing configs behave as before.
