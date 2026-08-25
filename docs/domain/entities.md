# Domain Entities

## State layers

WorkVCS separates four state layers. These layers classify mutable and
immutable state surfaces; scope containers, source-state associations, and
version-control refs surround those surfaces and follow the detailed boundary
rules below.

| Layer | Contents | Branch / restore / merge behavior |
|---|---|---|
| Versioned Work State | Goal, Plan, Task, Decision, Knowledge, Record, Acceptance Criterion, typed relations | Participates |
| Runtime Coordination | active Session state, Claim, Focus, merge-in-progress | Does not participate |
| Immutable Provenance and Version History | Event, Session timeline, Verification, Evidence, ChangeSet, WorkStateCommit | Is retained, not restored as Runtime Coordination |
| Derived Projection | context, next, why, ready, progress, diff | Recomputed |

Store, Workspace, and Knowledge Space define scope or ownership.
Work Branch refs select heads in immutable version history. They are not
versioned entities inside their own Work State.

## Scope containers

### Store

**Purpose:** A self-contained data and portability boundary for WorkVCS
data.

**Owned state:** Workspaces, Knowledge Spaces, Session provenance, structured
metadata, and immutable objects.

**Relations:** Contains Workspaces and Knowledge Spaces. It is independent of
any one project directory or Git repository.

**Version behavior:** A Store contains Work-State DAGs but is not itself a Work
Branch.

### Workspace

**Purpose:** The logical boundary within which a Work State can be branched,
diffed, merged, and restored as one unit.

**Owned state:** Workspace-local Goal, Plan, Task, Decision, Knowledge, Record,
Acceptance Criterion, and typed relation state.

**Relations:** May be associated with multiple repositories or directories;
multiple Workspaces may refer to different parts of one monorepo. A Workspace
may also be non-Git.

**Version behavior:** Every versioned mutation targets exactly one Workspace
and Work Branch. A Workspace is not a directory, repository, Store, or Session.

### Knowledge Space

**Purpose:** A long-lived knowledge-sharing boundary above individual
Workspaces.

**Owned state:** Published Knowledge statements and their provenance.

**Relations:** Workspaces publish Knowledge to and subscribe to a Knowledge
Space. Goal, Plan, and Task graphs never cross Workspace boundaries in V1.

**Version behavior:** Publication preserves the originating Workspace,
Decision, Verification, and WorkStateCommit lineage. It does not transfer
execution ownership. V1 requires publish/read semantics, but no independent
Knowledge Space DAG or concurrency model is confirmed yet; that mechanism
requires an explicit confirmed decision before implementation and may then be
recorded as an ADR under current repository policy.

## Versioned Work State

### Goal

**Purpose:** States what the work ultimately intends to make true.

**Lifecycle:** A Goal may be created before work or discovered after Tasks and
Plans already exist. Achievement or abandonment is explicit.

**Owned state:** Description, state, optional SubGoals, rationale for terminal
transitions, and references to Plans or other work.

**Relations:** May contain or reference Plans. Existing Plans and Tasks may be
attached later without changing their identity.

**Version behavior:** Goal state participates in branch, diff, merge, and
restore. Descendant completion may generate a readiness hint but never
automatically marks the Goal achieved.

**Example:** A Workspace-level investigation later yields the Goal “guarantee
deterministic state transitions”; the prior Task keeps its identity and gains a
confirmed path to the new Goal.

### Plan

**Purpose:** Describes a strategy, hypothesis, or decomposition for reaching a
Goal or solving a Workspace-level problem.

**Lifecycle:** A Plan may exist without a Goal, contain SubPlans and Tasks in
mixed order, evolve in place for ordinary edits, and be superseded when its
core strategy changes. A Goal may have multiple active Plans on one Work
Branch. Completion is explicit.

**Owned state:** Description, constraints, strategy, Plan-scoped Assumptions,
status, ordered children, Task-graph references, and optional completion
rationale.

**Relations:** May be contained by a Goal or another Plan; may contain or
reference Plans and Tasks. A Task may be referenced by multiple Plans.

**Version behavior:** Plan state and structure participate in Work-State
versioning. A Plan is not claimed or executed directly.

**Example:** A persistence Plan contains a benchmark Task, a migration SubPlan,
and a compatibility Task as mixed siblings.

### Task

**Purpose:** Represents the concrete unit an Agent executes.

**Lifecycle:** `pending`, `in_progress`, `blocked`, `done`, `failed`,
`cancelled`, or `superseded`. Execution status and outcome are separate. A Task
may be decomposed into SubTasks while remaining independently executable.
After terminal state it remains queryable and may receive later Findings,
Decisions, Knowledge, or links to newly discovered Tasks.

**Owned state:** Description, execution status, open-semantic outcome,
priority, stable local Acceptance Criteria, and optional child ordering.

**Relations:** Its home scope defaults to a Plan but may explicitly be the
Workspace. It may be referenced by other Plans, may contain SubTasks, and may
carry dependency, order, evolution, causal, and verification relations.

**Version behavior:** Versioned Task state participates in branch and merge.
Current Claims and Focus do not. A completed evaluation whose outcome rejects
an option is still `done`, not `failed`.

**Example:** “Evaluate SQLite” may finish with `status=done` and
`outcome=unsuitable for multi-process writers`.

### Acceptance Criterion

**Purpose:** Gives a Task a stable, verifiable success condition.

**Lifecycle:** Optional for simple Tasks and recommended for meaningful work.
It may be revised without losing identity.

**Owned state:** Stable Task-local identity, statement, required/optional
classification, and current verification projection.

**Relations:** Referenced by Verification, Evidence, and Decision objects.

**Version behavior:** It is versioned with its owning Task but uses a stable
local reference such as `T-18/AC-2`.

**Completion rule:** Criteria are optional. If they exist, every mandatory
criterion must have Verification before WorkVCS may automatically mark the
Task done.

**Example:** Editing the wording of `T-18/AC-2` does not break an existing
Verification reference.

### Decision

**Purpose:** Records a promoted material choice, its rationale, and what
evidence supports the current choice.

**Lifecycle:** An ordinary decision begins as `Record(kind=decision)`. An
important one may be explicitly promoted to a Decision. A Decision may be
active, superseded, or otherwise explicitly retired; later choices supersede
rather than rewrite it.

**Owned state:** Context, options, choice, rationale, consequences, optional
exclusive `scope` and `subject`, status, and provenance references.

**Relations:** Commonly based on Findings or Evidence, may supersede another
Decision, and may cause new Plans or Tasks.

**Version behavior:** Decision state participates in branch and merge.
Different active choices with the same exclusive scope and subject trigger a
deterministic merge review even when their IDs differ.

**Example:** `scope=plan:P-10`, `subject=persistence.backend`,
`choice=postgres`.

### Knowledge

**Purpose:** Preserves a reusable statement learned through work, independently
of whether the originating strategy was selected.

**Lifecycle:** Knowledge may be active, superseded, invalidated, or published
to a Knowledge Space. Natural-language topic similarity alone does not change
its state.

**Owned state:** Statement, scope, state, provenance, and optional publication
targets. V1 scopes include Workspace, Goal, Plan, Task, and path/module/tag.

**Relations:** May be supported, contradicted, validated, invalidated, derived
from, or superseded. It may be published across Workspaces; Tasks are not.

**Version behavior:** Workspace-scoped Knowledge participates in that
Workspace's Work-State DAG. Published Knowledge preserves source lineage.

**Example:** “SQLite is sufficient under serialized writes” may remain valid
knowledge even when a separate Decision selects PostgreSQL for a
multi-process workload.

### Record

**Purpose:** Represents explicit semantic observations and cognition that do
not need separate top-level behavior.

**Lifecycle:** Confirmed Record kinds include Finding, Assumption, Attempt,
ordinary decision, and Handoff. An Attempt may be `running`, `succeeded`,
`failed`, or `inconclusive`, and may also be recorded in one operation with its
approach and result. V1 records are created explicitly by an Agent semantic
operation, not inferred from a transcript. An important ordinary decision can
be promoted to a Decision without erasing its origin.

Whether `Question`, `Risk`, `Blocker`, `Review`, or `Note` should be distinct
V1 Record kinds is Open; no current confirmed requirement makes them distinct
V1 kinds.

**Owned state:** Kind, statement, scope, lifecycle fields appropriate to the
kind, and provenance.

**Relations:** Participates in the typed causal graph. An Attempt may produce a
Finding; a Finding may invalidate an Assumption; a Handoff supplements an
automatic Session diff.

**Version behavior:** Semantic Records in a Workspace are versioned. Their
creation and changes also leave immutable Events.

**Example:** A long Attempt moves from `running` to `failed`; a small Attempt
may be recorded once with its approach and result.

## Runtime Coordination

### Session

**Purpose:** Identifies one Agent execution provenance boundary and its current
coordination state.

**Lifecycle:** Starts, changes focus or active Workspace/Branch explicitly,
and ends with a deterministic Session diff plus an optional semantic Handoff.
When claimed or in-progress work remains, Session end recommends a Handoff but
does not require one. Normal end releases claims. After abnormal exit the
Session is marked `potentially_stale` and its claims remain until explicit
release or takeover.

**Owned state:** Context Set, active Workspace, active Branch, primary Focus,
current Claims, Agent identity, and activity metadata.

**Relations:** May read multiple Workspaces and Knowledge Spaces. Every
mutation still targets one active Workspace and Branch.

**Version behavior:** Current Session state does not branch, merge, or restore.
Its timeline is immutable provenance and remains readable by later Sessions.

**Example:** A Session consults Workspaces A and B but mutates only A/main until
an explicit Workspace switch event.

### Focus

**Purpose:** Defines what the Session is actively continuing and through which
context path.

**Owned state:** Entity reference plus a unique containment/reference path.

**Relations:** Usually points to a Task. If multiple Plan paths are possible,
the Agent must choose a path rather than letting the resolver guess.

**Version behavior:** Runtime-only; every change produces immutable
provenance.

### Claim

**Purpose:** Coordinates concurrent work without becoming an authorization
lock or a Work-State version.

**Lifecycle:** Exclusive by default, optionally shared, released on Branch
switch by default, released on normal Session end, and explicitly taken over
when a prior Session is stale. A takeover exposes the prior claimant and last
activity and emits provenance.

**Owned state:** Session, Task, Work Branch, mode, and activity metadata.

**Relations:** Branch-scoped. A Session without the claim may still read, add
Findings or Evidence, and link Decisions. Under an active exclusive claim,
terminal or structural mutation by another Session is rejected unless the
claim is explicitly transferred or the mutation carries force provenance.
Multiple shared claimants may add Findings, Evidence, Verification, and other
non-destructive updates; a terminal or structural mutation requires a unique
claimant or explicit force provenance.

**Version behavior:** Runtime-only. Claim creation, release, and takeover are
immutable Events.

## Provenance and version primitives

### Evidence and Verification

**Purpose:** Evidence is the immutable support material; Verification is the
structured judgment that an Acceptance Criterion or claim passed, failed, or
remained inconclusive.

**Lifecycle:** Evidence is captured or referenced, hashed, and retained under
its policy. Verification records a result and the Evidence it used.

**Owned state:** Evidence owns digest, locator/object reference, and capture
metadata. Verification owns target, result, method, time, and evidence links.

**Relations:** Verification `verifies` an Acceptance Criterion and is
`evidenced_by` immutable Evidence. A Finding or other semantic claim may
independently `support` a Decision or Knowledge statement.

**Version behavior:** Evidence and Verification objects are immutable
provenance. Their references are versioned relations; the current verification
view shown on an Acceptance Criterion is a derived projection.

**Example:** A command wrapper captures command, cwd, start/end, duration, exit
status, output artifact or digest, applicable Git SHA, and result for
`T-18/AC-2`.

### ChangeSet, Event, and WorkStateCommit

**Purpose:** A ChangeSet is one atomic semantic mutation; Events are its
immutable facts; a WorkStateCommit places the resulting state in the DAG.

**Lifecycle:** One accepted versioned semantic operation creates one ChangeSet,
one or more Events, and exactly one WorkStateCommit. Failure rolls back the
complete ChangeSet. A pure Runtime Coordination operation updates runtime state
atomically and emits provenance Events without creating a WorkStateCommit.

**Owned state:** ChangeSet metadata and semantic intent; Event type and
payload; commit parent(s), ChangeSet reference, authoring Session, and time.

**Relations:** A Workspace genesis commit has zero parents, a normal commit has
one parent, and a merge commit has two.

**Version behavior:** All three are immutable provenance/version primitives.
The exact persistent representation and reconstruction strategy are not fixed
by this baseline.
