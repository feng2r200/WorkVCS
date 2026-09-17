# Bundle Local Directory Profile v0.2

Status: current local implementation contract

This document defines the deterministic local Bundle directory profile used by
the current Rust implementation. It supersedes the v0.1 profile by making
locally stored raw Evidence content explicitly portable. It is not a packaged
archive, streaming protocol, exchange API, or cloud synchronization contract.

## Profile Identity

The profile identities are:

```text
manifest:      workvcs-local-export-manifest-v2
payload index: workvcs-local-payload-index-v2
import:        workvcs-local-payload-directory-v2
version:       2
```

The Store canonical JSON profile remains `workvcs-jcs-v1`.

## Directory Shape

`workvcs bundle export-dir` writes a create-new directory artifact:

```text
manifest.json
payload-index.json
payloads/<content-digest>.json
objects/<content-digest>.bin
```

`payloads/` contains fixed-point canonical JSON with media type
`application/json`. `objects/` contains opaque Evidence bytes with media type
`application/octet-stream`; zero-byte object files are valid. Both families
are named by the BLAKE3-256 digest of their exact bytes.

`payload-index.json` contains the manifest anchor, target identity, counts,
the deduplicated payload file list, and role-specific references. An
`evidence_content_body` reference owns the Evidence id, ordinal, and content
digest of one entry in `manifest.json`'s `portable_evidence_contents` array.

## Evidence Portability

The ordinary `evidence_contents` and `content_objects` arrays describe the
complete immutable metadata closure. `portable_evidence_contents` is a subset
whose raw bytes were locally available and verified at export time.

- A portable entry must match one EvidenceContent and one ContentObject in the
  same manifest.
- It must have exactly one matching payload-index reference.
- Its object payload digest and size must match both the reference and the
  manifest ContentObject.
- Digest-only EvidenceContent remains in the immutable closure but has no
  portable entry or object file.

This distinction prevents a missing object file from being mistaken for an
intentional digest-only reference.

## Validation

Validation checks fixed-point canonical JSON for the manifest, index, and JSON
payloads; exact profile identities; target consistency; payload path, media
type, digest, and size; payload and reference counts; raw-body reference
coverage; manifest graph integrity; and exported Branch-head consistency.
Opaque object payloads are never parsed as JSON.

An object file not named by an `evidence_content_body` reference, a portable
manifest entry without its reference or file, and any digest or size mismatch
all fail validation before import can write Store state.

## Same-Store Apply Scope

The apply path remains same-Store fast-forward only. Immutable rows are
imported and existing Branch heads advance by compare-and-swap only after
preflight reports `same_store_fast_forward_ready`.

Portable Evidence bytes are written to the target Store's own sibling
`.workvcs-objects/<store-id>/blake3-256/...` directory. The target locator is
derived locally; no source filesystem locator is transported. The
`content_storage_location` row is inserted or validated in the same database
transaction as the imported Evidence closure and Branch update.

Writing the content-addressed file precedes the transaction. If a later
transaction step fails, the database exposes neither the location nor the
incoming Branch head. A verified orphan file may remain and is safe to reuse
because its path, digest, and bytes are identical.

## External Store Boundary And Open Work

Direct canonical DAG apply from a different Store identity remains
unsupported. Packaged archives, compression, streaming, signatures,
exchange/access APIs, missing Branch creation, checkpoint raw bytes, and raw
content families other than Evidence remain open.
