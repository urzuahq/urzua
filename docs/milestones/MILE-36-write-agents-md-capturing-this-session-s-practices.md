---
Status: Done
Stable-Id: 01M1Y9H4N2TZ88D8CTWZP6G5RG
Phase: '0'
Track: roadmap-tracking
Implements: —
Blocked-on: —
Realized-by: code:AGENTS.md@sha256:76cd697c2a776fb5884d1aa961518217fe33d82940608245b3aea7300158ee10
---
# 36 — Write AGENTS.md capturing this session's practices

## What

`AGENTS.md`: durable instructions for any AI agent working in this repo -- use the tool's own
record types to track work, verify claims against real runs rather than memory, never silently
rewrite an Accepted decision, and don't build speculative capability.

## Why

A lot of hard-won practice from this project's own history lived only in conversation transcripts,
with no durable, discoverable home a fresh agent session would actually read. Named directly as
something worth writing rather than letting it keep living nowhere.

## Done means

- `AGENTS.md` exists at the repository root. — evidence: code:AGENTS.md@sha256:76cd697c2a776fb5884d1aa961518217fe33d82940608245b3aea7300158ee10
- `AGENTS.md` instructs an agent to track work in the tool's own record types. — evidence: code:AGENTS.md@sha256:76cd697c2a776fb5884d1aa961518217fe33d82940608245b3aea7300158ee10
- `AGENTS.md` instructs an agent to verify claims against real runs rather than memory. — evidence: code:AGENTS.md@sha256:76cd697c2a776fb5884d1aa961518217fe33d82940608245b3aea7300158ee10
- `AGENTS.md` instructs an agent never to silently rewrite an Accepted decision (amend with a dated `## Amendment`). — evidence: code:AGENTS.md@sha256:76cd697c2a776fb5884d1aa961518217fe33d82940608245b3aea7300158ee10
- `AGENTS.md` instructs an agent not to build speculative capability. — evidence: code:AGENTS.md@sha256:76cd697c2a776fb5884d1aa961518217fe33d82940608245b3aea7300158ee10
- Each of these is a section an agent reads at session start. — evidence: code:AGENTS.md@sha256:76cd697c2a776fb5884d1aa961518217fe33d82940608245b3aea7300158ee10

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial record: already complete at creation time. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: AGENTS.md@sha256:0d60e75506df Reason: Pinned the files as they are now after checking each against the record it evidences: schema_version.rs, commands/migrate.rs, table.rs and config.yaml still carry the schema version window and the cross-record rules (MILE-34), AGENTS.md still instructs agents to track work in the record types, verify against real runs, never silently rewrite an Accepted decision and not build speculative capability (MILE-36), prepare-release.yml still triggers only on push to main and opens a release pull request with knope.toml and release-guard.sh (MILE-39), section.rs still implements required sections and the y-statement shape (MILE-4, MILE-5), CHANGELOG.md, rust/Cargo.toml and the release workflows still show releases past v0.1.0 consuming changesets (MILE-40), report.rs, action.rs, table.rs and generate-rule-table.py still give every finding a suggested action (MILE-44), report.rs, config.rs and runner.rs still carry the info level (MILE-46), document_model.rs still reads a second corpus's declared model (MILE-51), and the Phase A rule modules, config.yaml and checks.yml still hold the header, pointer, field, filename and supersession rules (MILE-56). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `AGENTS.md` 0d60e75506df -> 426077782868. Reason: AGENTS.md changed only in its Accepted-risk list (commits seen on the checked-out branch, MCP shell and patch tools, invalid configuration, the unprotected instructions block, ungated waivers, which repositories judge an edit); the sections this record cites are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `AGENTS.md` 426077782868 -> 80f51eb5003b. Reason: Read the diff of AGENTS.md against main. It gained one section, Reading records over MCP (optional), before the generated harness block; nothing else changed, so the claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `AGENTS.md` 80f51eb5003b -> 48d431863e87. Reason: Read the diff of AGENTS.md against main: one bullet of the accepted-risk list changed, about the agent's settings (user-level settings outside the repository are neither protected nor read; the project's .claude/settings.local.json is protected as a path under .claude but neither read nor wired by the install). No other line changed. The claim this record evidences (AGENTS.md captures the session's practices) still holds: the practices are unchanged and one accepted-risk line is now exact. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `AGENTS.md` 48d431863e87 -> b4c539ee8370. Reason: Read the diff of AGENTS.md against main: one bullet changed, the one on urzua new's provisional display number, which now says the skipped numbers are listed in the skipped field of its JSON and never on stderr. Nothing else in the file changed, so what this record cites AGENTS.md for still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `AGENTS.md` 80f51eb5003b -> 16dbb7a239e9. Reason: Read the diff of AGENTS.md: one paragraph is added to 'A record's Status: who sets it' saying a maintainer-decision row quotes the instruction or cites the PR or review, and that a changed Done-when item keeps its text and is waived. No other line changed, so the claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `AGENTS.md` 48d431863e87 -> 2ea89528735a. Reason: Read the diff of AGENTS.md since the previous pin: one paragraph is added to 'A record's Status: who sets it' saying a maintainer-decision row quotes the instruction or cites the PR or review, and that a changed Done-when item keeps its text and is waived. No other line changed, so the claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `AGENTS.md` b4c539ee8370 -> 76cd697c2a77. Reason: Read the diff of AGENTS.md since the previous pin: one paragraph is added to 'A record's Status: who sets it' saying a maintainer-decision row quotes the instruction or cites the PR or review, and that a changed Done-when item keeps its text and is waived. No other line changed, so the claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
