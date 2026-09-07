# 0005 — Build the y-statement content shape

> Status: Planned
> Stable-Id: 01M1Y5JEPZRBT40QA2DEXT2CM2
> Phase: 0
> Track: section-checks
> Implements: RFC-0017

## What

Extract a Decision section's TL;DR block and check it contains the five Y-statement structural phrases, word-boundary matched, with a distinct error when the marker exists but no extractable block follows.

## Why

The first real content shape, proving the section-parser design; validated against two independent real implementations that converged on the same extraction/matching approach.

## Blocked on

Milestone: Build the closed section-parser and required_sections config schema

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
