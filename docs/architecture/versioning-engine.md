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
        +-- deterministic Change Operations
        +-- before/after EntityVersion and RelationVersion bindings
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
- a digest of its canonical resulting state for replay/checkpoint/integrity
  validation;
- target Workspace plus the originating Branch, time, and provenance of the
  accepted operation, without making the commit Branch-owned;
- the originating Session when the operation occurs within one, without
  requiring every valid WorkStateCommit source to be a Session.

```text
C1 -- C2 -- C3 -- C4  main
       \
        C5 -- C6      experiment

merge:

C4 --------- C9
 \           /
  C6 -------
```

Commit identity remains distinct from the resulting-state digest. V1 logical
IDs use UUIDv7/16-byte BLOB, digests use BLAKE3-256/32-byte BLOB, and canonical
structured payload is JSON text produced by the WorkVCS serializer. Canonical
reconstruction is fixed at the logical level: Commit + ChangeSet +
schema-versioned deterministic Change Operations are replay truth; Events are
provenance. Detailed storage is defined in
[Versioned-State Persistence Model](persistence-model.md),
[Logical Schema Boundaries](logical-schema-boundaries.md), and
[Physical Schema v0.1 Contract](physical-schema-v0.1.md).

## Branch semantics

A Work Branch may start at any historical WorkStateCommit. It represents a
divergent work or cognition route and may exist without a Git Branch. Git
branch, commit, or worktree associations are optional and do not define Work
Branch identity.

Branch creation is an O(1) ref operation and does not copy current Entity or
Relation state. Materialized current projections are rebuildable caches; only
HOT Branches need remain materialized.

Branch HEAD is the only canonical current-state pointer. A current projection
is complete only when it names HEAD as its projected Commit and its state digest
has been validated. Ordinary HOT-Branch mutation advances immutable history,
HEAD by expected-head compare-and-swap, and the complete projection atomically.

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
  -> store provisional runtime resolutions
  -> merge continue -> one atomic ChangeSet + two-parent commit
  -> or merge abort -> target Work State was never changed
```

Merge-in-progress is Runtime Coordination and may continue across commands or
Sessions. The attempt, classifications, and resolutions remain provenance.
One target Workspace/Branch has at most one active merge. Merge captures base,
target head, and source head but locks neither Branch. Continue rejects moved
source or target heads and requires restart or a later separately designed
recomputation path. Abort marks the attempt aborted and preserves provenance;
there is no target state to restore because provisional choices never became
Work State.

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
produce a structured mini-ChangeSet and create a third valid state or new
Entity rather than merely selecting one field. Until continue, every
resolution remains provisional Runtime Coordination. A resolution is a major
semantic transition and therefore records rationale or a causal entity
reference.

The resulting merge ChangeSet transforms primary parent=target into merged
state; secondary parent=source preserves ancestry. Historical replay applies
that recorded ChangeSet and never reruns the merge algorithm.

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
and requires rationale/provenance. It creates a new single-parent
WorkStateCommit from the current Branch head whose resulting Work State matches
the selected historical state. It does not move the current Branch reference
backward or delete the intervening history. Creating a new Branch at a
historical WorkStateCommit is the separately confirmed historical-branching
operation, not restore.

## Query semantics

The versioning and relation graphs support four distinct queries:

- `history`: which mutations and commits actually occurred;
- `show-at`: which Work State existed at a selected historical commit, without
  mutating the current Branch;
- `why`: which structural, evolution, epistemic, and verification paths explain
  the current object;
- `context`: which current, relevant, path-sensitive items are needed to
  continue work within a profile and budget.

`diff` compares Work State at commits or Branch heads. `next` resolves runnable
Tasks through this deterministic precedence:

```text
active Workspace and Work Branch
  -> active scope and Plan path
  -> executable Task descendants
  -> dependency readiness
  -> Task lifecycle eligibility
  -> priority
  -> explicit manual order
  -> Session and Claim coordination
```

Priority and explicit manual order are separate scheduling dimensions;
priority is evaluated first. Manual order cannot make a dependency-blocked or
lifecycle-ineligible Task runnable. ADR-0175 fixes the stable tie-breaker among
otherwise equal `next` candidates as ascending Task EntityId byte order. An
atomic “claim next” operation selects, claims, focuses, and returns context
without a race between separate read and claim steps.

Claim-next does not imply TaskStart. For a Task/Branch, active Claims are none,
exactly one exclusive, or one-or-more shared; exclusive and shared cannot
coexist.

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

If a focused entity has multiple valid context paths, the resolver requires an
explicit path or returns candidate paths; it never guesses.

Profiles define eligible categories:

- `brief`: Goal/Plan path, Task, Acceptance Criteria, blocker, dependency,
  active Decision, and critical previous failure;
- `normal`: `brief` plus Finding, Assumption, Attempt, Knowledge, and relevant
  Session/Handoff;
- `full`: `normal` plus deeper causal ancestry, inactive related cognition,
  and more provenance.

Budget controls the amount retained within the profile. The confirmed trimming
priority is:

```text
P0 current Task / Acceptance Criteria / blocker
P1 current Goal / Plan path
P2 dependencies / readiness
P3 direct causal chain
P4 active Decisions / Assumptions
P5 failed Attempts
P6 Findings
P7 scoped Knowledge
P8 relevant Handoff
P9 older provenance
```

The resolver removes complete low-priority items, never arbitrary text
fragments, and reports omitted categories and counts. Inactive cognition is
omitted unless a direct causal path requires a concise summary to explain
current state.

## Verification applicability and source-state drift

Verification result is an immutable historical judgment. Current applicability
is a branch-sensitive Derived Projection over declared Resource and Work-State
Basis:

```text
any stale        -> stale
else any unknown -> unknown
else             -> applicable
```

Resource Basis identifies a stable logical Resource, scope, and baseline
ResourceObservation/fingerprint. Work-State Basis identifies the verifying
WorkStateCommit and explicit semantic dependencies. A Resource Adapter, not
Core, supplies deterministic scoped fingerprint and difference. Git-backed
Verification records the actual verified working state, including relevant
uncommitted changes, rather than HEAD alone.

Observation or drift alone creates no WorkStateCommit. Exact path matching,
adapter implementations, and persisted capture points outside Verification or
explicit snapshot remain Open. See
[Verification and Resource Drift](verification-and-resource-drift.md).

## Failure guarantees

- A failed semantic operation leaves no partial entity, relation, Event, or
  WorkStateCommit state.
- A stale expected base never silently overwrites concurrent changes.
- A merge cannot continue with unresolved `CONFLICT` or `REVIEW` items.
- A merge cannot continue after its captured source or target head moves.
- A restore cannot resurrect Runtime Coordination.
- Store import cannot resurrect active Runtime Coordination or overwrite
  divergent same-Store refs by last-write-wins.
- A derived projection cannot certify history if its provenance is missing.
- Errors are concise and actionable, exposing the relevant current state and a
  safe next action in an Agent-readable form.
