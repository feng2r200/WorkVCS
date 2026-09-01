# Phase 4NF Self-Contained Run Log Summary Evidence

Status: current local evidence
Date: 2026-09-02

## Scope

Phase 4NF addresses an operator discoverability and actionable recovery
weakness found during Phase 4NE independent review: the maintained Store
portability script preserved the per-command `run.log`, but its final compact
key-value summary was available only from stdout and governance evidence.

The slice makes successful preserved run logs self-contained by appending the
final summary to `run.log` while keeping stdout unchanged.

This evidence does not claim V1 release readiness, V0.1 dogfood completion,
release-candidate authorization, runtime behavior change, CLI behavior change,
schema change, Bundle profile change, default smoke expansion, benchmark
maturity, external Store activation, remote/cloud behavior, or V2 behavior.

## Script Change

`scripts/maintained-store-portability-v0.1.sh` now emits the final success
summary through one grouped block piped to:

```bash
tee -a "$log_file"
```

The final stdout keys are preserved. The successful run log now ends with the
same summary block.

## Dogfood Command

```bash
WORKVCS_MAINTAINED_STORE_CYCLES=2 \
WORKVCS_MAINTAINED_STORE_SEED_TASKS=1 \
WORKVCS_MAINTAINED_STORE_TASKS_PER_CYCLE=2 \
WORKVCS_MAINTAINED_STORE_VERIFICATIONS_PER_CYCLE=1 \
WORKVCS_MAINTAINED_STORE_RELATION_PAIRS_PER_CYCLE=1 \
WORKVCS_MAINTAINED_STORE_OUTPUT_ROOT=.work-governance/runtime/logs/phase-4nf \
  ./scripts/maintained-store-portability-v0.1.sh
```

## Result

```text
maintained_store_result=PASS
store_id=01a05eb9-c10b-7bc1-af06-111e2b7b417a
workspace_id=01a05eb9-c144-71d1-ba05-55e45ed870fd
branch_id=01a05eb9-c144-71d1-ba05-5614d87c0019
cycles=2
source_reopens=2
same_target_applies=2
target_restore_checks=1
lineage_list_checks=2
seed_tasks=1
tasks_per_cycle=2
final_task_count=5
verification_requirements=2
verification_records=2
relation_versions=4
baseline_head_commit_id=01a05eb9-c159-7503-9892-7a28a9b94a41
baseline_state_digest=d66ea4c5b745f2f4b0652e9eb915574e641f39cfa306d2ca6ae4a44d51610772
final_source_head_commit_id=01a05eb9-c5ed-7341-b1f6-291da76a9e35
final_source_state_digest=d577c69bbe6ce034c1ae4b3907103ed55b248ab091383e468390d2be447b9b28
final_target_head_commit_id=01a05eb9-c5ed-7341-b1f6-291da76a9e35
final_target_state_digest=d577c69bbe6ce034c1ae4b3907103ed55b248ab091383e468390d2be447b9b28
cycle_1_payload_files=28
cycle_1_payload_references=68
cycle_2_payload_files=51
cycle_2_payload_references=125
elapsed_seconds=18
output_dir=.work-governance/runtime/logs/phase-4nf/maintained-store-portability.j1e5q5
log_file=.work-governance/runtime/logs/phase-4nf/maintained-store-portability.j1e5q5/run.log
```

## Evidence Checks

The stdout capture and the final same-length run-log tail are byte-identical:

```text
stdout_capture=.work-governance/runtime/logs/phase-4nf/small-run.stdout
tail_summary_capture=.work-governance/runtime/logs/phase-4nf/small-run.log-tail-summary
summary_stdout_log_tail_match=true
phase_4nf_log_failure_markers_absent=true
```

The final summary keys are visible in both captures:

```text
maintained_store_result=PASS
final_task_count=5
verification_records=2
relation_versions=4
elapsed_seconds=18
output_dir=.work-governance/runtime/logs/phase-4nf/maintained-store-portability.j1e5q5
log_file=.work-governance/runtime/logs/phase-4nf/maintained-store-portability.j1e5q5/run.log
```

Validated behaviors:

- the script still completed the maintained Store portability loop;
- the final stdout summary was preserved;
- the same final summary was appended to the preserved `run.log`;
- the per-command log remained available before the final summary;
- the run log contained no `FAIL`, `ERROR`, `panic`, or `error_code=` markers;
- final source and target head commit IDs matched; and
- final source and target WorkState digests matched.

## Final Validation

The closeout validation passed:

```text
bash -n scripts/maintained-store-portability-v0.1.sh
cargo fmt --all -- --check
git diff --check
workctl plan validate
phase_4nf_files_links_boundaries_present=true
release_false_boundary_preserved=true
summary_stdout_log_tail_match=true
phase_4nf_log_failure_markers_absent=true
```

Independent read-only review for `PLAN-20260901-058` / `T-005` found no
blocker, high, or medium issues. The review checked the script diff, stdout
summary preservation, `run.log` tail equality proof, failure-marker absence,
documentation links, release-false markers, and the unchanged `Partial` /
Blocks V1 release `Yes` operator gate.

## Boundary

This is a focused operator-log self-containedness improvement. It should be
used as evidence for reducing one real review and handoff ambiguity in the
operator discoverability/recovery gate. It is not a benchmark, not a release
candidate run, and not evidence for runtime, CLI, schema, Bundle profile,
remote/cloud, external Store activation, or V2 behavior.
