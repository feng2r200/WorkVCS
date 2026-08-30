# ADR-0206: Phase 4CY Session List CLI

Status: Accepted

Date: 2026-08-30

## Context

Phase 4CX added `session show` by exposing the existing `SessionSnapshot` read
path through the CLI. Session runtime now has a direct inspect command, but the
tool still cannot enumerate sessions without lower-level storage access.

Session state is runtime state, not WorkState membership. Listing sessions must
therefore remain a runtime Store-boundary query.

## Decision

1. Add `SessionListOptions` and `SessionListResult` in the runtime session
   module.
2. Add `Engine::sessions` as a read-only runtime API.
3. Enumerate session ids through `object_identity` + `session`, requiring object
   kind `session`.
4. Reuse `SessionSnapshot` loading for each row so active/ended state validation
   remains centralized.
5. Sort sessions by `started_at_us` and `session_id`.
6. Support optional lifecycle filtering for `active` and `ended`.
7. Add `workvcs session list STORE [--lifecycle active|ended]`.
8. Render compact `key=value` rows with session id, lifecycle, timestamps,
   metadata, active workspace/branch, focus, and session diff id.

## Non-Goals

- This slice does not add workspace-scoped or branch-scoped session filters.
- This slice does not alter session start/switch/end semantics.
- This slice does not add focus mutation commands.
- This slice does not change claim or runnable projections.

## Consequences

- CLI users can discover active and ended sessions from the Engine facade.
- Session runtime tooling now has start/show/list/switch/end coverage.
- More specialized filters remain later incremental tool slices.
