# Knowledge Federation and Store Portability

This document defines the confirmed Store-local KnowledgeExposure model and
portable Store semantics. It realizes
[ADR-0004](../decisions/adr/0004-knowledge-federation-and-store-portability.md).

## KnowledgeExposure

Workspace Knowledge retains its Workspace-local identity and immutable state
versions. Sharing creates a separate stable KnowledgeExposure that binds one
specific source version:

```text
Workspace A
  Knowledge K-17 / KnowledgeVersion KV-31
                  |
                  v
Knowledge Space KS-1
  KnowledgeExposure KE-9 -> A/K-17/KV-31
```

`KE-9` is neither a copy of the Knowledge statement nor an alias that follows
`latest`. If `K-17` later has `KV-40`, publishing it creates another Exposure.
The new Exposure may coexist with `KE-9`; replacement uses a new Exposure plus
explicit withdrawal of `KE-9`. The old binding remains immutable history.

V1 Knowledge Space evolution consists of append-only Exposure history and a
current availability projection. It does not introduce an independent
Knowledge Space branch/merge/restore DAG.

## Consult versus adopt

Consulting an Exposure in Context is read-only and creates no Work-State
mutation in the consuming Workspace. Adoption is explicit:

```text
Workspace B Knowledge K-B-7
  derived_from: KE-9
  source: A/K-17/KV-31
```

The adopted Knowledge has Workspace B identity and belongs to B's Work-State
DAG. Later source releases do not mutate it. B may explicitly adopt a later
Exposure and supersede its earlier local Knowledge.

Knowledge Space offers knowledge; the consuming Workspace chooses what enters
its worldview.

## Availability, withdrawal, and source drift

An Exposure can leave the current available set while its history remains. V1
semantic state is the closed vocabulary `active`/`withdrawn`; it does not add a
federation-level `superseded` state or relation.

A relevant source change or source invalidation does not silently withdraw an
Exposure. It changes a derived source-status/applicability projection and
warns that the binding may be stale. Exposure semantic transition remains
explicit because it has cross-Workspace consequences.

Knowledge Space queries distinguish:

- **current:** active Exposures eligible for default use;
- **historical:** withdrawn Exposure history;
- **source-stale:** an otherwise available Exposure whose source has relevant
  drift or invalidation.

The source-status projection uses `current`, `stale`, `unknown`, and
`unresolved`.

The exact default-context suppression or ranking policy for source-stale
Exposure remains Open.

## Store identity and fork

Backup/restore, move, copy, and export/import preserve Store identity by
default. These operations move one Store lineage.

An explicit Store fork creates a new Store identity and records
`derived_from_store` lineage. Fork and copy are different semantic operations;
import never invents a new Store identity merely because it runs on another
machine. A fork preserves internal local IDs by default under the new Store
namespace, so portable complete references use `(store_id, local_id)`.

## Bundle interchange

A Bundle is self-contained for its declared profile and includes:

- a manifest;
- canonical logical history and logical identities;
- required immutable objects;
- format/schema/capability metadata;
- integrity metadata.

Full and compact profiles are allowed. Compact may omit rebuildable caches,
indexes, projections, and optional checkpoints, but cannot omit anything
required to reconstruct and validate included canonical history. The physical
SQLite database file is not the interchange contract.

## Import rules

### Runtime Coordination

Session, Claim, and merge-attempt provenance is preserved, but imported
Runtime Coordination is never automatically active. Claims import inactive;
Sessions do not resume blindly; provisional merges require explicit recovery.
The exact recovery-state vocabulary remains an implementation decision.

### Same Store identity

Import compares local and incoming Commit DAG ancestry and refs:

- an incoming descendant can be recognized as a fast-forward;
- divergent refs produce an explicit divergence result and preserve both
  recoverable states;
- last-write-wins overwrite is forbidden.

V1 need not implement automatic distributed merge or live synchronization.

### Content-addressed objects

Imported objects are deduplicated by declared content hash and accepted only
when the bytes match that hash. V1 Store content/state digest uses BLAKE3-256;
Bundle container/archive encoding remains Open.

### Resource bindings

Logical Resource identity is portable. Environment-specific locators are not
assumed valid after import and may be unresolved until explicitly rebound. Old
locators may remain provenance or diagnostics, but applicability requires a
new observation proving content/source continuity.

### External provenance

If adopted Knowledge refers to an Exposure whose source Workspace is not
included, the Bundle preserves a portable unresolved external provenance
reference containing the Exposure, source Knowledge version, Workspace, and
Store identity/lineage. The reference may later resolve; V1 does not require
network lookup.

An ExternalObjectRef is allowed only in explicit federation, lineage, or
imported provenance metadata. It never replaces a canonical Relation endpoint.
When an included object has a canonical Relation to another Store-local object,
the exporter includes the Relation and endpoint object in the required local
reference closure. The endpoint object's own foreign-source provenance may
remain an ExternalObjectRef.

## Cross-Store boundary

V1 Knowledge Spaces are Store-local. Export/import and retained adoption
provenance carry knowledge across Store boundaries. A global Knowledge Space,
remote subscriptions, cross-Store live references, and distributed federation
are deferred beyond V1.

## Current implementation boundary

The V1 Exposure lifecycle is `active`/`withdrawn`, source-status projection is
`current`/`stale`/`unknown`/`unresolved`, full canonical import is same-Store
only. A different Store is opened separately, explicitly forked, or selectively
adopted through Knowledge/Evidence and ExternalObjectRef provenance; its
canonical DAG is never merged directly into the local namespace. The physical
identity/digest/FK boundaries are fixed by
[Physical Schema v0.1 Contract](physical-schema-v0.1.md). The complete
executable schema, performance indexes, exchange/access API, authorization
model, source-stale Context policy, Bundle container/profile,
streaming/compression, runtime recovery vocabulary, exact canonical JSON
profile, and cross-Store live protocol remain unfixed.
