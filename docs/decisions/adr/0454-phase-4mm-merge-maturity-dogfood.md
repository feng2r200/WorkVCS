# ADR-0454: Phase 4MM Merge Maturity Dogfood

Status: Accepted
Date: 2026-09-01

## Context

The V1 readiness ledger already marked the merge lifecycle as designed,
implemented, smoke-proven, and locally dogfood-proven after Phase 4LN. That
run covered divergent Work Branch classification, explicit resolution,
unresolved freeze guards, moved-head continue guards, abort/restart recovery,
and completed two-parent merge commits.

The remaining maturity gap was scale and variety: Phase 4LN used a very small
Store with one conflict item and one source-only auto item. Before using merge
as release-maturity evidence, the implementation needed a larger local
dogfood run with repeated conflict items, source-only Tasks, and source-side
scheduling Relation items.

## Decision

Treat Phase 4MM as a dogfood-only merge maturity slice. The slice does not
change code. It records a larger local CLI run that created a divergent
WorkState and completed a merge with:

```text
merge_items=24
conflict_items=12
auto_items=12
relation_items=4
theirs_resolutions=24
commit_kind=merge
parents=2
tasks_match_expected=true
relations_match_expected=true
valid_required=true
```

The evidence also records a script-assumption finding: `history` is a
first-parent query for the selected Branch. After merge, the target first-parent
history contains the target line plus the merge commit; source-side ordinary
commits remain reachable through the secondary parent and final WorkState, but
they are not target first-parent commits. Future dogfood scripts must not use
target first-parent history counts as proof that all source-side ordinary
commits are listed inline after merge.

## Non-Goals

- No product code change.
- No merge algorithm change.
- No schema change.
- No semantic or LLM merge.
- No custom semantic conflict resolver.
- No GUI/TUI flow.
- No release-scale performance claim.
- No remote, distributed, or multi-operator merge claim.

## Evidence

- Larger/varied merge dogfood:
  `/tmp/workvcs-4mm-merge-maturity-dogfood-20260901T095120Z`.
- Summary:
  `/tmp/workvcs-4mm-merge-maturity-dogfood-20260901T095120Z/summary.txt`.
- Delivery validation and independent review closeout are tracked in
  `docs/provenance/phase-4mm-merge-maturity-dogfood.md`.

## Consequences

The merge lifecycle now has bounded larger-Store dogfood evidence beyond the
small Phase 4LN workflow. This improves release-maturity confidence for local
merge mechanics while preserving the open boundary around broader real-project
write-mode merge, semantic merge, and release-scale performance.
