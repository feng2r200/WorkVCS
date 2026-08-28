# V1 and V2 Boundary

This document defines the confirmed release boundary. “V1” means required
product semantics; it does not imply that the capability is already
implemented.

## V1: required foundation

### Store and scope model

- A self-contained, portable **Store** is the data boundary.
- A Store may contain multiple Workspaces, Knowledge Spaces, Sessions,
  provenance, and evidence objects.
- A **Workspace** is the versioning boundary for one Work State; it is not
  equal to a directory or Git repository.
- A Workspace may span repositories or directories, multiple Workspaces may
  refer to different parts of one monorepo, and a Workspace may be non-Git.
- A Session has a multi-Workspace Context Set and one active Workspace plus
  active Branch for mutation.
- Cross-Workspace reuse occurs through Knowledge Spaces. Cross-Workspace Task
  dependency or containment is outside V1.

V1 cross-Workspace Knowledge reuse uses Store-local KnowledgeExposure records.
An Exposure has stable identity and binds one specific immutable source
Knowledge version. Consulting it is read-only; explicit adoption creates new
Workspace-local Knowledge with complete source provenance. Exposure history is
append-only with a rebuildable current availability projection; V1 does not add
an independent Knowledge Space branch/merge/restore DAG.

The confirmed V1 storage direction is SQLite metadata plus a content-addressed
object store. This is an implementation direction, not a domain invariant;
replacing it requires a later explicit confirmed decision, which current
repository policy may record as an accepted ADR. The Physical DDL contract is
fixed by [ADR-0006](../decisions/adr/0006-sqlite-physical-schema-v0.1.md), and
the first executable schema assembly is closed by
[ADR-0007](../decisions/adr/0007-schema-v0.1-assembly-install-and-integrity.md).
Object layout and implementation language are not yet fixed.

The confirmed logical persistence model is commit/delta-based: immutable
EntityVersion and RelationVersion state; canonical WorkStateCommit + ChangeSet
and deterministic Change Operation reconstruction; semantic Events as
provenance; rebuildable current projections and checkpoints; O(1) Branch refs;
and resulting-state digests distinct from Commit identity. This fixes logical
semantics, not tables or encoding.

### Versioned work and cognition

V1 versions:

- Goal, Plan, Task, Decision, Knowledge, Record, Acceptance Criterion, and
  typed relations;
- Finding, Assumption, Question, Attempt, ordinary decision, Risk, and Handoff
  semantics as Records;
- versioned Verification state and relations, with references to immutable
  Evidence provenance;
- immutable single-target Verification judgments, stable AC-local Verification
  Requirements, and branch-sensitive applicability/effective AC projections;
- top-down Goal -> Plan -> SubPlan -> Task decomposition and bottom-up
  Task -> Finding -> Plan -> Goal discovery;
- Plan descriptions, constraints, scoped Assumptions, Task-graph references,
  and Plan hierarchy;
- Task hierarchy, mixed Plan/Task siblings, references, explicit order,
  dependency, and priority;
- Tasks whose home scope defaults to a Plan but may explicitly be
  Workspace-level and may be referenced by multiple Plans;
- multiple active Plans for the same Goal in one Work Branch;
- a parent Task that remains independently executable after SubTasks are
  introduced;
- stable object identity when later attaching an existing object to a newly
  discovered Plan or Goal;
- separate Task execution status and open-semantic outcome;
- explicit Plan completion and Goal achievement;
- later Findings, Decisions, Knowledge, and new Task links on a terminal Task;
- ordinary `Record(kind=decision)` entries that can be promoted to a Decision
  with context, options, choice, rationale, and consequences, with later
  change represented by supersession;
- optional Acceptance Criteria, with all mandatory criteria effectively
  `verified` before Task completion when criteria exist; an ordinary
  coordination force cannot bypass this gate.

V1 stable logical identity uses UUIDv7 stored as a 16-byte BLOB. Stable
Task-local Acceptance Criterion identities remain owner-scoped so references
survive wording changes and sibling reordering.

Task, Plan, Goal, Assumption, Attempt, and Decision follow the state machines
defined in [Semantic Operations and State Machines](../architecture/semantic-operations-and-state-machines.md).
In particular, explicit blockers and dependency readiness are distinct,
terminal Attempts are never reopened, and superseded entities require
supersession-aware transitions rather than ordinary reopen.

### Work-State versioning

V1 includes:

- immutable WorkStateCommit objects and a commit DAG;
- automatic commits for successful semantic mutations;
- atomic ChangeSets and a batch/transaction interface;
- branches from any historical WorkStateCommit;
- diff, history, why, lineage, and read-only historical-state inspection
  (`show-at` semantics without fixing command spelling);
- restore as a new WorkStateCommit that makes selected historical Work State
  current without moving a Branch reference backward or deleting later
  history;
- persistent three-way merge with `start`, `resolve`, `continue`, and `abort`;
- two-parent merge commits and `ours`, `theirs`, and `custom` resolution;
- deterministic merge classification as `AUTO`, `CONFLICT`, or `REVIEW`;
- deterministic review for active Decisions sharing an exclusive scope and
  subject but selecting different choices;
- retention of useful findings, attempts, verification, evidence, and
  completed evaluation work from unselected branches;
- preservation of the source Branch after merge;
- merge resolutions remain provisional Runtime Coordination until `continue`;
  one target Workspace/Branch has at most one active merge, and continue
  rejects moved source or target heads rather than locking either Branch.

### Sessions and concurrency

V1 includes:

- Session provenance independent of Work Branch state;
- deterministic cross-Workspace Session-end diff plus zero or more optional
  semantic Handoffs; each Handoff belongs to exactly one Workspace and Work
  Branch and may bind a Focus/context path;
- multiple Sessions on the same Work Branch without a branch-wide lock;
- optimistic concurrency using an expected base and deterministic
  reconciliation when safe;
- exclusive Task claims by default, with explicit shared claims;
- unclaimed Sessions may read and add non-terminal semantic facts such as
  Findings and Evidence or link Decisions, while terminal and structural Task
  mutations respect the active exclusive claim;
- one primary focus per Session, expressed as entity plus context path;
- abnormal exit retains claims and marks the Session potentially stale;
- explicit takeover of a stale claim, with prior claimant and last-activity
  information, instead of silent TTL release;
- release of old-Branch claims on Session Branch switch by default, with an
  explicit keep option;
- explicit claim transfer or force provenance when another Session must take
  terminal or structural action under an active exclusive claim;
- shared claimants may add Verification and other non-destructive updates, but
  terminal or structural changes require a unique claimant or explicit force
  provenance;
- atomic “select next runnable Task and claim it” behavior;
- explicit atomic Session Workspace/Branch switches with deterministic Focus
  preservation/clearing, and atomic Session end that leaves Handoff creation
  separate;
- a Claim active-set invariant of none, one exclusive, or one-or-more shared,
  with explicit provenance-bearing mode change and takeover.

### Deterministic Agent interface

V1 includes:

- versioned semantic operations that create canonical relations, Events,
  ChangeSets, and WorkStateCommits without asking the Agent to manage those
  primitives, plus runtime-only operations that create runtime transitions and
  provenance Events without empty WorkStateCommits;
- a canonical typed relation vocabulary plus `related_to` with a custom label
  and an optional explanation;
- distinct `context`, `why`, and `history` query semantics;
- deterministic Context Resolver profiles `brief`, `normal`, and `full`;
- fixed profile contents and the confirmed `P0` through `P9` priority order;
- a hard context budget with item-priority omission rather than string
  truncation, including an omission summary;
- path-sensitive resolution for scoped Knowledge and Decisions;
- exclusion of superseded or invalidated content by default, with a causal
  exception when it explains current state;
- deterministic `next` resolution across active Workspace and Work Branch,
  active scope and Plan path, executable Task descendants, dependency
  readiness, lifecycle eligibility, priority, explicit manual order, and
  Session/Claim coordination, in that order; priority precedes manual order,
  and manual order cannot override readiness or eligibility;
- a lightweight Attempt lifecycle with `running`, `succeeded`, `failed`, and
  `inconclusive` states plus a one-shot shortcut;
- a deterministic single-target Verification command wrapper that captures
  applicable execution Evidence and the actual verified Resource state,
  including relevant uncommitted Git changes rather than Git HEAD alone; one
  Evidence object may support multiple separately recorded judgments;
- an Agent-readable operation protocol and concise, actionable errors; the
  concrete encoding and command spelling are not fixed by this baseline;
- Agent adapters based on the same CLI and semantic operation contract.

### Source-state traceability and portability

V1 includes:

- a Resource Adapter boundary with stable logical Resource identity,
  rebindable locators, immutable ResourceObservations, deterministic scoped
  fingerprints and differences, and conservative `applicable`/`stale`/
  `unknown` combination across Resource and Work-State basis;
- independently controlled inclusion or exclusion of WorkVCS data from Git;
- self-contained Bundle interchange that preserves Store identity for ordinary
  copy/move/export/import/backup/restore, distinguishes an explicit Store fork,
  validates/deduplicates immutable objects by content, detects same-Store ref
  divergence, and never resurrects imported active Runtime Coordination;
- HOT/WARM/COLD projections that remove terminal history from default working
  context without destructively deleting core provenance.
- the confirmed logical schema families and authority split in
  [Logical Schema Boundaries](../architecture/logical-schema-boundaries.md),
  including ObjectIdentity family ownership, immutable Verification closure,
  stable Relation-key reuse, staged import, and required Bundle local-reference
  closure;

## Explicitly deferred beyond V1

- transcript parsing or automatic extraction of Findings and Decisions;
- LLM-generated semantic records without explicit Agent confirmation;
- embeddings, vector search, and semantic retrieval;
- LLM-based semantic merge or natural-language conflict detection;
- automatic knowledge distillation;
- hooks that infer and prompt for possible semantic records;
- Agent launching, scheduling, orchestration, or automatic execution;
- cloud synchronization, distributed collaboration, and a replication
  protocol for branch refs and object exchange;
- cross-Store live Knowledge federation, a global Knowledge Space service,
  remote subscriptions, and live cross-Store references;
- destructive compaction of core Decision, Finding, Knowledge, ChangeSet, or
  WorkStateCommit history;
- a required TUI, GUI, or human-first storage format.

Large Evidence may later receive configurable retention policies, but critical
metadata, digests, and provenance remain preserved. Derived caches, indexes,
and projections may be regenerated and garbage-collected.

## Open implementation and later-architecture boundary

The earlier Verification-representation and cross-Workspace Handoff questions
are closed by Accepted ADRs 0001 and 0002. The following remain deliberately
unresolved:

- final equal-candidate tie-breaker for `next`;
- final CLI spelling, protocol encoding, complete operation/error catalogue,
  and any future semantic AC-waiver operation;
- concrete performance indexes, programming language/SQLite binding, exact
  canonical JSON profile, object layout, physical sibling-order
  representation, checkpoint strategy, and typed-projection count/shape;
  UUIDv7/BLOB IDs, BLAKE3-256 digests, JSON/timestamp storage, core
  FK/transaction policy, and executable schema assembly are closed by
  [ADR-0006](../decisions/adr/0006-sqlite-physical-schema-v0.1.md) and
  [ADR-0007](../decisions/adr/0007-schema-v0.1-assembly-install-and-integrity.md);
- exact Resource path/glob normalization and persisted observation capture
  policy outside Verification/explicit snapshots;
- source-stale Context policy, access/security model, exchange API,
  Bundle container/profile details, and import recovery-state vocabulary;
- any cross-Store live federation or distributed synchronization protocol.

## Scope-control rule

A concept appearing in the V1 model does not authorize an unconfirmed
implementation choice. Detailed database schema, programming language,
identity scheme, protocol encoding, exact Resource Adapter rules, specific CLI
spelling, sync transport, UI, and deployment model require later planning and
explicit confirmation when they become material decisions. Current repository
policy may record such a decision as an ADR.

The broader Record taxonomy remains Open only for `Blocker`, `Review`, and
`Note`; no current confirmed requirement makes them distinct V1 Record kinds.
