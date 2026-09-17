# ADR-0512: Local Object Store v2 Cutover

Status: Accepted
Date: 2026-09-17

## Context

WorkVCS hashes raw Evidence content with BLAKE3-256, but the original local
object layout placed those bytes below a directory named `sha256` and labelled
their database location `workvcs.local-object-v1`. The stored digests were
correct; the locator vocabulary was not. Keeping the misleading name would
make operational inspection, evidence recovery, and future storage work prone
to incorrect assumptions.

The configured WorkVCS registry contains three Stores. Two have no local raw
objects and one has nine. There is no requirement to keep an old binary writing
the original layout, and permanent dual readers, dual writers, or fallback
locators would turn a bounded correction into ongoing compatibility policy.

Bundle profile v2 does not transport a source filesystem locator. It carries
verified raw bytes as `objects/<digest>.bin`, and apply derives a fresh locator
from the target Store. The Bundle wire contract therefore does not need a new
version for this target-local layout correction.

## Decision

1. Advance `object_store_format_version` from `1` to `2`. Store format and
   schema versions remain `1`.
2. The only current local raw-content backend is
   `workvcs.local-object-v2`. Its canonical locator is
   `.workvcs-objects/<store-id>/blake3-256/<first-two-hex>/<digest>`.
3. Normal v2 open, read, write, extract, Bundle export, and Bundle apply paths
   accept only the v2 manifest and locator contract. A v1 Store fails closed.
4. The three configured Stores are cut over once with a dedicated migration
   candidate. It verifies every source file by BLAKE3-256 digest and size,
   prepares and verifies all v2 files before changing the database, then
   atomically updates the manifest and local location rows while recording one
   Store migration outcome.
5. The migration retains v1 files until the migrated Store, Evidence
   extraction, and Bundle behavior have been validated with the final v2
   binary. Only then may the exact old directories be removed.
6. The migration candidate is transitional delivery machinery, not a product
   capability. After the configured Stores are cut over, its command, v1
   reader, v1 locator helper, and migration-specific tests are removed from the
   final source and are not installed globally.
7. Existing Bundle profile v2 artifacts remain structurally compatible because
   they contain verified object bytes rather than source locators. Bundle v1
   remains unsupported under ADR-0506; no additional Bundle compatibility shim
   is introduced.

## Consequences

- Filesystem names, database backend identity, manifest version, and the actual
  BLAKE3-256 digest algorithm now agree.
- v1 can exist briefly as verified migration input but consumes no permanent
  runtime branch or future compatibility obligation.
- A migration interrupted before database commit may leave only verified v2
  object files. The v1 Store and its source files remain authoritative and a
  retry reuses the identical content-addressed targets.
- A migrated database never points at v2 files until all of its target objects
  are present and verified.
- Store snapshots and the retained v1 files provide the recovery boundary
  during cutover; they are cleanup artifacts, not alternate live authorities.

## Non-goals

- Supporting writes from v1 and v2 binaries against the same Store.
- Retaining a v1 fallback locator, dual backend, or user-facing migration
  command after the configured cutover.
- Changing the BLAKE3-256 digest itself, the SQLite schema, Bundle profile v2,
  or digest-only Evidence behavior.
- Migrating unknown Stores outside the three registry-selected targets.
