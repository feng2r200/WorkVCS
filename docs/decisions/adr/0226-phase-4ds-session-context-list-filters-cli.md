# ADR-0226: Phase 4DS Session Context List Filters CLI

Status: Accepted

Date: 2026-08-30

## Context

Session snapshots already expose active Workspace, active Branch, lifecycle,
focus, and context Workspaces. `workvcs session list` could filter by lifecycle
only. A CLI operator resuming work often needs to find Sessions active in one
Workspace or Branch before inspecting focus, claims, Evidence, or Resource
Observation provenance.

Session remains runtime state plus immutable SessionDiff provenance. These
filters do not make Session part of WorkState replay.

## Decision

1. Extend `SessionListOptions` with optional active Workspace and active Branch
   filters.
2. Preserve the existing lifecycle filter and allow all filters to combine.
3. Add `--workspace WORKSPACE` and `--branch BRANCH` to
   `workvcs session list`.
4. Parse both values as typed IDs.
5. Keep list rendering unchanged, because entries already expose the filtered
   active context fields.

## Non-Goals

- This slice does not change Session lifecycle transitions.
- This slice does not infer or select a current Session.
- This slice does not change claim ownership behavior.
- This slice does not make Session replay-authoritative WorkState.

## Consequences

- CLI users can recover active execution context by Workspace or Branch.
- Runtime provenance queries can be narrowed before inspecting Session detail,
  claims, Evidence, or Resource Observations.
- The change remains a read-only Engine facade extension over existing Session
  snapshots.
