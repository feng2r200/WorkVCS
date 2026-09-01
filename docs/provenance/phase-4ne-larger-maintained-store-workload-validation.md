# Phase 4NE Larger Maintained Store Workload Validation Evidence

Status: current local evidence
Date: 2026-09-02

## Scope

Phase 4NE addresses the larger Store and performance evidence gate by running a
larger maintained Store workload through the existing opt-in portability
script. The run stays inside the current V1-local same-Store copied-target
profile and uses public CLI commands only.

This evidence does not claim release readiness, V0.1 dogfood completion,
external Store canonical DAG activation, packaged Bundle containers, remote
exchange, cloud sync, federation, index design, general performance maturity,
or V2 behavior.

## Command

```bash
WORKVCS_MAINTAINED_STORE_CYCLES=5 \
WORKVCS_MAINTAINED_STORE_SEED_TASKS=12 \
WORKVCS_MAINTAINED_STORE_TASKS_PER_CYCLE=20 \
WORKVCS_MAINTAINED_STORE_VERIFICATIONS_PER_CYCLE=4 \
WORKVCS_MAINTAINED_STORE_RELATION_PAIRS_PER_CYCLE=8 \
WORKVCS_MAINTAINED_STORE_OUTPUT_ROOT=.work-governance/runtime/logs/phase-4ne \
  ./scripts/maintained-store-portability-v0.1.sh
```

## Result

```text
maintained_store_result=PASS
store_id=01a05e9c-8954-70a0-805c-245b02a0c7d9
workspace_id=01a05e9c-898d-7442-9997-6d0f391c56b3
branch_id=01a05e9c-898d-7442-9997-6d342eea063c
cycles=5
source_reopens=5
same_target_applies=5
target_restore_checks=1
lineage_list_checks=5
seed_tasks=12
tasks_per_cycle=20
final_task_count=112
verification_requirements=20
verification_records=20
relation_versions=80
baseline_head_commit_id=01a05e9c-8a83-7221-9df4-c1147da3cf75
baseline_state_digest=2e7ed165e6d78a3eb80482579baf5662187699300ac86d8b7e756c924a893a71
final_source_head_commit_id=01a05ea0-8e7e-7e62-8169-e6b3be863b78
final_source_state_digest=437c61bd224372024e31b98c23db218f7eeac371e2cdd058d59c7964aa8c6762
final_target_head_commit_id=01a05ea0-8e7e-7e62-8169-e6b3be863b78
final_target_state_digest=437c61bd224372024e31b98c23db218f7eeac371e2cdd058d59c7964aa8c6762
cycle_1_payload_files=151
cycle_1_payload_references=425
cycle_2_payload_files=275
cycle_2_payload_references=773
cycle_3_payload_files=399
cycle_3_payload_references=1121
cycle_4_payload_files=523
cycle_4_payload_references=1469
cycle_5_payload_files=647
cycle_5_payload_references=1817
elapsed_seconds=331
output_dir=.work-governance/runtime/logs/phase-4ne/maintained-store-portability.cn8anW
log_file=.work-governance/runtime/logs/phase-4ne/maintained-store-portability.cn8anW/run.log
```

The result block above is the final stdout summary captured in the T-002
governance evidence blob
`.work-governance/evidence/blobs/bdda6334d80633c8c09796614f9f2c933df0fa1109ccf49cbda27a4bf1c9370a`.
The `run.log` file records per-command output and does not append that final
stdout summary at its tail.

The `relation_versions=80` result is the script-maintained count for new
task-scheduling relation versions created by the configured workload: five
cycles times eight relation pairs per cycle times two scheduling relation
commands per pair. Per-command Bundle export lines in `run.log` report the
wider exported WorkState relation closure; for cycle 5 that wider export count
is `relation_versions=120`, so it is not the same metric as the Phase 4NE
minimum workload contract.

## Evidence Checks

The final run meets the Phase 4NE minimum workload contract:

```text
final_task_count=112 >= 100
verification_records=20 >= 20
scheduling_relation_versions=80 >= 80
same_target_applies=5 >= 5
source_reopens=5 >= 5
lineage_list_checks=5 >= 5
cycle_5_payload_files=647
cycle_5_payload_references=1817
source_target_head_match=true
source_target_state_digest_match=true
```

Log spot checks:

```text
bundle_apply_dir_commands=5
store_lineage_list_commands=5
doctor_commands=10
store_integrity_commands=12
restore_commands=1
failure_markers_absent=true
```

Validation and review:

```text
cargo_fmt_check=PASS
git_diff_check=PASS
script_syntax_check=PASS
phase_4ne_files_links_present=true
release_false_boundary_preserved=true
phase_4ne_log_failure_markers_absent=true
workctl_plan_validate=PLAN_VALID
independent_review=PASS
```

The independent review initially found one medium documentation issue: the
provenance and ADR cited `run.log` next to the final result block even though
the script writes final stdout summary outside the per-command log, and the
`relation_versions=80` workload count needed to be distinguished from the
cycle 5 Bundle export's wider `relation_versions=120` closure count. The
documentation now records those source and metric boundaries explicitly. A
follow-up read-only review found no blocker, high, or medium issues.

Validated behaviors:

- one source Store and one copied target Store were used across the whole run;
- every cycle reopened the source Store through public CLI commands;
- every cycle added current V1 semantic state through public CLI commands;
- every cycle created a Checkpoint, exported and validated a Bundle directory,
  preflighted the target, and applied the Bundle to the same copied target
  Store;
- every apply was a same-Store fast-forward and updated one Branch head;
- after every apply, the target Branch head and WorkState digest matched the
  source;
- every cycle confirmed same-Store copied-target lineage lists zero cross-Store
  lineage records;
- target-local post-apply maintenance and restore were proven on a forked
  target-local Branch without corrupting later target main Branch applies; and
- every cycle ran source and target integrity plus source and target doctor in
  require-valid mode.

## Boundary

This run is larger and more varied than previous smoke and bounded portability
runs, but it is still one bounded local run. It should be used as release-gate
evidence for the V1-local scope, not as a general benchmark or as justification
for indexes, storage tuning, external Store import, packaged Bundle exchange,
remote/cloud behavior, or V2 features.
