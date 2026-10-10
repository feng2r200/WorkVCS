# Product Definition

## Product identity

- **Name:** WorkVCS
- **Role:** Agent-facing version control for explicit work and knowledge state
- **Category:** a local-first, CLI-first, Agent-first versioned work and
  knowledge state system

WorkVCS manages the explicit state an Agent needs to continue, explain, review,
branch, merge, and restore work independently of source-code version control.
It does not claim to manage model internals or chain of thought.

## Canonical terminology

- **Work State:** the versioned state of one Workspace, including work,
  cognition, Workspace-scoped Knowledge, Acceptance Criteria, and typed
  relations.
- **Work & Knowledge State:** the product-level phrase emphasizing that
  reusable Knowledge is managed alongside work and may also be made available
  through a Knowledge Space.
- **Work-State:** an adjective, as in Work-State DAG or Work-State versioning.

These forms are related but not interchangeable: a Knowledge Space is outside
any one Workspace's Work State.

## Problem

Agent work is usually fragmented across sessions and transcripts. Important
state is lost or becomes difficult to distinguish from stale discussion:

- goals, plans, and tasks drift without a durable lineage;
- decisions lose their rationale and supporting evidence;
- findings and failed attempts are rediscovered or repeated;
- valuable cognition can be silently skipped or written under the wrong
  project when an ambient repository, mirror, or CWD is mistaken for semantic
  ownership;
- a new Session cannot deterministically recover the smallest sufficient
  context;
- concurrent Sessions lack explicit focus and claim coordination;
- work-state alternatives cannot be branched and semantically merged;
- source changes and work-state claims can drift without an auditable
  comparison basis.

Task tracking alone does not solve this problem. The managed object is the
evolving **Work & Knowledge State**, including both what the project intends to
do and why its current state should be believed.

## Product promise

The confirmed product promise spans active capability and accepted capability
whose core foundation is not yet integrated. Runtime support is stated
separately under Current product stage. The product contract provides:

- semantic operations to create and update work objects, change Task status,
  adjust ordering, and query current or historical work state without fixing
  the final command spelling;
- a versioned Work-State DAG with commit, branch, diff, merge, restore, and
  lineage semantics;
- a structured Work Graph for Goal, Plan, Task, Decision, Knowledge,
  Verification, and semantic records;
- immutable provenance for Sessions, ChangeSets, Events, and Evidence;
- explicit coordination state for focus, claims, and merge-in-progress;
- a deterministic Context Resolver for `context`, `next`, and `why`;
- low-coupling source-state drift evidence without making Git the database;
- portable Stores and cross-Workspace Knowledge Spaces;
- stable ProjectRef ownership resolved from tool-neutral semantic Project,
  repository, and CWD locators without equating any locator with the project;
- journal-first durable capture after value admission, including recoverable
  pending routing when the owning project is not yet bound;
- an explicitly authorized continuation from a newly admitted cognition intent
  to its exact existing, fully valid, non-shared binding, while journal-only
  admission remains the default; and
- explicit cross-project CaptureGroups with one canonical mutable Record and
  immutable secondary references that are not semantic authorities.

## Primary user and interaction model

The primary user is an Agent such as Codex, Claude, OpenCode, or another coding
harness. The CLI and semantic operation contract are the core interface.
Agent-specific instruction files and skills are adapters; the engine does not
depend on a particular Agent product.

Humans may inspect or operate the system through an Agent. Human-readable
storage is not a product requirement, but every command must remain
human-debuggable: errors explain what happened, why it happened, the relevant
current state, and a safe next action.

## Product boundary

WorkVCS does not launch Agents, execute their reasoning, or orchestrate their
work. An Agent decides the semantic intent and calls WorkVCS; WorkVCS records
and versions the resulting state atomically.

```text
Agent
  -> versioned semantic operation
WorkVCS
  -> atomic ChangeSet
  -> immutable Events and provenance
  -> WorkStateCommit
  -> deterministic projections
```

Pure Runtime Coordination operations update runtime state atomically and emit
provenance Events without creating empty WorkStateCommits. The system
automatically records facts it can observe mechanically, such as a
Session start, branch switch, claim, status change, or commit. The Agent must
explicitly record meaning only it knows, such as a Finding, Assumption,
Question, Decision, Risk, or Knowledge statement. V1 does not infer these
records from a transcript.

## Design principles

1. **Work state is the versioned object.** A Work Branch expresses a divergent
   work or knowledge state, not merely Agent concurrency and not a Git branch.
2. **Current state, runtime coordination, and provenance are distinct.** They
   have different lifecycle and merge behavior.
3. **Top-down and bottom-up work are equally valid.** A Task may exist before a
   Plan or Goal is discovered; later organization must preserve identity and
   history.
4. **Cross-Workspace sharing transfers knowledge, not execution graphs.** Work
   Graphs remain Workspace-local. V1 exposes one immutable Workspace Knowledge
   version through a stable Store-local KnowledgeExposure. Consulting it is
   read-only; explicit adoption creates Workspace-local Knowledge with source
   provenance.
5. **Semantic intent is the Agent API.** WorkVCS derives low-level relations,
   events, and commits from an atomic high-level versioned operation; a
   runtime-only operation produces runtime state and Events without a commit.
6. **Context is deterministic and bounded.** V1 uses explicit structure,
   scope, causality, state, provenance, recency, profiles, and budgets rather
   than LLM or embedding inference.
7. **History is preserved by default.** Terminal state exits the current
   working set but remains queryable for `why`, `history`, and restoration.
8. **Value admission precedes routing.** A missing ProjectRef or binding cannot
   turn valuable content into a no-record decision; read-only/no-record remains
   zero-write, and admitted content is journaled before Store delivery.
9. **Project ownership is semantic and tool-neutral.** Explicit ProjectRef,
   verified semantic Project, Git, and CWD are ranked locator evidence;
   provider-specific adapters do not become core product dependencies.

## Current product stage

This repository contains the confirmed WorkVCS design baseline and a locally
release-ready V1 implementation in Rust, followed by focused post-V1 local
improvements. The crate and CLI package version is currently `0.2.0`; an
external public release is still a separate decision, and that package number
must not be confused
with the bounded V1 product-maturity judgment.

Implementation readiness is tracked in
[V1 Readiness Ledger](../provenance/v1-readiness-ledger.md). That ledger
distinguishes implemented behavior, smoke proof, dogfood proof, and remaining
V1 exclusions. The [V1 Release Gate Matrix](../provenance/v1-release-gate-matrix.md)
records the exact locally validated candidate boundary. No public release, tag,
push, deployment, or production activation follows from that local judgment;
those remain separate decisions and operations.

The confirmed logical persistence architecture defines canonical
Commit/ChangeSet history, immutable Entity/Relation versions, rebuildable
projections, checkpoints, KnowledgeExposure, and portable Bundle semantics. It
also defines ObjectIdentity/typed-family ownership and Runtime/Provenance/
Resource/Federation infrastructure boundaries.

ADR-0513 additionally confirms the ProjectRef control plane, journal-first
capture routing, cross-project CaptureGroup, and registry v2 migration
contracts. Those contracts are current product authority and the initial
eleven-round local delivery roadmap is complete. The installed CLI exposes
migration preview and evidence-bound repair, digest-locked migration/rollback,
versioned reads, separate read and journal activation markers, target-neutral
`cognition_v2` admission, immutable recovery events, deterministic
projections, status-first binding convergence, guarded primary delivery,
complete receipts, CaptureGroup secondary references, missing-only retry, and
Store-free recall by secondary ProjectRef.

ADR-0519's narrower next increment is implemented in the source tree: one
per-invocation `capture --deliver-existing-binding` continuation, one read-only
open-operation inventory, and a dedicated clean-unbound classification. The
continuation reuses the existing recovery state machine and succeeds only after
a current receipt. It does not infer authority from control-plane health and
does not cover bootstrap, conflicting or shared ownership, target changes,
CaptureGroups, or historical batch recovery. Exact local installed adoption
and one bounded same-binding canary are complete. The installed cognition
`capture` command still remains admission-only by default; only an invocation
carrying the explicit option may continue through this route.

Isolated fixtures remain the authority for injected faults and concurrency.
The bounded configured-local canary proves one canonical work-governance
Record, one immutable Hernes association with a byte-stable Hernes Store, and
zero-write replay while the earlier legacy intent remains unchanged. Global
Hook activation, historical backfill, live rollback after v2 use, push,
release, and deployment remain separate future decisions.
