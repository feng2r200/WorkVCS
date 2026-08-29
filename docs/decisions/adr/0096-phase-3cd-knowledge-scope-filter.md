# ADR-0096: Phase 3CD Knowledge Scope Filter

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed scoped Knowledge model.

## Context

Knowledge state already carries a canonical object `scope`. Phase 3BP exposed
Knowledge creation, show, and list, but list queries could only filter by
status and statement text. The Agent-facing workflow needs a way to narrow
Knowledge to the exact semantic scope being inspected without introducing a new
projection or storage table.

## Decision

1. Phase 3CD adds exact scope filtering to `KnowledgeListOptions`.
2. Scope filters must be canonical objects, using the same validation boundary
   as Knowledge creation.
3. Scope equality is evaluated through WorkVCS canonical bytes, so object key
   insertion order does not affect query results.
4. The CLI exposes the filter as `knowledge list --scope-json`.
5. Scope filtering composes with the existing status and statement filters.
6. This slice does not change Knowledge state, schema, Replay, context
   projection, or relation semantics.

## Consequences

- Tools can retrieve Knowledge for a concrete scope without scanning unrelated
  active Knowledge.
- Canonical semantic equality remains the authority for user-supplied scope
  filters.

## Implementation Findings

- `CanonicalValue::Object` preserves construction order internally, while
  canonical encoding sorts keys. The filter therefore compares canonical bytes
  rather than relying on direct structural equality.
