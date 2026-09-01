# Phase 4NB Goal/Plan/Task and AC/VR Recovery Dogfood Evidence

Date: 2026-09-02

## Scope

Phase 4NB advances two release-maturity gates that remained blocking after
Phase 4NA:

- Goal, Plan, Task, ordering, dependencies, and containment;
- AC, VR, Verification, Evidence, and the top-level verification wrapper in a
  recovery and Handoff-consumption scenario.

This is dogfood and documentation evidence only. It does not change Rust code,
Store schema, CLI command shape, release state, Push state, tags, remote state,
or V2 scope.

## Contract Inspection

The current release matrix, readiness ledger, and Phase 4LV, Phase 4MS, and
Phase 4NA evidence were inspected before the dogfood run. The contract
inspection passed in:

```text
/tmp/workvcs-4nb-contract-inspection-20260901T181900Z/summary.txt
```

Key inspection results:

```text
phase4nb_contract_inspection=pass
source_commit=dade547388019fe45ba5cf44de50027632ea6a1c
target_gates=goal_plan_task_dependencies_containment,ac_vr_recovery_handoff
resource_gate_after_4na=pass
goal_plan_task_gate_before=partial_blocks_v1
ac_vr_gate_before=partial_blocks_v1
command_goal_create_present=0
command_plan_create_present=0
command_task_contain_present=0
command_task_depends_on_present=0
command_task_ordered_before_present=0
command_verify_resource_scope_present=0
command_handoff_consume_present=0
command_cache_refresh_basis_present=0
policy_scope=v1_local_dogfood_only
```

The selected dogfood target was the pre-existing local Git repository:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul
```

Only a `/tmp` clone was mutated.

## T-002 Dogfood

The successful T-002 run is recorded in:

```text
/tmp/workvcs-4nb-goal-plan-task-ac-vr-recovery-dogfood-20260901T184100Z/summary-t002.txt
```

Store and target:

```text
store=/tmp/workvcs-4nb-goal-plan-task-ac-vr-recovery-dogfood-20260901T184100Z/work/workvcs.sqlite
clone=/tmp/workvcs-4nb-goal-plan-task-ac-vr-recovery-dogfood-20260901T184100Z/work/agent_soul-4nb-clone
clone_head=e3004b29c8bf3791e87d877b021432dcd1158705
clone_status_initial_bytes=0
```

The Store created one Goal, one Plan, and three contained Tasks:

```text
goal_id=01a05e1d-afc0-7c93-b3a5-12cd0df9b75f
plan_id=01a05e1d-afd3-7d50-8e78-57c275e4cb38
task_a_id=01a05e1d-afe6-7590-a4f5-4f4e46454fe6
task_b_id=01a05e1d-affa-7d50-99c5-a21ed10ecf5d
task_c_id=01a05e1d-b00d-75b3-84cc-bad04f3ea38b
containment_relations=4
depends_on_relations=2
ordered_before_relations=2
```

Dependency and completion-gate evidence:

```text
blocked_context_contains_dependency=yes
runnable_before_a=task_a_only
task_b_blocked_before_a=yes
task_c_blocked_before_b=yes
blocked_transition_status=1
branch_head_unchanged_after_blocked_transition=yes
ac_a_status_before=unverified
ac_a_status_after=verified
verification_a_id=01a05e1d-b2d7-7d90-977c-806e0caee3e7
task_a_status=done
task_b_runnable_after_a=yes
task_c_still_blocked_after_a=yes
```

One earlier harness attempt reached useful state but failed because the source
Session focus still pointed at completed Task A. The corrected run released
Task A's Claim and cleared the Session focus before inspecting global runnable
projection. That correction matches current CLI semantics: focused Sessions
intentionally narrow runnable context.

## T-003 Dogfood

T-003 continued the same Store rather than recreating the scenario. The
successful continuation is recorded in:

```text
/tmp/workvcs-4nb-goal-plan-task-ac-vr-recovery-dogfood-20260901T184100Z/summary-t003.txt
```

Focused Handoff and continuation evidence:

```text
handoff_record_id=01a05e21-2c74-7121-9f34-ab51c4fcb160
handoff_commit_id=01a05e21-2c74-7121-9f34-ab2c7e075add
source_session_diff_id=01a05e21-2c55-70b3-b5fc-ddb47412acd5
continuation_session_id=01a05e21-2ca2-7eb2-a0ab-eeac9142d9ee
task_b_id=01a05e1d-affa-7d50-99c5-a21ed10ecf5d
blocked_b_preverify_status=1
```

Resource-backed stale and recovery evidence:

```text
verification_b_baseline_id=01a05e21-2ea1-72a3-b98c-a034fc3d044d
clone_readme_dirty=yes
batch_reobservation_policy=explicit_operator_batch_refresh
batch_background_reobservation=disabled
batch_reobservation_trigger=operator_explicit
batch_reobservation_execution=foreground_command
batch_selection_policy=current_head_resource_backed_verifications
batch_branch_head_mutation=disabled
batch_refreshed_caches=2
branch_head_unchanged_by_batch=yes
cache_b_after_drift_applicability=stale
cache_b_after_drift_reason_code=resource_drift
ac_b_status_before=unverified
ac_b_status_after_baseline=verified
ac_b_status_after_drift=stale
blocked_b_after_drift_status=1
blocked_b_after_drift_message=stale_expected_verified
ac_b_status_after_recovery=verified
verification_b_recovery_id=01a05e22-fd50-7301-b2f4-7434aea52dfe
```

Closeout evidence:

```text
task_b_status=done
task_c_status=done
plan_status=complete
goal_status=achieved
final_head_commit_id=01a05e22-ff33-7533-9562-c568b077cffd
final_history_entries=27
doctor_valid_required=yes
integrity_valid_required=yes
```

The original `agent_soul` repository remained unchanged:

```text
external_original_branch_before=main
external_original_branch_after=main
external_original_head_before=e3004b29c8bf3791e87d877b021432dcd1158705
external_original_head_after=e3004b29c8bf3791e87d877b021432dcd1158705
external_original_status_before_sha256=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
external_original_status_after_sha256=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
external_original_diff_before_sha256=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
external_original_diff_after_sha256=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
external_original_unchanged=yes
```

## Readiness Impact

Phase 4NB closes the named release-maturity gaps for Goal/Plan/Task ordering,
dependencies, and containment, and for AC/VR closure in recovery and
Handoff-consumption scenarios. The evidence is bounded to V1-local CLI behavior
and real-project temporary clone dogfood.

The release matrix remains non-ready because other blocking gates remain
`Partial` or `Blocked`.

## Validation

Full post-review validation passed in:

```text
/tmp/workvcs-4nb-final-validation-post-review-20260901T193000Z/summary.txt
```

Covered checks:

```text
git diff --check
cargo fmt --all -- --check
scripts/validate-schema-v0.1.sh
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/smoke-v0.1-cli-workflow.sh
workctl plan validate
README link checks
release-state literal checks
release-gate matrix consistency checks
Phase 4NB provenance unchanged-original check
```

The summary ended with:

```text
phase4nb_candidate_validation=pass
```

Independent read-only review checked the Plan, README, ADR, provenance,
readiness ledger, release gate matrix, contract inspection summary, T-002
summary, T-003 summary, and candidate validation summary. It found no blocker,
high, or medium issues. Its only low finding was that this Validation section
still described final validation and independent review as pending after the
candidate validation had passed. That finding is resolved by this section.

The formal WorkVCS `plan independent-review record` path was not used because
this lightweight Plan has no `independent_validation` object or trusted
attestation structure. The review is therefore treated as read-only subagent
evidence, not as a formal attested review record.

The validation and review did not authorize a release candidate, release, tag,
Push, deployment, remote/cloud Handoff, cross-Store synchronization,
distributed collaboration, automatic takeover, daemon, watcher, Agent
orchestration, original-project mutation, schema change, Rust code change, or
V2 feature.
