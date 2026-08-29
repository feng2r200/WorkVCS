# ADR-0115: Phase 4L Checkpoint Foundation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4 acceleration/recovery sequence after ADR-0114.

## Context

ADR-0003 defines Checkpoints as digest-validated, format-versioned, rebuildable
acceleration objects associated with a WorkStateCommit. ADR-0036 also confirms
that the current `content_object` table records digest metadata only; raw bytes
are not stored by the Phase 3V Evidence slice.

## Decision

1. Phase 4L adds explicit Engine APIs to create and read a WorkState checkpoint.
2. Checkpoint creation uses replay of the target WorkStateCommit as the
   authority, verifies the replayed WorkState digest, and writes only
   `checkpoint`, `checkpoint_status`, and `content_object` rows.
3. The checkpoint content digest is the raw-byte BLAKE3-256 digest of a
   canonical JSON checkpoint payload with format
   `workvcs-workstate-checkpoint-v1` and format version `1`.
4. The checkpoint payload records workspace id, commit id, state digest, and
   sorted entity/relation current-version mappings. It is rebuildable from
   replay and does not become a second history authority.
5. CLI support is limited to `workvcs checkpoint create` and
   `workvcs checkpoint show`.
6. This slice does not add checkpoint scheduling, checkpoint restore, eviction,
   bundle transport, migration logic, or raw payload persistence.

## Consequences

- Tools can now materialize checkpoint metadata for a specific commit and later
  inspect its digest/status without replaying the whole command path.
- Same-commit checkpoints share the same content digest while retaining distinct
  checkpoint ids.
- Commit state digest and checkpoint content digest remain distinct concepts.

## Implementation Findings

- The v0.1 physical schema has `content_object` metadata but no payload byte
  table or storage-location table. Phase 4L therefore records a rebuildable
  checkpoint content digest and metadata, but does not persist the canonical
  checkpoint bytes.
