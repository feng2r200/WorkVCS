# System Boundaries

## Boundary model

WorkVCS separates data storage, versioned work, reusable knowledge, execution
provenance, and associated source state.

```text
Store (portable data boundary)
|
|-- Workspace A (Work-State versioning boundary)
|   |-- Versioned Work State
|   `-- Work Branch DAG
|
|-- Workspace B
|
|-- Knowledge Space (cross-Workspace knowledge boundary)
|
|-- Sessions and immutable provenance
`-- Evidence object storage
```

The following inequalities are architectural constraints:

```text
Store != Workspace
Workspace != Repository
Session != Workspace
Knowledge Space != Work Graph
Work Branch != Git branch
```

## Store

A Store is the self-contained data and portability boundary. One Store may
contain multiple Workspaces and Knowledge Spaces plus Sessions, provenance,
evidence, and projections.

The confirmed V1 implementation direction is:

```text
Store
|-- metadata (SQLite)
`-- content-addressed object store
```

This direction is revisable only through an accepted ADR; it is not a domain
invariant. It does not freeze a SQL schema, object layout, implementation
language, or project-marker format.

## Workspace

A Workspace answers one question:

> Which Goal, Plan, Task, Decision, Knowledge, and Record state evolves in one
> branch/diff/merge/restore history?

Workspace is a logical versioning boundary, not an assumption about filesystem
layout. Valid configurations include:

- multiple Workspaces for different parts of one monorepo;
- one Workspace referencing multiple repositories and directories;
- multiple Workspaces associated with the same repository or directory;
- a Workspace with no Git repository.

Every versioned mutation has exactly one active Workspace and Work Branch.
Cross-Workspace Task containment and dependency are not allowed in V1 because
they would couple independent branch and restore histories.

## Knowledge Space

A Knowledge Space is the long-term reuse boundary above Workspace. It carries
published Knowledge statements and source provenance, not the originating
execution graph.

```text
Workspace A Finding/Decision/Verification
                 |
                 `-- publish Knowledge K-17
                              |
                              v
                    Knowledge Space agent-tooling
                              |
                              `-- readable by Workspace B
```

Workspace B may use K-17 in context and trace its origin. It does not inherit
Workspace A's Tasks, Plans, Claims, or current execution state.

The confirmed boundary requires publish/read and origin provenance. Whether a
Knowledge Space uses its own DAG, an append-only publication log, or another
concurrency mechanism is still open and must be settled by an ADR before
implementation. This baseline does not authorize mutable global Knowledge
state with unspecified conflict behavior.

## Session and Context Set

A Session is an Agent execution provenance boundary. It may temporarily consult
multiple Workspaces and Knowledge Spaces:

```text
Session S-10
|-- Context Set: Workspace A, Workspace B, Knowledge Space K
|-- Active Workspace: A
|-- Active Branch: main
`-- Focus: T-18 via G-1 -> P-3 -> T-18
```

Read scope may be multi-Workspace. Mutation scope is one active Workspace and
one active Branch. Mutating another Workspace or Branch requires an explicit
switch and an immutable provenance event.

Co-consulting Workspaces in a Session records a time-bounded provenance fact;
it does not permanently relate the Workspaces. Current Focus and Claims are
runtime state. The Session timeline and automatic end diff are immutable
provenance.

## Source-state traceability

WorkVCS does not use Git as its core database. Work-State data may enter Git or
remain independent under separate control. V1 must support reviewable drift
inspection so an Agent can identify whether associated source state changed
and state the exact comparison basis without tightly coupling every
WorkStateCommit to a Git commit.

The concrete entity model and captured fields for source-state evidence are
not fixed by this baseline.

## State ownership matrix

| State | Owner | Versioned with Work Branch | Immutable history | Restore as current state |
|---|---|---:|---:|---:|
| Goal / Plan / Task | Workspace Work State | Yes | Yes | Yes |
| Decision / Knowledge / Record | Workspace Work State | Yes | Yes | Yes |
| Acceptance Criterion / typed relation | Workspace Work State | Yes | Yes | Yes |
| active Session / Focus / Claim | Runtime Coordination | No | Changes emit Events | No |
| merge-in-progress | Runtime Coordination | No | Attempt and resolution Events | No |
| Event / Session timeline / ChangeSet / WorkStateCommit | Provenance and version history | No | Yes | Historical Work State only |
| Verification / Evidence | Provenance/object storage | No | Yes | No |
| context / next / why / ready / progress | Derived Projection | No | Recomputable | No |

## Working set and retention boundary

The Store distinguishes relevance from existence:

- **HOT:** active and relevant state used by default projections;
- **WARM:** terminal or superseded state indexed for `why` and `history` but
  omitted from ordinary context;
- **COLD:** old raw Events and Evidence loaded only for explicit
  trace or restoration.

This is a projection and access policy, not destructive revision of history.
Derived caches and indexes are always regenerable. Configurable retention may
archive large Evidence while preserving critical metadata and digests. Core
Decision, Finding, Knowledge, ChangeSet, Event, and WorkStateCommit lineage is
not destructively compacted by default.

## Integration boundary

The WorkVCS engine exposes one semantic operation contract through an
Agent-readable protocol. Codex, Claude, OpenCode, and other harnesses receive
adapter-specific instructions or skills that translate the same canonical
workflow. The engine does not launch, select, or orchestrate Agents.

Human-friendly presentation may be added above the protocol. It cannot weaken
accurate Agent interpretation, actionable errors, or machine-readable
provenance. The concrete protocol encoding is not fixed by this baseline.
