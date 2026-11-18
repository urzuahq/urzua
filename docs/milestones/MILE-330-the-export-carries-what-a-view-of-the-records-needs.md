---
Stable-Id: 01M7KP4X2M714V5R150MMWHC4Y
Status: Planned
Phase: '1'
Track: integrations
Blocked-on: null
---
# 330 — The export carries what a view of the records needs

## What

Add to `urzua export --format json` the derived facts every consumer of the export otherwise recomputes: each record's title and a map of its fields, the category of its state, the records that refer to it with counts, its chain of referred-to records, checkbox counts for its sections, and a link target.

## Why

On 2026-11-17 an agent drew a board of one repository's records (41 of them) from the export alone, in about fifty lines of script. That the export already carries the order of work made it a one-file job. What the script had to work around shows what any view would: a record's title exists only in its raw text and preamble, so it was taken with a regular expression on the heading line, which breaks where a repository's heading differs; the header is a list of name, value and line that every consumer first turns into a map, with a list-valued field as one string; a state has no category, so the script hard-coded which state names mean finished and under way; grouping does not follow a reference to the record it names, so the script joined records itself (tracked as `MILE-301`); nothing counts the records that refer to a record, by state, so "1 of 2 done" was computed by the page (the state itself is `MILE-300`); sections are exported as text, with their checkbox lines uncounted; and there is only a path, no link target for a hosted page.

## Done means

- Each exported record carries `title`, taken by the same rule the engine's title checks use, shown by a test with two heading forms.
- Each exported record carries a `fields` map beside the existing header list, with a list-valued field as an array, shown by a test; the header list is unchanged.
- Each exported record carries the category of its state, one of a small fixed set (not started, under way, finished, dropped, none), derived only from what the configuration declares for the type (its initial, success and failure sets and any further declaration this milestone adds), and the export says which declaration decided it, shown by a test.
- Each exported record lists, per reference field, the records that refer to it, with counts by state category, shown by a test with a record referred to by three others.
- Each exported record carries its chain of referred-to records through reference fields that declare target types, so a consumer can group by an ancestor without joining, shown by a test of a three-level chain.
- Each exported section carries the counts of its ticked and unticked checkbox lines, shown by a test; the existing section keys are unchanged.
- A configuration can declare a link prefix, and each exported record then carries a link, shown by a test; without the declaration there is none.
- The additions are additive: a consumer of the release's export reads the new one unchanged, shown by a test comparing the two on this repository's records with the new keys removed.
- The read-only server's record tools return the same additions.

## Amendment (2026-11-18): what a second look at the same board found the export still lacks

The agent that drew the first board checked its script again on 2026-11-18 against release 0.4.9 and reported what it had computed itself, or got wrong, beyond what the Done-means above lists. These are added to this milestone's scope; the items above are unchanged.

- A state for every record, not only for records the order of work lists: the order of work leaves out finished records, records of types outside `work_types`, and parents, so "absent" was read as "ready" and finished work was counted as ready. Every exported record carries its state and, separately, what it waits on; a record that is under way and waiting carries both.
- A dropped record (a status in the type's failure set) is its own state, not "ready".
- A record's parent and children through the references the configuration declares as containment, with a count of the children by state, so a consumer can show that a parent's stated status and its children disagree. `roadmap` grouping by a field does not follow a reference from a child to its parent's own group today; the export must.
- Records in numeric order within a type; the export's order is by text today, so 10 sorts before 2.
- For a section the configuration declares as a checklist, the per-item result the engine already computes (the item's text, whether it resolves, its evidence locator, whether it is waived), not only counts.
- The resolved references between records as a list of edges, as `urzua graph` has them, inbound and outbound.
- When and from what the data was read: commit, branch, whether records have uncommitted changes, and the time.
- Size: on a 60-record repository the export was 193 KB, 107 KB of it `raw_text`, with `sections` repeating most of that. A way to export without the raw text.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Reported on 2026-11-17 by an agent in an adopting repository at the maintainer's request, after it drew a board of that repository's records from the export; the maintainer asked for such a view to be something the tool supports. | **structural** |
> | 2026-11-18 | Amendment added: eight further things the export lacks for a view, from the same adopting repository's second report against release 0.4.9. **Why:** it was asked for input on a built-in board (`MILE-344`), and listed what its own script had computed or got wrong. | **substantive** |
