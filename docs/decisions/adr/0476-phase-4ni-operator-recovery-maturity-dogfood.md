# ADR-0476: Phase 4NI Operator Recovery Maturity Dogfood

Status: Accepted
Date: 2026-09-02

## Context

ADR-0441, ADR-0451, ADR-0452, and ADR-0456 made WorkVCS errors
script-readable through stable key-value and JSON stderr plus per-code
operator guidance. The V1 release gate matrix still kept Operator
discoverability and actionable recovery at `Partial` because the evidence did
not yet prove a broader recovery matrix through one current CLI workflow.

Phase 4NI targets that release gate directly. It does not introduce a recovery
engine; it verifies that operators and scripts can use the existing CLI output
and guidance to identify a failure, choose a bounded recovery action, and
confirm the recovered state.

## Decision

Add `scripts/operator-recovery-maturity-v0.1.sh` as a local dogfood harness for
the current V1-local recovery surface.

The script builds the current CLI, creates a temporary local Store, and proves:

```text
guide_coverage_missing=0
guide_coverage_extra=0
guide_retryability_matches_core_rule=true
cli_parse_error_json_recovery=passed
branch_head_conflict_recovery=passed
resource_drift_recovery=applicable
resource_unavailable_recovery=applicable
resource_error_recovery=applicable
claim_takeover_recovery=passed
merge_unresolved_recovery=completed
```

The guide coverage check compares every current `ErrorCode::as_str()` value
against `docs/operator/error-recovery-guide.md`, with `cli_parse_error` added
as the top-level syntax failure code. It also checks that only
`branch_head_conflict` is documented as `retryable=true`.

The recovery matrix exercises current CLI output and operator actions:

- `cli_parse_error` JSON failure followed by successful help lookup.
- `branch_head_conflict` key-value failure followed by branch-head refresh and
  retry.
- Resource applicability moving through `stale` / `resource_drift`,
  `unknown` / `resource_unavailable`, and `unknown` / `resource_error`, each
  restored to `applicable` / `all_basis_applicable`.
- Claim guard blocking on `exclusive_claim_owned_by_other_session`, failed
  premature takeover, explicit `session mark-stale`, forced takeover, and
  successful guard re-check.
- Merge unresolved freeze guard failure, explicit item resolution, freeze, and
  successful merge continuation.

## Non-Goals

- No Store schema change.
- No Rust runtime behavior change.
- No CLI command semantics change.
- No new error codes or output fields.
- No automatic recovery, stale detection, background worker, watcher, daemon,
  or retry loop.
- No remote, distributed, cloud sync, Agent orchestration, GUI/TUI, V2,
  release-candidate, release, Push, tag, or deployment action.

## Evidence

The Phase 4NI proof run passed:

```text
phase4ni_operator_recovery_maturity=PASS
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
claim_takeover_recovery=passed
merge_unresolved_error_code=workspace_invalid
merge_unresolved_recovery=completed
final_integrity_valid_required=true
```

Detailed evidence is recorded in
`docs/provenance/phase-4ni-operator-recovery-maturity-dogfood.md`.

## Consequences

The Operator discoverability and actionable recovery release gate is now `Pass`
for the bounded V1-local scope. Operators still should reduce command friction
only when a future dogfood loop exposes a repeated, concrete blockage.

The overall V1 release decision remains false because the Context resolver,
packets, and `why` explanations gate remains `Partial`, and the candidate
release operation gate remains `Blocked` until a named candidate commit has
fresh validation and explicit release authorization.
