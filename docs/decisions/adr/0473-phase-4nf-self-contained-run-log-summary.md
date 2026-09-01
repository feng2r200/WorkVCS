# ADR-0473: Phase 4NF Self-Contained Run Log Summary

Status: Accepted
Date: 2026-09-02

## Context

Phase 4NE proved a larger maintained Store workload, but the independent
review found one documentation and evidence usability issue: the script's final
key-value result summary was captured in governance evidence and stdout, while
the preserved `run.log` contained only per-command output. A reviewer could
therefore inspect the run log and miss the final `maintained_store_result`,
`final_task_count`, `relation_versions`, `elapsed_seconds`, `output_dir`, and
`log_file` values unless they also located the governance evidence blob.

That was not a WorkVCS runtime defect. It was an operator discoverability and
handoff weakness in an opt-in validation script used for dogfood evidence.

## Decision

Update `scripts/maintained-store-portability-v0.1.sh` so the final success
summary is emitted through one grouped block piped to `tee -a "$log_file"`.
This keeps the stdout key-value summary intact and appends the same lines to
the preserved run log.

The change is intentionally limited to the script's successful final summary
emission. It does not change the script's command sequence, failure cleanup,
input environment variables, default workload sizes, or summary key names.

## Non-Goals

- No Rust runtime behavior change.
- No WorkVCS CLI command behavior or command spelling change.
- No schema change.
- No Bundle profile change.
- No default smoke expansion.
- No benchmark or performance tuning claim.
- No external Store canonical DAG activation.
- No remote exchange, cloud sync, federation, or deployment behavior.
- No V2 behavior.
- No release-readiness, V0.1 dogfood-complete, or release-candidate claim.

## Evidence

A small opt-in maintained Store run passed after the change:

```text
maintained_store_result=PASS
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
cycle_2_payload_files=51
cycle_2_payload_references=125
elapsed_seconds=18
output_dir=.work-governance/runtime/logs/phase-4nf/maintained-store-portability.j1e5q5
log_file=.work-governance/runtime/logs/phase-4nf/maintained-store-portability.j1e5q5/run.log
```

The captured stdout summary and the final same-length `run.log` tail were
byte-identical:

```text
summary_stdout_log_tail_match=true
phase_4nf_log_failure_markers_absent=true
```

## Consequences

Maintained Store portability dogfood logs are now self-contained for successful
preserved runs. A reviewer or operator can inspect the named `run.log` alone
and find both the per-command trace and the final compact result summary.

This advances the operator discoverability and actionable recovery gate by
removing a concrete evidence-source ambiguity found during real review. It
does not fully close broader operator recovery maturity, and the overall V1
release decision remains false.
