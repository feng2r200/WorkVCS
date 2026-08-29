# ADR-0097: Phase 3CE Record Scope Filter

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed scoped Record model.

## Context

Record state already carries a canonical object `scope`, and Record creation
commands can set it. The list query can filter by kind, status, and statement
text, but cannot yet narrow results to a specific semantic scope. This leaves
Agent-facing workflows to scan unrelated records when inspecting a concrete
workspace, module, or task context.

## Decision

1. Phase 3CE adds exact scope filtering to `RecordListOptions`.
2. Scope filters must be canonical objects, matching the Record state
   validation boundary.
3. Scope equality is evaluated through WorkVCS canonical bytes, so object key
   insertion order does not affect query results.
4. The CLI exposes the filter as `record list --scope-json`.
5. Scope filtering composes with existing kind, status, and statement filters.
6. This slice does not change Record state, schema, Replay, context projection,
   or relation semantics.

## Consequences

- Tools can retrieve Records for a concrete scope without scanning unrelated
  active or lifecycle-specific records.
- Record and Knowledge query surfaces now share the same exact-scope filtering
  behavior.

## Implementation Findings

- Record scope has the same order-preserving internal `CanonicalValue::Object`
  representation as Knowledge scope, so the filter compares canonical bytes.
