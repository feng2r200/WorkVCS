# ADR-0159: Phase 4BD Bundle Checkpoint Candidate Apply

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Phase 4L introduced rebuildable WorkState checkpoints backed by `checkpoint`,
`checkpoint_status`, and digest-only `content_object` metadata. Earlier Bundle
export surfaced checkpoint candidates, but same-Store Bundle apply rejected any
manifest containing them because the candidate record did not include enough
data to restore the rows immutably.

## Decision

1. Same-Store Bundle export now treats checkpoint candidates as restorable
   metadata for the exported commit.
2. A checkpoint candidate records workspace id, commit id, state digest,
   checkpoint format version, content digest/size/media type, timestamps,
   usability state, and the digest/size of `checkpoint_status.detail_json`.
3. Checkpoint content digests are added to the Bundle `content_objects` closure.
4. `checkpoint_status.detail_json` is carried as a canonical JSON payload with
   role `checkpoint_status_detail`.
5. Same-Store Bundle apply restores `checkpoint` and `checkpoint_status` rows
   only after content objects and commit closure rows are present.
6. Existing local checkpoint/status rows are immutable: matching rows are reused,
   and any field mismatch fails import.

## Non-Goals

- No checkpoint raw payload storage is added.
- No checkpoint restore, scheduling, eviction, cross-Store import, or transport
  behavior is implemented.
- Checkpoints remain rebuildable acceleration objects and do not become a second
  history authority.

## Consequences

- A same-Store fast-forward Bundle can now carry the target commit's checkpoint
  metadata and latest status projection.
- Bundle preflight no longer rejects valid checkpoint candidates as unsupported.
- Old manifests with incomplete checkpoint candidates remain outside same-Store
  apply support because the candidate cannot be parsed as a restorable record.

## Implementation Findings

- The physical schema still stores only content-object metadata. Phase 4BD
  therefore imports checkpoint metadata and status detail, but not raw checkpoint
  bytes.
