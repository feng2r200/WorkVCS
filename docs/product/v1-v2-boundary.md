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

V1 requires publish/read behavior with traceable source provenance. The
Knowledge Space's own publication versioning and concurrency mechanism is not
yet confirmed and must be resolved by an ADR before implementation; no mutable
global Knowledge DAG is implied by this baseline.

The confirmed V1 storage direction is SQLite metadata plus a content-addressed
object store. This is an implementation direction, not a domain invariant;
replacing it requires an accepted ADR. The exact schema, object layout, and
implementation language are not fixed by this baseline.

### Versioned work and cognition

V1 versions:

- Goal, Plan, Task, Decision, Knowledge, Record, Acceptance Criterion, and
  typed relations;
- Finding, Assumption, Question, Attempt, Risk, Handoff, Verification, and
  Evidence semantics;
- top-down Goal -> Plan -> SubPlan -> Task decomposition and bottom-up
  Task -> Finding -> Plan -> Goal discovery;
- Plan and Task hierarchy, mixed Plan/Task siblings, references, explicit
  order, dependency, and priority;
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
- optional Acceptance Criteria, with all mandatory criteria verified before
  automatic Task completion when criteria exist.

The exact persistent identity scheme is not fixed by this baseline. Stable
Task-local Acceptance Criterion identities are required so references survive
wording changes.

### Work-State versioning

V1 includes:

- immutable WorkStateCommit objects and a commit DAG;
- automatic commits for successful semantic mutations;
- atomic ChangeSets and a batch/transaction interface;
- branches from any historical WorkStateCommit;
- diff, history, why, restore, and lineage queries;
- persistent three-way merge with `start`, `resolve`, `continue`, and `abort`;
- two-parent merge commits and `ours`, `theirs`, and `custom` resolution;
- deterministic merge classification as `AUTO`, `CONFLICT`, or `REVIEW`;
- deterministic review for active Decisions sharing an exclusive scope and
  subject but selecting different choices;
- retention of useful findings, attempts, verification, evidence, and
  completed evaluation work from unselected branches;
- preservation of the source Branch after merge.

### Sessions and concurrency

V1 includes:

- Session provenance independent of Work Branch state;
- multiple Sessions on the same Work Branch without a branch-wide lock;
- optimistic concurrency using an expected base and deterministic
  reconciliation when safe;
- exclusive Task claims by default, with explicit shared claims;
- one primary focus per Session, expressed as entity plus context path;
- abnormal exit retains claims and marks the Session potentially stale;
- explicit takeover of a stale claim, with prior claimant and last-activity
  information, instead of silent TTL release;
- release of old-Branch claims on Session Branch switch by default, with an
  explicit keep option;
- unique-claimant or explicit-force requirements for terminal or structural
  changes under a shared claim;
- atomic “select next runnable Task and claim it” behavior.

### Deterministic Agent interface

V1 includes:

- versioned semantic operations that create canonical relations, Events,
  ChangeSets, and WorkStateCommits without asking the Agent to manage those
  primitives, plus runtime-only operations that create runtime transitions and
  provenance Events without empty WorkStateCommits;
- a canonical typed relation vocabulary plus `related_to` with a labeled,
  reasoned escape hatch;
- distinct `context`, `why`, and `history` query semantics;
- deterministic Context Resolver profiles `brief`, `normal`, and `full`;
- fixed profile contents and the confirmed `P0` through `P9` priority order;
- a hard context budget with item-priority omission rather than string
  truncation, including an omission summary;
- path-sensitive resolution for scoped Knowledge and Decisions;
- exclusion of superseded or invalidated content by default, with a causal
  exception when it explains current state;
- a lightweight Attempt lifecycle plus one-shot shortcut;
- a Verification command wrapper that captures command, working directory,
  start/end, exit status, duration, output artifact/digest, Git SHA when
  applicable, and result;
- an Agent-readable operation protocol and concise, actionable errors; the
  concrete encoding and command spelling are not fixed by this baseline;
- Agent adapters based on the same CLI and semantic operation contract.

### Source-state traceability and portability

V1 includes:

- low-coupling source-state traceability that reports the observed source
  state, its comparison baseline, and the resulting difference without making
  Git the Work-State database; when source state supports a Verification, its
  provenance identifies the source observation, including the Git SHA when
  Git is applicable;
- independently controlled inclusion or exclusion of WorkVCS data from Git;
- portable export/bundle behavior suitable for moving a Store without a live
  distributed synchronization protocol;
- HOT/WARM/COLD projections that remove terminal history from default working
  context without destructively deleting core provenance.

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
- destructive compaction of core Decision, Finding, Knowledge, ChangeSet, or
  WorkStateCommit history;
- a required TUI, GUI, or human-first storage format.

Large Evidence may later receive configurable retention policies, but critical
metadata, digests, and provenance remain preserved. Derived caches, indexes,
and projections may be regenerated and garbage-collected.

## Scope-control rule

A concept appearing in the V1 model does not authorize an unconfirmed
implementation choice. Detailed database schema, programming language,
identity scheme, protocol encoding, source-state evidence data model, specific
CLI spelling, sync transport, UI, and deployment model require later planning
or an accepted ADR.
