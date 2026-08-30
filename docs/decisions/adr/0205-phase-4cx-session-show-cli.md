# ADR-0205: Phase 4CX Session Show CLI

Status: Accepted

Date: 2026-08-30

## Context

Session runtime APIs already support starting, switching, ending, and reading a
`SessionSnapshot` through the Engine facade. The CLI exposed start, switch, and
end, but users could not inspect a known session without relying on adjacent
context or claim commands.

## Decision

1. Add `workvcs session show STORE --session SESSION`.
2. Use the existing `Engine::session_snapshot` read API.
3. Render the snapshot in the existing `key=value` style with lifecycle,
   started/last-activity timestamps, metadata, active workspace/branch,
   context-workspace count, focus, focus path entries, and session diff id.
4. Keep Session state in the runtime boundary; this CLI command is read-only.

## Non-Goals

- This slice does not add Session listing.
- This slice does not alter Session start/switch/end semantics.
- This slice does not add new focus mutation commands.
- This slice does not change claim or runnable projections.

## Consequences

- CLI users can verify runtime session state directly after start and switch.
- Session-related workflows can inspect focus and active branch without using
  lower-level storage reads.
- Session listing remains a later tool slice.
