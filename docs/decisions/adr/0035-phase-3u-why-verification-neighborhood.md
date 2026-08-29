# ADR-0035: Phase 3U Why Verification Neighborhood

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed verification relation, query semantics, WorkState, and
  replay requirements.

## Context

Phase 3T introduced a structural `why` foundation for current `contains` and
`references` neighborhoods, while marking Verification paths deferred. Phase
3D already writes immutable Verification entities and exactly one current
defining `verifies` Relation from a Verification to an Acceptance Criterion or
Verification Requirement. That relation participates in replayed WorkState,
but it has not been visible through the `why` query.

The confirmed relation vocabulary defines `verifies` as a Verification
Relation and states that a Verification therefore verifies one Verification
Requirement when present, otherwise its Acceptance Criterion. Evidence capture
and `evidenced_by` remain deferred.

## Decision

1. Phase 3U extends the existing Engine-owned `why(WhyQueryOptions)` query
   with current `verifies` relation neighborhoods.
2. The `verifies` read path stays inside the existing Verification/Task
   semantic module and reuses the existing relation validation rules. The
   `why` module consumes a crate-internal semantic snapshot instead of reading
   raw SQL.
3. A Verification edge is reported in canonical direction:

   ```text
   verification -> acceptance criterion / verification requirement
   ```

4. The result reports whether that edge is incoming or outgoing relative to
   the queried subject.
5. `WhyEntityKind` now distinguishes Verification, Acceptance Criterion, and
   Verification Requirement endpoints in addition to Goal, Plan, and Task.
6. `WhyRelationKind` includes the `verifies` relation kind.
7. Commit and Branch-head selector behavior remains unchanged from Phase 3T.
8. The queried subject Entity must still be current in the selected WorkState.
   If it is absent from that WorkState, the query is rejected as a structured
   query error.
9. `why` still uses replayed WorkState as authority and ignores projection
   rows, Event rows, and runtime state.
10. The query remains read-only. It creates no Event, ChangeSet,
    ChangeOperation, WorkStateCommit, Branch HEAD movement, projection row,
    runtime row, Entity version, or Relation version.
11. Because Evidence capture and `evidenced_by` Relations remain deferred,
    the result narrows the previous coarse Verification deferred marker to
    `VerificationEvidence`.
12. Evolution and epistemic relation families remain deferred.
13. Phase 3U does not implement `evidenced_by`, Evidence objects, Resource
    Basis, Artifact/Input Basis, ResourceObservation drift comparison, full
    causal traversal, scheduling readiness explanations, context ranking,
    budgeted context resolution, `next`, restore, merge, projection
    materialization, migrations, or business CLI commands.
14. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3U does not add a CLI `why` command.

## Consequences

- WorkVCS can now explain the current Verification judgment edge for a
  Verification, Acceptance Criterion, or Verification Requirement without
  mutating WorkState.
- Later Evidence and Resource slices can extend the same `why` result shape
  with `evidenced_by` and drift-aware explanation paths.
- The deferred-family list now communicates the more precise remaining gap:
  Verification Evidence, plus evolution and epistemic relation families.

## Implementation Findings

- The existing `verifies` loader was private to Verification projection.
  Phase 3U adds a crate-internal `verification_relations_at` helper so `why`
  can reuse the same invariant checks without exposing raw relation storage
  through the public Engine API.
- Independent review found that the first `verification_relations_at`
  implementation validated the stored relation row but did not prove both
  relation endpoints were current in the selected WorkState. Phase 3U now
  checks current source and target Entity memberships before exposing a
  `verifies` edge.
- `evidenced_by` cannot be represented until Evidence capture exists. Phase
  3U therefore does not treat Verification explanation as complete.
