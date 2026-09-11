---
Stable-Id: 01M286A8CHGVMQ2WTSPVX8G3CK
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/report.rs, code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-core/src/report.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
Date: 2026-09-11
Author: '@beauwilliams'
Deciders: '@beauwilliams'
Supersedes / Superseded-by: —
Derives-from: BUG-19
---
# 46 — One shared output contract: `Report`, `Notice`, `emit()`

## Context

BUG-19 found `check`, `fix`, and `new`/`explain`/`graph`/`doctor` each hand-rolling a different,
incompatible JSON shape, contradicting ADR-7's own stated rule that every future command must share
one contract. The immediate trigger was narrower: `resolve_identity`'s gh-vs-`--by` divergence notice
was a bare `eprintln!` citing an internal ADR number -- meaningless to a tool consumer with no such
ADR in their own repo, and unparseable by an agent. Fixing that one call site surfaced the wider gap.

Three real design questions had to be settled before writing any of this, in order:

1. **How does a non-fatal observation (a "notice") coexist with `check`'s existing `Finding`?**
   `Finding.file: PathBuf` is mandatory -- every `check` finding is about a file. A notice like the
   identity divergence isn't about a file at all; forcing it into `Finding` means lying with a
   placeholder path.
2. **Does a shared shape mean one generic envelope, or something narrower?** An early design
   (`Envelope{status, notices}` composed in via `#[serde(flatten)]`) was killed by a compiled repro:
   flattening a struct with its own `status` field into `CheckReport` (which already has one)
   produces silent duplicate `"status"` keys in the output JSON -- `serde_json` emits both, no error,
   and any real consumer resolves that by silently dropping one. Renaming fields around this doesn't
   remove the hazard class; only removing the composition mechanism does.
3. **Can a non-fatal notice ever contradict the exit code?** Not if the type makes it
   unrepresentable -- see Decision below.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Generic `Envelope` struct, `#[serde(flatten)]`-composed into every report | One literal shared type | Confirmed duplicate-key hazard on any report that already owns a same-named field; degrades typed payloads (`GraphEdge`, etc.) toward an untyped `serde_json::Value` if pushed further |
| Bare `"warnings": [String]` (the stopgap already shipped for the identity notice) | Cheapest | No `severity`/`subject` an agent can match on; a second command copying it becomes a third incompatible shape by omission |
| `Report` trait (behavior, not shared fields) + a real `Notice` type, no envelope | Every report stays its own fully-typed struct; `notices()`/`exit_code()` are the only shared surface | Slightly more boilerplate per report type (each declares its own `notices: Vec<Notice>` field directly) |

## Decision

In the context of five incompatible output shapes and one confirmed duplicate-key bug in the
composition approach, facing the need for one real contract without sacrificing type safety, we
decided:

1. **No `Envelope` struct, no `#[serde(flatten)]` anywhere in this design.** `CheckReport` keeps its
   own `status: ReportStatus` field exactly as before. Every report struct (`CheckReport`,
   `GraphReport`, `ExplainReport`, `NewReport`, `FixReport`, `MigrateSchemaReport`, `DoctorReport`,
   `CouldNotRun`) plainly declares its own `notices: Vec<Notice>` field directly -- nothing composed
   in, nothing that can collide.
2. **A `Report` trait carries behavior, not shared fields**: `fn notices(&self) -> &[Notice]` and
   `fn exit_code(&self) -> ExitCode`. One shared `emit<T: Report>(report: &T) -> ExitCode` function
   (in `urzua-cli`, not `urzua-core` -- printing is I/O, and `urzua-core` stays pure per ADR-5)
   replaces the five hand-rolled `println!`/`json!` call sites.
3. **`Notice`'s severity is a deliberately smaller enum than `Finding`'s** -- `NoticeSeverity` has
   only `Info`/`Warning`, no `Error` variant, so a notice that would contradict "notices never move
   the exit code" is unrepresentable, not just undocumented. `subject` is a `const &str` per emitter,
   matching `Finding.rule`'s existing convention exactly (an owned `String` field populated from a
   constant), not a new, subtly different convention.
4. **Every fatal "could not run" path collapses onto one `CouldNotRun` type.** Before this, there were
   three different patterns: `report_could_not_run` (a full `CheckReport` shape with empty fields,
   used by `check`/`audit`), `report_fix_could_not_run` (`fix`'s own smaller shape), and bare
   `eprintln!` with nothing on stdout at all (`new`, `explain`, `graph`, `doctor`'s early guard,
   `migrate schema --report`). `check`/`audit` specifically lose the `scope`/`rules_executed`/
   `files_examined` keys they used to emit even on failure (present-but-empty before, absent now) --
   a real, narrower breaking change than "previously-silent stdout now emits JSON," which is what
   changes for the other commands.
5. **`DoctorReport` keeps its own real exit code, not one derived from `status` alone.** `run_doctor`
   has three real exit codes (2 = config missing, 1 = config parse error or any check `Error`, 0 =
   otherwise) but the first two both produce the same `DoctorStatus::Error` value -- deriving
   `exit_code()` from `status` alone would silently collapse them. `DoctorReport` carries an explicit,
   non-serialized `exit_code: u8` set by whichever branch constructs it, so the JSON shape (`status`,
   `checks`) is untouched but the real 2/1/0 split can't be lost to a future refactor.
6. **`Cli::parse()` becomes `Cli::try_parse()`**, with an explicit carve-out: `ErrorKind::DisplayHelp`/
   `DisplayVersion` print as clap already rendered them and exit 0 (never JSON-wrapped) -- matching
   `cargo`'s own convention that help/version are never gated behind a machine-format flag. Every
   other parse error goes through the same `CouldNotRun`/`emit()` path as any other fatal error.
7. **A `std::panic::set_hook` closes the one remaining stderr leak**: Rust's default panic hook always
   writes raw text to stderr regardless of anything else in this decision. The replacement hook writes
   a minimal ad hoc JSON object (`{"status": "not-run", "panic": ...}`) via `write_all` with the
   `Result` ignored -- deliberately not `println!`/any panicking macro, since a broken pipe during the
   hook itself would double-panic and abort with no output at all. Deliberately **not** routed through
   `Notice`/`Report` -- a panic is a lower-level escape hatch outside the five-command contract, not
   another instance of it.
8. **Stderr is eliminated for anything this code can structure into JSON, including genuine errors** --
   narrower than but building on ADR-26 (see that ADR's own dated Amendment for the specific reasoning
   and the sentence it supersedes).

## Reversibility

Every report type's JSON shape changes (new `notices` field; `check`/`audit`'s failure path loses
three keys; `"warnings": [String]` becomes `"notices": [Notice]`). Pre-1.0, no known external
consumer -- cheap now, expensive once one exists, the same reversibility profile ADR-23 already
named for this exact contract.

## Consequences

- Every future command's report type must implement `Report` and go through `emit()` -- a new
  command inventing its own `println!`/`json!` block is now a real regression, not just untidy.
- `Notice.severity` can never be `Error` by construction -- a future maintainer who wants to add a
  fatal-severity notice will hit a compile error reaching for a variant that doesn't exist, which is
  the point.
- `urzua-core` gained zero new dependencies -- `emit()`'s `serde_json`/`println!` live in
  `urzua-cli`, keeping `urzua-core`'s "no I/O, no filesystem, no process" claim (ADR-5) intact and
  mechanically checked (`urzua-core/tests/purity.rs`).
- `run_migrate_ids` and `run_init` remain outside this contract -- both are plain-text prose end to
  end today (not just their error paths), a materially different redesign than a drop-in `Report`
  struct. Filed as its own bug, not fixed here.

## References

- BUG-19 -- the finding this ADR resolves.
- ADR-7 -- "every future command must emit this same shape, not invent its own," the rule this
  finally realizes.
- ADR-23 -- stdout is always JSON, unconditionally; this ADR's contract is the concrete shape that
  claim now has for every command, not just `check`.
- ADR-26 -- narrowed further by this ADR's point 8; see its own dated Amendment.
- ADR-5/ADR-9 -- `urzua-core`'s purity boundary, which `emit()` living in `urzua-cli` (not
  `urzua-core`) respects.
- `rust/crates/urzua-core/src/report.rs` -- `Report`, `Notice`, `NoticeSeverity`, `CouldNotRun`, and
  every concrete report type.
- `rust/crates/urzua-cli/src/main.rs` -- `emit()`, the panic hook, `try_parse()`.
