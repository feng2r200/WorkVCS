# Phase 4NI Operator Recovery Maturity Dogfood Evidence

Status: current local evidence
Date: 2026-09-02

## Scope

Phase 4NI advances the Operator discoverability and actionable recovery release
gate by adding and running a single local script-readable recovery matrix.

The slice verifies existing behavior. It does not change Store schema, Rust
runtime semantics, command semantics, error codes, output fields, release
state, Push state, tags, remote state, deployment state, V2 scope, automatic
recovery, background re-observation, stale detection, GUI/TUI behavior, or
Agent orchestration.

## Contract Inspection

The current `AGENTS.md`, V1 readiness ledger, release gate matrix, operator
quickstart, error recovery guide, core error taxonomy, CLI error renderer, and
repository smoke script were inspected before implementation.

Key boundaries confirmed:

```text
target_gate=Operator discoverability and actionable recovery
gate_status_before=Partial
blocking_gap=broader_recovery_maturity
core_error_codes=41
operator_guide_codes=42_including_cli_parse_error
missing_in_guide=none
extra_in_guide=none
runtime_semantics_change_allowed=false
store_schema_change_allowed=false
release_or_remote_action_allowed=false
```

## Implementation

Added:

```text
scripts/operator-recovery-maturity-v0.1.sh
```

The script builds the current `workvcs` binary, creates a temporary local Store,
records command outputs under `.work-governance/runtime/logs/phase-4ni/`, and
prints a compact key-value summary.

The script checks:

```text
guide coverage against current ErrorCode::as_str values plus cli_parse_error
guide retryability rule: only branch_head_conflict is true
cli_parse_error JSON shape and help recovery
branch_head_conflict key-value shape and branch-head refresh retry
Resource drift/unavailable/error projection and restoration to applicable
Claim guard stale takeover hints, precondition failure, mark-stale, takeover
Merge unresolved freeze guard, explicit resolve, freeze, continue
final Store integrity with --require-valid
```

## Dogfood Proof

The passing run reported:

```text
phase4ni_operator_recovery_maturity=PASS
log_dir=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ni-operator-recovery-matrix/.work-governance/runtime/logs/phase-4ni/operator-recovery-maturity.20260901T230436Z
tmp_dir=/var/folders/9y/54ywjc8j2_lbc8fcs0ns8pyw0000gn/T//workvcs-operator-recovery.20260901T230436Z.wbaupw
store=/var/folders/9y/54ywjc8j2_lbc8fcs0ns8pyw0000gn/T//workvcs-operator-recovery.20260901T230436Z.wbaupw/operator-recovery.sqlite
core_error_codes=41
guide_error_codes=42
guide_coverage_missing=0
guide_coverage_extra=0
guide_retryability_matches_core_rule=true
cli_parse_error_json_recovery=passed
branch_head_conflict_error_code=branch_head_conflict
branch_head_conflict_retryable=true
branch_head_conflict_recovery=passed
resource_drift_reason_code=resource_drift
resource_drift_recovery=applicable
resource_unavailable_reason_code=resource_unavailable
resource_unavailable_recovery=applicable
resource_error_reason_code=resource_error
resource_error_recovery=applicable
resource_final_ac_status=verified
claim_guard_reason=exclusive_claim_owned_by_other_session
claim_takeover_precondition_error_code=claim_invalid
claim_takeover_recovery_claim_id=01a05f37-a6d1-79f2-b994-eac15f92c7b1
claim_takeover_recovery=passed
merge_unresolved_error_code=workspace_invalid
merge_unresolved_recovery=completed
merge_result_commit_id=01a05f37-a789-7222-9cdf-be06d5be2fb1
final_integrity_valid_required=true
```

## Final Validation

The closeout pass also ran:

```text
bash -n scripts/operator-recovery-maturity-v0.1.sh
scripts/operator-recovery-maturity-v0.1.sh
cargo fmt --all -- --check
scripts/validate-schema-v0.1.sh
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --quiet
scripts/smoke-v0.1-cli-workflow.sh
git diff --check
release false flag scan
```

Observed results:

```text
phase4ni_operator_recovery_maturity=PASS
schema-v0.1 validation ok
smoke_result=passed
cargo_test_all_quiet=passed
cargo_clippy_all_targets_all_features=passed
diff_check=passed
release_false_flags_preserved=true
```

## Interpretation

The run proves that the current V1-local CLI and recovery guide can support a
broader operator recovery matrix without source inspection. The proof is still
bounded: it covers representative local workflows and script-readable recovery,
not automatic recovery, remote/distributed coordination, cloud synchronization,
GUI/TUI use, Agent orchestration, V2 behavior, or a release-candidate process.

## Release Gate Impact

Phase 4NI closes the Operator discoverability and actionable recovery gate for
the bounded V1-local release scope.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```

Remaining blockers are the Context resolver, packets, and `why` explanations
gate, plus the separately blocked candidate release operation.
