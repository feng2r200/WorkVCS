# Typed Relationships

WorkVCS uses one typed relation graph across Work and Cognition entities. The
graph supports deterministic `why`, readiness, context, evolution, and merge
behavior without parsing natural language.

## Canonical relation vocabulary

| Category | Relation | Canonical direction | Deterministic meaning |
|---|---|---|---|
| Structural | `contains` | container -> child | Owns hierarchical scope or decomposition |
| Structural | `references` | referrer -> target | Uses an entity without changing its home scope |
| Scheduling | `depends_on` | dependent -> prerequisite | The source is not runnable until the target satisfies dependency rules |
| Scheduling | `ordered_before` | earlier -> later | Preferred sibling sequence; does not create a dependency |
| Evolution | `derived_from` | new/result -> source | The source explains the origin of the result |
| Evolution | `supersedes` | replacement -> prior | The replacement becomes current while preserving the prior object |
| Epistemic | `supports` | finding/claim -> target cognition | Expresses an epistemic claim that the source strengthens the target |
| Epistemic | `contradicts` | claim -> target | Explicitly identifies incompatible knowledge or evidence |
| Epistemic | `validates` | finding/evidence -> assumption/knowledge | Confirms the target under the recorded scope |
| Epistemic | `invalidates` | finding/evidence -> assumption/knowledge | Rejects the target under the recorded scope |
| Verification | `verifies` | verification -> criterion/claim | Records a structured verification result for the target |
| Verification | `evidenced_by` | semantic object -> evidence | Attaches immutable source material without duplicating an epistemic edge |

`supports` relates two semantic assertions; `evidenced_by` attaches an
immutable Evidence object to a semantic object. They are not inverse forms of
one edge. A Verification therefore `verifies` an Acceptance Criterion and is
`evidenced_by` captured Evidence; a Finding may independently `support` a
Decision.

Only the canonical direction is stored. Reverse views such as `blocks`,
`contained_by`, or `superseded_by` are projections and must not be stored as a
second independent edge.

## Semantic aliases and causal traversal

Human explanations may use verbs such as `based_on`, `produced_by`, or
`caused_by`. V1 must map an accepted semantic operation to the canonical edge
set rather than silently add new core algorithm semantics. For example:

```text
supersede T-18 with T-21 because F-17

=> T-21 supersedes T-18
=> T-21 derived_from F-17
```

Likewise, invalidating an Assumption because of a Finding creates the canonical
`Finding invalidates Assumption` edge. `why T-21` traverses the confirmed
evolutionary and epistemic neighborhood and may render friendlier causal
language without changing the stored graph.

## Generic escape hatch

V1 provides:

```text
related_to
label: <custom meaning>
reason: <required explanation>
```

Custom relationships are preserved and queryable but do not affect
deterministic readiness, merge, context ranking, or other core algorithms until
their semantics become a separately confirmed canonical relation.

## Relation creation rules

- Semantic operations such as split, supersede, invalidate, promote, verify,
  adopt, and reparent create or update the required canonical edges
  automatically.
- The Agent may add an explicit relation when no semantic operation represents
  the intended fact.
- One versioned semantic operation commits all entity and relation changes in
  one atomic ChangeSet. A runtime-only operation does not create versioned
  relations or an empty WorkStateCommit.
- Major transitions require rationale text, a `because=<EntityRef>` causal
  anchor, or both. This applies to supersession, invalidation, restore, merge
  resolution, cancellation of active work, and Goal achievement/abandonment.
- Ordinary creation and non-semantic edits such as correcting a title do not
  require invented rationale.

## Structural and scheduling independence

Containment, order, dependency, and priority are four orthogonal dimensions:

```text
contains       where work was decomposed
ordered_before preferred sibling sequence
depends_on     execution prerequisite
priority       value or urgency field
```

`T-1 ordered_before T-2` does not imply `T-2 depends_on T-1`. A Task's
containment parent does not determine its priority. Plan and Task children may
be mixed within one ordered container.

## Cross-Workspace boundary

Canonical Work Graph relations never connect Goal, Plan, or Task entities
across Workspaces in V1. Cross-Workspace reuse publishes Knowledge to a
Knowledge Space with source provenance. A Session Context Set may consult
multiple Workspaces temporarily, but that does not create a permanent Work
Graph relation.

## Merge and context participation

- Structural, scheduling, evolution, and explicit epistemic edges participate
  in Work-State branch, diff, merge, and restore.
- Runtime Claim and Focus associations do not participate.
- Sibling Branch state is excluded from current context unless explicitly
  queried or merged.
- Superseded or invalidated objects are excluded from default context unless a
  direct causal path makes a concise summary necessary to explain current
  state.
- Knowledge is not considered conflicting merely because two statements share
  a topic. Conflict requires an explicit canonical relation or a separately
  confirmed deterministic rule.
