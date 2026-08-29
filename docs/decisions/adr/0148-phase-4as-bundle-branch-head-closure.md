# ADR-0148: Phase 4AS Bundle Branch Head Closure

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0004 and INV-061 require same-Store import to distinguish fast-forward from
divergence. Existing Bundle export already contains commit closure, WorkState
mapping, object-version metadata, payload bytes, membership changes, and
KnowledgeExposure provenance closure, but it does not yet identify which Branch
heads the exported Store considered current for the included commits.

Without an exported ref closure, a later import implementation cannot evaluate
Branch ancestry or divergence from Bundle data alone.

## Decision

1. Phase 4AS extends `BundleExportManifest` with `exported_branch_heads`.
2. The exported Branch head closure includes active Branch rows from the target
   Workspace whose `head_commit_id` is inside the exported commit closure.
3. Each Branch head reference records Workspace id, Branch id, Branch name,
   head Commit id, head state digest, lifecycle state, and Branch creation time.
4. The closure is read from current Branch refs at export time and sorted
   deterministically by Branch name and id.
5. CLI `bundle export` reports the `exported_branch_heads` count.
6. This slice does not create Branches, move refs, ingest Bundle rows, implement
   fast-forward application, resolve divergence, or perform cross-Store import.

## Consequences

- Later same-Store import slices can compare local Branch refs against exported
  Branch heads without changing the Bundle manifest profile.
- Exporting a historical commit does not automatically include Branches whose
  current heads have moved outside that commit closure.

## Implementation Findings

- None.
