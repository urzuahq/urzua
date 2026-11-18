---
Stable-Id: 01M7QNN8WM2WZXNWE7M2X8TPDP
Status: Fixed
Found-in: "an adopting repository's first use of `urzua board` on a phone, 0.4.11 (2026-11-18)"
Realized-by: test:rust/crates/urzua-cli/tests/board_integration.rs#at_phone_width_a_title_wraps_and_the_detail_drops_to_a_line_under_it@sha256:3a5ed3a4fa84efb0d83b42f4faef285293003d80de2bb0dc8ad3fe71d67e7d5a
---
# 202 — At phone width the board cuts each row's title to about fifteen characters and hides the row's detail

## What was wrong

At about 390 px wide every row's title was cut to roughly fifteen characters and an ellipsis, so rows
under one parent could not be told apart, and the right-hand detail of a tree row (what it waits on, a
declared-value chip, the checklist count, how many records refer to it, a parent's finished count and
its "disagrees" mark) was not shown at all. The narrow-screen rules hid every detail span and kept the
title on one line beside the id and a status group that took up to 45% of the row.

## Why nothing caught it

The page's tests checked that it carried a viewport declaration and both palettes, and that it had no
sideways scroll at 360 px, but nothing asserted that a row's content survives at that width: the
rules that gave a row room by dropping its detail were the intended design, and no one read the
result on a phone with titles of real length.

## References

Below 460 px a row's title wraps to up to three lines and its details, dot and status move to a line
under it; the wide layout is unchanged. `board_integration::at_phone_width_a_title_wraps_and_the_detail_drops_to_a_line_under_it`
asserts the rules and the markup they act on. See `docs/reference/board.md`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-18 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 0 file-level, 1 scoped. Evidence: rust/crates/urzua-cli/tests/board_integration.rs#at_phone_width_a_title_wraps_and_the_detail_drops_to_a_line_under_it@sha256:3a5ed3a4fa84 Reason: the regression test asserts the narrow-screen rules that wrap the title and move the detail under it, and the markup they act on; run green on this branch **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-18 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `Realized-by computes to Verified` -- stated `Open` disagreed with the computed value `Fixed`. | **structural** |
