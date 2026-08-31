# ADR-0407: Phase 4KR Bundle Branch Preflight Detail

Status: Accepted
Date: 2026-08-31

## Context

ADR-0406 made same-Store Bundle Branch divergence an explicit non-apply
outcome through `same_store_divergence_detected`. That gives scripts and import
journals a stable action string, and it proves that a divergent target Branch is
not overwritten.

The remaining gap is observability. Current preflight output exposes only
aggregate counts such as `branch_heads_diverged`. A caller can know that a
Bundle contains a divergent Branch head, but cannot identify the exact Branch,
source head, target head, or shared merge base from preflight/import/apply
output alone. That weakens the handoff into later Branch creation or merge-shape
slices.

## Decision

Phase 4KR extends Bundle preflight results with deterministic per-Branch head
detail. Each exported Branch head is reported with:

- Workspace id, Branch id, and Branch name;
- source head Commit and state digest from the Bundle;
- local target head Commit and state digest when the Branch exists locally;
- a status of `already_present`, `missing`, `fast_forward`, or `diverged`;
- a common `merge_base_commit_id` for `fast_forward` and `diverged` cases when
  one exists.

The CLI surfaces the same detail in `bundle preflight-dir`, `bundle import-dir`,
and `bundle apply-dir` output, and adds focused expectation flags for scripts to
assert detail count and the first detail's status/source/target/base fields.

This is an observability and acceptance-gate slice. It does not create missing
Branches, start or continue merge attempts from Bundle data, import divergent
commit closures, implement external Store import, change Bundle format or
schema, or permit last-write-wins Branch updates.

## Consequences

Same-Store Bundle divergence becomes actionable evidence rather than only a
count. Operators and smoke tests can identify the exact Branch head pair that
blocked apply and can carry the source head, target head, and common base into a
future merge-shape workflow.

The existing safety boundary remains unchanged: fast-forward Bundles can apply,
divergent Bundles remain non-applied, and the target Branch head is preserved.
