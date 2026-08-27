# ADR-0004: Knowledge Federation and Store Portability

- **Status:** Accepted
- **Accepted by:** explicit user confirmation for decisions 171–188
- **Confirmation turn:** `804bc570-7f67-4c43-a6ef-e9140320d697`

**Subsequent resolution:**
[ADR-0006](0006-sqlite-physical-schema-v0.1.md) later fixes the V1 Exposure
lifecycle to `active`/`withdrawn`, replacement to new Exposure plus old
withdrawal, source status to `current`/`stale`/`unknown`/`unresolved`, and
bounded ExternalObjectRef/Store-lineage physical constraints. Earlier
`supersede` and Open-enum wording below records the state when ADR-0004 was
accepted and is superseded in those exact scopes.

## Context

The baseline required cross-Workspace Knowledge reuse and portable Stores but
left the Knowledge Space identity model and transport semantics Open. The
accepted decisions close the high-level federation and portability model while
leaving its physical schema and protocols unfixed.

## Decision

1. A Knowledge Space is Store-local in V1. It contains stable
   KnowledgeExposure identities that bind an originating Workspace Knowledge
   identity and one specific immutable KnowledgeVersion. Exposure does not
   copy or replace the source Knowledge identity and never follows source
   `latest` implicitly.
2. V1 has append-only Exposure history plus a current availability projection,
   not an independent Knowledge Space branch/merge/restore DAG. A new Exposure
   may coexist with or supersede an older Exposure.
3. Consulting an Exposure in another Workspace is read-only and does not create
   Workspace Knowledge. Adoption is an explicit versioned semantic operation
   that creates Workspace-local Knowledge with Exposure/source-version
   provenance. Later source releases do not mutate adopted Knowledge.
4. An Exposure may leave the current available set without deleting its
   history. Source Knowledge drift or invalidation produces a derived stale/
   source-status warning; it does not silently transition Exposure semantic
   state. Query semantics distinguish current, historical, and source-stale
   Exposure sets.
5. Copy, move, export/import, backup, and restore preserve Store identity by
   default. An explicit Store fork creates a new Store identity and records
   source-store lineage.
6. A Bundle is a self-contained interchange format with manifest, canonical
   logical history, required immutable objects, format/schema metadata, and
   integrity metadata. Full and compact profiles are allowed, but neither may
   omit data needed to validate canonical history. The Bundle format is not the
   physical SQLite file format, and rebuildable caches/projections need not be
   transported.
7. Import preserves Session, Claim, and merge-attempt provenance but never
   blindly resurrects active Runtime Coordination. Runtime recovery is
   explicit.
8. Importing the same Store identity compares Commit DAG ancestry and refs. A
   fast-forward is distinguishable from divergence; last-write-wins overwrite
   is forbidden and divergence preserves both recoverable states.
9. Object transport verifies declared content hashes and deduplicates by
   content. Logical Resource identity is portable; the environment locator may
   import unresolved and be explicitly rebound. A prior locator may remain
   diagnostic provenance but is not presumed valid.
10. Cross-boundary Knowledge provenance must remain either resolved or as a
    portable unresolved external reference, including source Knowledge version,
    Workspace, and Store lineage. V1 does not require online resolution.
11. Live cross-Store Knowledge federation, remote subscriptions, and a global
    Knowledge Space service are outside V1.

The later accepted [ADR-0005](0005-logical-schema-family-boundaries.md)
clarifies that unresolved external references are allowed metadata rather than
canonical Relation endpoints. Bundle export includes required Store-local
Relation endpoint objects before foreign source provenance may remain external.

## Consequences

- A Store can move without identity loss while an intentional fork remains
  distinguishable.
- A Workspace controls which shared knowledge enters its own Work-State DAG.
- Historical Exposure and adopted-Knowledge provenance survive withdrawal,
  source invalidation, and partial bundles.
- Portability cannot create ghost claims or active merge state.

## Deliberately not decided

- exact KnowledgeExposure lifecycle enum and source-status enum;
- exact Context policy for source-stale Exposure;
- access control, exchange API, CLI spelling, subscription protocol, or
  physical Knowledge Space tables;
- exact Bundle profile contents beyond canonical sufficiency, compression,
  archive container, or streaming protocol;
- exact runtime-recovery states after import;
- hash algorithm, canonical encoding, or cross-Store live synchronization.
