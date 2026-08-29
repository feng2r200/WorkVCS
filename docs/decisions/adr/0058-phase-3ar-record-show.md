# ADR-0058: Phase 3AR Record Show Command

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  existing semantic Record API.

## Context

Phase 3AQ added `record list`, which lets a local Agent discover semantic
Record IDs at a specific commit. The CLI still needs a thin way to inspect one
listed Record without manually decoding WorkState or writing a purpose-specific
query outside the Engine facade.

## Decision

1. Phase 3AR adds `record show STORE --commit <id> --record <id>`.
2. The command uses the existing `Engine::record_at` API and does not add a new
   Store or history primitive.
3. Output remains key/value oriented and includes workspace ID, commit ID,
   Record entity ID, Record version ID, state digest, kind, status, statement,
   and scope.
4. `record_statement_json` is rendered as a JSON string so embedded newlines and
   quotes remain one-line tool output.
5. `record_scope_json` is rendered with the WorkVCS canonical JSON encoder.
6. This slice does not implement text search, pagination, context ranking,
   Record editing, Attempt records, Handoff records, or promoted Decision
   entities.

## Consequences

- `record list` and `record show` now form a minimal inspectable Record tool
  loop for local Agents.
- The CLI remains thin and continues to avoid direct SQL access.

## Implementation Findings

- No core semantic model or schema changes were required.
