# ADR-0130: Phase 4AA Store Lineage Journal

- **Status:** Accepted for implementation
- **Date:** 2026-08-30

## Context

ADR-0004 and INV-060/INV-080 require ordinary Store movement to preserve Store
identity while explicit Store forks create a new Store namespace with
source-store lineage. The v0.1 schema already contains `store_lineage`, but the
Engine and CLI did not yet expose a narrow way to record or inspect that
infrastructure provenance.

## Decision

1. Add a typed `LineageId` for `store_lineage.lineage_id`.
2. Add Engine-level Store lineage APIs for append-only record, show, and list.
3. A Store lineage record contains `source_store_id`, `derivation_kind`, a
   canonical JSON object `source_root_descriptor`, optional
   `source_bundle_digest`, and `created_at_us`.
4. `source_root_descriptor_json` is written as WorkVCS fixed-point canonical
   JSON and revalidated on read.
5. Recording rejects same-Store lineage because ordinary copy/move/import keeps
   Store identity and should not create fork lineage.
6. CLI adds `workvcs store lineage-record`, `workvcs store lineage-show`, and
   `workvcs store lineage-list`.
7. List supports filters by source Store, derivation kind, and source Bundle
   digest.
8. This slice does not perform Store fork creation, Bundle import activation,
   canonical row ingestion, ref movement, runtime recovery, migration, remote
   synchronization, or final Bundle container definition.

## Consequences

Store lineage is now auditable through the Engine facade and CLI without
exposing SQLite handles or expanding Store identity itself. Later Store fork or
Bundle import application slices can reuse this journal after their own
validation and activation boundaries are implemented.

## Implementation Finding

No new contract ambiguity was found. The existing schema gives StoreLineage an
independent infrastructure identity, so this slice keeps it separate from
WorkState history and Bundle import attempts.
