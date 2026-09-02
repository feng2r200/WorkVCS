# ADR-0480: Phase 4NM Why Knowledge Relation Endpoint Evolution

Status: Accepted
Date: 2026-09-02

## Context

ADR-0479 made direct Record-to-Record relation remove and restore operations
visible as endpoint evolution from `workvcs why --entity <Record endpoint>`.
The same dogfood pattern exposed the next concrete relation-subject gap:
Record-to-Knowledge and Knowledge-to-Knowledge relation removals changed a
queried Knowledge endpoint's neighborhood, but the ChangeOperation subject was
the Relation, not the Knowledge Entity.

At the removal commit, `why --entity <Knowledge endpoint>` reported
`relation_edges=0`, `causal_anchor_changesets=0`,
`evolution_change_operations=0`, and `deferred_relation_families=0`. A script
asserting `--expected-evolution-change-operations 1` failed with
`query_invalid`.

## Decision

For Entity-subject `why` queries, first-parent direct relation-subject evolution
projection now recognizes these operation-local remove and restore paths:

```text
record.relation.remove / record.relation.restore
  - Record-to-Record relation versions
  - Record-to-Knowledge relation versions

knowledge.relation.remove / knowledge.relation.restore
  - Knowledge-to-Knowledge supersedes relation versions
```

The projection continues to use `relation_membership_change` to find the
operation-local relation version: after version first for restore, otherwise
before version for remove. It then loads the relation version by shape and
projects the operation only when the queried Entity is one of that relation's
endpoints.

The CLI command and flag surface is unchanged. Existing
`--expected-evolution-change-operations` assertions now work for direct
Record-to-Knowledge and Knowledge-to-Knowledge relation remove/restore endpoint
slices.

## Non-Goals

- No Store schema change.
- No mutation semantics change.
- No CLI command or flag surface change.
- No relation create timeline.
- No containment, verification, evidence, scope-link, Knowledge Exposure, or
  ChangeSet endpoint relation-subject slice.
- No full relation-subject traversal beyond direct Record-to-Record,
  Record-to-Knowledge, and Knowledge-to-Knowledge remove/restore endpoint
  projection.
- No multi-hop or full evolution graph traversal.
- No broader causal traversal.
- No broader context or Resource resolver behavior change.
- No transcript parsing, LLM extraction, ranking, or Agent orchestration.
- No release-candidate, release, Push, tag, deployment, remote, or V2 action.

## Evidence

Pre-change CLI probes reproduced both Knowledge endpoint gaps:

```text
rk_relation_edges=0
rk_causal_anchor_changesets=0
rk_evolution_change_operations=0
rk_deferred_relation_families=0
rk_expected_status=1

kk_relation_edges=0
kk_causal_anchor_changesets=0
kk_evolution_change_operations=0
kk_deferred_relation_families=0
kk_expected_status=1
```

Focused tests failed before implementation and passed after implementation:

```text
cargo test -p workvcs-core --test why_knowledge_relation_endpoint_evolution_phase4nm --quiet
4 passed; 0 failed

cargo test -p workvcs-cli cli_why_projects_removed_ --quiet
3 passed; 0 failed

cargo test -p workvcs-core --test why_record_relations_phase3ax why_projects_ --quiet
2 passed; 0 failed
```

The post-change CLI dogfood run proved the behavior through public commands:

```text
rk_remove_relation_edges=0
rk_remove_evolution_change_operations=1
rk_remove_subject_relation_kind=record_supports
rk_restore_relation_edges=1
rk_restore_evolution_change_operations=2

kk_remove_relation_edges=0
kk_remove_evolution_change_operations=1
kk_remove_subject_relation_kind=knowledge_supersedes
kk_restore_relation_edges=1
kk_restore_evolution_change_operations=2
```

Detailed evidence is recorded in
`docs/provenance/phase-4nm-why-knowledge-relation-endpoint-evolution.md`.

## Consequences

`workvcs why --entity <Knowledge endpoint>` can now explain a direct
Record-to-Knowledge or Knowledge-to-Knowledge relation removal or restore that
changed that endpoint's neighborhood, even when the relation is absent from
current `relation_edges`.

The Context resolver, packets, and `why` explanations release gate remains
`Partial` and blocking. Full relation-subject traversal beyond the direct
remove/restore endpoint slices, multi-hop/full evolution graph traversal,
broader causal traversal, broader context/Resource resolver maturity,
release-candidate validation, and release authorization remain open.
