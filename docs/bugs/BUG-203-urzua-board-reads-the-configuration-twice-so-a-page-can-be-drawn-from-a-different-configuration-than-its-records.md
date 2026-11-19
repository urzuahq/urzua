---
Stable-Id: 01M7SKSP7Y7GD2MJ72CYB813F5
Status: Fixed
Found-in: "an edit to the configuration made while `urzua board` was building (0.4.12)"
Realized-by: test:rust/crates/urzua-cli/tests/board_integration.rs#a_page_is_drawn_from_the_one_configuration_its_records_were_read_with@sha256:e1506a30f742bcdf98323847a296db96a9173f83eeeecdef0cdd6849bb28c46a
---
# 203 — urzua board reads the configuration twice, so a page can be drawn from a different configuration than its records

## What was wrong

`urzua board` opened the repository once to read the records and a second time to read the declared types and statuses it prints in the page's prompt and About panel. A configuration edit between the two reads gave a page whose records came from one configuration and whose declared vocabulary came from another.

## Why nothing caught it

Both reads happened in one short call, and no test changed the configuration between them. The existing build-delay seam sat between the two reads but only the server tests used it.

## References

The page is now built from one session, so the records and the declared vocabulary come from the same configuration. `board_integration::a_page_is_drawn_from_the_one_configuration_its_records_were_read_with` adds a status to the configuration while the build waits, and asserts the page does not name it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
