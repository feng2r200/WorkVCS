# ADR-0082: Phase 3BP Knowledge Statement Kernel

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed V1 Knowledge boundary.

## Context

The confirmed V1 boundary includes Workspace-local Knowledge as versioned Work
State. Knowledge preserves reusable statements learned through work, separately
from ordinary Records and Decisions. The current implementation already has
Goal, Plan, Task, Verification, Resource, Record, context, next, why, and
branch-aware query foundations, but it still lacks a semantic Knowledge API.

## Decision

1. Phase 3BP introduces only Workspace-local Knowledge statement creation and
   read projection.
2. Knowledge is stored as an Entity with `entity_kind = "knowledge"`.
3. Knowledge state schema version 1 is a canonical JSON object with exactly:
   `provenance`, `scope`, `statement`, and `status`.
4. `scope` and `provenance` must be canonical JSON objects. They default to
   empty objects in this slice.
5. Creation always creates `status = "active"`.
6. The parser recognizes `active`, `invalidated`, and `superseded` so future
   lifecycle slices can read historical states without redefining the schema.
7. `knowledge_at(commit_id, knowledge_entity_id)` reads one Knowledge snapshot
   from replayed Work State.
8. `knowledges_at(options)` lists Knowledge snapshots from replayed Work State
   and supports exact, case-sensitive `statement_contains` filtering plus
   status filtering.
9. The public generic Entity transition boundary rejects `entity_kind =
   "knowledge"`; callers must use the semantic Knowledge API.
10. The CLI exposes thin `knowledge create`, `knowledge show`, and
    `knowledge list` commands.
11. This slice does not implement Knowledge lifecycle transitions,
    KnowledgeExposure, KnowledgeSpace operations, cross-Workspace adoption,
    path-sensitive context ranking, Knowledge relation semantics, merge review,
    or federation.

## Consequences

- Agents can now persist reusable learned statements as first-class Work State.
- Knowledge participates in branch, history, diff, and restore through the
  existing Entity transition and replay paths.
- Later slices can add lifecycle transitions and relation semantics without
  changing the v1 Knowledge state shape.

## Implementation Findings

- The confirmed domain requires Knowledge provenance, but the exact
  provenance vocabulary is not yet narrower than canonical JSON. Phase 3BP
  stores it as a constrained object and leaves typed provenance relations for
  later Knowledge relation slices.
