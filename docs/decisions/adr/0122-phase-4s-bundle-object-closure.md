# ADR-0122: Phase 4S Bundle Object Closure Metadata

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4Q/4R Bundle manifest foundation.

## Context

Phase 4Q created a deterministic export manifest with commit closure and
WorkState mapping. Phase 4R added canonical manifest byte output and validation.
The manifest still lacked direct metadata for the EntityVersion and
RelationVersion objects currently referenced by the exported WorkState.

ADR-0004 requires portable Bundles not to omit data needed to validate canonical
history. The next narrow step is to include object version closure metadata in
the manifest, while still deferring actual payload byte export and final Bundle
container format.

## Decision

1. Phase 4S extends `BundleExportManifest` with `entity_versions` and
   `relation_versions`.
2. EntityVersion closure metadata includes Entity id, EntityVersion id,
   Entity kind, state schema version, domain-separated state digest, raw
   canonical `state_json` digest, and canonical `state_json` byte size.
3. RelationVersion closure metadata includes Relation id, RelationVersion id,
   Relation type, source/target object ids, discriminator, state schema
   version, domain-separated metadata digest, raw canonical `metadata_json`
   digest, and canonical `metadata_json` byte size.
4. Export reloads each current WorkState version from SQLite and validates
   fixed-point canonical JSON plus the domain-separated digest before adding it
   to the manifest.
5. CLI `bundle export` reports `entity_versions` and `relation_versions`
   counts.
6. This slice does not export payload bytes, create a binary Bundle container,
   write `import_attempt`, write `store_lineage`, implement import, or perform
   remote/federated transport.

## Consequences

- A manifest now names the current object-version closure needed by later
  Bundle packaging and import preflight checks.
- The manifest remains deterministic and replay-backed, and payload byte policy
  stays deferred.

## Implementation Findings

- No new contract ambiguity was found. Because the schema has no generic
  ObjectId typed public wrapper, Relation endpoints are represented in the
  manifest as canonical lowercase UUID text.
