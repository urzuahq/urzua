# `platform/` — deployment targets and infrastructure

Reserved per [ADR-0004](../docs/adr/0004-polyglot-monorepo-layout.md). Empty of code for now; this
README exists so the convention is discoverable rather than tribal.

## What belongs here

Anything about *running* Urzua somewhere, organized by environment or deployment target:

- Infrastructure-as-code for hosted components.
- Environment-specific configuration (`dev/`, `prod/`).
- Deployment and release automation targets.

## Nothing is hosted yet

v0 is an offline, deterministic CLI ([SPEC-0001](../docs/specs/0001-v0-cli.md)) — it has no server,
no service, and no infrastructure. This directory stays empty until something is genuinely deployed.

That's deliberate rather than incidental. The moment Urzua has a hosted component it acquires
uptime, data-handling, and access-control obligations for other organizations' decision records —
which is a materially different product with materially different risk. **That step deserves its own
ADR before any code lands here**, not an infrastructure directory that quietly fills up.

## Release artifacts

Release builds come from [`rust/`](../rust), not from here — release automation must not assume a
workspace at the repo root.
