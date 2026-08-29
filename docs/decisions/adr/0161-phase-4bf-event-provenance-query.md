# ADR-0161: Phase 4BF Event Provenance Query

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Events are immutable semantic provenance for ChangeSets, Sessions, and runtime
operations. Phase 4BE added Bundle preservation for ChangeSet-linked Event
provenance, but the public Engine and CLI did not yet expose a minimal read
path for inspecting those rows.

## Decision

1. The Engine exposes read-only Event provenance queries through `event` and
   `events`.
2. Event list queries are scoped by exactly one target: ChangeSet, Session, or
   Workspace.
3. Event snapshots expose optional Workspace, ChangeSet, and Session links,
   event kind, occurrence timestamp, canonical payload JSON, raw payload digest,
   and payload size.
4. Query reads validate stored Event payloads as fixed-point WorkVCS canonical
   JSON before returning them.
5. The CLI adds a thin `event show` and `event list` surface using existing
   key-value output conventions.

## Non-Goals

- This slice does not introduce Event mutation APIs, event replay, cross-Store
  import, remote transport, or projection-specific event indexes.
- Event queries do not redefine Event ordering authority. They sort only for
  deterministic inspection output.

## Consequences

- Imported Bundle Event provenance can be inspected through the same public
  Engine/CLI boundary as other V0.1 read-side capabilities.
- Corrupt or non-canonical Event payload JSON fails as a query integrity error
  instead of being rendered as authoritative provenance.
