# ADR-0230: Phase 4DW Goal And Plan Status List Filter CLI

Status: Accepted

Date: 2026-08-30

## Context

Goal and Plan list commands can enumerate semantic state at a Branch head or
historical Commit, but they cannot narrow the result by lifecycle status. Status
filtering is the common read-side query needed to find active or terminal
planning objects before using `next`, context, or provenance commands.

The first Goal/Plan list slice intentionally kept filtering out of scope. This
slice adds only read-side filtering and does not change lifecycle mutation
semantics.

## Decision

1. Add `--status STATUS` to `workvcs goal list`.
2. Add `--status STATUS` to `workvcs plan list`.
3. Resolve the existing Branch/Commit target exactly as before.
4. Load snapshots through the existing Engine facade and filter by lifecycle
   status before rendering.
5. Allow all confirmed read-side statuses, including `superseded` for Plan.

## Non-Goals

- This slice does not add description, containment, priority, or owner filters.
- This slice does not change Goal or Plan lifecycle transitions.
- This slice does not make Plan supersession writable through the generic
  complete/abandon/reopen commands.
- This slice does not change runnable or `next` selection.

## Consequences

- CLI users can query active and terminal Goal/Plan state directly.
- Planning workflows can inspect a smaller semantic state surface before
  deciding the next operation.
- The commands remain thin read-only wrappers over existing Engine snapshots.
