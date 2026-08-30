# ADR-0191: Phase 4CJ Goal And Plan List CLI

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Goal and Plan now have CLI create, transition, and show commands. Users still
need a compact read-only inventory of semantic Goal/Plan entities at a branch
head or historical commit to drive CLI-only workflows.

## Decision

1. Add minimal read-only `Engine::goals_at` and `Engine::plans_at` APIs.
2. Implement both APIs by replaying WorkState at the target commit and loading
   the current Goal/Plan entity versions already referenced by that WorkState.
3. Sort returned snapshots by entity id for deterministic output.
4. Add `goal list` and `plan list` CLI commands with the same `--branch` or
   `--commit` target selector used by `goal show` and `plan show`.
5. Render list outputs as stable key/value lines using `goal.N.*` and
   `plan.N.*` prefixes. Free-text fields and arrays are rendered as canonical
   JSON values.

## Non-Goals

- This slice does not add status or text filtering.
- This slice does not alter Goal or Plan lifecycle semantics.
- This slice does not change Task, Verification, Merge, Federation, Runtime, or
  Store mutation behavior.
- This slice does not materialize a new projection table.

## Consequences

- CLI users can enumerate Goal and Plan semantic state at current or historical
  targets without falling back to `show-at`.
- The read-only API surface remains behind the Engine facade and does not leak
  SQLite handles or Store internals.

## Implementation Findings

- None.
