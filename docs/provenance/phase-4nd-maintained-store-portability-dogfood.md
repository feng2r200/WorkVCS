# Phase 4ND Maintained Store Portability Dogfood Evidence

Status: current local evidence
Date: 2026-09-02

## Scope

Phase 4ND closes the maintained Store portability gap named by the V1 release
gate matrix. It stays inside the current V1-local profile: one source Store,
one copied target Store with the same Store identity, directory Bundle
export/import/apply, Checkpoints, local restore, integrity, and doctor.

This evidence does not claim release readiness, V0.1 dogfood completion,
external Store canonical DAG activation, packaged Bundle containers, remote
exchange, cloud sync, distributed collaboration, performance maturity, or V2
behavior.

## Script

The reusable script is:

```text
scripts/maintained-store-portability-v0.1.sh
```

It is not called by the default smoke script. It writes full command output to
a run log and emits a compact key-value summary on success.

Default workload:

```text
WORKVCS_MAINTAINED_STORE_CYCLES=3
WORKVCS_MAINTAINED_STORE_SEED_TASKS=4
WORKVCS_MAINTAINED_STORE_TASKS_PER_CYCLE=6
WORKVCS_MAINTAINED_STORE_VERIFICATIONS_PER_CYCLE=1
WORKVCS_MAINTAINED_STORE_RELATION_PAIRS_PER_CYCLE=2
```

The workload can be adjusted by overriding those environment variables. Keep
the run opt-in until a larger default smoke matrix is explicitly justified.

## Final Run

Command:

```bash
WORKVCS_MAINTAINED_STORE_OUTPUT_ROOT=.work-governance/runtime/logs/phase-4nd \
  ./scripts/maintained-store-portability-v0.1.sh
```

Result:

```text
maintained_store_result=PASS
store_id=01a05e8a-ea0b-7092-8941-95a5ad458361
workspace_id=01a05e8a-ea45-7570-941f-367485bacbe0
branch_id=01a05e8a-ea45-7570-941f-36a78488c89f
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
baseline_head_commit_id=01a05e8a-ea96-7771-b97c-05c3b214c77f
baseline_state_digest=bece4cca9622c0003d672394215f6412753738e298291123e63371c5d850a36d
final_source_head_commit_id=01a05e8a-f88a-75d0-bb07-863de3b26c08
final_source_state_digest=c05438072a235a9b6008f07d04ef879666a5fe9cc0126164fe91730358844484
final_target_head_commit_id=01a05e8a-f88a-75d0-bb07-863de3b26c08
final_target_state_digest=c05438072a235a9b6008f07d04ef879666a5fe9cc0126164fe91730358844484
cycle_1_head_commit_id=01a05e8a-ec31-7c52-a67a-7bdb7cc40f9b
cycle_1_state_digest=92f77da2a5fbc1fef4620eedc5c6e82233910b578bd188f1b59a4fee4fb346e6
cycle_1_checkpoint_id=01a05e8a-ec57-72a3-8db8-06debe8e52df
cycle_1_target_lineages=0
cycle_1_bundle_import_id=01a05e8a-ed30-7770-8750-a5d0ef1a6426
cycle_1_bundle_digest=74afaf5636bba0fcfd64921c1b23583955052264f8189a021679b2b6ee33d707
cycle_1_payload_files=44
cycle_1_payload_references=122
cycle_1_restore_commit_id=01a05e8a-ef4b-7262-85cc-b193c82d243c
cycle_2_head_commit_id=01a05e8a-f16e-7da0-aeed-2e8ff810d80a
cycle_2_state_digest=bb42ad3ace7b3a402d4c5b40a35e1fab8a8272cd4468816bf1b0514b80a16b93
cycle_2_checkpoint_id=01a05e8a-f198-7063-8057-b465885e0701
cycle_2_target_lineages=0
cycle_2_bundle_import_id=01a05e8a-f2ae-70d1-a1ba-892e8c6a265d
cycle_2_bundle_digest=94cb79bb63379f4e45653915b2d4047e83caff8bcfeac601c49af939bab62009
cycle_2_payload_files=77
cycle_2_payload_references=215
cycle_2_restore_commit_id=none
cycle_3_head_commit_id=01a05e8a-f88a-75d0-bb07-863de3b26c08
cycle_3_state_digest=c05438072a235a9b6008f07d04ef879666a5fe9cc0126164fe91730358844484
cycle_3_checkpoint_id=01a05e8a-f8b5-7970-87bb-d6204d555bf9
cycle_3_target_lineages=0
cycle_3_bundle_import_id=01a05e8a-fa08-7362-96fd-a75267eff00a
cycle_3_bundle_digest=76478c9d5acfb29c61b3666de3321f1d2038e30acae993aeed82a76f4e4b822d
cycle_3_payload_files=110
cycle_3_payload_references=308
cycle_3_restore_commit_id=none
elapsed_seconds=7
output_dir=.work-governance/runtime/logs/phase-4nd/maintained-store-portability.wXgeYW
log_file=.work-governance/runtime/logs/phase-4nd/maintained-store-portability.wXgeYW/run.log
```

Validated behaviors:

- the source Store was initialized once, then reopened in three maintenance
  cycles;
- every cycle added current V1 semantic state through the public CLI;
- every cycle created a Checkpoint, exported and validated a Bundle directory,
  preflighted the target, and applied to the same copied target Store;
- every apply was a same-Store fast-forward and updated one Branch head;
- after every apply, the target Branch head and WorkState digest matched the
  source;
- every cycle validated the imported Checkpoint and Bundle import metadata;
- every cycle confirmed same-Store copied-target lineage lists zero cross-Store
  lineage records;
- cycle 1 created a target-local Branch, added post-apply local work, restored
  that Branch to the imported Bundle head, and confirmed the target main Branch
  remained at the imported head for later cycles; and
- every cycle ran source and target integrity plus source and target doctor in
  require-valid mode.

Log spot checks:

```text
bundle_apply_dir_commands=3
store_lineage_list_commands=3
doctor_commands=6
restore_commands=1
source_store_info_commands=4
```

## Boundary Finding

While tightening lineage coverage, `store lineage-record` rejected a
same-Store copied-target lineage attempt with:

```text
error_code=query_invalid
message=query invalid: store lineage source_store_id must differ from local store_id
```

That is a same-Store profile boundary, not a Phase 4ND defect. The final
dogfood therefore uses `store lineage-list --expected-lineages 0` to prove the
same-Store copied-target workflow does not create cross-Store lineage records.

## Remaining Gaps

This run closes the maintained Store portability gap for the bounded V1-local
same-Store copied-target profile. It does not close context/Resource resolver
and broader `why` maturity, broader operator recovery maturity, larger or more
varied performance evidence, or release-candidate operation.

## Validation And Review

Focused validation passed:

```text
bash -n scripts/maintained-store-portability-v0.1.sh
WORKVCS_MAINTAINED_STORE_OUTPUT_ROOT=.work-governance/runtime/logs/phase-4nd ./scripts/maintained-store-portability-v0.1.sh
```

Documentation and boundary checks passed:

```text
cargo fmt --all -- --check
git diff --check
Phase 4ND file existence and executable-bit checks
Phase 4ND README and release matrix link checks
release-state literal checks preserving V1_RELEASE_READY=false,
  V0_1_DOGFOOD_COMPLETE=false, and RELEASE_CANDIDATE_ALLOWED=false
formal run summary checks in ADR-0471 and this provenance file
final run log failure-marker check
```

Independent read-only review found no blocker, high, or medium issues. The
review checked the script against the 4ND contract, the final run log, ADR,
provenance, README, operator guide, readiness ledger, and release gate matrix.
It confirmed that the evidence supports the bounded maintained Store
portability claim and that the documents do not claim V1 release readiness,
V0.1 dogfood completion, release-candidate authorization, external Store,
remote/cloud, V2, or performance maturity.

The formal WorkVCS `plan independent-review record` path was not used because
this lightweight Plan has no `independent_validation` object or trusted
attestation structure. The review is therefore treated as read-only subagent
evidence, not as a formal attested review record.
