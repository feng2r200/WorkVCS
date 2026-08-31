# ADR-0397: Phase 4KH CLI Smoke Workflow

Status: Accepted
Date: 2026-08-31

## Context

The WorkVCS CLI now exposes a broad enough local V0.1 surface to exercise the
current Engine-backed workflow from Store bootstrap through Workspace, Task,
Acceptance Criterion, Evidence, Resource Observation, Verification,
Applicability Cache, History, Show-at, Session, Runnable projection, and Next
selection. The same behavior is covered by focused Rust tests, but there is no
repository-level command that proves the current CLI workflow at the real
process boundary.

The long-running implementation goal needs a repeatable smoke check that can be
run by a local operator or future Agent before claiming a slice preserves the
usable CLI loop.

## Decision

Phase 4KH adds `scripts/smoke-v0.1-cli-workflow.sh`.

The script:

- runs `workvcs-cli` through `cargo run -q -p workvcs-cli --` against a
  temporary Store;
- parses line-oriented `key=value` CLI output and reuses generated identifiers
  from earlier steps;
- exercises `init`, `store info`, `workspace create`, `task create`,
  `ac create/status`, `evidence create/show`, `resource create/observe`,
  `resource observation-show/list`, `verification record/list/cache-record`,
  `verification cache-show`, `history`, `show-at`, `session start`,
  `runnable tasks`, `claim guard`, and `next`;
- uses existing expectation flags where they are implemented, including
  recently added Verification cache-record expectations;
- avoids fixed UUID and timestamp assumptions.

The script is a smoke workflow, not a replacement for focused Rust tests. It
does not introduce new CLI semantics, does not add Engine or Store behavior,
and does not change the schema.

## Consequences

Local automation has a single executable entrypoint for proving the currently
implemented CLI loop works at the process boundary. Future CLI expectation
slices can extend this smoke workflow when they add script-verifiable behavior
that belongs in the end-to-end local loop.

Because the workflow is intentionally process-level, failures should first be
classified as implementation findings in the command surface or script parsing
contract. A failure is not evidence that a new domain concept or architecture
boundary should be introduced.
