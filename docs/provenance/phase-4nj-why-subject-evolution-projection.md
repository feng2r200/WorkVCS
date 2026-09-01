# Phase 4NJ Why Subject Evolution Projection Evidence

Status: current local evidence
Date: 2026-09-02

## Scope

Phase 4NJ advances the Context resolver, packets, and `why` explanations
release gate by making `workvcs why --entity <changed Entity>` project that
Entity's own direct first-parent evolution ChangeOperation.

The slice is intentionally narrow. It does not change Store schema, stored
Relation semantics, mutation semantics, CLI flags, release state, Push state,
tags, remote state, deployment state, V2 scope, full evolution traversal,
broader causal traversal, broader context/Resource resolver behavior, GUI/TUI
behavior, or Agent orchestration.

## Gap Proof

The pre-change dogfood gap used a temporary local Store with:

```text
prior Decision: Use optimistic writes
causal Finding: Concurrent write tests fail without serialization
replacement Decision: Use serialized writes
Decision supersede with --because-record <finding>
```

The supersede ChangeSet had a direct ChangeOperation for the prior Decision
Entity, but `why --entity <prior decision>` did not project it:

```text
changeset_operations_for_prior_subject=1
changeset_operations_match_expected=true
why_prior_relation_edges=1
why_prior_causal_anchor_changesets=0
why_prior_evolution_change_operations=0
why_finding_causal_anchor_changesets=1
why_finding_evolution_change_operations=3
```

Focused failing tests then captured the same gap:

```text
core_failure=left 0 right 1 for evolution_change_operations
cli_failure=why evolution change operations 0 does not match expected 1
```

## Implementation

Changed:

```text
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/tests/why_evolution_operation_projection_phase4ng.rs
crates/workvcs-cli/src/main.rs
```

The implementation keeps the existing causal-anchor ChangeSet path, then adds
a direct Entity-subject path for `WhyQuerySubject::Entity`:

```text
target_history=first_parent_from_target_commit
workspace_filter=same_workspace
subject_filter=ChangeOperationSubject::Entity(query_entity_id)
evolution_filter=entity_membership_change.before_entity_version_id is not null
dedupe_key=operation_id
```

Initial Entity creation operations are excluded from this direct evolution
slice because they have no prior Entity version.

## Focused Validation

The post-change focused validation passed:

```text
cargo fmt --all
cargo test -p workvcs-core --test why_evolution_operation_projection_phase4ng why_changed_entity_projects_own_direct_evolution_operation --quiet
cargo test -p workvcs-cli cli_supersedes_decision_record_atomically --quiet
cargo test -p workvcs-core --test why_evolution_operation_projection_phase4ng --quiet
```

Observed results:

```text
core_direct_subject_test=passed
cli_supersede_test=passed
core_why_evolution_projection_tests=2_passed
```

## Dogfood Proof

The old-main/new-binary dogfood run built:

```text
old_main_commit=4aab99a36bd79d799f04c263e5ae8c07989bbf84
new_branch=phase-4nj-why-subject-evolution
```

It used the old main binary to create one Store and queried the same Store and
supersede commit with both binaries.

Observed output:

```text
phase4nj_why_subject_evolution=PASS
tmp_dir=/var/folders/9y/54ywjc8j2_lbc8fcs0ns8pyw0000gn/T//workvcs-4nj-dogfood.B84eHM
store=/var/folders/9y/54ywjc8j2_lbc8fcs0ns8pyw0000gn/T//workvcs-4nj-dogfood.B84eHM/workvcs.sqlite
prior_record_entity_id=01a05f52-41ae-70d1-b9ec-29a9a36249a6
finding_record_entity_id=01a05f52-41c5-7f52-a815-b8c41e6524fd
replacement_record_entity_id=01a05f52-41d9-7681-9038-5a5100760b22
supersede_commit_id=01a05f52-41ee-7701-be77-d6adaf1a37fd
supersede_changeset_id=01a05f52-41ee-7701-be77-d69ed1c15bea
old_prior_evolution_change_operations=0
new_prior_evolution_change_operations=1
new_prior_evolution_match_expected=true
new_prior_operation_changeset_id=01a05f52-41ee-7701-be77-d69ed1c15bea
new_prior_operation_type=record.decision.supersede
new_prior_subject_family=entity
new_prior_subject_object_id=01a05f52-41ae-70d1-b9ec-29a9a36249a6
new_prior_subject_statement_json="Use optimistic writes"
new_prior_deferred_relation_families=1
new_prior_deferred_relation_family_0=evolution
new_finding_causal_anchor_changesets=1
new_finding_evolution_change_operations=3
new_finding_evolution_match_expected=true
```

## Interpretation

Phase 4NJ removes the concrete post-4NG/4NH lookup gap for a directly changed
queried Entity. The prior Decision remains non-anchor
(`causal_anchor_changesets=0`), but `why` now shows the direct
`record.decision.supersede` operation that changed it.

The proof is still bounded. It does not show relation-subject traversal, a
ChangeSet endpoint, multi-hop/full evolution graph traversal, broader causal
traversal, or broader context/Resource resolver maturity.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NJ advances the gate by closing one concrete direct Entity-subject
evolution projection gap.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
