---
default: minor
closes: []
---

Add the opt-in `citation.issue-exists` rule and the `urzua refs refresh` command. An issue or pull
request cited in a declared reference section, as `#123` or as a link to the declared repository,
must be in a snapshot file committed to the repository; `issue_refs` names the snapshot, the
repository and the citation forms to read (`number`, `url`). `check` only reads the file and never
uses the network. If the snapshot is absent, unreadable or describes another repository the rule
reports that instead of passing, and every finding and an info notice name the date the snapshot
was generated. `urzua refs refresh` rewrites the snapshot from the GitHub REST API: it reads a
token from `GITHUB_TOKEN` or `GH_TOKEN` when one is set, pages through the listing, writes sorted
output, and on any failure (no network, a rate limit, an unknown repository) exits 2 and leaves the
existing file untouched.
