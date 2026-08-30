# ADR-0195: Phase 4CN Verification Show And List CLI

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Phase 3 verification slices made Verification judgments immutable semantic
entities, and later CLI slices exposed Acceptance Criterion and Verification
Requirement creation/readback. The CLI can record a Verification but did not yet
provide direct show/list readback for the Verification entity itself.

## Decision

1. Add read-only `Engine::verifications_at` and Store facade methods.
2. Implement list readback by replaying WorkState at the target commit, loading
   current Verification entity versions, and validating each defining `verifies`
   relation plus Evidence closure.
3. Sort Verification snapshots by target kind, target entity id, and
   Verification entity id.
4. Add `verification show` and `verification list` CLI commands with the same
   `--branch` or `--commit` target selector used by adjacent query commands.
5. Render Verification outputs with target, result, method, semantic dependency,
   Evidence, Resource Basis, and defining relation fields.

## Non-Goals

- This slice does not change Verification creation semantics.
- This slice does not infer or recompute applicability beyond existing stored
  and effective-status paths.
- This slice does not add filtering, pagination, or policy evaluation.
- This slice does not materialize a new projection table.

## Consequences

- CLI users can audit immutable Verification judgments after recording them.
- Historical Verification lists reflect the WorkState-visible entity versions at
  the target commit.

## Implementation Findings

- None.
