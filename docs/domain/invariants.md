# Domain Invariants

These invariants are mandatory design constraints and future test sources. An
implementation that violates one must first revise the confirmed design under
explicit authority. An ADR may record that revision under current repository
policy, but does not create the authority by itself.

## P0 cutover entry invariants

### INV-081 — Project identity uses the Git common directory

Project binding identifies a repository through its Git common directory, not
through a mutable worktree path. Worktree aliases must not create duplicate
project identities.

### INV-082 — Store discovery is external and double-validated

The registry is selected, in order, by explicit `--registry PATH`, the
`WORKVCS_HOME` environment variable, or the versioned XDG config file at
`$XDG_CONFIG_HOME/workvcs/config.toml` (falling back to
`$HOME/.config/workvcs/config.toml`). It is not stored in the repository. A
discovered Store undergoes a second identity/format/integrity validation
before use.

### INV-083 — Ambiguous active Session state fails closed

Zero, multiple, or otherwise ambiguous active Sessions cannot be silently
selected by project bind, resume, admission, or closeout inspection.

### INV-084 — Read-only entrypoints do not write

Configuration inspection, project discovery/audit, `recall`, `resume --cwd`,
and current `closeout inspect` must not create or mutate WorkVCS state,
receipts, Sessions, Claims, or Work-State commits.

### INV-085 — Plan admission is atomic and idempotent

P0-2a admits one manifest as one atomic WorkVCS transition. Repeating the same
idempotency key and manifest identity reuses the prior result rather than
duplicating Goal, Plan, Task, Record, or Evidence state. Expected head/state
guards are manifest fields, not CLI flags.

### INV-086 — Registry writes are externally serialized and replace atomically

The external project registry is outside the project/repository. Updates use
cross-process mutual exclusion and atomic replacement; a partially written
registry is never an accepted binding source.

### INV-087 — Store use requires complete integrity validation

Project discovery and Plan admission must fully validate Store identity,
format, schema, and integrity before using the Store as an authority.

### INV-088 — WorkVCS does not decide authorization policy

WorkVCS may expose mechanical state and target-bound receipt Records, but
authorization policy, confirmation gates, and completion judgment remain
outside WorkVCS.

### INV-089 — In-place Plan evolution preserves omitted state

P0-2b `plan evolve` with `mode=in_place` updates only explicitly supplied Plan
fields, preserves omitted fields, and atomically appends declared Tasks,
Acceptance Criteria, Verification Requirements, Records, and Evidence. It
does not implicitly delete or replace omitted state. Expected guards, target
Plan identity/version/digest, and idempotency are manifest fields.

### INV-090 — Supersede evolution preserves explicit Plan ancestry

P0-2b2 `plan evolve` with `mode=supersede` transitions the old active Plan to
`superseded`, creates a new active Plan under the same unique Goal, retains the
old `contains` relation, adds the new `contains` relation, and creates the
machine `new_plan→old_plan` `supersedes` relation. Constraints require explicit
`carry_all` or `replace`; Tasks, Records, and Evidence are not automatically
migrated. Expected IDs, versions, digests, and branch head provide CAS guards;
the operation is one idempotent transaction.

### INV-091 — P0-3a receipt inspection is mechanical and redacted

Current receipt issue/show/list uses a dedicated AuthorizationReceipt Record
subtype with branch, target, action, contract-digest, transaction, idempotency,
and target-guard semantics. The authority reference contributes only its
type/digest and a redacted marker; original text is absent from scope, payload,
CLI/show/list, and debug output. P0-3b consume is branch-scoped single-use;
idempotent replay may reuse only a committed `workstate_commit` result, never
an orphan ChangeSet. No Store-global lock across restore histories is promised,
and consume is not atomic with an external action. Receipt revoke remains
deferred, and receipt projection into `context` or `why` is not current; these
are not prerequisites for the current P0 surface.

### INV-092 — Closeout inspect is a bounded read-only projection

P0-4 `closeout inspect` requires an explicit goal/plan/task target and uses
OS-level read-only plus database `query_only` access. It expands only the
target's documented direct scope, uses a default budget of 50 and hard maximum
of 200 with stable truncation/omitted reporting, aggregates exact-target
runtime state, and proves before/after source state and Store main-WAL-SHM
metadata without creating state or making policy conclusions.

### INV-093 — Plan admission and durable cognition are independent

WorkVCS may atomically and idempotently capture Records, Knowledge, Evidence
metadata, and valid semantic relations without creating or requiring a Goal,
Plan, Task, Session, or Claim. “No-Plan” describes the planning decision; it
does not prohibit valuable durable cognition. A later Plan admission carries
forward still-relevant prior cognition and records why planning became useful.

### INV-094 — Raw Evidence content is verifiable and recoverable

Raw bytes supplied to Evidence creation are stored in the local
content-addressed object area, linked through `content_storage_location`, and
verified by size and digest when read. Digest-only content remains a valid
external reference and is not falsely reported as locally recoverable.

### INV-095 — Mutating result assertions expose completed operations

Deterministic expected values for mutating commands are validated before the
write. Assertions that depend on a generated or committed result may run after
the operation, but any mismatch must explicitly report that the operation
completed, expose its rendered result, and direct the caller to inspect that
result before retrying. Atomic Engine guards remain inside their transition.

### INV-096 — Bounded Recall prioritizes current recoverable truth

Recall reserves the current Goal/Plan/Task spine and orders semantic categories
newest-first before filling historical context. It exposes the scope/provenance
needed to follow source identifiers. Branch-based current projections evaluate
Runtime Coordination at read time; historical commit projections label and use
their historical cutoff explicitly.

### INV-097 — Skill installation verifies the complete tree

WorkVCS packaging and installation compare a deterministic manifest of every
regular file in `skills/workvcs`. Missing, modified, or extra files must fail
verification; checking only `SKILL.md` is insufficient.

### INV-098 — Finding correction preserves currentness and ancestry

A Finding correction is an atomic Entity-plus-Relation transition. Superseding
an active Finding creates `replacement -> prior` `supersedes`; invalidating an
active Finding creates `cause -> target` `invalidates`. The source is an active
Finding, the target version is explicitly guarded, and a terminal Finding
cannot transition again. Brief and handoff recovery must not present terminal
Findings as current truth, while retrospective projections preserve them and
their causal ancestry.

### INV-099 — Record currentness review is bounded, explicit, and independent

`record currentness-audit` may select Records only from their explicit
lifecycle state. It must be read-only, bounded to 1 through 200 returned items,
report omitted candidates, preserve full statement and scope, and distinguish a
current Branch-head source from a historical Commit source. It never infers
staleness, mutates a Record, or turns independent cognition into a Goal, Plan,
Task, Verification, or closeout gate.

### INV-100 — Project first-use bootstrap is explicit and convergent

Project discovery remains zero-write. Explicit project ensure either verifies
an existing binding unchanged or converges concurrent and repeated first-use
calls on one identity-derived external Store, one Workspace, one initial Work
Branch, and one atomic registry binding. It creates no semantic work objects.
Interrupted bootstrap may be resumed only from the exact deterministic Store
marker and a missing or pristine Genesis Workspace; foreign, ambiguous, or
non-pristine Stores fail closed without overwrite or adoption.

## State and ownership

### INV-001 — State layers remain separate

Versioned Work State, Runtime Coordination, Immutable Provenance, and Derived
Projection have distinct lifecycle and authority. A projection never becomes
independent truth.

### INV-002 — Mutation has one versioning target

Every versioned mutation belongs to exactly one Workspace and one Work Branch,
even when its Session reads multiple Workspaces.

### INV-003 — Workspace is not infrastructure

`Store != Workspace`, `Workspace != Repository`, and `Session != Workspace`.
A Workspace may span repositories or directories, may be non-Git, and is not
identified by any one physical source location.

### INV-004 — Cross-Workspace sharing is knowledge-only in V1

Goal, Plan, Task, and their execution relations remain Workspace-local.
Cross-Workspace reuse uses Store-local KnowledgeExposure bound to one immutable
source Knowledge version; it does not share a Work Graph.

### INV-005 — Runtime state is not restored or merged

Current Session, Claim, Focus, and merge-in-progress state never participate in
Work-State restore or branch merge. Their changes remain in immutable
provenance.

## Identity and evolution

### INV-006 — WorkStateCommit is immutable

A committed Work-State node, its parent references, and its ChangeSet identity
never change. A correction creates a new commit. Restore likewise creates a new
WorkStateCommit from the current Branch head rather than moving the Branch
reference backward or erasing later history.

### INV-007 — Semantic operations are atomic

One accepted versioned semantic operation produces exactly one atomic
ChangeSet, one or more Events, and one WorkStateCommit. A pure Runtime
Coordination operation produces an atomic runtime transition plus Events and no
WorkStateCommit. Partial state is never externally visible. Batch applies the
same all-or-nothing rule to its declared state surfaces.

### INV-008 — Logical identity survives reorganization

Attaching an existing Task or Plan to a later-discovered Plan or Goal preserves
the object's identity and prior history.

### INV-009 — Logical references survive mutation

A logical entity keeps referential identity across ordinary mutation and
reorganization. V1 persistent logical IDs use UUIDv7 encoded as 16-byte BLOBs;
the encoding does not replace semantic identity or causal ordering.

### INV-010 — Terminal does not mean deleted

Done, failed, cancelled, superseded, invalidated, or abandoned objects leave
the default working set as appropriate but remain queryable through history,
why, diff, and restoration.

## Work semantics

### INV-011 — Status and outcome are independent

Task execution status never encodes the semantic result. A successfully
completed evaluation may have an outcome that rejects the evaluated option.

### INV-012 — Organization dimensions are orthogonal

Containment, sibling order, dependency, and priority never imply one another.

### INV-013 — Plan completion and Goal achievement are explicit

Derived descendant state may create a readiness hint, but it cannot
automatically complete a Plan or achieve a Goal.

### INV-014 — Acceptance Criteria have stable local identity

An Acceptance Criterion remains referentially stable across wording changes so
Verification and Evidence links cannot drift.

### INV-015 — Major semantic transitions carry provenance

Supersession, invalidation, restore, merge resolution, cancellation of active
work, and Goal achievement/abandonment require a reason, a causal entity
reference, or both.

## Relations and knowledge

### INV-016 — Canonical relations have one stored direction

Reverse names are projections. Storing both directions as independent edges is
forbidden.

### INV-017 — Semantic operations maintain canonical edges

The Agent expresses semantic intent; for a versioned operation WorkVCS creates
the required low-level relations, Events, and commit metadata atomically. A
runtime-only operation creates runtime state and provenance, not an empty
WorkStateCommit.

### INV-018 — Custom relations do not alter core algorithms

`related_to` plus a custom label, and an optional explanation, is preserved but
cannot affect readiness, merge, or context rules until promoted to a confirmed
canonical semantic.

### INV-019 — Knowledge conflict is explicit

Two Knowledge statements may coexist under different conditions. Similar
topics do not create a conflict; V1 requires explicit contradiction,
invalidation, supersession, or another confirmed deterministic rule.

## Branch, merge, and concurrency

### INV-020 — Work Branch represents state divergence

A Work Branch expresses a divergent work or cognition state. It is independent
of Git branch identity and is not required merely to run multiple Sessions.

### INV-021 — Merge commit has two parents

A completed Branch merge records target and source heads as parents; it is not
represented as a linear replay that destroys ancestry.

### INV-022 — Merge is persistent and reversible before commit

Merge follows `start -> resolve -> continue` or `abort`. Abort restores the
target Work State exactly because provisional resolutions never mutate it;
merge-attempt provenance remains.

### INV-023 — Sibling Branches do not contaminate current state

Unmerged sibling Branch Decisions and Knowledge are not current facts. They are
available only through explicit cross-Branch queries or after merge.

### INV-024 — Same-Branch concurrency is optimistic

Multiple Sessions may mutate the same Work Branch without a branch-wide lock.
Stale expected bases are deterministically reconciled only when compatible;
real conflicts are rejected for refresh and explicit resolution.

### INV-025 — Claims coordinate; they do not rewrite history

Claims are Branch-scoped Runtime Coordination. Exclusive is the default,
shared is explicit, stale ownership changes through explicit takeover, and
terminal or structural work protected by another Session's claim requires an
explicit transfer, a unique claimant where applicable, or force provenance.
Unclaimed Sessions may still add non-terminal semantic facts; shared claimants
may add Verification and other non-destructive updates.

## Context and provenance

### INV-026 — V1 semantic records are explicit

Finding, Assumption, Question, Attempt result, Decision, Risk, Verification,
Knowledge promotion, and Handoff are created by explicit Agent semantic
operations. V1 never infers them from transcript text. `Blocker`, `Review`, and
`Note` remain Open as distinct Record kinds unless separately confirmed.

### INV-027 — Context resolution is deterministic and path-sensitive

Context selection uses explicit Focus path, scope, containment, dependency,
causal relations, state, Session provenance, recency, profile, and budget. It
does not require an LLM, embedding, or vector search.

### INV-028 — Context budgets remove complete low-priority items

The resolver never satisfies a budget by truncating arbitrary strings. It
removes whole low-priority items and reports omitted categories and counts.

### INV-029 — Inactive cognition has a causal exception

Superseded or invalidated cognition is absent by default but may appear as a
concise summary when it lies on a direct causal path explaining current state.

### INV-030 — Session continuity never depends on a hand-written summary

Session end always permits a deterministic structured diff. A semantic Handoff
is optional and supplementary.

### INV-031 — Core provenance is not destructively compacted by default

Decision, Finding, Verification, Knowledge, ChangeSet, Event, and
WorkStateCommit history is retained. HOT/WARM/COLD projections and
configurable large-Evidence retention may reduce the active footprint without
falsifying lineage.

## Confirmed lifecycle details

### INV-032 — Decision promotion is explicit

An ordinary decision is recorded as `Record(kind=decision)`. Promotion creates
a Decision carrying context, options, choice, rationale, and consequences;
later change supersedes rather than rewrites the promoted Decision.

### INV-033 — A Goal may have multiple active Plans

One Work Branch may carry multiple active Plans for the same Goal. WorkVCS does
not force complementary strategies into one oversized Plan.

### INV-034 — Task decomposition preserves parent executability

Adding SubTasks does not automatically turn the parent Task into a non-
executable composite. The parent remains independently executable.

### INV-035 — Terminal Tasks remain open to later cognition

A terminal Task's execution target is closed, but later Findings, Decisions,
Knowledge, and newly discovered Task links may still be attached with history.

### INV-036 — Mandatory criteria gate automatic completion

Acceptance Criteria are optional. When they exist, WorkVCS may automatically
or explicitly mark a Task done only after every mandatory criterion's effective
status is `verified`. Ordinary coordination force cannot bypass this gate.

### INV-037 — `next` resolution is deterministic

Runnable Task resolution evaluates, in order:

1. active Workspace and Work Branch;
2. active scope and Plan path;
3. executable Task descendants;
4. dependency readiness;
5. Task lifecycle eligibility;
6. priority;
7. explicit manual order;
8. Session and Claim coordination.

Priority and manual order are distinct, with priority evaluated first. Manual
order never overrides dependency readiness or lifecycle eligibility. The final
stable tie-breaker among otherwise equal candidates is not fixed by this
baseline.

## Architecture Specification invariants

### INV-038 — Verification judgments are immutable

One Verification records one historical target/result judgment. Re-verification
creates another judgment and does not automatically supersede the first.

### INV-039 — Result and applicability are distinct

Verification result is immutable. Applicability is a branch-sensitive Derived
Projection of Resource and Work-State Basis with `applicable`, `stale`, or
`unknown`; it never rewrites the result.

### INV-040 — Verification Requirements retain local identity

A required Verification intent has stable AC-local identity and remains
historically referential. It does not encode an execution command. V1
judgments are single-target, while immutable Evidence may be reused.

### INV-041 — Resource identity, binding, and observation remain separate

A Resource has portable logical identity, a rebindable environment locator,
and immutable ResourceObservations. Locator rebinding alone never proves
continuity or applicability.

### INV-042 — Mechanical drift is not semantic mutation

Resource observation or derived drift/applicability creates no
WorkStateCommit. Applicability combines conservatively: any stale component
yields stale; otherwise any unknown component yields unknown.

### INV-043 — Semantic lifecycles require explicit transitions

Task, Plan, Goal, Finding, Assumption, Attempt, and Decision states change only
through their confirmed semantic operations. Terminal Findings and Attempts
never reopen; a superseded Entity cannot use an ordinary reopen that ignores
its supersession relation.

### INV-044 — Dependency blocking is derived

Task `blocked` means an explicit blocker. Dependency readiness is a separate
Derived Projection and never silently rewrites Task status.

### INV-045 — Session switching and ending are atomic

Workspace/Branch switch and SessionEnd either apply all Runtime Coordination
effects and Events or leave the prior runtime state intact. Handoff remains a
separate versioned semantic operation.

### INV-046 — Active Claim modes cannot mix

For one Task and Work Branch, active claims are none, exactly one exclusive,
or one-or-more shared. Claiming never changes Task status or implies TaskStart.

### INV-047 — Merge resolutions remain provisional

Before `continue`, merge resolutions exist only in Runtime Coordination and
provenance. One target Workspace/Branch has at most one active merge. Continue
requires the captured source and target heads to remain unchanged and creates
one atomic two-parent commit.

### INV-048 — Change Operations reconstruct Work State

WorkStateCommit + ChangeSet + deterministic Change Operations are canonical
replay truth. Semantic Events are immutable explanation/provenance and cannot
serve as an alternative state-reconstruction authority.

### INV-049 — Merge replay uses the primary parent

A merge ChangeSet transforms primary parent=target into merged state; secondary
parent=source preserves ancestry. Historical replay applies the recorded
ChangeSet and never reruns merge logic.

### INV-050 — Projections and checkpoints are rebuildable

Current projections, indexes, and checkpoints accelerate reads but are not
canonical state. Checkpoints are digest-validated snapshots associated with a
commit and may be regenerated.

### INV-051 — Branch creation does not copy Work State

Creating a Branch is an O(1) ref operation at a WorkStateCommit. Only HOT
Branches need remain materialized.

### INV-052 — Logical identity is separate from immutable version state

Entity and Relation identities survive semantic change. Each changed state is
an immutable schema-versioned EntityVersion or RelationVersion that may be
shared by multiple Branches.

### INV-053 — Canonical logical history is not physically deleted

Removing an Entity or Relation from current Work State means absence or
retirement in later state. It cannot erase canonical identity, versions, or
history.

### INV-054 — Primary containment and dependency are acyclic

An object has at most one primary containment parent in one Work State;
multi-Plan reuse uses `references`. Primary containment and Task dependency
must remain acyclic, and every Work Graph Relation shares a Workspace with both
endpoints.

### INV-055 — Historical formats remain self-describing

EntityVersion state and Change Operation payloads carry schema versions.
Readers may upcast immutable history but cannot rewrite it. A Store describes
its identity, format, schema, capabilities, and object format sufficiently for
reading or migration.

### INV-056 — Commit identity is not state identity

A commit's resulting-state digest verifies state, replay, and checkpoints but
does not identify the historical commit. Different histories may produce the
same state digest.

### INV-057 — KnowledgeExposure binds an exact source version

A Store-local KnowledgeExposure has stable identity and an immutable binding
to source Store, Workspace, Knowledge, and KnowledgeVersion. It never follows
source `latest` implicitly.

### INV-058 — Consulting and adopting Knowledge are different

Consulting an Exposure is read-only. Adoption explicitly creates
Workspace-local Knowledge with retained Exposure/source-version provenance;
later source changes never silently rewrite the adopted Knowledge.

### INV-059 — Exposure history survives availability changes

Withdrawal removes an Exposure from the current set but never deletes its
history. V1 replacement uses a new Exposure plus explicit withdrawal of the
old Exposure rather than a federation-level `superseded` state or relation.
Source drift creates a derived warning/status and does not silently change
Exposure semantic state.

### INV-060 — Ordinary portability preserves Store identity

Copy, move, export/import, backup, and restore preserve Store identity. Only an
explicit Store fork creates a new identity, with source-store lineage.

### INV-061 — Import cannot resurrect runtime or overwrite divergence

Import preserves Session/Claim/merge provenance but never blindly restores
active Runtime Coordination. Same-Store import detects ref ancestry and
divergence; last-write-wins is forbidden and recoverable states are preserved.
Full canonical-history import is same-Store only; content from a different
Store cannot be inserted into the local canonical namespace.

### INV-062 — Bundle integrity covers canonical history

A Bundle is self-contained for its profile and cannot omit data required to
reconstruct and validate included canonical history. Rebuildable projections
need not be transported. Immutable objects are hash-verified and deduplicated.

### INV-063 — Portable references may remain unresolved, not severed

An imported Resource locator may be unresolved and rebound. Cross-Bundle
Knowledge provenance remains resolved or a portable unresolved external
reference; source lineage cannot be erased merely because the source is absent.

### INV-064 — V1 Knowledge federation is Store-local

Knowledge Spaces do not provide cross-Store live references, remote
subscriptions, or a global federation service in V1.

## Logical Schema invariants

### INV-065 — Object identity has one typed owner

Every committed ObjectIdentity belongs to exactly one kind-matching typed
family. Orphan identities, missing owners, kind mismatch, and multiple family
owners are invalid committed state. ObjectIdentity is an address registry, not
a polymorphic semantic-state table, and Object is not Entity.

### INV-066 — Identity and version ownership do not drift

Entity Workspace/kind and Relation Workspace/type/source/target/discriminator
are immutable identity properties. EntityVersion and RelationVersion are
complete immutable semantic states and are not Branch-owned or linked through
a single previous-version chain.

### INV-067 — Relation logical identity is reused

Within one Workspace, one relation logical key has one Relation identity. The
key is canonical type, source, target, and immutable discriminator; removal
changes membership, and later re-addition reuses the identity.

### INV-068 — Work State is membership/version selection

Canonical Work State is the Entity-to-EntityVersion and
Relation-to-RelationVersion mapping selected through Branch HEAD. Presence,
absence, and selected version are mapping facts; semantic terminal status is a
different Entity/Relation-state fact.

### INV-069 — Branch HEAD is the sole current-state pointer

No current table competes with Branch HEAD. A complete current projection must
bind the same Commit as HEAD and validate its state digest before a missing row
can mean absence.

### INV-070 — ChangeSet transition semantics are normalized and set-based

One ChangeSet has at most one final before-to-after transition per Entity or
Relation membership key. All before conditions are checked against the parent
state and all resulting-state invariants are checked before commit; operation
ordinal does not supply correctness.

### INV-071 — Commit, ChangeSet, and Event responsibilities remain distinct

Every WorkStateCommit has exactly one non-reusable ChangeSet. Events are
immutable provenance and may exist without a ChangeSet for Runtime or
infrastructure operations; they are never canonical replay input.

### INV-072 — Branch ref movement is guarded and auditable

Branch HEAD points only to a Commit in the same Workspace, moves by
expected-head compare-and-swap, and leaves immutable old/new provenance.
Branch creation and fast-forward import do not invent empty Commits.

### INV-073 — Stable occurrences outlive runtime projections

Session, Claim ownership/mode periods, and MergeAttempt are stable occurrences.
Their mutable current coordination is kept in separate runtime projections.
Release, takeover, mode change, completion, or abort never rewrites or deletes
the historical occurrence.

### INV-074 — Evidence identity is not content identity

Evidence is immutable logical provenance and may reference zero or more
digest-identified ContentObjects. ContentObject metadata is retained
independently of storage location and may be reused by multiple Evidence
objects.

### INV-075 — Resource identity, association, binding, and observation differ

Resource kind is stable identity state; Workspace-to-Resource association is
many-to-many infrastructure configuration; ResourceBinding is environment-
local current config; ResourceObservation is immutable provenance. None of the
latter three becomes Branch Work State merely because it changes.

### INV-076 — Verification defining closure is immutable

A Verification has exactly one target, of kind Acceptance Criterion or
Verification Requirement. Its result, target, Basis, Evidence set, method,
semantic state, and defining `verifies`/`evidenced_by` edges form one immutable
judgment closure. Changing any member creates a new Verification. Applicability
remains derived and branch-sensitive.

### INV-077 — ExternalObjectRef cannot escape canonical endpoint rules

ExternalObjectRef is permitted only in explicitly allowed federation,
lineage, and imported provenance metadata. Canonical Relation endpoints remain
local ObjectIdentities; Bundle export includes the required local Relation
endpoint closure.

### INV-078 — Store infrastructure history is auditable

StoreManifest may reflect the current migrated format, while each migration is
immutable provenance. BundleManifest is separate from StoreManifest, and every
ImportAttempt is immutable infrastructure provenance.

### INV-079 — Import cannot partially activate canonical refs

Import ingests and validates immutable candidates before accepted refs move
atomically. Equal immutable identity and content is idempotent; equal identity
with unequal content is an integrity conflict and is never updated in place.

### INV-080 — Portable identity is Store-namespaced

Cross-Store complete reference semantics are `(store_id, local_id)`. Ordinary
movement preserves Store identity; explicit Store fork creates a new Store
namespace while retaining internal local IDs by default and recording lineage.
