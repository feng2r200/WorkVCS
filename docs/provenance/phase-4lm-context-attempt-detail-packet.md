# Phase 4LM Context Attempt Detail Packet Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LM enriches existing `attempt` and `failed_attempt` ContextPacket items
with deterministic Attempt execution detail. The intent is to make continuation
and recovery packets show current Attempt status and nearby relation context
without requiring separate `record show` or relation list calls.

This evidence does not claim full Context Resolver completion, transition
rationale projection, Record-to-Knowledge relation detail for Attempts,
path-sensitive Knowledge ranking, context packet persistence, another-project
dogfood, or release readiness.

## Dogfood Setup

Commands were run from:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lm-context-attempt-detail-packet
```

The local dogfood Store and log were:

```text
.work-governance/runtime/dogfood/phase-4lm-20260831212914.sqlite
.work-governance/runtime/logs/phase-4lm-dogfood-20260831212914.log
.work-governance/runtime/dogfood/phase-4lm-20260831214811.sqlite
.work-governance/runtime/logs/phase-4lm-dogfood-20260831214811.log
```

Captured IDs:

```text
session_id=01a059cb-78df-7f33-b068-7612bee2fa2a
running_attempt_id=01a059cb-65c4-7103-a77a-91c7bbe056ce
succeeded_attempt_id=01a059cb-688f-75a0-948c-dcf27e0aee9c
succeeded_attempt_version=01a059cb-6b4e-71a0-9f1e-824b3336474b
failed_attempt_id=01a059cb-6e14-7791-ae30-0a99cc82575a
failed_attempt_version=01a059cb-70cc-77e0-a3b6-0caae980e91f
failed_attempt_digest=d848229bbb5ab842c48797161b4140a3d75b67668437f70f567d186b506942c8
inconclusive_attempt_id=01a059cb-7381-71b0-9e6b-a41e2ba9c1d5
inconclusive_attempt_version=01a059cb-763f-7413-8538-caffbc194993
```

## Dogfood Run

The operator created four Attempt records through the CLI:

```text
running
succeeded
failed
inconclusive
```

The operator then started a real Session and requested brief and normal
context packets:

```text
workvcs context STORE --session SESSION --profile brief
workvcs context STORE --session SESSION --profile normal
```

The brief packet exposed the failed Attempt as critical previous failure
context:

```text
context_item.2.priority=P5
context_item.2.category=failed_attempt
context_item.2.subject=record:01a059cb-6e14-7791-ae30-0a99cc82575a
context_item.2.summary_json="attempt detail status=failed terminal=true record=01a059cb-6e14-7791-ae30-0a99cc82575a version=01a059cb-70cc-77e0-a3b6-0caae980e91f state_digest=d848229bbb5ab842c48797161b4140a3d75b67668437f70f567d186b506942c8 scope_json={} record_relations_out=0 record_relations_in=0 record_relation_types=none statement=Dogfood failed attempt for 4LM"
```

The normal packet exposed running, succeeded, failed, and inconclusive Attempt
detail:

```text
context_item.2.summary_json="attempt detail status=running terminal=false record=01a059cb-65c4-7103-a77a-91c7bbe056ce version=01a059cb-65c4-7103-a77a-91b094abc7b4 state_digest=fd9579224d751b87122de90713eccef033ac994a491c7a8414711eeaf5b770a9 scope_json={} record_relations_out=0 record_relations_in=0 record_relation_types=none statement=Dogfood running attempt for 4LM"
context_item.3.summary_json="attempt detail status=succeeded terminal=true record=01a059cb-688f-75a0-948c-dcf27e0aee9c version=01a059cb-6b4e-71a0-9f1e-824b3336474b state_digest=9b856ad7613fc7bf59b9f0b1fe8f9773d5dd24ed05e6ed7506c6bf69f5623e91 scope_json={} record_relations_out=0 record_relations_in=0 record_relation_types=none statement=Dogfood succeeded attempt for 4LM"
context_item.4.summary_json="attempt detail status=failed terminal=true record=01a059cb-6e14-7791-ae30-0a99cc82575a version=01a059cb-70cc-77e0-a3b6-0caae980e91f state_digest=d848229bbb5ab842c48797161b4140a3d75b67668437f70f567d186b506942c8 scope_json={} record_relations_out=0 record_relations_in=0 record_relation_types=none statement=Dogfood failed attempt for 4LM"
context_item.5.summary_json="attempt detail status=inconclusive terminal=true record=01a059cb-7381-71b0-9e6b-a41e2ba9c1d5 version=01a059cb-763f-7413-8538-caffbc194993 state_digest=f1b353877f887aa3e3b833203d59d3d5058a9adb6bca5c0eaf7e70b0b2d0b7da scope_json={} record_relations_out=0 record_relations_in=0 record_relation_types=none statement=Dogfood inconclusive attempt for 4LM"
```

Full summary:

```text
dogfood_result=passed
dogfood_store=.work-governance/runtime/dogfood/phase-4lm-20260831214811.sqlite
dogfood_session_id=01a059cb-78df-7f33-b068-7612bee2fa2a
failed_attempt_id=01a059cb-6e14-7791-ae30-0a99cc82575a
```

## Validation

Targeted validation passed before documentation closeout:

```text
cargo check -p workvcs-core
bash -n scripts/smoke-v0.1-cli-workflow.sh
cargo test -p workvcs-core --test context_profile_budget_phase4kx
```

Final validation matrix passed:

```text
cargo fmt --all -- --check
cargo clippy --quiet --all-targets --all-features -- -D warnings
cargo test --workspace --quiet
scripts/validate-schema-v0.1.sh
scripts/smoke-v0.1-cli-workflow.sh
git diff --check
```

Validation logs:

```text
/tmp/workvcs-4lm-cargo-check.log
/tmp/workvcs-4lm-context-test.log
.work-governance/runtime/logs/phase-4lm-dogfood-20260831212914.log
.work-governance/runtime/logs/phase-4lm-dogfood-20260831214811.log
.work-governance/runtime/logs/phase-4lm-final-validation-20260831214056.log
.work-governance/runtime/logs/phase-4lm-final-validation-20260831214949.log
```

## Findings

- `failed_attempt` remains a brief-eligible `P5` packet item.
- `attempt` remains normal/full eligible for running, succeeded, and
  inconclusive Attempts.
- Attempt summaries now expose status, terminality, current Record identity,
  state digest, canonical scope, Record relation counts, Record relation type
  buckets, and statement.
- Non-Attempt Record packet summaries keep their existing format.
- Attempt-to-Knowledge relation detail is not rendered because current V1
  semantics only allow Finding records to source Record-to-Knowledge relations.
- Transition rationale is not part of `RecordSnapshot`; a future slice must
  decide whether V1 needs explicit transition-rationale projection.
- Remaining Context Resolver gaps are path-sensitive Knowledge policy, context
  packet persistence, transition-rationale projection decision, and broader
  multi-project dogfood.
