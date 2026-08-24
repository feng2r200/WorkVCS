# Versioning Engine

## Versioned object

The engine versions a Workspace's Work State, not source files,
database pages, a transcript, Runtime Coordination, or model internals.

```text
Versioned Semantic Operation
        |
        v
Atomic ChangeSet
        |
        +-- 1..N immutable Events
        +-- canonical entity/relation changes
        `-- exactly 1 WorkStateCommit
```

The versioned operation either commits completely or exposes no partial state. A batch
applies the same rule to a larger semantic unit such as creating one Plan,
three SubPlans, Tasks, Acceptance Criteria, order, and dependencies.

A pure Runtime Coordination operation follows a different write path:

```text
Runtime Coordination Operation
        |
        +-- atomic runtime transition
        `-- 1..N immutable provenance Events
```

It does not create an empty WorkStateCommit. Read-only queries create neither
state transition nor commit. A composite operation declares all state surfaces
up front and is atomic across them; only its versioned Work-State component
creates one ChangeSet and WorkStateCommit.

## WorkStateCommit DAG

A WorkStateCommit is immutable and records:

- zero parents for a Workspace genesis commit, one parent for an ordinary
  mutation, or two parents for a completed merge;
- its ChangeSet and semantic operation metadata;
- authoring Session and active Workspace/Branch;
- a state fingerprint and time;
- optional references to relevant Resource Anchors.

```text
C1 -- C2 -- C3 -- C4  main
       \
        C5 -- C6      experiment

merge:

C4 --------- C9
 \           /
  C6 -------
```

The exact hash algorithm and serialization format remain implementation
choices. Commit identity is content-based; mutable entity identity is stable
and independent of content changes.

## Deltas, projections, and checkpoints

The engine does not snapshot the complete Store or Workspace on every
mutation. It stores ChangeSet deltas and maintains a materialized current
projection for ordinary reads. Periodic internal checkpoints bound historical
replay cost.

```text
historical state = nearest checkpoint + ordered delta replay
current state    = validated materialized projection
```

Checkpoint creation and derived indexes never create new semantic truth. They
must be reproducible from immutable history.

## Branch semantics

A Work Branch may start at any historical WorkStateCommit. It represents a
divergent work or cognition route and may exist without a Git Branch. Git
branch, commit, or worktree associations are optional Resource Anchors.

Multiple Sessions may work on one Work Branch. Agent concurrency alone is not
a reason to create branches. Unmerged sibling Branch state is isolated from
the current Branch's context unless explicitly queried.

After merge, the source Branch remains present and its head does not move. It
may be marked merged, abandoned, or become active again through later commits.

## Optimistic concurrency

Every mutation supplies or derives an expected base:

```text
expected_base == branch HEAD
  -> apply ChangeSet and commit

expected_base != branch HEAD
  -> inspect intervening Changes
     -> disjoint: reapply deterministically to current HEAD
     -> overlapping but equivalent/compatible: deterministic reconciliation
     -> genuine conflict: reject, refresh, and resolve explicitly
```

This same-Branch reconciliation does not create a two-parent merge commit; it
is not a Branch merge. A rejection leaves the current branch unchanged.

Claims reduce duplicated work but do not replace optimistic concurrency.

## Persistent three-way merge

Branch merge is a persistent workflow:

```text
merge start
  -> compute base, target head, source head
  -> classify changes
  -> resolve or investigate
  -> merge continue -> two-parent commit
  -> or merge abort -> exact pre-merge target state
```

Merge-in-progress is Runtime Coordination and may continue across commands or
Sessions. The attempt, classifications, and resolutions remain provenance.
Abort removes the provisional effect from current state but does not erase that
the attempt occurred.

### Classification

- **AUTO:** disjoint or deterministically compatible change.
- **CONFLICT:** incompatible edits to the same structured state.
- **REVIEW:** structurally mergeable changes with a deterministic semantic
  condition that requires a choice.

V1 semantic review does not call an LLM. The central confirmed rule detects
different active choices for Decisions with the same exclusive `scope` and
`subject`. Other natural-language similarity does not create a hidden conflict.

### Resolution

Each unresolved item supports `ours`, `theirs`, or `custom`. `custom` may
produce a third valid state rather than editing one field. A resolution is a
major semantic transition and therefore records rationale or a causal entity
reference.

### Retaining unselected work

Merge distinguishes execution facts from current choices:

- Findings, Attempts, Verifications, Evidence, and completed evaluation Tasks
  are normally retained when useful;
- active Decision choice, Assumption state, Plan direction, active Knowledge,
  and future Task state may require resolution;
- an unselected strategy does not invalidate knowledge learned under its
  stated conditions;
- Knowledge conflict requires explicit semantic evidence, not a shared topic.

## Restore

Restore reconstructs a selected historical Versioned Work State without
restoring old Claims, Focus, active Sessions, or merge-in-progress state. It
never mutates the selected historical commit. The restore action is auditable
and requires rationale/provenance.

The final command-level choice between moving a new Branch reference and
creating a restorative commit is implementation planning; either form must
preserve the immutable DAG and the action's provenance.

## Query semantics

The versioning and relation graphs support three distinct queries:

- `history`: which mutations and commits actually occurred;
- `why`: which structural, evolution, epistemic, and verification paths explain
  the current object;
- `context`: which current, relevant, path-sensitive items are needed to
  continue work within a profile and budget.

`diff` compares Work State at commits or Branch heads. `next` derives runnable
Tasks from state, dependency, order, priority, and Claims. An atomic
“claim next” operation selects, claims, focuses, and returns context without a
race between separate read and claim steps.

## Context projection

Context resolution follows a deterministic pipeline:

```text
Focus entity and path
  -> active scope
  -> structural context
  -> execution constraints
  -> causal neighborhood
  -> scoped active cognition
  -> Session continuity
  -> profile filter
  -> whole-item budget trimming and omission summary
```

Without explicit focus, the anchor is the only claimed Task when unique;
otherwise the resolver returns a current Branch overview. It never silently
turns `context` into “select next work.”

Profiles `brief`, `normal`, and `full` define eligible categories. Budget
removes complete low-priority items, never arbitrary text fragments. Inactive
cognition is omitted unless a direct causal path requires a concise summary to
explain current state.

## Resource drift

WorkStateCommit and external source history are related but not lock-stepped.
A claim such as Verification or Task completion references the Resource Anchor
that makes it reviewable. Drift compares the current Resource observation with
that anchor and reports the exact basis. Changing a Task title does not require
an unnecessary Git snapshot.

## Failure guarantees

- A failed semantic operation leaves no partial entity, relation, Event, or
  WorkStateCommit state.
- A stale expected base never silently overwrites concurrent changes.
- A merge cannot continue with unresolved `CONFLICT` or `REVIEW` items.
- A restore cannot resurrect Runtime Coordination.
- A derived projection cannot certify history if its provenance is missing.
- Errors use stable machine-readable codes and include the relevant current
  state and safe next action.
