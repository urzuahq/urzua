---
Stable-Id: 01M7SKSP7Y7GD2MJ72CYB813F5
Status: Fixed
Found-in: "an edit to the configuration made while `urzua board` was building (0.4.12)"
Realized-by: test:rust/crates/urzua-cli/tests/board_integration.rs#a_page_is_drawn_from_the_one_configuration_its_records_were_read_with@sha256:265090929139094c01cf2a8e8e4ecb0c3dcde0fcfc4f882e6e29df4db47991ef
---
# 203 — urzua board reads the configuration twice, so a page can be drawn from a different configuration than its records

## What was wrong

`urzua board` opened the repository once to read the records and a second time to read the declared types and statuses it prints in the page's prompt and About panel, and the check summary it prints in file mode ran `check` through a third session. A configuration edit between the reads gave a page whose records came from one configuration and whose declared vocabulary or check summary came from another.

## Why nothing caught it

Both reads happened in one short call, and no test changed the configuration between them. The existing build-delay seam sat between the two reads but only the server tests used it.

## References

The page is now built from one session, and the check summary is computed on that session (`check::execute_on`), so the records, the declared vocabulary and the summary come from the same configuration. `board_integration::a_page_is_drawn_from_the_one_configuration_its_records_were_read_with` adds a status to the configuration while the build waits, and asserts the page does not name it and that its check summary is the one the unedited configuration gives.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-19 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/board_integration.rs#a_page_is_drawn_from_the_one_configuration_its_records_were_read_with` (lines 3442-3481) e1506a30f742 -> 265090929139. Reason: the test also edits the configuration to enable a rule that fires, and asserts the check summary is the unedited configuration's, so it fails when `check` reads a different configuration than the records **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
