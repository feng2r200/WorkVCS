# Bundle Local Directory Profile v0.1

Status: current V1-local implementation contract

This document defines the deterministic local Bundle directory profile used by
the current Rust implementation. It realizes the Bundle interchange
requirements from
[Knowledge Federation and Store Portability](knowledge-federation-and-portability.md)
for local copied-Store operation, but it is not a packaged archive, streaming
protocol, exchange API, or cloud synchronization contract.

## Profile Identity

The local payload-index profile is:

```text
workvcs-local-payload-index-v1
```

The payload-index version is:

```text
1
```

The Store canonical JSON profile remains `workvcs-jcs-v1`. Bundle payloads are
canonical semantic JSON bytes unless a later accepted profile explicitly
defines another byte family.

## Directory Shape

`workvcs bundle export-dir` writes a deterministic directory artifact:

```text
manifest.json
payload-index.json
payloads/<content-digest>.json
```

`manifest.json` is the canonical Bundle export manifest. `payload-index.json`
is canonical JSON that contains:

- `bundle_payload_index_profile`;
- `bundle_payload_index_version`;
- `manifest.path`, `manifest.digest`, and `manifest.size_bytes`;
- `target.workspace_id`, `target.commit_id`, and `target.state_digest`;
- `payload_count`;
- `reference_count`;
- the deduplicated `payloads` array; and
- the role-specific `references` array.

Payload files are stored below `payloads/` by their declared raw content
digest. Export uses create-new file writes and must not overwrite existing
files.

## Validation

Validation reads `manifest.json`, `payload-index.json`, and the regular files
under `payloads/`. The implementation verifies:

- fixed-point canonical JSON for the manifest, payload index, and JSON
  payloads;
- manifest digest and size in the payload index;
- target Workspace, Commit, and state digest consistency between manifest and
  payload index;
- payload count and reference count consistency;
- every listed payload path, digest, and size;
- every observed payload file is listed by the payload index;
- payload role ownership and digest/size against the manifest closure; and
- Bundle manifest graph and exported Branch-head self-consistency before local
  target Store comparison.

The payload-index digest is the current deterministic local Bundle artifact
digest used by import attempt records. It is not a final archive byte-stream
digest.

## Same-Store Apply Scope

The current apply path is a same-Store fast-forward path. A Bundle can apply
only when preflight reports `same_store_fast_forward_ready`, which requires:

- Store format compatibility;
- source Store identity equal to the target Store identity;
- incoming target Commit absent from the target Store;
- at least one exported Branch head;
- at least one target Branch can fast-forward;
- no missing exported Branch heads; and
- no diverged exported Branch heads.

Apply imports missing immutable rows and then advances existing Branch heads
with compare-and-swap. Existing identical immutable rows are no-ops; content or
identity conflicts fail closed.

The supported same-Store closure families include the canonical Commit closure,
ChangeSet and ChangeOperation payloads, Task and other typed entity identity
rows added by accepted Bundle ADRs, RelationVersion and relation membership
rows, Verification object-family rows, stable ended Session provenance,
KnowledgeExposure local-source closure, changeset causal anchors, and
Checkpoint candidate metadata/status.

Checkpoint raw payload bytes are not stored by the v0.1 schema and are not
added by this profile. Checkpoints remain rebuildable acceleration metadata;
the WorkStateCommit and replayed Work State remain authoritative.

## External Store Boundary

Direct canonical DAG apply from a different Store identity is not part of this
V1-local profile. A Bundle from another Store may validate as a self-consistent
artifact, but preflight/apply reports `external_store_import_not_implemented`
for canonical history activation.

This does not remove the V1 portability model. A Store copy, move,
backup/restore, or same-identity export/import preserves Store identity.
Different Store identities are opened separately, explicitly forked with
lineage, or selectively adopted through Knowledge/Evidence and
ExternalObjectRef provenance. Directly merging another Store's canonical DAG
into the local namespace remains outside the current implementation contract.

## Still Open

- packaged Bundle archive/container encoding;
- persisted exported Bundle bytes;
- compression, streaming, signatures, and exchange/access APIs;
- missing Branch creation from imported Bundles;
- external Store canonical DAG activation;
- runtime recovery vocabulary beyond stable ended Session provenance; and
- larger Store portability performance evidence.
