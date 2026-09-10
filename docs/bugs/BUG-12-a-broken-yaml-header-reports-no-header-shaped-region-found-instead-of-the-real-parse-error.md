---
Stable-Id: 01M2405KW390GQJDF7XSRR3JX8
Status: Fixed
Found-in: 'hit live, twice in one session: a hand-written Subject value starting with a backtick (an invalid unquoted YAML plain-scalar start) on 9 specs, and an unquoted colon-space inside a plain scalar on a bug record''s own header -- both produced the same unhelpful message'
Regression-test: 'rust/crates/urzua-core/src/header.rs :: a_value_starting_with_a_backtick_surfaces_the_real_parse_error_observed_failing, invalid_yaml_surfaces_the_real_parse_error_observed_failing; rust/crates/urzua-core/src/rules.rs :: a_broken_yaml_header_surfaces_the_real_parse_error_observed_failing'
---
# 12 — A broken YAML header reports "no header-shaped region found" instead of the real parse error

## What was wrong

`parse_yaml_frontmatter` (`header.rs`) discards the actual parse error whenever the YAML block fails
to parse as a mapping:

```rust
let Ok(yaml_serde::Value::Mapping(mapping)) =
    yaml_serde::from_str::<yaml_serde::Value>(&yaml_text)
else {
    return Header { fields: Vec::new(), region: None };
};
```

The `Err` variant -- which `yaml_serde` populates with a real line number and reason -- is thrown
away. Every rule that then examines this record reports only the generic, uninformative
`header.required-fields` message `"no header-shaped region found -- required fields [...] cannot be
checked"`, plus every required field showing as `field.quality`'s `Blank`. Nothing in the output
tells a reader *why* the header failed to parse, only that it did.

Concretely hit twice this session: a hand-written `Subject:` value starting with a backtick (`` ` ``
is a reserved YAML indicator character and cannot start an unquoted plain scalar) on 9 spec files,
and separately, a bug record's own `Regression-test:` value containing an unquoted `: ` (colon-space)
mid-scalar, ambiguous with a nested mapping. Both were diagnosed by manually inspecting the raw bytes
of the file rather than from anything `check` reported -- the second instance wasn't caught locally
at all and only surfaced when CI ran the same check against the same content.

## Why nothing caught it

No test plants a genuinely malformed YAML header and asserts on the resulting message content --
existing tests for `header.required-fields`/`field.quality` all use headers that parse successfully
but are missing or blank on individual fields, never a header that fails to parse as YAML at all.
The discarded `Err` was a deliberate, reasonable design choice at the time (`region: None` needing to
mean the same thing regardless of *why* parsing failed, across all three header shapes) but nobody
weighed the cost: for `yaml-frontmatter` specifically, `yaml_serde`'s error already contains exactly
the diagnostic information a human or agent needs, and dropping it forces manual byte-level
diagnosis every time.

## References

- `rust/crates/urzua-core/src/header.rs` -- `parse_yaml_frontmatter`, where the `Err` is discarded.
- `rust/crates/urzua-core/src/rules.rs` -- `header_required_fields`/`field_quality`, whose messages
  are the reader's only signal that something is wrong.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-09 | Initial bug record, `Status: Open`. Not yet fixed -- whether to surface `yaml_serde`'s raw error message verbatim, or wrap it in a still-generic-but-more-specific diagnosis, is not yet decided. | **structural** |
> | 2026-09-10 | Fixed: `Header` gains a `parse_error: Option<String>` field, populated for `yaml-frontmatter` with `yaml_serde`'s own error verbatim on a genuine syntax failure, or a specific "must be a mapping" message when the YAML parses but isn't the right shape. `header.required-fields`'s message now appends `-- YAML parse error: <detail>` when present, instead of the same generic "no header-shaped region found" every other `None` case reports. Scope decision: surface the raw message verbatim (not a re-wrapped diagnosis) -- `yaml_serde` already includes a line and reason, and re-deriving that ourselves would just be a second, likely-drifting copy of the same information. `Status: Fixed`. | **substantive** |
