# Phase 4NA Resource Re-Observation Scheduling Policy Dogfood Evidence

Date: 2026-09-02

## Scope

Phase 4NA advances the Resource registration, observation, applicability, and
drift gate by deciding and dogfooding the bounded V1-local Resource
re-observation scheduling policy.

The slice is intentionally narrow:

- make explicit foreground operator-triggered batch refresh visible as the
  V1-local Resource re-observation scheduling policy;
- prove the batch command selects current-head Resource-backed Verifications;
- prove non-resource-backed Verifications are skipped;
- prove a changed sandbox Resource projects `stale` / `resource_drift`;
- prove an unchanged sandbox Resource remains `applicable`;
- prove the batch refresh does not move Branch head;
- leave the original project unchanged.

Out of scope:

- daemon, watcher, automatic polling, cron integration, or implicit refresh;
- read-command side effects;
- Agent orchestration or automatic task scheduling;
- Store schema, command-shape, manifest fingerprint profile, or cache semantic
  changes;
- remote, distributed, cross-Store, V2, release, release-candidate, tag, Push,
  or deployment behavior.

## Implementation Evidence

`verification cache-refresh --all-resource-backed --resource-content-from-basis`
now renders explicit policy metadata:

```text
reobservation_policy=explicit_operator_batch_refresh
background_reobservation=disabled
reobservation_trigger=operator_explicit
reobservation_execution=foreground_command
selection_policy=current_head_resource_backed_verifications
branch_head_mutation=disabled
```

The fields are output metadata only. The command shape, Store schema, Resource
basis shape, manifest fingerprint profiles, current-head selection logic, cache
applicability semantics, and foreground execution behavior remain unchanged.

Current-behavior inspection passed in
`/tmp/workvcs-4na-reobservation-scheduling-inspection-20260901T171900Z/summary.txt`:

```text
resource_gate_gap=background re-observation scheduling
batch_refresh_command_present=0
batch_requires_basis=0
batch_rejects_explicit_verification=0
batch_rejects_per_cache_expectations=0
batch_selects_current_head_verifications=0
batch_filters_resource_backed=0
batch_prevalidates_inputs_before_writes=0
batch_renderer_lacks_policy_metadata=0
adr_0448_no_scheduler=0
adr_0455_no_scheduler=0
ledger_background_open=0
matrix_background_required_next=0
policy_contract=explicit_foreground_operator_batch_refresh
background_reobservation=disabled
automatic_scheduler=none
implicit_refresh=disabled
phase4na_reobservation_scheduling_inspection=pass
```

Focused regression coverage extends
`cli_batch_refreshes_resource_backed_verifications_from_basis` so the
successful batch refresh path asserts the new policy fields while still proving
current-head resource-backed selection, non-resource-backed skip behavior,
expected refreshed count matching, and Branch head stability. The existing
`cli_batch_basis_refresh_rejects_unsupported_basis_before_observation_writes`
test remains the atomic unsupported-basis guard.

Focused validation passed in
`/tmp/workvcs-4na-focused-validation-corrected-20260901T172740Z/summary.txt`:

```text
diff-check=0
fmt-check=0
test-batch-refresh=0
test-batch-atomic=0
test-cache-refresh-help=0
zero-test-check=0
anchors=0
phase4na_focused_validation=pass
```

## Dogfood Evidence

Dogfood log:

```text
/tmp/workvcs-4na-resource-reobservation-scheduling-dogfood-20260901T173650Z
```

The source project was the existing local Git repository:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul
```

Only the `/tmp` clone was mutated:

```text
external_original_branch_before=main
external_original_branch_after=main
external_original_head_before=e3004b29c8bf3791e87d877b021432dcd1158705
external_original_head_after=e3004b29c8bf3791e87d877b021432dcd1158705
external_original_status_before_sha256=0e657cf1c09bc2bf315c38979352a913fe18b31c4f16c32443306a1b03aa81c3
external_original_status_after_sha256=0e657cf1c09bc2bf315c38979352a913fe18b31c4f16c32443306a1b03aa81c3
external_original_unchanged=yes
external_clone=/tmp/workvcs-4na-resource-reobservation-scheduling-dogfood-20260901T173650Z/work/agent_soul-reobserve-clone
external_clone_head=e3004b29c8bf3791e87d877b021432dcd1158705
```

The Store created two Resource-backed Verifications and one non-resource-backed
Verification at current head:

```text
verification_readme=01a05dfb-24e9-78c3-9251-6f7c4681dad3
verification_agents=01a05dfb-250a-7d53-80b7-c41c5731793a
verification_plain=01a05dfb-252c-7ca0-a91d-8dc2ad369727
observations_before=2
```

After mutating only the clone's `README.md`, the explicit foreground batch
refresh reported the V1-local scheduling policy:

```text
batch.reobservation_policy=explicit_operator_batch_refresh
batch.background_reobservation=disabled
batch.reobservation_trigger=operator_explicit
batch.reobservation_execution=foreground_command
batch.selection_policy=current_head_resource_backed_verifications
batch.branch_head_mutation=disabled
batch.refreshed_caches=2
batch.refreshed_caches_match_expected=true
```

The refreshed caches projected drift and unchanged state correctly:

```text
readme_cache_applicability=stale
readme_cache_reason_code=resource_drift
agents_cache_applicability=applicable
agents_cache_reason_code=all_basis_applicable
plain_cache_found=false
observations_after=4
branch_head_unchanged_by_batch=0
clone_dirty_readme=0
doctor_valid_required=yes
integrity_valid_required=yes
phase4na_resource_reobservation_scheduling=pass
```

Two harness attempts were corrected before the successful run:

- `/tmp/workvcs-4na-resource-reobservation-scheduling-dogfood-20260901T173100Z`
  failed before exercising Resource behavior because `verify` evidence content
  omitted the required `--evidence-content-role`.
- `/tmp/workvcs-4na-resource-reobservation-scheduling-dogfood-20260901T173330Z`
  reached the core batch refresh successfully, but the harness used obsolete
  `--required-valid` instead of `--require-valid` for final doctor/integrity
  checks.

## Readiness Impact

Phase 4NA closes the background re-observation scheduling policy gap for the
bounded V1-local Resource registration, observation, applicability, and drift
gate. The release-scope policy is:

```text
reobservation_policy=explicit_operator_batch_refresh
background_reobservation=disabled
```

The Resource gate is now `Pass` for the bounded V1-local release gate matrix.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Final Validation

Candidate full pre-merge validation passed before independent review in
`/tmp/workvcs-4na-full-validation-candidate-20260901T174530Z/summary.txt`:

```text
diff-check=0
fmt-check=0
schema-v0-1=0
clippy=0
cargo-test-workspace=0
smoke-v0-1=0
anchors=0
phase4na_candidate_full_validation=pass
```

Read-only independent review agent
`01a05e02-ea38-7990-bd14-754e630b8a61` found no blocker and two issues:

- High: Plan T-004 mentioned commit, merge, and cleanup without an in-Plan
  confirmation gate. The Plan now records accepted confirmation `C-001`,
  binding commit, fast-forward merge, and exact worktree cleanup to the current
  active `/goal` Git-strategy authorization while keeping Push, release, tag,
  deployment, remote state, branch deletion, and unrelated cleanup
  unauthorized.
- Medium: Resource gate `Pass` was documented before full validation and
  independent review evidence were referenced. This section and ADR-0468 now
  reference candidate full validation, independent review, and final
  post-review validation before local git delivery.

Final post-review validation is recorded in
`/tmp/workvcs-4na-final-validation-post-review-20260901T175200Z/summary.txt`
and covers the final Phase 4NA diff:

```text
diff-check=0
fmt-check=0
schema-v0-1=0
clippy=0
cargo-test-workspace=0
smoke-v0-1=0
anchors=0
phase4na_final_validation=pass
```
