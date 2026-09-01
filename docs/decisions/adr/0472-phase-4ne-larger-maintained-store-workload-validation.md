# ADR-0472: Phase 4NE Larger Maintained Store Workload Validation

Status: Accepted
Date: 2026-09-02

## Context

After Phase 4ND, the V1 release gate matrix still marked larger Store and
performance evidence as `Partial`. The project had real evidence from Phase
4LQ's one-shot 48-Task portability run, Phase 4MM's larger merge-path Store,
and Phase 4ND's repeated 3-cycle maintained Store portability run. The
remaining evidence gap was a workload that is both larger than those bounded
runs and varied enough to exercise repeated maintained Store portability,
Checkpoint, Bundle, restore, integrity, and doctor behavior together.

The goal was evidence, not tuning. A single bounded run can improve
release-maturity confidence, but it cannot justify new indexes, storage
changes, or general performance claims.

## Decision

Treat Phase 4NE as a dogfood-only workload validation slice. It does not change
runtime behavior, CLI behavior, schema, or the Bundle profile.

Run the existing opt-in script
`scripts/maintained-store-portability-v0.1.sh` with a larger workload:

```text
WORKVCS_MAINTAINED_STORE_CYCLES=5
WORKVCS_MAINTAINED_STORE_SEED_TASKS=12
WORKVCS_MAINTAINED_STORE_TASKS_PER_CYCLE=20
WORKVCS_MAINTAINED_STORE_VERIFICATIONS_PER_CYCLE=4
WORKVCS_MAINTAINED_STORE_RELATION_PAIRS_PER_CYCLE=8
```

The run must exceed the Phase 4LQ and Phase 4ND portability workloads by
covering at least 100 final Tasks, 20 Verifications, 80 script-counted
scheduling relation versions, five same-target Bundle applies, five source
reopens, repeated Bundle payload growth, source and target integrity, source
and target doctor, and final source/target Branch head plus WorkState digest
convergence.

## Non-Goals

- No product code change.
- No schema change.
- No CLI behavior or command spelling change.
- No default smoke matrix expansion.
- No index, query-plan, or storage tuning.
- No general benchmark or throughput claim.
- No external Store canonical DAG activation.
- No packaged Bundle archive/container format.
- No remote exchange, signing, compression, streaming, cloud sync, or
  federation.
- No V2 behavior.
- No release-readiness, dogfood-complete, or release-candidate claim.

## Evidence

The larger maintained Store run passed:

```text
maintained_store_result=PASS
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
cycle_5_payload_files=647
cycle_5_payload_references=1817
final_source_head_commit_id=01a05ea0-8e7e-7e62-8169-e6b3be863b78
final_source_state_digest=437c61bd224372024e31b98c23db218f7eeac371e2cdd058d59c7964aa8c6762
final_target_head_commit_id=01a05ea0-8e7e-7e62-8169-e6b3be863b78
final_target_state_digest=437c61bd224372024e31b98c23db218f7eeac371e2cdd058d59c7964aa8c6762
elapsed_seconds=331
output_dir=.work-governance/runtime/logs/phase-4ne/maintained-store-portability.cn8anW
log_file=.work-governance/runtime/logs/phase-4ne/maintained-store-portability.cn8anW/run.log
```

The result block is the final stdout summary captured in T-002 governance
evidence, not the tail of `run.log`. The `run.log` file records per-command
output. Its Bundle export entries report a wider exported WorkState relation
closure; cycle 5 reports `relation_versions=120`. The Phase 4NE
`relation_versions=80` result is the script-maintained scheduling relation
workload count: five cycles times eight relation pairs times two scheduling
relation commands per pair.

Log spot checks for the final run:

```text
bundle_apply_dir_commands=5
store_lineage_list_commands=5
doctor_commands=10
store_integrity_commands=12
restore_commands=1
failure_markers_absent=true
```

## Consequences

The larger Store and performance evidence release gate can move to `Pass` for
the bounded V1-local release scope. The project now has larger and more varied
workload evidence than smoke, including repeated maintained Store portability
and final integrity/doctor proof.

This does not make WorkVCS generally performance mature. Future tuning should
still require workload-specific profiling evidence. The overall V1 release
decision remains false because context/Resource resolver and broader `why`
maturity, broader operator recovery maturity, and release-candidate operation
remain blocking.
