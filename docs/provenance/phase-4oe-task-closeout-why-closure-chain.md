# Phase 4OE Task Closeout Why Closure Chain Evidence

Status: current local evidence
Date: 2026-09-03

## Scope

Phase 4OE advances the Context resolver, packets, and `why` explanations gate
by closing one concrete closeout explanation gap: after a VR-backed
Verification with Evidence closed a Task, `workvcs why` on the Task or its
Acceptance Criterion did not show the AC -> VR -> Verification -> Evidence
closure chain.

The slice is intentionally narrow. It does not change Store schema, CLI flags,
ContextPacket fields, snapshot schema, Verification/Evidence semantics,
Resource observation, cache-refresh, Task closeout semantics, or direct
relation endpoint evolution behavior. It does not perform release,
release-candidate, Push, tag, deployment, remote, cloud, V2, GUI/TUI,
distributed collaboration, or Agent orchestration action.

## Gap Proof

The corrected pre-change public CLI probe used a temporary local Store under:

```text
log_dir=/tmp/workvcs-4oe-task-closeout-why-chain-probe-20260903T100242Z
```

The probe created:

```text
one Task
one Acceptance Criterion
one Verification Requirement
one Evidence item
one passed Verification targeting the Verification Requirement
one closed Task
```

Observed before implementation:

```text
probe_execution_status=PASS
probe_result=GAP_FOUND
task_show_after_closeout_status=done
ac_status_after_verify=verified
task_why_has_ac=false
task_why_has_vr=false
task_why_has_verification=false
task_why_has_evidence=false
ac_why_has_vr=false
ac_why_has_verification=false
ac_why_has_evidence=false
control_vr_verifies=PASS
control_verification_evidenced_by=PASS
control_evidence_evidenced_by=PASS
```

The first discarded probe at
`/tmp/workvcs-4oe-task-closeout-why-chain-probe-20260903T100036Z` failed
because it used a stale Task version after AC creation. That was a probe
mistake, not product evidence. The corrected probe used the current Task
version before closeout.

## Implementation

Changed:

```text
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/src/history/mod.rs
crates/workvcs-core/src/lib.rs
crates/workvcs-core/tests/why_verification_neighborhood_phase3u.rs
crates/workvcs-cli/src/main.rs
docs/decisions/adr/0493-phase-4oe-task-closeout-why-closure-chain.md
docs/provenance/phase-4oe-task-closeout-why-closure-chain.md
docs/provenance/v1-readiness-ledger.md
docs/provenance/v1-release-gate-matrix.md
```

`WhyQueryResult` now includes `verification_closure_chains`. The projection is
populated only for current Task and current Acceptance Criterion subjects. For
a Task subject, the query walks the Task's current Acceptance Criteria and
their current Verification Requirements at the queried commit. For an
Acceptance Criterion subject, the query walks only that criterion's current
Verification Requirements. It then reports current Verifications targeting
those Verification Requirements and the Evidence ids cited by those
Verifications.

The CLI renders the projection as stable key-value fields under
`verification_closure_chain.<index>.*`.

## Focused Validation

Focused tests passed:

```text
log_dir=/tmp/workvcs-4oe-implementation-20260903T101500Z
cargo test -p workvcs-core --test why_verification_neighborhood_phase3u why_reports_vr_backed_verification_closure_from_task_and_criterion_after_task_closeout=PASS
cargo test -p workvcs-cli cli_why_projects_vr_backed_verification_closure_from_task_and_criterion=PASS
```

The core test proves the read API reports exactly one closure chain from both
the Task and the Acceptance Criterion after Task closeout, with matching AC,
VR, Verification, result, and Evidence ids.

The CLI test proves the same behavior through public key-value `why` output.

## Dogfood Proof

The post-change public CLI dogfood used a temporary Store and the newly built
`target/debug/workvcs` binary:

```text
log_dir=/tmp/workvcs-4oe-task-closeout-why-closure-chain-20260903T105642Z
phase4oe_dogfood=PASS
commands_exit_failures=0
failed_assertions=0
task_show_done=true
ac_status_verified=true
task_why_verification_closure_chains=true
ac_why_verification_closure_chains=true
task_why_chain_has_ac=true
task_why_chain_has_vr=true
task_why_chain_has_verification=true
task_why_chain_has_evidence=true
task_why_chain_result_passed=true
ac_why_chain_has_vr=true
ac_why_chain_has_verification=true
ac_why_chain_has_evidence=true
ac_why_chain_result_passed=true
control_vr_verifies_relation_edges=true
control_vr_verifies_evolution_ops=true
control_verification_evidenced_by_relation_edges=true
control_verification_evidenced_by_evolution_ops=true
control_evidence_evidenced_by_relation_edges=true
control_evidence_evidenced_by_evolution_ops=true
```

## Final Validation

Final validation passed:

```text
log_dir=/tmp/workvcs-4oe-task-closeout-why-closure-chain-20260903T105642Z/final-validation
validation_status=PASS
validation_passes=14
validation_failures=0
```

The matrix covered:

```text
git status --short --branch
cargo fmt --all -- --check
cargo test -p workvcs-core --test why_verification_neighborhood_phase3u why_reports_vr_backed_verification_closure_from_task_and_criterion_after_task_closeout -- --exact --nocapture
cargo test -p workvcs-cli tests::cli_why_projects_vr_backed_verification_closure_from_task_and_criterion -- --exact --nocapture
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

Phase 4OE closes one concrete Task/Acceptance Criterion closeout explanation
gap. The direct VR, Verification, and Evidence endpoint controls already
worked; the new projection lets an operator identify the same closure chain
from the closed Task or the relevant Acceptance Criterion.

The proof is bounded. It does not show broad relation traversal,
relation-subject queries, multi-hop/full evolution traversal, broader causal
traversal, broader context/Resource resolver maturity, or release-candidate
readiness.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
The Operator discoverability and actionable recovery gate remains `Pass` with
less Task-closeout id plumbing in this narrow `why` path.

The Candidate release operation gate remains `Blocked`. The overall release
decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
