---
Stable-Id: 01M7KRXCQTZ4YDNK2MP7NJ6PEG
Status: Planned
Phase: '1'
Track: embodiment-model
Blocked-on: null
---
# 334 — A checklist item can cite an observation as evidence

## What

Give a checklist item an evidence form for something a person or agent observed and attests to, recorded with who, when and what was seen, and counted separately from evidence that resolves to a file, a test or a record.

## Why

A checklist item's evidence clause accepts only locators: a path under a category such as code or test, or a cited record. On release 0.4.7 a commit hash, a pull request number and free text such as "ran it on 2026-11-17 and saw the line" are each rejected with the same shape error. In the adopting agent's records many acceptance checks are observations: a hook was seen to fire in a live session, a page was seen to render. An adopter's choices today are to commit a log and cite it under a category that misdescribes it, or to waive the item, which reports it as not met. Either way the record says something other than what happened, and the pressure is towards prose acceptance checks the tool does not read, which is how an adopting repository came to hold a record about to be marked finished with two of three checks unmet. That repository now commits its run evidence with private content removed; its agent asked on 2026-11-17 for a category that names such a file truthfully, and judged a free-text observation weaker than a committed file.

## Done means

- A configuration can declare an evidence form for an attested observation, with the fields an observation must carry (who attests, the date, what was observed) and whether the type accepts it, shown by a test; a type that does not declare it rejects it as today.
- An item evidenced only by an observation is reported as met by attestation, a state distinct from met by a resolving locator, and the configuration declares whether that state satisfies the finished status. The default is that it does not: a record at the finished status with only attested items is reported at the level of the rule that holds a finished status to its items. Information is for an item met by attestation on a record not at a finished status, shown by a test of each.
- An observation is written by a command that records the attesting identity the way other attested rows are recorded, not typed by hand, and a hand-typed one is reported, shown by a test.
- A configuration can declare an evidence category for a committed record of an observation (a log extract, a report), which requires a file and is checked like any other pin, so such a file is not cited under a category that calls it code, shown by a test.
- An observation can carry a locator to a committed log or transcript, which is then checked like any other pin, and the reference page shows the three forms side by side: a test, a committed log, an attested observation.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Found on 2026-11-17 while setting up a checklist for an adopting repository's type on release 0.4.7, after the maintainer asked why that repository's unmet acceptance checks had not been caught without a model. | **structural** |
