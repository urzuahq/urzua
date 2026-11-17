---
Stable-Id: 01M7KP4X2M714V5R150MMWHC4Y
Status: Planned
Phase: '1'
Track: integrations
Blocked-on: null
---
# 330 — The export carries what a view of the records needs

## What

Add to `urzua export --format json` the derived facts every consumer of the export otherwise recomputes: each record's title and a map of its fields, the category of its state, the records that refer to it with counts, its chain of referred-to records, its sections, and a link target.

## Why

On 2026-11-17 an agent drew a board of one repository's records (41 of them) from the export alone, in about fifty lines of script. That the export already carries the order of work made it a one-file job. What the script had to work around shows what any view would: a record's title exists only in its raw text, so it was taken with a regular expression on the heading line, which breaks where a repository's heading differs; the header is a list of name, value and line that every consumer first turns into a map, with a list-valued field as one string; a state has no category, so the script hard-coded which state names mean finished and under way; grouping does not follow a reference to the record it names, so the script joined records itself (tracked as `MILE-301`); nothing counts the records that refer to a record, by state, so "1 of 2 done" was computed by the page (the state itself is `MILE-300`); body sections and their checkbox lines are unparsed; and there is only a path, no link target for a hosted page.

## Done means

- Each exported record carries `title`, taken by the same rule the engine's title checks use, shown by a test with two heading forms.
- Each exported record carries a `fields` map beside the existing header list, with a list-valued field as an array, shown by a test; the header list is unchanged.
- Each exported record carries the category of its state, one of a small fixed set (not started, under way, finished, dropped, none), derived only from what the configuration declares for the type (its initial, success and failure sets and any further declaration this milestone adds), and the export says which declaration decided it, shown by a test.
- Each exported record lists, per reference field, the records that refer to it, with counts by state category, shown by a test with a record referred to by three others.
- Each exported record carries its chain of referred-to records through reference fields that declare target types, so a consumer can group by an ancestor without joining, shown by a test of a three-level chain.
- Each exported record carries its sections keyed by heading with, per section, the counts of ticked and unticked checkbox lines, shown by a test.
- A configuration can declare a link prefix, and each exported record then carries a link, shown by a test; without the declaration there is none.
- The additions are additive: a consumer of the release's export reads the new one unchanged, shown by a test comparing the two on this repository's records with the new keys removed.
- The read-only server's record tools return the same additions.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Reported on 2026-11-17 by an agent in an adopting repository at the maintainer's request, after it drew a board of that repository's records from the export; the maintainer asked for such a view to be something the tool supports. | **structural** |
