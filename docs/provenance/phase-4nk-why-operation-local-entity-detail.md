# Phase 4NK Why Operation-Local Entity Detail Evidence

Status: current local evidence
Date: 2026-09-02

## Scope

Phase 4NK advances the Context resolver, packets, and `why` explanations
release gate by making direct queried-Entity evolution operation detail
operation-local.

The slice targets the concrete post-4NJ residual risk: when a queried Entity
has multiple first-parent direct Entity changes before the target commit,
`workvcs why` can list the direct evolution operations, but older operation
details must not reuse only the final target commit's current Entity version.

The slice is intentionally narrow. It does not change Store schema, stored
Relation semantics, mutation semantics, CLI flags, release state, Push state,
tags, remote state, deployment state, V2 scope, full evolution traversal,
relation-subject traversal, broader causal traversal, broader context/Resource
resolver behavior, GUI/TUI behavior, or Agent orchestration.

## Gap Proof

The pre-change gap used a temporary local Store with one Assumption Record:

```text
create assumption: The cache is always fresh
transition assumption to validated
transition assumption to invalidated
query why at the invalidated commit for the same Assumption Entity
```

The current Phase 4NJ baseline could report both direct Entity evolution
operations, but the older validated operation still showed the final
invalidated Entity version:

```text
why_evolution_change_operations=2
why_match_expected=true
why_op0_operation_id=01a05f6e-6423-7f33-a4d0-95d3072c822d
why_op0_subject_entity_version_id=01a05f6e-6423-7f33-a4d0-95e1bffa0bd1
why_op0_detail_matches_invalidated=true
why_op1_operation_id=01a05f6e-640d-7671-9af1-ecdabe7f112e
why_op1_subject_entity_version_id=01a05f6e-6423-7f33-a4d0-95e1bffa0bd1
why_op1_expected_validated_version_id=01a05f6e-640d-7671-9af1-ece03a24e212
why_op1_actual_matches_final_invalidated=true
why_op1_actual_matches_operation_validated=false
```

Focused failing tests then captured the same gap:

```text
core_failure=older operation subject detail entity_version_id was final invalidated version, not validated operation version
cli_failure=evolution_change_operation.1.subject_entity_version_id was final invalidated version, not validated operation version
```

## Implementation

Changed:

```text
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/tests/why_record_relations_phase3ax.rs
crates/workvcs-cli/src/main.rs
```

The implementation keeps the existing causal-anchor ChangeSet path and
ADR-0477 direct Entity-subject history scan. For direct Entity operations found
through the queried-Entity path, it now loads the operation's
`entity_membership_change` row and uses the operation-local after version for
Entity detail:

```text
operation_is_direct_evolution=before_entity_version_id is not null
subject_detail_version=after_entity_version_id
statement_lookup_commit=operation_commit_id
```

Initial Entity creation operations remain excluded from the direct evolution
slice because they have no `before_entity_version_id`. Operations with a prior
version and no after version remain projected as direct evolution operations,
but they do not reuse the target-current Entity detail.

## Focused Validation

The post-change focused validation passed:

```text
cargo test -p workvcs-core --test why_record_relations_phase3ax why_reports_operation_local_entity_detail_for_multiple_direct_assumption_changes --quiet
cargo test -p workvcs-cli cli_why_reports_operation_local_entity_detail_for_multiple_direct_assumption_changes --quiet
cargo test -p workvcs-core --test why_record_relations_phase3ax --quiet
cargo test -p workvcs-core --test why_evolution_operation_projection_phase4ng --quiet
```

Observed results:

```text
core_operation_local_detail_test=passed
cli_operation_local_detail_test=passed
core_why_record_relations_tests=2_passed
core_why_evolution_projection_tests=2_passed
```

## Dogfood Proof

The old-main/new-binary dogfood run built:

```text
old_commit=7a4ad9fa00d3ac96a80b2124eca222b605146e6f
new_branch=phase-4nk-why-operation-local-detail
```

It used the old main binary to create one Store and queried the same Store and
invalidated commit with both binaries.

Observed output:

```text
phase4nk_dogfood=true
store=/var/folders/9y/54ywjc8j2_lbc8fcs0ns8pyw0000gn/T/workvcs-4nk-dogfood.XXXXXX.dJ6peVKUXp/workvcs.sqlite
entity_id=01a05f79-cd38-7720-8a10-ea646e1fb616
validated_operation_id=01a05f79-cd4f-7992-82a7-e7d9c83e9288
validated_version_id=01a05f79-cd4f-7992-82a7-e7e90d01fa80
invalidated_operation_id=01a05f79-cd64-7cd0-85a5-62a2769d5b0d
invalidated_version_id=01a05f79-cd64-7cd0-85a5-62b1cd3da597
old_evolution_change_operations=2
old_op1_operation_id=01a05f79-cd4f-7992-82a7-e7d9c83e9288
old_op1_subject_entity_version_id=01a05f79-cd64-7cd0-85a5-62b1cd3da597
old_op1_actual_matches_final_invalidated=true
old_op1_actual_matches_operation_validated=false
new_evolution_change_operations=2
new_op1_operation_id=01a05f79-cd4f-7992-82a7-e7d9c83e9288
new_op1_subject_entity_version_id=01a05f79-cd4f-7992-82a7-e7e90d01fa80
new_op1_actual_matches_final_invalidated=false
new_op1_actual_matches_operation_validated=true
```

## Interpretation

Phase 4NK removes the concrete post-4NJ detail gap for direct queried-Entity
evolution operations. `why` now keeps both direct operations visible and makes
the older operation detail reflect the Entity version produced by that
operation.

The proof is still bounded. It does not show relation-subject traversal, a
ChangeSet endpoint, multi-hop/full evolution graph traversal, broader causal
traversal, or broader context/Resource resolver maturity.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NK advances the gate by closing one concrete operation-local direct
Entity detail gap.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
