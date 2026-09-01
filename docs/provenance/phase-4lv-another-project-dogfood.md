# Phase 4LV Another Project Dogfood Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LV corrects the narrow smoke/detail bias called out in the V1 readiness
ledger by running WorkVCS against another real local project. The target was:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul
```

The target repository had pre-existing unrelated local changes. Phase 4LV did
not edit, clean, stage, commit, or otherwise mutate that repository. All
WorkVCS Store and log writes were kept under:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lv-another-project-dogfood
```

## Dogfood Run

Store and log directory:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lv-another-project-dogfood/.work-governance/runtime/dogfood/phase-4lv-20260901T013513Z.sqlite
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lv-another-project-dogfood/.work-governance/runtime/logs/phase-4lv/another-project-dogfood.20260901T013513Z
```

The run modeled this external-project work item:

```text
Check agent_soul README and SYSTEM for provider-neutral Skill boundary consistency.
```

It exercised:

- Store initialization and Workspace creation;
- Resource create/bind/associate for the external local project;
- Goal, Plan, Task, AC, and VR creation;
- Session start, `claim next`, and scoped context packet output;
- `context-packet save/show/list` for the scoped packet;
- top-level `verify` with evidence content, Resource observation, Resource
  basis, and applicability cache output;
- `ac status` reaching `verified`;
- Task transition to `done`;
- Plan completion and Goal achievement with rationale;
- a full ContextPacket snapshot proving the Goal transition rationale is
  visible;
- Session end, SessionDiff creation, focused Handoff create/show; and
- `doctor --require-valid` plus `store integrity --require-valid`.

Summary:

```text
phase4lv_dogfood_result=PASS
dogfood_store=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lv-another-project-dogfood/.work-governance/runtime/dogfood/phase-4lv-20260901T013513Z.sqlite
log_dir=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lv-another-project-dogfood/.work-governance/runtime/logs/phase-4lv/another-project-dogfood.20260901T013513Z
target_project=/Users/example/Projects/HeXun/Hernes/agent_soul
workspace_id=01a05a9b-56b8-7e71-93f4-90faea43f719
branch_id=01a05a9b-56b8-7e71-93f4-91226b0941cb
goal_entity_id=01a05a9b-5706-7730-abbf-1c73ccac46da
plan_entity_id=01a05a9b-571d-7b00-8a0c-e2b8c2ffa13a
task_entity_id=01a05a9b-5735-7f12-93cb-345a4b7cb4ee
acceptance_criterion_entity_id=01a05a9b-5764-7d61-8416-d8355b90d3e2
verification_requirement_entity_id=01a05a9b-577f-7033-a2c9-5a3eca10db9c
session_id=01a05a9b-5794-7392-ab7f-12f44dec71a4
claim_id=01a05a9b-57aa-79d0-8f47-92cfdfcd110b
context_packet_id=01a05a9b-580c-7661-890d-e96cb2f0551a
context_packet_digest=f637830473cc082ff1d37f2c8093f61a7126cccdcbfe9404f795b4aefc297f75
transition_context_packet_id=01a05a9b-596c-75a0-a42e-21b427229e3a
verification_entity_id=01a05a9b-5867-78a1-99fe-cada9a0f0cb9
resource_id=01a05a9b-56d2-7630-b328-8a5098b53af6
observation_id=01a05a9b-5862-7d20-9c97-eb66980a6182
session_diff_id=01a05a9b-5993-7a53-baba-b47256c606b7
handoff_record_id=01a05a9b-59b3-7ff1-a480-99fc768a7e3a
final_head_commit_id=01a05a9b-59b3-7ff1-a480-99c84bae479e
target_status_unchanged=PASS
```

## Target Project Evidence

The dogfood script captured before/after target `git status --porcelain=v1`
and `git diff --name-status` snapshots in the log directory, then compared
them byte-for-byte. Both comparisons passed.

The target project remained dirty because it was already dirty before this
slice. That dirty state is explicitly unrelated to Phase 4LV.

## Findings

- The current CLI can run a realistic external-project, read-only WorkVCS loop
  without a code change.
- The `verify` wrapper is useful when the operator can provide the target
  Resource, path scope, evidence file, and observed content.
- Manual id capture is still present, but it was not a blocker in this loop.
- This evidence is single-project and read-only. It does not prove write-mode
  project management, adapter-backed re-observation, automatic extraction, or
  broad performance maturity.

## Validation

The final pre-commit validation matrix passed:

```text
git_diff_check=PASS log=/tmp/workvcs-4lv-final-validation-20260901T014000Z/git_diff_check.log
cargo_fmt=PASS log=/tmp/workvcs-4lv-final-validation-20260901T014000Z/cargo_fmt.log
schema=PASS log=/tmp/workvcs-4lv-final-validation-20260901T014000Z/schema.log
cargo_clippy=PASS log=/tmp/workvcs-4lv-final-validation-20260901T014000Z/cargo_clippy.log
cargo_test=PASS log=/tmp/workvcs-4lv-final-validation-20260901T014000Z/cargo_test.log
cli_smoke=PASS log=/tmp/workvcs-4lv-final-validation-20260901T014000Z/cli_smoke.log
```

Summary path:

```text
/tmp/workvcs-4lv-final-validation-20260901T014000Z/summary.log
```
