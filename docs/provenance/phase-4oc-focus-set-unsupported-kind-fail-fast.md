# Phase 4OC Focus-Set Unsupported Kind Fail-Fast Evidence

Status: current local evidence
Date: 2026-09-03

## Scope

Phase 4OC advances the Session, Claim, Runnable, `claim next`, and `next`
release gate by closing a concrete Session focus contract mismatch:
`session focus-set` accepted a current Verification Requirement as focus even
though focused runtime consumers only support current Goal, Plan, and Task
focus entities.

The slice is intentionally narrow. It does not add new focus kinds, does not
change Store schema, ContextPacket fields, snapshot schema, CLI flags,
Resource observation or verification semantics, runnable selection, Claim
behavior, `claim next`, `next`, or `why`, and does not perform release,
release-candidate, Push, tag, deployment, remote, cloud, V2, GUI/TUI,
distributed collaboration, or Agent orchestration action.

## Gap Proof

The corrected pre-change public CLI probe used a temporary local Store under:

```text
log_dir=/tmp/workvcs-4oc-vr-focus-resource-context-probe-20260903T081318Z
```

The probe created:

```text
one Task
one Acceptance Criterion
one Resource-backed Verification Requirement
one active Session
```

Observed before implementation:

```text
phase4oc_corrected_probe_status=PASS
probe_result=GAP_FOUND
gap_kind=session_focus_contract_mismatch
commands_exit_failures=4
control_unfocused_full_vr_basis_hint=true
control_task_focus_brief_vr_basis_hint=true
session_focus_set_accepts_verification_requirement=true
context_vr_focus_returns_session_invalid=true
runnable_vr_focus_returns_session_invalid=true
```

The same Resource-backed Verification Requirement recovery hint was visible in
unfocused full context and valid Task-focused brief context. The mismatch was
specific to `session focus-set` accepting an unsupported focus type; it was not
a Resource basis, context budget, or renderer failure.

## Implementation

Changed:

```text
crates/workvcs-core/src/runtime/session.rs
crates/workvcs-core/tests/session_runtime_phase3e.rs
crates/workvcs-cli/src/main.rs
docs/decisions/adr/0492-phase-4oc-focus-set-unsupported-kind-fail-fast.md
docs/provenance/phase-4oc-focus-set-unsupported-kind-fail-fast.md
docs/provenance/v1-readiness-ledger.md
docs/provenance/v1-release-gate-matrix.md
```

`set_session_focus` now checks that the requested focus entity is present at
the active Branch head and then verifies that the current entity resolves as a
Goal, Plan, or Task before writing `session_focus`, `session_focus_path`, or a
`session.focus_set` event.

The error remains a structured runtime error:

```text
error_code=session_invalid
error_category=runtime
retryable=false
message contains "is not a current Goal, Plan, or Task at branch head"
```

Historical persisted focus selection validation still checks only WorkState
presence. This avoids schema migration or runtime row rewrite in this narrow
slice.

## Focused Validation

Focused tests and build passed:

```text
log_dir=/tmp/workvcs-4oc-focus-set-unsupported-kind-20260903T082308Z/focused
cargo test -p workvcs-core --test session_runtime_phase3e set_focus_rejects_verification_requirement_without_partial_rows=PASS
cargo test -p workvcs-cli cli_rejects_verification_requirement_session_focus=PASS
cargo build -p workvcs-cli=PASS
```

The core test proves a current Verification Requirement focus is rejected with
`SessionInvalid`, no partial runtime rows are written, and the Session snapshot
keeps `focus=None`.

The CLI test proves the public `session focus-set --focus
<verification_requirement>` path rejects the unsupported focus type and the
Session remains inspectable with `--expected-focus none`.

## Dogfood Proof

The post-change public CLI dogfood used a temporary Store and the newly built
`target/debug/workvcs` binary:

```text
log_dir=/tmp/workvcs-4oc-focus-set-unsupported-kind-20260903T082308Z/dogfood
phase4oc_dogfood=PASS
commands_exit_failures=0
failed_assertions=0
focus_set_vr_exit=expected_failure
focus_set_vr_error_code=session_invalid
focus_set_vr_error_category=runtime
focus_set_vr_message_contains_supported_kind=true
session_after_reject_focus=none
context_unfocused_succeeded=true
task_focus_succeeded=true
context_task_focus_has_requirement=true
context_task_focus_has_resource_basis=true
context_task_focus_has_refresh_hint=true
```

## Final Validation

Final validation passed:

```text
log_dir=/tmp/workvcs-4oc-focus-set-unsupported-kind-20260903T084201Z/final-validation
validation_status=PASS
validation_passes=16
validation_failures=0
```

The matrix covered:

```text
git status --short --branch
cargo fmt --all -- --check
cargo test -p workvcs-core --test session_runtime_phase3e set_focus_rejects_verification_requirement_without_partial_rows -- --exact --nocapture
cargo test -p workvcs-core --test session_runtime_phase3e set_focus_rejects_absent_entities_and_relations_without_partial_rows -- --exact --nocapture
cargo test -p workvcs-cli tests::cli_rejects_verification_requirement_session_focus -- --exact --nocapture
cargo test -p workvcs-cli tests::cli_runs_session_switch_to_forked_branch_workflow -- --exact --nocapture
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

Phase 4OC closes one concrete Session focus contract mismatch. A current but
unsupported Verification Requirement can no longer be persisted as focus, and
the operator receives a stable immediate `session_invalid` error. The failure
does not mutate the Session focus, and valid Task focus still surfaces the
same Resource-backed Verification Requirement recovery hint.

The proof is bounded. It does not show support for non Goal/Plan/Task focus
entities, broader context/Resource resolver maturity, full relation-subject
traversal, multi-hop/full evolution traversal, broader causal traversal, or
release-candidate readiness.

## Release Gate Impact

The Session, Claim, Runnable, `claim next`, and `next` gate remains `Pass` with
the focus contract mismatch closed.

The Context resolver, packets, and `why` explanations gate remains `Partial`
because broader context/Resource resolver maturity, full relation-subject
traversal, multi-hop/full evolution traversal, and broader causal traversal
remain open.

The Candidate release operation gate remains `Blocked`. The overall release
decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
