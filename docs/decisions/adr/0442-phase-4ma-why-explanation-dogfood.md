# ADR-0442: Phase 4MA Why Explanation Dogfood

Status: Accepted
Date: 2026-09-01

## Context

The V1 readiness ledger called out a risk that WorkVCS could keep improving
smoke expectations and output details without proving that existing workflows
help real continuation work. The `why` surface already had several implemented
neighborhoods: structural containment, `verifies`, `evidenced_by`, Record
relations, Knowledge relations, and Knowledge exposure links. Evolution and
epistemic families remain outside the current implementation.

Before adding more `why` output fields or relation families, the next useful
step was dogfood: model a real external-project review and ask whether existing
`why` queries can explain the work from several angles.

## Decision

Accept Phase 4MA as dogfood-only evidence for current `why` capability. The
dogfood uses a local temporary Store and a read-only target file:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/SYSTEM.md
```

It creates a Goal, Plan, Task, Acceptance Criterion, Evidence, Verification,
Assumption Record, Finding Record, Decision Record, and Knowledge statement.
Then it proves five existing `why` paths:

```text
Task -> incoming primary_containment from Plan
Acceptance Criterion -> incoming verifies from Verification
Evidence -> incoming evidenced_by from Verification
Decision Record -> incoming record_supports from Finding
Knowledge -> incoming record_supports from Finding
```

No code change is made in this slice. If dogfood had exposed an implementation
gap, the gap would have been recorded before selecting a code change.

## Non-Goals

- No new relation family.
- No new CLI output field.
- No evolution or epistemic `why` implementation.
- No broad causal traversal.
- No target-project mutation.
- No claim that `why` is complete for release maturity.

## Evidence

The passing dogfood run:

```text
phase4ma_dogfood_result=PASS
log_dir=/tmp/workvcs-4ma-why-dogfood-20260901T035304Z
store=/tmp/workvcs-4ma-why-dogfood-20260901T035304Z/store.sqlite
target_project=/Users/example/Projects/HeXun/Hernes/agent_soul
target_file=/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/SYSTEM.md
workspace_id=01a05b19-646c-7a12-88b8-dd91ab99cdd0
branch_id=01a05b19-646c-7a12-88b8-ddc42ea99a5c
head_commit_id=01a05b19-65b7-7260-b37c-39a782c79212
goal_id=01a05b19-6485-7700-92d8-57b02b3e6348
plan_id=01a05b19-649c-73e2-809a-1a843508b150
task_id=01a05b19-64b1-7523-ba95-159c4a062b99
criterion_id=01a05b19-64f6-7771-a35b-1a57e56a3112
verification_id=01a05b19-6523-7dc0-9bb0-d130eb78982a
evidence_id=01a05b19-650e-7880-a5a0-27d020e64bfd
assumption_id=01a05b19-653b-7583-9ced-52036a939ccf
finding_id=01a05b19-6553-7093-bfdc-c699e4426836
decision_id=01a05b19-656a-7112-911e-43cc510b6d78
knowledge_id=01a05b19-659b-7b90-bd08-8dd9f65cb7f7
why_queries=5
recovered_modeling_errors=2
```

Two failed modeling attempts before the passing run were also useful:

```text
record invalid: supports source must be a Finding Record, found decision
record invalid: supports target must be a Decision Record, found assumption
```

The Phase 4LZ structured error output made those modeling errors directly
actionable as `record_invalid`.

## Consequences

The current `why` implementation is usable for a realistic continuation
explanation across structure, verification, evidence, Record, and Knowledge
neighborhoods. That reduces pressure to add more display fields before another
real workflow proves a missing explanation.

The remaining `why` maturity gap is still real but narrower: evolution,
epistemic relations, broader causal traversal, and write-mode external-project
dogfood remain outside this evidence.
