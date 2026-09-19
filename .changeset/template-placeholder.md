---
default: minor
---

`field.quality` recognises an unedited field by comparing it against its type's
template, not against a fixed vocabulary (`BUG-22`).

A record whose every field still held template text passed with zero findings.
Placeholders were matched against seven tokens -- `name`, `name(s)`,
`yyyy-mm-dd`, `tbd`, `todo`, `(project lead)`, `(session author)` -- every one
of them transcribed by hand from this project's own templates. A corpus whose
template reads `<status>` or `Your Name Here` got no protection at all, which is
exactly the adopter path: run `init`, fill in a template, be told the corpus is
clean.

A field still holding its template's value for that field is now reported,
whatever the convention. A type with no template falls back to the token list
rather than passing everything.
