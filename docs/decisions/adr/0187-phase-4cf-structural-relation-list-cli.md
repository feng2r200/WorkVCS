# ADR-0187: Phase 4CF Structural Relation List CLI

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

ADR-0186 exposed CLI creation commands for Task scheduling and primary
containment relations. A usable CLI workflow also needs a read-only way to
inspect those structural relations at a Branch head or historical Commit.

## Decision

1. Add `task scheduling-list` with `--branch` or `--commit` as the target
   selector.
2. Add `task containment-list` with `--branch` or `--commit` as the target
   selector.
3. Both commands use existing Engine read APIs and render deterministic
   key/value rows.
4. The CLI target selector resolves Branch heads through the Engine facade.

## Non-Goals

- This slice does not add relation remove/restore commands.
- This slice does not add filtering, pagination, or graph traversal.
- This slice does not change structural relation ordering or projection
  semantics.

## Consequences

- CLI users can create and inspect Task dependency, manual-order, and
  containment structure without leaving the Engine boundary.
- Historical relation inspection remains explicit through `--commit`.

## Implementation Findings

- The existing Engine list APIs already provide deterministic ordering, so the
  CLI only renders the returned order.
