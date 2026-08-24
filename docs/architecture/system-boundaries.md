# System Boundaries

## Boundary model

WorkVCS separates physical storage, versioned work, reusable knowledge,
execution provenance, and external resources.

```text
Store (portable physical boundary)
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

External Resources
|-- Git repository
|-- directory
`-- other anchored material
```

The following inequalities are architectural constraints:

```text
Store != Workspace
Workspace != Repository
Workspace != Resource
Session != Workspace
Knowledge Space != Work Graph
Work Branch != Git branch
```

## Store

A Store is the self-contained data and portability boundary. One Store may
contain multiple Workspaces and Knowledge Spaces plus Sessions, provenance,
evidence, and projections.

The confirmed default for V1 implementation planning is:

```text
Store
|-- structured metadata, transactions, indexes, projections (SQLite)
`-- immutable or large content (content-addressed object store)
```

This default is revisable only through an accepted ADR; it is not a domain
invariant. It does not yet freeze a SQL schema, object layout, checkpoint
interval, implementation language, or project-marker format.

A project-local hidden directory may be one initialization/deployment mode, and
a global Store with a small project locator may be another. Neither deployment
form changes the domain boundary: the Store, not the current directory, owns
the data.

## Workspace

A Workspace answers one question:

> Which Goal, Plan, Task, Decision, Knowledge, and Record state evolves in one
> branch/diff/merge/restore history?

Workspace is a logical versioning boundary, not an assumption about filesystem
layout. Valid configurations include:

- multiple Workspaces for different parts of one monorepo;
- one Workspace referencing multiple repositories and directories;
- multiple Workspaces referencing the same Resource;
- a Workspace with no Git Resource.

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

## Resource and Resource Anchor

A Resource represents external material such as a Git repository or directory.
Workspace and Resource are N:M. WorkVCS does not use Git as its core database.

A Resource Anchor captures reviewable external state at meaningful points such
as Session start, Verification, Task completion, an important Decision, merge,
or explicit snapshot. A Git anchor may include HEAD, branch, dirty state, and a
working-tree digest. A multi-resource source snapshot may group anchors.

Not every WorkStateCommit captures a fresh Resource Anchor. This keeps code
state and work-state history associated but not lock-stepped. Drift inspection
compares current Resource observations with an anchor relevant to the claim
being reviewed.

## State ownership matrix

| State | Owner | Versioned with Work Branch | Immutable history | Restore as current state |
|---|---|---:|---:|---:|
| Goal / Plan / Task | Workspace Work State | Yes | Yes | Yes |
| Decision / Knowledge / Record | Workspace Work State | Yes | Yes | Yes |
| Acceptance Criterion / typed relation | Workspace Work State | Yes | Yes | Yes |
| active Session / Focus / Claim | Runtime Coordination | No | Changes emit Events | No |
| merge-in-progress | Runtime Coordination | No | Attempt and resolution Events | No |
| Event / Session timeline / ChangeSet / WorkStateCommit | Provenance and version history | No | Yes | Historical Work State only |
| Verification / Evidence / Resource Anchor | Provenance/object storage | No | Yes | No |
| context / next / why / ready / progress | Derived Projection | No | Recomputable | No |

## Working set and retention boundary

The Store distinguishes relevance from existence:

- **HOT:** active and relevant state used by default projections;
- **WARM:** terminal or superseded state indexed for `why` and `history` but
  omitted from ordinary context;
- **COLD:** old raw Events, Evidence, and checkpoints loaded only for explicit
  trace or restoration.

This is a projection and access policy, not destructive revision of history.
Derived caches and indexes are always regenerable. Configurable retention may
archive large Evidence while preserving critical metadata and digests. Core
Decision, Finding, Knowledge, ChangeSet, Event, and WorkStateCommit lineage is
not destructively compacted by default.

## Integration boundary

The WorkVCS engine exposes one semantic operation contract and one stable JSON
protocol. Codex, Claude, OpenCode, and other harnesses receive adapter-specific
instructions or skills that translate the same canonical workflow. The engine
does not launch, select, or orchestrate Agents.

Human-friendly presentation may be added above the protocol. It cannot weaken
stable schemas, exit codes, actionable errors, or machine-readable provenance.
