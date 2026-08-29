# ADR-0120: Phase 4Q Bundle Export Manifest Foundation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed Store portability boundary in ADR-0004/ADR-0005.

## Context

ADR-0004 defines Bundle as a self-contained interchange format with manifest,
canonical object payloads, and validation proofs. ADR-0005 states that Bundle
export computes the required local reference closure, while BundleManifest and
ImportAttempt remain distinct infrastructure families. The exact Bundle
container profile, compression, and import workflow are still not fully
specified.

Checkpoint slices 4L-4P added replay-verified WorkState checkpoints and basic
checkpoint health reporting. The next useful transport step is a read-only,
deterministic export manifest over an existing Store commit, without freezing a
complete bundle container or mutating import state.

## Decision

1. Phase 4Q introduces `BundleExportOptions::for_commit` and
   `Engine::export_bundle_manifest`.
2. The export target is an existing WorkStateCommit. Export first replays the
   target commit and verifies the replayed WorkState mapping digest.
3. The manifest profile is `workvcs-local-export-manifest-v1`, version `1`.
4. The manifest includes Store manifest metadata, target workspace/commit/state
   digest, all reachable commit closure references, the replayed WorkState
   entity/relation mapping, and checkpoint content candidates for the target
   commit.
5. The commit closure walks all `commit_parent` edges, so merge secondary
   ancestry is included in the manifest boundary.
6. The `manifest_digest` is the raw content-object digest of the canonical
   manifest bytes. It is not the final Bundle container digest.
7. CLI adds only `workvcs bundle export --store --commit` style read-only output
   using existing `key=value` rendering.
8. This slice does not create a binary Bundle container, persist exported bytes,
   write `import_attempt`, write `store_lineage`, implement import validation,
   implement compression, or perform remote/federated transport.

## Consequences

- Tools can now compute a deterministic, replay-backed local export manifest
  for a commit.
- Later Bundle packaging can use this manifest as its closure contract without
  retroactively changing import or transport semantics.

## Implementation Findings

- The schema contains `import_attempt` and `store_lineage`, but no confirmed
  Bundle container table or payload storage policy. Phase 4Q therefore records
  only an in-memory/exported manifest and leaves persisted bundle bytes to a
  later contract.
