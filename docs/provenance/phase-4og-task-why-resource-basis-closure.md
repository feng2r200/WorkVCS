# Phase 4OG Task Why Resource Basis Closure Evidence

Status: current local evidence
Date: 2026-09-03

## Scope

Phase 4OG advances the Context resolver, packets, and `why` explanations gate
by closing one concrete Resource-backed closeout explanation gap: after a
Resource-backed VR Verification is part of the Task/Acceptance Criterion
closure chain, Task and Acceptance Criterion `why` did not expose the Resource
basis needed for recovery.

The slice is intentionally narrow. It does not change Store schema, CLI flags,
ContextPacket fields, snapshot schema, Verification/Evidence semantics,
Resource observation, applicability, cache-refresh, Task closeout semantics, or
direct relation endpoint evolution behavior. It does not perform release,
release-candidate, Push, tag, deployment, remote, cloud, V2, GUI/TUI,
distributed collaboration, or Agent orchestration action.

## Gap Proof

The pre-change public CLI probe used a temporary local Store under:

```text
log_dir=/tmp/workvcs-4of-next-gap-probe-20260903T112544Z
```

The probe created:

```text
one Task
one Acceptance Criterion
one Verification Requirement
one Evidence item
one local-file Resource
one Resource observation
one passed Resource-backed Verification targeting the Verification Requirement
one closed Task
```

Observed before implementation:

```text
probe_execution_status=PASS
probe_result=GAP_FOUND
gap_kind=task_ac_why_omits_resource_basis_for_resource_backed_closeout
task_why_has_resource_basis=false
ac_why_has_resource_basis=false
verification_show_resource_basis=true
verification_show_resource_id=true
verification_show_observation_id=true
task_why_closure=true
ac_why_closure=true
task_why_has_verification=true
ac_why_has_verification=true
task_why_has_evidence=true
ac_why_has_evidence=true
```

The direct Verification detail already exposed `resource_basis=1`; only the
Task and Acceptance Criterion `why` closure projection omitted it.

## Implementation

Changed:

```text
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/tests/why_verification_neighborhood_phase3u.rs
crates/workvcs-cli/src/main.rs
docs/decisions/adr/0494-phase-4og-task-why-resource-basis-closure.md
docs/provenance/phase-4og-task-why-resource-basis-closure.md
docs/provenance/v1-readiness-ledger.md
docs/provenance/v1-release-gate-matrix.md
```

`WhyVerificationClosureChain` now carries the existing
`VerificationResourceBasis` entries from each Verification state already
selected for the closure chain.

The CLI renders those entries as stable key-value fields under
`verification_closure_chain.<index>.resource_basis.<basis_index>.*`, including:

```text
resource_id
adapter_kind
adapter_schema_version
scope_kind
scope_schema_version
scope_payload_json
baseline_observation_id
baseline_fingerprint
```

## Focused Validation

Focused tests passed:

```text
log_dir=/tmp/workvcs-4og-resource-basis-why-closure-20260903T113715Z
cargo test -p workvcs-core --test why_verification_neighborhood_phase3u why_reports_resource_basis_in_vr_backed_verification_closure_from_task_and_criterion -- --exact --nocapture=PASS
cargo test -p workvcs-cli tests::cli_why_projects_resource_basis_in_vr_backed_verification_closure -- --exact --nocapture=PASS
```

The core test proves the read API reports the same Resource basis from both the
Task and Acceptance Criterion closure chain after Task closeout.

The CLI test proves the same behavior through public key-value `why` output,
including resource id, adapter, scope, canonical scope payload JSON, baseline
observation id, and baseline fingerprint.

## Dogfood Proof

The post-change public CLI dogfood used a temporary Store and the newly built
`target/debug/workvcs` binary:

```text
log_dir=/tmp/workvcs-4og-resource-basis-why-closure-20260903T113715Z
phase4og_dogfood=PASS
commands_exit_failures=0
failed_assertions=0
task_show_done=true
ac_status_verified=true
ac_status_after_closeout=stale
verify_resource_basis=true
verify_resource_observation=true
verification_show_resource_basis=true
task_why_resource_basis=true
ac_why_resource_basis=true
task_why_scope_payload=true
ac_why_scope_payload=true
task_why_baseline_observation=true
ac_why_baseline_observation=true
task_why_baseline_fingerprint=true
ac_why_baseline_fingerprint=true
release_flag_positive_hits=0
```

The first dogfood attempt in the same log directory failed one assertion
because the script expected the Resource-backed Acceptance Criterion to remain
`verified` after Task closeout advanced the Branch head. The corrected dogfood
keeps the verified assertion before closeout and uses post-closeout Task and
Acceptance Criterion `why` as the evidence surface for Resource basis
projection.

## Final Validation

Final validation passed:

```text
log_dir=/tmp/workvcs-4og-resource-basis-why-closure-20260903T113715Z/final-validation
validation_status=PASS
validation_passes=14
validation_failures=0
```

The matrix covered:

```text
git status --short --branch
cargo fmt --all -- --check
cargo test -p workvcs-core --test why_verification_neighborhood_phase3u why_reports_resource_basis_in_vr_backed_verification_closure_from_task_and_criterion -- --exact --nocapture
cargo test -p workvcs-cli tests::cli_why_projects_resource_basis_in_vr_backed_verification_closure -- --exact --nocapture
scripts/validate-schema-v0.1.sh
release flag scan for accidental positive release-flag assignments
git diff --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --quiet
cargo build -p workvcs-cli
post-change public CLI dogfood
scripts/smoke-v0.1-cli-workflow.sh
workctl work status
git status --short --branch
```

## Interpretation

Phase 4OG closes one concrete Resource-backed Task/Acceptance Criterion
closeout explanation gap. The direct `verification show` Resource basis output
already worked; the new projection lets an operator identify the same Resource
basis from the closed Task or relevant Acceptance Criterion.

The proof is bounded. It does not show broad relation traversal,
relation-subject queries, multi-hop/full evolution traversal, broader causal
traversal, broader context/Resource resolver maturity, or release-candidate
readiness.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
The Operator discoverability and actionable recovery gate remains `Pass` with
less Resource-backed Task closeout id plumbing in this narrow `why` path.

The Candidate release operation gate remains `Blocked`. The overall release
decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
