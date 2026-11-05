# ADR 0539: Explicit Historical Source Disposition

Status: Accepted
Date: 2026-11-05
Implementation: Implemented

## Context

The owner explicitly authorized processing recoverable historical sessions and
excluding individual sessions that cannot be processed. The confirmed contract
requires preserved archives, per-source evidence, invalidation when the source
or its authority basis changes, explicit disclosure in catalog and query
results, and reversibility. A source failure is not itself a disposition.

## Decision

An owner-authorized disposition is an explicit, durable, source-specific
record. It identifies the exact source path, observed failure, observed absence
or content digest, and the registry and source-policy bases. Its application
must verify these inputs against an exact current catalog. The owner can inspect
and revoke the decision with an exact current digest guard.

A disposed historical source is outside the selected retrieval scope. Its
provenance and disposition remain visible in the immutable catalog and query
result. Completeness means that every source still inside the selected scope
is ready and each excluded source has a valid explicit disposition. Existing
verified archives remain available. A new error, changed file, reappearing
source, changed registry, changed source policy, or invalid disposition is not
covered by the old decision and cannot be represented as successful.

Disposition does not assign ownership, modify source files, alter canonical
Stores, rewrite the registry, or delete existing archives. The worker checks the
observed basis again before publishing. Readback validates the catalog's scope
and disposition evidence. Existing catalogs retain their original completeness
semantics. The new catalog format and runtime must fail closed when an older
reader cannot interpret their disposition semantics.

When the owner explicitly confirms a session's project, a separate revocable
source-owner authorization may bind that exact source path, thread identifier,
session working directory and ProjectRef to the current registry and source
policy. It is not an inferred registry binding. Import verifies the session
metadata and publishes the authorization with the archive provenance; changed
identity, revoked authorization or changed authority basis cannot reuse it.
Appending messages to the same session does not change its ownership.

On macOS, a rollout currently held with a writable file descriptor by a Codex
process is active and is outside the historical-processing scope. The catalog
and query disclose it as active, rather than as a historical failure. A prior
verified archive remains available. Publication and query recheck the writer;
when the writer closes, the old active observation is stale and the next
reconciliation processes the source as historical. Failure to inspect process
handles is an error, not evidence that a source is inactive. Other source
failures remain visible and keep the catalog incomplete.

## Verification

Validation covers exact-input application, stale and unlisted decisions,
reappearing and changed sources, basis changes, revoked dispositions, preserved
archive retrieval, fresh undisposed failures, catalog and query disclosure,
concurrent reconciliation, native readback, and restart recovery. Actual
adoption additionally requires exact live preflight, service identity and
catalog/query readback. Prior configuration and evidence are retained for
rollback; binary rollback is not attempted until format compatibility is proven.
