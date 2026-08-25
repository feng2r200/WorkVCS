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
  reusable Knowledge is managed alongside work and may also be published to a
  Knowledge Space.
- **Work-State:** an adjective, as in Work-State DAG or Work-State versioning.

These forms are related but not interchangeable: a Knowledge Space is outside
any one Workspace's Work State.

## Problem

Agent work is usually fragmented across sessions and transcripts. Important
state is lost or becomes difficult to distinguish from stale discussion:

- goals, plans, and tasks drift without a durable lineage;
- decisions lose their rationale and supporting evidence;
- findings and failed attempts are rediscovered or repeated;
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

WorkVCS provides:

- semantic operations to create and update work objects, change Task status,
  adjust ordering, and query current or historical work state without fixing
  the final command spelling;
- a versioned Work-State DAG with commit, branch, diff, merge, restore, and
  lineage semantics;
- a structured Work Graph for Goal, Plan, Task, Decision, Knowledge, and
  semantic records;
- immutable provenance for Sessions, ChangeSets, Events, and Evidence;
- explicit coordination state for focus, claims, and merge-in-progress;
- a deterministic Context Resolver for `context`, `next`, and `why`;
- low-coupling source-state drift evidence without making Git the database;
- portable Stores and cross-Workspace Knowledge Spaces.

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
Decision, or Knowledge statement. V1 does not infer these records from a
transcript.

## Design principles

1. **Work state is the versioned object.** A Work Branch expresses a divergent
   work or knowledge state, not merely Agent concurrency and not a Git branch.
2. **Current state, runtime coordination, and provenance are distinct.** They
   have different lifecycle and merge behavior.
3. **Top-down and bottom-up work are equally valid.** A Task may exist before a
   Plan or Goal is discovered; later organization must preserve identity and
   history.
4. **Cross-Workspace sharing transfers knowledge, not execution graphs.** Work
   Graphs remain Workspace-local; Knowledge may be published to a Knowledge
   Space.
5. **Semantic intent is the Agent API.** WorkVCS derives low-level relations,
   events, and commits from an atomic high-level versioned operation; a
   runtime-only operation produces runtime state and Events without a commit.
6. **Context is deterministic and bounded.** V1 uses explicit structure,
   scope, causality, state, provenance, recency, profiles, and budgets rather
   than LLM or embedding inference.
7. **History is preserved by default.** Terminal state exits the current
   working set but remains queryable for `why`, `history`, and restoration.

## Current product stage

This repository currently contains a confirmed design baseline, not an
implemented WorkVCS runtime. Language selection, detailed storage schema, and
the final CLI command surface remain implementation-planning work unless a
confirmed document says otherwise.
