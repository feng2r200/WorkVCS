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

Changing this direction requires a later explicit confirmed decision. Current
repository policy may record that decision as an accepted ADR, but the ADR
format does not supply the confirmation. The direction is not a domain
invariant and does not freeze a SQL schema, object layout, implementation
language, or project-marker format.

A Store is self-describing for Store identity, format/schema version,
capabilities, and object-format version. Ordinary copy, move, export/import,
backup, and restore preserve Store identity; explicit fork creates a new
identity with source lineage. Bundle interchange is self-contained and is not
defined by copying the physical SQLite file. See
[Versioned-State Persistence Model](persistence-model.md) and
[Knowledge Federation and Store Portability](knowledge-federation-and-portability.md).
The current Store manifest may be migration-updated while each migration
remains immutable provenance.

## Workspace

A Workspace answers one question:

> Which Goal, Plan, Task, Decision, Knowledge, Record, and Verification state
> evolves in one branch/diff/merge/restore history?

Workspace is a logical versioning boundary, not an assumption about filesystem
layout. Valid configurations include:

- multiple Workspaces for different parts of one monorepo;
- one Workspace referencing multiple repositories and directories;
- multiple Workspaces associated with the same repository or directory;
- a Workspace with no Git repository.

Every versioned mutation has exactly one active Workspace and Work Branch.
Cross-Workspace Task containment and dependency are not allowed in V1 because
they would couple independent branch and restore histories.

Workspace identity is independent of mutable name and filesystem path.
Workspace and portable Resource identity have a many-to-many infrastructure
association outside Branch Work State. Association and binding changes emit
provenance but do not create WorkStateCommits.

## Knowledge Space

A Knowledge Space is the long-term reuse boundary above Workspace. It carries
Knowledge made available for reuse plus source provenance, not the originating
execution graph.

```text
Workspace A Finding/Decision/Verification
                 |
                 `-- expose Knowledge K-17
                              |
                              v
                    Knowledge Space agent-tooling
                              |
                              `-- usable by Workspace B
```

Workspace A exposes one specific immutable Knowledge version through a stable
Store-local KnowledgeExposure. Workspace B may consult the Exposure in Context
and trace its origin without creating Work-State mutation. Explicit adoption
creates B-local Knowledge with Exposure/source-version provenance; B does not
inherit A's Tasks, Plans, Claims, or current execution state.

Knowledge Space uses an immutable linear Exposure-transition history plus a
current availability projection and has no independent V1 branch/merge/restore
DAG. V1 semantic state is `active`/`withdrawn`; source drift creates a derived
`current`/`stale`/`unknown`/`unresolved` source status rather than silently
withdrawing an Exposure. Source-stale Context policy, access control, the
complete executable schema, and exchange API remain Open. Cross-Store live
federation is outside V1.

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

WorkVCS does not use Git as its core database. A Resource has stable logical
identity and a rebindable environment locator. WorkVCS Core requests scoped
observation, fingerprint, and difference from a Resource Adapter instead of
implementing Git-specific comparison.

Verification persists the ResourceObservation/fingerprint it used. Git-backed
observations represent the actual verified working state, including relevant
uncommitted changes rather than HEAD alone. Missing or incomparable Resources
produce `unknown`; locator rebind does not prove continuity. Mechanical drift
changes only Derived Projection until an Agent explicitly records semantic
cognition or re-verifies. Exact path normalization, adapter implementation,
and non-Verification capture policy remain Open.

## State ownership matrix

| State | Owner | Versioned with Work Branch | Immutable history | Restore as current state |
|---|---|---:|---:|---:|
| Goal / Plan / Task | Workspace Work State | Yes | Yes | Yes |
| Decision / Knowledge / Record | Workspace Work State | Yes | Yes | Yes |
| Verification | Workspace Work State | Yes | Yes | Yes |
| Verification Requirement | Owning AC / Workspace Work State | Yes | Yes | Yes |
| Acceptance Criterion / typed relation | Workspace Work State | Yes | Yes | Yes |
| EntityVersion / RelationVersion | Canonical version history | Selected by Branch | Yes | Historical state |
| ObjectIdentity + typed owner | Store-local addressable registry | No | Yes | No |
| KnowledgeExposure source binding | Store-local Knowledge Space | No independent DAG | Yes | Availability projection only |
| Resource | Store / Workspace association | No | Identity history | Rebind, not restore |
| ResourceBinding | Store environment current config | No | Changes emit Events | Rebind, not restore |
| ResourceObservation | Immutable provenance/object storage | No | Yes | No |
| active Session / Focus / Claim | Runtime Coordination | No | Changes emit Events | No |
| merge-in-progress | Runtime Coordination | No | Attempt and resolution Events | No |
| Event / Session timeline / ChangeSet / WorkStateCommit | Provenance and version history | No | Yes | Historical Work State only |
| Evidence | Provenance/object storage | No | Yes | No |
| checkpoint | Derived acceleration/object storage | No | Rebuildable | No |
| StoreManifest / migration history | Store infrastructure | No | Migration history yes | Current manifest only |
| BundleManifest / ImportAttempt | Transport / infrastructure provenance | No | Import attempt yes | No |
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
Decision, Finding, Verification, Knowledge, ChangeSet, Event, and
WorkStateCommit lineage is not destructively compacted by default.

## Integration boundary

The WorkVCS engine exposes one semantic operation contract through an
Agent-readable protocol. Codex, Claude, OpenCode, and other harnesses receive
adapter-specific instructions or skills that translate the same canonical
workflow. The engine does not launch, select, or orchestrate Agents.

Human-friendly presentation may be added above the protocol. It cannot weaken
accurate Agent interpretation, actionable errors, or machine-readable
provenance. The concrete protocol encoding is not fixed by this baseline.

The complete logical ownership matrix is
[Logical Schema Boundaries](logical-schema-boundaries.md). Confirmed SQLite
physical boundaries are defined separately in
[Physical Schema v0.1 Contract](physical-schema-v0.1.md); its executable schema
assembly remains a later stage.
