# Domain Invariants

These invariants are mandatory design constraints and future test sources. An
implementation that violates one must first revise the confirmed design under
explicit authority. An ADR may record that revision under current repository
policy, but does not create the authority by itself.

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
Cross-Workspace reuse publishes Knowledge with provenance; it does not share a
Work Graph.

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
reorganization. This baseline does not select the persistent ID scheme.

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
target Work State exactly to its pre-merge state while retaining merge-attempt
provenance.

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

Finding, Assumption, Attempt result, Decision, Knowledge promotion, and Handoff
are created by explicit Agent semantic operations. V1 never infers them from
transcript text. Additional Record kinds remain Open unless separately
confirmed.

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

Decision, Finding, Knowledge, ChangeSet, Event, and WorkStateCommit history is
retained. HOT/WARM/COLD projections and configurable large-Evidence retention
may reduce the active footprint without falsifying lineage.

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
mark a Task done only after every mandatory criterion has Verification.
