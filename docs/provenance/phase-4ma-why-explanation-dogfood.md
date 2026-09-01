# Phase 4MA Why Explanation Dogfood Evidence

Date: 2026-09-01

## Scope

Phase 4MA is a dogfood-only slice. It validates existing `why` explanation
paths against a real external-project review scenario and intentionally avoids
changing code or adding display fields.

The read-only external file used as work scope was:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/SYSTEM.md
```

## Dogfood Evidence

The passing run:

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

The five `why` probes proved:

```text
why_task: incoming primary_containment from plan_id
why_ac: incoming verifies from verification_id
why_evidence: incoming evidenced_by from verification_id
why_decision: incoming record_supports from finding_id
why_knowledge: incoming record_supports from finding_id
```

## Modeling Findings

The first two attempts failed before the passing run:

```text
log_dir=/tmp/workvcs-4ma-why-dogfood-20260901T035044Z
record invalid: supports source must be a Finding Record, found decision

log_dir=/tmp/workvcs-4ma-why-dogfood-20260901T035208Z
record invalid: supports target must be a Decision Record, found assumption
```

These were not product regressions. They confirmed existing Record relation
constraints:

```text
record-to-record supports source must be Finding
record-to-record supports target must be Decision
```

The new Phase 4LZ error output made both recovery steps actionable through
stable error metadata.

## Validation

This slice made no code changes. Docs-only validation passed:

```text
git diff --check
cargo fmt --all -- --check
```

Independent review reported no blocker/high/medium findings and confirmed the
documents do not claim complete `why` coverage or release maturity.

## Boundary

This evidence supports the current implemented `why` relation families. It does
not claim release maturity for evolution, epistemic explanations, full causal
traversal, or write-mode external-project usage.
