---
Stable-Id: 01M7SKSQHN3CDJ2PZ2K2X8HF47
Status: Fixed
Found-in: "`urzua board --prompt --port 8080`, 0.4.12"
Realized-by: test:rust/crates/urzua-cli/tests/board_integration.rs#the_prompt_is_the_same_in_the_page_and_a_flag_that_takes_no_other@sha256:af731ebec827d0b9c91c3fddff9add9c182721975ffe7d9d1dea69e04283deb9
---
# 204 — urzua board --prompt accepts --port and --allowed-host, which only mean something with --serve

## What was wrong

`urzua board --prompt` accepted `--port` and `--allowed-host`, which only mean something with `--serve`, and ignored them. `--prompt` with `--serve` or `--out` already exited 2.

## Why nothing caught it

The two flags `require` `--serve`, and `--prompt` conflicted only with `--serve` and `--out`, so a flag that conflicts with `--prompt` through `--serve` was not stated for the flags that need it. The test listed only the two existing conflicts.

## References

`--port` and `--allowed-host` now conflict with `--prompt`, and the command exits 2. `board_integration::the_prompt_is_the_same_in_the_page_and_a_flag_that_takes_no_other` covers each.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
