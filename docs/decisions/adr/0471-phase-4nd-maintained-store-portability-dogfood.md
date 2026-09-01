# ADR-0471: Phase 4ND Maintained Store Portability Dogfood

Status: Accepted
Date: 2026-09-02

## Context

After Phase 4NC, the V1 release gate matrix still marked Core Store, lineage,
integrity, and local portability as `Partial`. Phase 4LO and Phase 4LP had
proved the local copied-target Bundle profile and its directory contract, and
Phase 4LQ had proved a bounded larger Store portability run. The remaining gap
was narrower: one maintained source Store had not yet been dogfooded through
repeated reopen, Checkpoint, Bundle export/apply, integrity, and doctor cycles
against the same copied target Store.

The current V1-local Bundle profile remains same-Store copied-target
fast-forward only. Cross-Store canonical DAG activation, packaged Bundle
containers, remote exchange, and V2 semantics remain outside this scope.

## Decision

Add `scripts/maintained-store-portability-v0.1.sh` as an opt-in dogfood script.
It is not part of the default smoke path.

The script builds and uses the real local CLI, creates one source Store, copies
one target Store after a seed baseline, and then keeps using the same source and
target Stores across multiple maintenance cycles. Each cycle:

- reopens the source Store through public CLI commands;
- adds semantic state with Tasks, Acceptance Criteria, Verification
  Requirements, Verifications, Evidence, and scheduling Relations;
- creates a Checkpoint and exports a Bundle directory;
- validates the Bundle directory and preflights the same target Store;
- applies the Bundle to the same target Store as a same-Store fast-forward;
- checks target Branch head and WorkState digest convergence;
- validates the imported Checkpoint and Bundle import metadata;
- checks that same-Store copied-target lineage has no cross-Store lineage
  records; and
- runs source and target integrity plus source and target doctor in
  require-valid mode.

The script also proves target-local post-apply maintenance by forking a
target-local Branch inside the same target Store, adding local work, restoring
that Branch to the imported Bundle head, and confirming the target main Branch
remains at the imported head so later Bundle cycles can continue.

One attempted tightening confirmed a boundary rather than a defect:
`store lineage-record` rejects `source_store_id == local_store_id`, so the final
same-Store dogfood uses `store lineage-list --expected-lineages 0` to prove the
copied-target profile does not create cross-Store lineage records.

## Non-Goals

- No schema change.
- No command spelling change.
- No default smoke matrix expansion.
- No new Bundle profile.
- No packaged archive/container format.
- No external Store canonical DAG activation.
- No remote exchange, signing, compression, streaming, or cloud sync.
- No distributed collaboration or federation.
- No performance-index or query-plan design.
- No V2 behavior.
- No release-readiness, dogfood-complete, or release-candidate claim.

## Evidence

The final default opt-in run passed:

```text
maintained_store_result=PASS
cycles=3
source_reopens=3
same_target_applies=3
target_restore_checks=1
lineage_list_checks=3
seed_tasks=4
tasks_per_cycle=6
final_task_count=22
verification_requirements=3
verification_records=3
relation_versions=12
final_source_head_commit_id=01a05e8a-f88a-75d0-bb07-863de3b26c08
final_source_state_digest=c05438072a235a9b6008f07d04ef879666a5fe9cc0126164fe91730358844484
final_target_head_commit_id=01a05e8a-f88a-75d0-bb07-863de3b26c08
final_target_state_digest=c05438072a235a9b6008f07d04ef879666a5fe9cc0126164fe91730358844484
elapsed_seconds=7
output_dir=.work-governance/runtime/logs/phase-4nd/maintained-store-portability.wXgeYW
log_file=.work-governance/runtime/logs/phase-4nd/maintained-store-portability.wXgeYW/run.log
```

Log spot checks for the final run:

```text
bundle_apply_dir_commands=3
store_lineage_list_commands=3
doctor_commands=6
restore_commands=1
source_store_info_commands=4
```

Focused script validation also passed:

```text
bash -n scripts/maintained-store-portability-v0.1.sh
```

## Consequences

The Core Store, lineage, integrity, and local portability release gate can move
to `Pass` for the bounded V1-local scope. The evidence now covers ordinary and
maintained same-Store copied-target portability with repeated source reopen,
Checkpoint, Bundle apply, integrity, doctor, and target-local restore proof.

The overall release decision remains false. Context/Resource resolver and
broader `why` maturity, broader operator recovery maturity, larger or more
varied performance evidence, and release-candidate operation remain blocking
until separately proven from fresh candidate evidence.
