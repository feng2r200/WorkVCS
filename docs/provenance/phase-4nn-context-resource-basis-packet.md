# Phase 4NN Context Resource Basis Packet Recovery Evidence

Status: current local evidence
Date: 2026-09-02

## Scope

Phase 4NN advances the Context resolver, packets, and `why` explanations
release gate by making brief ContextPacket `verification_requirement` items
name Resource basis recovery information for current-task Verification
Requirements that already have current-head Resource-backed Verifications.

The slice is intentionally narrow. It does not change Store schema, packet
schema, context packet snapshot schema, Resource observation, applicability,
cache refresh semantics, Branch-head mutation behavior, CLI flags, `why`
traversal, release state, Push state, tags, remote state, deployment state, V2
scope, background re-observation, daemons, watchers, polling, implicit refresh,
GUI/TUI behavior, or Agent orchestration.

## Gap Proof

The pre-change gap used a temporary local Store and public CLI commands.

The probe created:

```text
one Task
one required Acceptance Criterion
one Verification Requirement
one local-file Resource
one baseline ResourceObservation
one Resource-backed Verification targeting the Verification Requirement
one active Session on the same Branch
```

Observed before implementation:

```text
context_items=6
context_item.4.category=verification_requirement
context_item.4.summary_json="verification requirement criterion=... local_key=VR-1: Refresh from the local file Resource basis"
has_resource_basis_in_context=false
```

The brief packet showed the current Verification Requirement, but it omitted
the Resource id, adapter and scope identity, baseline observation id, and the
explicit `verification cache-refresh --resource-content-from-basis` recovery
path. A continuation operator would have had to inspect verification detail
separately before knowing how to refresh the stale Resource-backed proof.

Focused failing tests captured the same gap:

```text
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_resource_basis_recovery_for_current_task_requirements -- --nocapture
core_status=101
failed at backed_item.summary.contains("resource_basis=1")

cargo test -p workvcs-cli cli_context_brief_exposes_resource_basis_recovery_hint_for_requirement -- --nocapture
cli_status=101
failed at summary_json.contains("resource_basis=1")
```

## Implementation

Changed:

```text
crates/workvcs-core/src/runtime/context.rs
crates/workvcs-core/tests/context_profile_budget_phase4kx.rs
crates/workvcs-cli/src/main.rs
```

ContextPacket generation now loads current-head `VerificationSnapshot` rows
from the same Branch head as the rest of the context overview. The packet
resolver checks that the verification snapshots belong to the active workspace
and same head commit before using them. The public `ContextOverview` shape and
the CLI overview output are unchanged.

`collect_context_items` builds a deterministic index of Resource-backed
Verifications whose target is `VerificationTarget::VerificationRequirement`.
When a current Task's Acceptance Criterion references a matching Verification
Requirement, the existing `verification_requirement` summary text is appended
with:

```text
resource_basis=<count>
verification_id=<verification_entity_id>
resource_id=<first_basis_resource_id>
adapter=<adapter_kind>@<adapter_schema_version>
scope=<scope_kind>@<scope_schema_version>
baseline_observation_id=<first_basis_baseline_observation_id|none>
refresh_hint="verification cache-refresh --verification <verification_entity_id> --resource-content-from-basis"
```

When multiple current-head Resource-backed Verifications target the same
Verification Requirement, the summary counts all matching Resource basis
entries. It selects the first matching Verification whose Resource basis
entries all have persisted baseline observations for the displayed recovery
command. If none are fully basis-refreshable, the summary reports the first
Resource-backed Verification and uses
`refresh_hint=unavailable_missing_baseline_observation`.

Verification Requirements without Resource-backed Verifications keep their
previous summary shape. The Resource detail remains summary text; no packet
field named `resource_basis` or `refresh_hint` is added.

## Focused Validation

The post-change focused validation passed:

```text
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_resource_basis_recovery_for_current_task_requirements -- --nocapture
1 passed; 0 failed

cargo test -p workvcs-cli cli_context_brief_exposes_resource_basis_recovery_hint_for_requirement -- --nocapture
1 passed; 0 failed

cargo test -p workvcs-cli cli_context_packet_renders_resource_basis_recovery_hint_summary -- --nocapture
1 passed; 0 failed

cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_prefers_refreshable_resource_basis_for_requirement_summary -- --nocapture
1 passed; 0 failed

cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_includes_current_task_verification_obligations -- --nocapture
1 passed; 0 failed
```

## Final Validation

The final local validation matrix for the implemented 4NN slice passed with
logs preserved under:

```text
log_dir=/tmp/workvcs-phase4nn-validation2.Ft5ftR
phase4nn_validation=PASS
```

The matrix covered:

```text
cargo fmt --all -- --check
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_ -- --nocapture
cargo test -p workvcs-cli cli_context_brief_exposes_resource_basis_recovery_hint_for_requirement -- --nocapture
cargo test -p workvcs-cli cli_context_packet_renders_resource_basis_recovery_hint_summary -- --nocapture
scripts/validate-schema-v0.1.sh
release flag scan for V1_RELEASE_READY, V0_1_DOGFOOD_COMPLETE, RELEASE_CANDIDATE_ALLOWED
git diff --check
workctl plan validate
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --quiet
scripts/smoke-v0.1-cli-workflow.sh
```

The smoke log ended with:

```text
smoke_result=passed
```

## Dogfood Proof

The post-change public CLI dogfood used `target/debug/workvcs` and a temporary
local Store under:

```text
log_dir=/tmp/workvcs-phase4nn-dogfood.XS378N
```

The flow was:

```text
init
workspace create
task create
ac create
vr create
resource create
resource observe
verification record --verification-requirement ... --resource ... --baseline-observation ...
session start
context --profile brief
```

The dogfood output proved:

```text
phase4nn_dogfood=PASS
context_items=6
resource_id=01a0600e-4627-7012-953a-ade7055ab82f
baseline_observation_id=01a0600e-4638-7761-88e2-d5e3699c08ff
verification_id=01a0600e-464e-7561-be8a-fcce6e72b9f0
summary_json="verification requirement criterion=01a0600e-45fc-7cc3-bf21-48510541fe0e local_key=VR-resource: Refresh the local file basis. resource_basis=1 verification_id=01a0600e-464e-7561-be8a-fcce6e72b9f0 resource_id=01a0600e-4627-7012-953a-ade7055ab82f adapter=local-file@1 scope=path@1 baseline_observation_id=01a0600e-4638-7761-88e2-d5e3699c08ff refresh_hint=\"verification cache-refresh --verification 01a0600e-464e-7561-be8a-fcce6e72b9f0 --resource-content-from-basis\""
```

## Interpretation

Phase 4NN closes the concrete recovery hint gap where a current Task's
Resource-backed Verification Requirement was visible in brief context but its
Resource basis refresh path was not. The proof is bounded to current-head
Resource-backed Verifications targeting current-task Verification Requirements.

It does not prove broad Resource resolver maturity, automatic Resource
re-observation, background scheduling, Resource discovery, cross-Store or
remote Resource refresh, full relation-subject traversal, multi-hop/full
evolution traversal, broader causal traversal, or release-candidate readiness.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NN advances the gate by closing one current-task Resource-backed VR
brief-context recovery hint gap.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
