# ADR-0514: Digest-Locked Same-Lineage Activation Marker Refresh

Status: Accepted, implemented, and live-verified on one exact registry-sidecar route
Date: 2026-09-30

## Context

ProjectRef recovery can legitimately advance a registry-v2 revision while
preserving the registry ID. Because read-routing and journal-admission markers
bind the exact registry ID, revision, and digest, that advance correctly makes
both installed markers stale.

The original activation contract allowed creation when a marker was absent and
idempotent reuse when it was already exact, but rejected every stale marker.
It also correctly prohibited manual deletion or overwrite. Together those
rules left no supported path to reactivate a control plane after an authorized
same-lineage registry advance. The observed live failure stopped before any
write, but it also prevented the required WorkVCS read and journal routes from
being restored.

The operator explicitly authorized exact read-routing and journal-admission
reactivation on 2026-09-30. This ADR defines the minimum safe lifecycle needed
to carry out those operations without weakening stale-marker protection.

## Decision

Both activation apply commands accept optional
`--expected-activation-digest OLD_DIGEST` for one narrowly defined refresh:

- the current registry digest and newly derived candidate digest must still
  match the operator's two existing digest locks;
- the installed marker must be a canonical regular non-symlink file whose
  digest exactly matches `OLD_DIGEST`;
- the marker must name the same registry ID and a strictly earlier revision;
- the marker is revalidated immediately before one Unix atomic replacement;
- read-routing refresh holds the registry lock, while journal-admission
  refresh holds the registry lock and then the existing journal-quiescence
  lock; and
- journal admission still requires exact current read routing, so operators
  refresh read routing first.

An absent marker with an old-marker digest, an invalid or symlinked marker, a
wrong registry lineage, an equal or newer revision, any digest mismatch, or a
concurrent change fails closed before replacement. Ordinary apply without the
old-marker digest continues to refuse stale state.

If replacement completed or may have completed before a later cleanup,
directory-sync, or verification failure, the existing
`routing_activation_install_indeterminate` contract applies. The operator must
inspect status first. An exact active marker makes a later exact apply an
idempotent no-write result; no command guesses rollback or deletes the marker.

Authority remains separate. Registry bootstrap does not authorize marker
refresh, read-routing refresh does not authorize journal-admission refresh,
and neither operation authorizes capture recovery, Store delivery, rollback,
Push, release, or deployment.

## Consequences

- A same-lineage registry advance no longer permanently strands otherwise
  valid read and journal routes.
- The old installed marker becomes a third compare-and-swap input rather than
  something an operator removes manually.
- Refresh remains intentionally Unix-only because it relies on the same
  atomic-replacement contract as registry migration and recovery.
- There is no automatic reactivation, wildcard revision acceptance, lineage
  migration, marker repair, or relaxation of the fail-closed read path.

## Verification

The isolated fixture advances a registry through ProjectRef recovery, proves
both markers stale, injects a fault immediately after each atomic replacement,
uses status to prove the new marker active, and verifies an exact retry is
idempotent. Separate negative coverage rejects another registry lineage and
retains malformed/symlink fail-closed behavior. Installation and any live
refresh still require current digest observations and authority for those
exact operations.

The first authorized live use completed on 2026-09-30. Both exact stale
revision-1 markers for one revision-2 registry-sidecar route were atomically
refreshed and read back active without registry, journal, or Store writes. The
full digest and configuration-selection evidence is recorded in
[ProjectRef Same-Lineage Activation Refresh Live Evidence](../../provenance/projectref-activation-same-lineage-refresh-live-evidence.md).
