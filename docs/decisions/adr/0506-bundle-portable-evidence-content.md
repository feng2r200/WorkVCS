# ADR-0506: Bundle Portable Evidence Content

Status: Accepted
Date: 2026-09-12

## Context

Bundle v1 exports the immutable Evidence, EvidenceContent, and ContentObject
metadata closure, but it does not carry locally stored Evidence bytes or the
target-local storage location needed by `evidence extract`. A copied Store can
therefore import a Verification successfully while losing the raw proof that
made the Verification reviewable.

The same dogfood pass found that the top-level `verify` wrapper bypassed the
Store layer that persists raw Evidence content. That defect must be corrected
before Bundle portability can make a truthful raw-content claim.

Silently adding binary files to the v1 profile is unsafe: an importer that
understands only v1 could accept the metadata while dropping the new bytes.
There is no current compatibility requirement and no durable v1 Bundle
artifact was found in the configured WorkVCS registry or the participating
repositories. A compatibility layer therefore has no demonstrated benefit.

## Decision

1. `verify` preflights first, then uses the same local content-addressed object
   preparation as direct Evidence creation before committing Evidence and
   Verification rows.
2. The local Bundle directory profile advances to v2. Manifest, payload-index,
   and import profiles all use explicit v2 identities and version `2`; v1
   artifacts fail closed instead of being partially interpreted.
3. The manifest contains `portable_evidence_contents`. Each entry identifies
   one EvidenceContent by Evidence id and ordinal and anchors its content
   digest and size. Digest-only EvidenceContent is retained in the ordinary
   closure but is not listed as portable.
4. Canonical JSON payloads remain under `payloads/<digest>.json` with
   `application/json`. Portable raw Evidence objects use
   `objects/<digest>.bin` with `application/octet-stream`. Files are
   content-addressed and deduplicated by path and digest.
5. Export includes a raw object only when the source Store has an available,
   contract-valid local WorkVCS storage location and the physical bytes match
   the declared digest and size. A location that claims availability but
   cannot be verified makes export fail.
6. Validation requires every manifest portable-content entry to have exactly
   one matching `evidence_content_body` reference and requires every raw
   object payload to be referenced. Missing, additional, mismatched, or
   tampered bodies fail validation and preflight.
7. Same-Store apply derives the target locator from the target Store path and
   Store id; source locators are never copied. It writes verified
   content-addressed bytes before the database transaction and makes the
   target storage-location row visible in the same transaction as the
   imported Evidence closure.
8. A failed database transaction may leave only an unreferenced,
   content-addressed file that has already passed digest and size validation.
   It must not leave a visible storage-location row or advance a Branch.
9. Empty raw Evidence content is valid and portable. Digest-only content
   remains intentionally unextractable after import.

## Consequences

- Bundle validation can distinguish a complete portable proof from metadata
  that only names an unavailable object.
- Applying a copied-Store Bundle restores `evidence extract` without assuming
  that the source and target Store directories share an object directory.
- Existing v1 artifacts require regeneration with the current binary. No dual
  reader, dual writer, or migration shim is introduced.
- External-Store canonical DAG activation, packaged archives, remote exchange,
  checkpoint bytes, and non-Evidence content bodies remain outside this slice.

