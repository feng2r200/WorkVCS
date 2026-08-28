# ADR-0009: Phase 2 Store Bootstrap and Open Boundary

- **Status:** Accepted for implementation
- **Accepted by:** current Work request to merge Phase 1 locally into `main`,
  branch from `main`, and continue Phase 2 implementation.

## Context

ADR-0008 closed Phase 1 and allowed Phase 2 to start after canonical tests
passed. ADR-0007 closed install/open compatibility rules: fixed
`application_id`, per-connection foreign-key enforcement, exactly one Store and
StoreManifest, supported Store/schema/object-store versions, ID scheme, digest
algorithm, canonical JSON profile, and fail-closed handling for unsupported
versions.

## Decision

1. Phase 2 starts with the minimal Store bootstrap/open slice in
   `workvcs-core`.
2. The SQLite binding is `rusqlite`, kept behind the core Store boundary.
   Public API enters through `Engine`; callers do not receive SQLite
   connections or transactions.
3. `Engine::init` installs the frozen `schema/schema-v0.1.sql`, creates exactly
   one Store and StoreManifest, and writes a canonical manifest JSON payload.
4. `Engine::open` validates the bootstrap contract before returning an Engine:
   `application_id`, per-connection foreign keys, expected schema structure,
   single Store/Manifest cardinality, supported version/profile parameters, and
   canonical manifest fixed point.
5. Unsupported newer or older Store/schema/object-store versions fail closed.
   Ordinary open does not silently migrate.
6. Phase 2 slice 1 does not implement Workspace Genesis, ChangeSet/Commit
   persistence, replay, Branch CAS, Task, Runtime, Verification, Merge,
   Federation, Projection, Checkpoint, Bundle, Doctor, or migration chains.

## Consequences

- Store/bootstrap code is organized by authority boundary rather than by
  table-by-table repositories.
- The CLI remains a compiling skeleton and gains no business commands in this
  slice.
- The frozen schema remains unchanged except for aligning the validation
  harness sample manifest profile with the Phase 1 canonical profile.

## Implementation findings

- IF-0003: The pre-Phase-1 schema validation harness used
  `workvcs-canonical-json-v0.1` as sample StoreManifest profile text. CE-02
  later froze the StoreManifest canonical JSON profile as `workvcs-jcs-v1`.
  The DDL itself only requires a non-empty profile string, so this was a sample
  harness mismatch rather than a schema conflict. Phase 2 resolves it by using
  `workvcs-jcs-v1` in Engine bootstrap and in the schema validation harness
  fixture.
- IF-0004: Independent review found that checking only user table count was not
  enough to prove the database matched the frozen schema during `Engine::open`.
  Phase 2 resolves this by generating the expected SQLite schema object set
  from the frozen `schema-v0.1.sql` using the same SQLite binding, comparing it
  with the opened database object set, and adding a drift regression where the
  table count remains correct but an index is replaced.
