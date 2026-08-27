# Domain Entities

## State layers

WorkVCS separates four state layers. These layers classify mutable and
immutable state surfaces; scope containers, source-state associations, and
version-control refs surround those surfaces and follow the detailed boundary
rules below.

| Layer | Contents | Branch / restore / merge behavior |
|---|---|---|
| Versioned Work State | Goal, Plan, Task, Decision, Knowledge, Record, Verification, Acceptance Criterion, Verification Requirement, typed relations | Participates |
| Runtime Coordination | SessionRuntime, ClaimRuntime, Focus, MergeRuntime | Does not participate |
| Immutable Provenance and Version History | Event, Session/Claim/Merge occurrences, Evidence, ResourceObservation, ChangeSet, WorkStateCommit | Is retained, not restored as Runtime Coordination |
| Derived Projection | context, next, why, ready, progress, diff | Recomputed |

Store, Workspace, and Knowledge Space define scope or ownership.
Work Branch refs select heads in immutable version history. They are not
versioned entities inside their own Work State.

`ObjectIdentity` is a lower Store-local registry shared by addressable typed
families. It does not make every registered object an Entity. Store and
Workspace use their own container identities above that registry.

KnowledgeExposure history is an additional Store-local federation surface
outside any Workspace Work State: Exposure source bindings are immutable,
while current availability and source-stale status are projections or explicit
federation state. V1 does not give Knowledge Space its own Work-State DAG.

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

**Portability behavior:** Copy, move, export/import, backup, and restore retain
Store identity. An explicit fork creates a new Store identity with source-store
lineage. A fork preserves internal local IDs by default under the new Store
namespace. Import never blindly restores active Runtime Coordination.

### Workspace

**Purpose:** The logical boundary within which a Work State can be branched,
diffed, merged, and restored as one unit.

**Owned state:** Workspace-local Goal, Plan, Task, Decision, Knowledge, Record,
Verification, Acceptance Criterion, and typed relation state.

**Relations:** May be associated with multiple repositories or directories;
multiple Workspaces may refer to different parts of one monorepo. A Workspace
may also be non-Git. Workspace-to-Resource association is many-to-many
infrastructure state, not Work-State membership.

**Version behavior:** Every versioned mutation targets exactly one Workspace
and Work Branch. A Workspace is not a directory, repository, Store, or Session.

### Knowledge Space

**Purpose:** A long-lived knowledge-sharing boundary above individual
Workspaces.

**Owned state:** Immutable KnowledgeExposure bindings, append-only Exposure
history, current availability projection, and source provenance.

**Relations:** A Workspace may expose one specific immutable KnowledgeVersion
through a stable KnowledgeExposure. Another Workspace may consult that
Exposure without mutation or explicitly adopt it into new Workspace-local
Knowledge. Goal, Plan, and Task graphs never cross Workspace boundaries in V1.

**Version behavior:** An Exposure binds a source Knowledge identity and one
specific immutable KnowledgeVersion; it never follows `latest`. New source
versions use new Exposures that may coexist with older ones; replacement is a
new Exposure plus explicit withdrawal of the old. Exposure may leave current
availability, but its history is not deleted.
Source drift changes a derived warning/status rather than silently mutating
Exposure semantic state. V1 has no independent Knowledge Space DAG and no live
cross-Store federation.

### KnowledgeExposure

**Purpose:** Publishes one exact Workspace Knowledge version through one
Store-local Knowledge Space without copying or replacing the Knowledge.

**Owned state:** Stable Exposure identity, Knowledge Space, source Store and
Workspace identity, source Knowledge identity, source immutable version, and
publication provenance.

**Lifecycle:** The source binding is immutable. V1 semantic state is `active`
or `withdrawn`; replacement creates a new Exposure and explicitly withdraws
the old one while history remains. Relevant source change produces a derived
`current`, `stale`, `unknown`, or `unresolved` source status; it does not
automatically transition Exposure semantic state.

**Version behavior:** V1 uses append-only Exposure history and a current
availability projection, not a Knowledge Space DAG. Consulting creates no
Workspace mutation; adoption creates Workspace-local Knowledge with a
`derived_from` edge and retained source-version provenance.

## Versioned Work State

### Goal

**Purpose:** States what the work ultimately intends to make true.

**Lifecycle:** A Goal may be created before work or discovered after Tasks and
Plans already exist. Core states are `active`, `achieved`, and `abandoned`.
Achievement, abandonment, and reopening are explicit and carry required
provenance. Goal replacement is expressed through relation.

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

**Lifecycle:** A Plan may be `active`, `completed`, `abandoned`, or
`superseded`. It may exist without a Goal, contain SubPlans and Tasks in mixed
order, evolve in place for ordinary edits, and be superseded when its core
strategy changes. A Goal may have multiple active Plans on one Work Branch.
Completion, abandonment, and reopening are explicit; a superseded Plan cannot
use an ordinary reopen.

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
Decisions, Knowledge, or links to newly discovered Tasks. `blocked` is an
explicit non-dependency blocker; dependency blocking is derived readiness.
`done`, `failed`, and `cancelled` require an explicit rationale-bearing reopen
or retry to become non-terminal. `superseded` requires supersession-aware
resolution.

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
classification, and zero or more stable AC-local Verification Requirements.

**Derived state:** Current effective verification projection.

**Relations:** Referenced by Verification, Evidence, and Decision objects. A
Verification targets a Requirement when one exists; without Requirements it
may target the AC directly.

**Version behavior:** It is versioned with its owning Task but uses a stable
local reference such as `T-18/AC-2`.

**Completion rule:** Criteria are optional. A criterion's effective projection
is `unverified`, `verified`, `failed`, `stale`, or `conflicted`. Every mandatory
criterion must be `verified` before Task completion; an ordinary coordination
force cannot bypass this semantic gate.

**Example:** Editing the wording of `T-18/AC-2` does not break an existing
Verification reference.

### Verification Requirement

**Purpose:** Gives one Acceptance Criterion a stable, explicit statement of
what must be proven when the criterion needs more than one required coverage
item.

**Owned state:** Stable AC-local identity and the intent to be proven. It does
not prescribe a command, test framework, or execution method.

**Relations:** A Verification targets one Requirement when the owning AC has
Requirements. Multiple single-target Verifications may share one immutable
Evidence object.

**Version behavior:** Requirement identity remains historically referential
when its statement is revised or when current AC structure changes. Revision,
retirement, or supersession cannot erase a referenced historical Requirement.
This domain contract does not decide whether the persistent representation is
a separate table, embedded versioned state, or another SQLite shape.

### Decision

**Purpose:** Records a promoted material choice, its rationale, and what
evidence supports the current choice.

**Lifecycle:** An ordinary decision begins as `Record(kind=decision)`. An
important one may be explicitly promoted to a Decision. A Decision is
`active`, `superseded`, or `withdrawn`; later choices supersede rather than
rewrite it. An old choice can return only through a new Decision that
supersedes the current one.

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

**Lifecycle:** Knowledge may be active, superseded, invalidated, or explicitly
made available through a KnowledgeExposure that binds one immutable version.
Natural-language topic similarity alone does not change its state.

**Owned state:** Statement, scope, state, and provenance. V1 scopes include
Workspace, Goal, Plan, Task, and path/module/tag. Cross-Workspace availability
uses Store-local KnowledgeExposure; the physical schema and access protocol
remain unfixed.

**Relations:** May be supported, contradicted, validated, invalidated, derived
from, or superseded. Consulting a KnowledgeExposure does not copy Knowledge.
Explicit adoption creates new Workspace-local Knowledge `derived_from` the
Exposure with source-version provenance; Tasks are never shared this way.

**Version behavior:** Workspace-scoped Knowledge participates in that
Workspace's Work-State DAG. Knowledge reused through a Knowledge Space
preserves source lineage.

**Example:** “SQLite is sufficient under serialized writes” may remain valid
knowledge even when a separate Decision selects PostgreSQL for a
multi-process workload.

### Record

**Purpose:** Represents explicit semantic observations and cognition that do
not need separate top-level behavior.

**Lifecycle:** Confirmed Record kinds include Finding, Assumption, Question,
Attempt, ordinary decision, Risk, and Handoff. An Attempt may be `running`,
`succeeded`, `failed`, or `inconclusive`, and may also be recorded in one
operation with its approach and result. V1 records are created explicitly by
an Agent semantic operation, not inferred from a transcript. An important
ordinary decision can be promoted to a Decision without erasing its origin.

Whether `Blocker`, `Review`, or `Note` should be distinct V1 Record kinds is
Open; no current confirmed requirement makes them distinct V1 kinds.

An Assumption may move `unverified -> validated`, `unverified -> invalidated`,
or `validated -> invalidated`; an invalidated Assumption is not ordinarily
revalidated. A terminal Attempt is never reopened; another try creates another
Attempt. One Session may create zero or more Handoff Records, but each Handoff
belongs to exactly one Workspace and Work Branch and may bind a Focus/context
path.

**Owned state:** Kind, statement, scope, lifecycle fields appropriate to the
kind, and provenance.

**Relations:** Participates in the typed causal graph. An Attempt may produce a
Finding; a Finding may invalidate an Assumption; a Handoff supplements an
automatic Session diff.

**Version behavior:** Semantic Records in a Workspace are versioned. Their
creation and changes also leave immutable Events.

**Example:** A long Attempt moves from `running` to `failed`; a small Attempt
may be recorded once with its approach and result.

### Verification

**Purpose:** Represents the structured judgment that an Acceptance Criterion
or another claim passed, failed, or remained inconclusive, using explicit
Evidence references.

**Lifecycle:** A Verification is one immutable single-target judgment created
explicitly or by a deterministic command wrapper. Re-verification creates
another judgment; it does not update or automatically supersede the prior
instance. V1 does not infer a Verification from transcript text.

**Owned state:** One AC or Verification Requirement target, immutable result,
open structured method descriptor, structured Verification Basis, Evidence
references, and provenance sufficient to explain the judgment. The Basis may
include Resource scope/observation/fingerprint plus the verifying
WorkStateCommit and explicit semantic dependencies.

**Relations:** A Verification `verifies` a Verification Requirement when one
exists, otherwise its Acceptance Criterion, and is `evidenced_by` immutable
Evidence. No other target kind is valid.

**Version behavior:** Judgment instances are immutable Versioned Work-State
facts and their relations participate in branch, diff, merge, and restore.
Evidence remains immutable provenance. Current applicability is the derived,
branch-sensitive `applicable`, `stale`, or `unknown` projection and is not the
historical result field.

**Example:** Branch A may record `T-18/AC-2` as passed while Branch B records it
as failed using different immutable Evidence; merge must preserve or resolve
the semantic difference.

## Runtime Coordination

### Session

**Purpose:** Identifies one Agent execution provenance boundary and its current
coordination state.

**Lifecycle:** `starting -> active -> ending -> ended`, with
`active <-> potentially_stale`. It changes Focus or active Workspace/Branch
explicitly and ends with a deterministic Session diff plus optional semantic
Handoffs.
When claimed or in-progress work remains, Session end recommends a Handoff but
does not require one. Normal end releases claims. After abnormal exit the
Session is marked `potentially_stale` and its claims remain until explicit
release or takeover.

**Occurrence state:** Stable Session identity, creation fact, Agent/adapter
metadata, and immutable provenance links.

**Runtime state:** Context Set, active Workspace, active Branch, primary Focus,
current Claims, lifecycle state, and activity metadata.

**Relations:** May read multiple Workspaces and Knowledge Spaces. Every
mutation still targets one active Workspace and Branch.

**Version behavior:** Current Session state does not branch, merge, or restore.
Its timeline is immutable provenance and remains readable by later Sessions.
The stable ObjectIdentity-backed Session occurrence is separate from mutable
SessionRuntime. Context Set membership and structured Focus path are runtime
children; a final SessionDiff is immutable provenance.

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
when a prior Session is stale. The active set is none, exactly one exclusive,
or one-or-more shared; exclusive and shared cannot coexist. Mode change and
takeover are explicit atomic operations. A forced takeover requires rationale
and records prior claimant and last activity.

**Owned state:** Session, Task, Work Branch, mode, and activity metadata.

**Relations:** Branch-scoped. A Session without the claim may still read, add
Findings or Evidence, and link Decisions. Under an active exclusive claim,
terminal or structural mutation by another Session is rejected unless the
claim is explicitly transferred or the mutation carries force provenance.
Multiple shared claimants may add Findings, Evidence, Verification, and other
non-destructive updates; a terminal or structural mutation requires a unique
claimant or explicit force provenance.

**Version behavior:** Runtime-only. Claim creation, release, and takeover are
immutable Events. Each ownership/mode period is also a stable immutable Claim
occurrence; active ClaimRuntime is a separate current projection. Mode change
or takeover ends one occurrence and creates another rather than rewriting its
owner or mode.

Claiming or claim-next does not change Task status or imply TaskStart.

### MergeAttempt

**Purpose:** Identifies one persistent merge attempt independently of whether
it remains active, completes, or aborts.

**Occurrence state:** ObjectIdentity-backed merge identity, Workspace, target
and source Branches, merge base, captured target/source heads, creation fact,
and immutable provenance.

**Runtime state:** Active status, structured MergeItems, and provisional
resolutions live in separate MergeRuntime children. Successful continue writes
canonical Commit/ChangeSet history atomically; completion or abort removes the
attempt from active runtime without deleting its occurrence, items,
resolutions, or Events.

## Provenance and version primitives

### ObjectIdentity

**Purpose:** Provides one Store-local stable addressable identity namespace
across Entity, Relation, Session/SessionDiff, Claim/MergeAttempt, Evidence,
Resource/ResourceObservation, and KnowledgeSpace/KnowledgeExposure families.

**Owned state:** Stable object ID and controlled object kind only. Semantic
state, Branch selection, runtime status, and relation endpoints belong to typed
families or other authority layers.

**Invariant:** Every committed ObjectIdentity has exactly one kind-matching
typed family owner. Store, Workspace, version infrastructure, ContentObject,
VerificationBasis, runtime projections, Checkpoint, and transport bookkeeping
use their own typed identities outside this registry.

### EntityVersion and RelationVersion

**Purpose:** Separate stable logical identity from complete immutable semantic
state. An Entity or Relation may have many versions, and multiple Branches may
select the same version.

**Owned state:** Logical identity, state-schema version, complete canonical
semantic state, and semantic-state digest where applicable. Transition
provenance and optional field delta belong to ChangeOperation/ChangeSet rather
than creating a second version chain inside the Version.

**Version behavior:** Versions do not belong to a Branch. Branch heads and
rebuildable current projections select active versions. Historical readers
upcast old schema versions without rewriting them. Logical removal preserves
identity and history. Equal state digests do not require equal Version IDs.

### Resource

**Purpose:** Gives an associated external source or artifact set stable logical
identity independently of its machine-local location.

**Owned state:** ObjectIdentity-backed logical identity and immutable Resource
kind. The rebindable environment locator belongs to a separate ResourceBinding
current-config family.

**Version behavior:** Identity is portable. Locator rebind does not prove
content continuity or Verification applicability.

### ResourceObservation

**Purpose:** Records immutable mechanical observation of one Resource at a
point in time for Verification, diagnostics, or an explicit snapshot.

**Owned state:** Resource identity, Adapter/format metadata, captured time,
scope, fingerprint, availability metadata, and optional content-addressed
manifest or diff reference.

**Version behavior:** Immutable provenance, not Work-State semantic cognition.
Observation or derived drift alone creates no WorkStateCommit. Verification
persists the observation/fingerprint used as its basis; other capture points
remain policy unless explicitly requested.

### Evidence

**Purpose:** Evidence is immutable support material captured or referenced by
a Verification or another semantic object.

**Lifecycle:** Evidence is captured or referenced and retained under its
policy. Stored content objects are digest-identified; metadata-only or external
Evidence need not invent a local blob.

**Owned state:** ObjectIdentity-backed Evidence identity, capture/provenance
metadata, external references, and zero or more links to digest-identified
ContentObjects. ContentObject storage location is separate from its digest
metadata.

**Relations:** A Verification or another semantic object may be `evidenced_by`
Evidence. A Finding or other semantic claim may independently `support` a
Decision or Knowledge statement.

**Version behavior:** Evidence is immutable provenance and does not branch.
References to it and the semantic interpretation expressed by Verification
belong to Versioned Work State.

**Example:** A command wrapper may capture command, cwd, start/end, duration,
exit status, output artifact or digest, and an observation/fingerprint of the
actual verified Git state—including relevant uncommitted changes—as Evidence
for `T-18/AC-2`.

### ChangeSet, Event, and WorkStateCommit

**Purpose:** A ChangeSet is one atomic semantic mutation; Change Operations are
its deterministic state transformation; Events explain its semantics; a
WorkStateCommit places the resulting state in the DAG.

**Lifecycle:** One accepted versioned semantic operation creates exactly one
non-reusable ChangeSet and one WorkStateCommit plus one or more Events. Genesis
uses its own initialization ChangeSet and may contain zero ChangeOperations.
Failure rolls back the complete ChangeSet. A pure Runtime Coordination or
infrastructure operation updates its own state atomically and emits provenance
Events without creating a WorkStateCommit.

**Owned state:** ChangeSet metadata and semantic intent; schema-versioned
Change Operations with expected-before/after state; Event type and payload;
commit parent(s), ChangeSet reference, resulting-state digest, time, and
provenance sufficient to identify the originating operation. When the
operation occurs within a Session, that Session is recorded; not every valid
WorkStateCommit source must be a Session.

**Relations:** A Workspace genesis commit has zero parents, a normal commit has
one parent, and a merge commit has two.

**Version behavior:** All are immutable provenance/version primitives.
Commit + ChangeSet + Change Operations reconstruct canonical Work State;
Events are not replay truth. Commit identity and state digest remain distinct.
The V1 ID, digest, JSON, timestamp, and SQLite integrity boundaries are fixed
by [Physical Schema v0.1 Contract](../architecture/physical-schema-v0.1.md);
the complete executable schema remains the next assembly stage.
