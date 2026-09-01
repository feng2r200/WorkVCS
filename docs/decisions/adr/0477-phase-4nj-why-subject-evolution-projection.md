# ADR-0477: Phase 4NJ Why Subject Evolution Projection

Status: Accepted
Date: 2026-09-02

## Context

ADR-0474 and ADR-0475 made `workvcs why` project direct ChangeOperation
subjects and recognized subject detail for ChangeSets that were already found
through a queried Entity's causal anchors.

That still left a concrete dogfood gap. In a Decision supersede workflow, the
superseded prior Decision Entity has an incoming `record_supersedes` relation,
and the same first-parent-reachable supersede ChangeSet has a direct
ChangeOperation whose subject is that prior Decision Entity. Before Phase 4NJ,
`why --entity <prior decision>` reported `causal_anchor_changesets=0` and
`evolution_change_operations=0`, so an operator still had to inspect the
ChangeSet separately to understand the queried Entity's own direct evolution.

## Decision

`workvcs why` now combines the existing causal-anchor ChangeSet operation
projection with a bounded direct Entity-subject projection.

For `WhyQuerySubject::Entity`, `why` walks the target commit's first-parent
history in the same workspace and projects ChangeOperations whose subject is
the queried Entity. The direct projection includes only operations with an
existing prior Entity version in `entity_membership_change`, so initial Entity
creation is not counted as this evolution slice.

The projected output reuses the existing `WhyEvolutionChangeOperation` fields
and `subject_detail` behavior:

```text
evolution_change_operation.<i>.changeset_id
evolution_change_operation.<i>.changeset_operation_type
evolution_change_operation.<i>.subject_family
evolution_change_operation.<i>.subject_object_id
evolution_change_operation.<i>.subject_statement_json
```

Operations are deduplicated by `operation_id` when both the causal-anchor path
and the direct Entity-subject path can see the same operation. `why` reports
`deferred_relation_family.0=evolution` when either causal-anchor ChangeSets or
direct evolution operations are present.

Evidence and knowledge exposure subjects keep their existing behavior; this
slice only adds the direct Entity-subject path.

## Non-Goals

- No Store schema change.
- No mutation semantics change.
- No CLI command or flag surface change.
- No initial Entity creation timeline.
- No relation-subject traversal.
- No ChangeSet endpoint.
- No multi-hop or full evolution graph traversal.
- No broader causal traversal.
- No context or Resource resolver behavior change.
- No transcript parsing, LLM extraction, ranking, or Agent orchestration.
- No release-candidate, release, Push, tag, deployment, remote, or V2 action.

## Evidence

Focused pre-change tests failed as expected:

```text
cargo test -p workvcs-core --test why_evolution_operation_projection_phase4ng why_changed_entity_projects_own_direct_evolution_operation --quiet
left: 0
right: 1

cargo test -p workvcs-cli cli_supersedes_decision_record_atomically --quiet
why evolution change operations 0 does not match expected 1
```

Focused validation passed after implementation:

```text
cargo fmt --all
cargo test -p workvcs-core --test why_evolution_operation_projection_phase4ng
cargo test -p workvcs-cli cli_supersedes_decision_record_atomically --quiet
```

The old-main/new-binary dogfood run proved the behavior change on one Store:

```text
old_prior_evolution_change_operations=0
new_prior_evolution_change_operations=1
new_prior_evolution_match_expected=true
new_prior_operation_type=record.decision.supersede
new_prior_subject_family=entity
new_prior_subject_statement_json="Use optimistic writes"
new_prior_deferred_relation_family_0=evolution
new_finding_causal_anchor_changesets=1
new_finding_evolution_change_operations=3
```

Detailed evidence is recorded in
`docs/provenance/phase-4nj-why-subject-evolution-projection.md`.

## Consequences

`workvcs why --entity <changed entity>` can now explain a queried Entity's own
direct first-parent evolution operation even when that Entity was not the
causal anchor for the ChangeSet.

The Context resolver, packets, and `why` explanations release gate remains
`Partial` and blocking. Relation-subject traversal, multi-hop/full evolution
graph traversal, broader causal traversal, broader context/Resource resolver
maturity, release-candidate validation, and release authorization remain open.
