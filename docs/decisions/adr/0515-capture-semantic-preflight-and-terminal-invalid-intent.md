# ADR-0515: Capture Semantic Preflight and Terminal Invalid Intent

Status: Accepted and implemented; locally installed and verified through bounded live recovery
Date: 2026-10-01

## Context

ADR-0513 requires a bounded semantic payload to be validated before journal
admission. The public `capture` route parsed canonical JSON and its serde shape,
but the complete Record, Knowledge, Evidence, local-ID, and relation semantics
remained inside target Store delivery preparation.

One admitted `cognition_v2` intent therefore contained a relation from a
Finding to a Handoff using `supports`. The domain contract permits
`supports` only from a Finding to a Decision or Knowledge. Admission succeeded,
`delivery_started` became durable, and Store delivery then failed with the
non-retryable `record_invalid` error. The target Store remained unchanged, but
the recovery projection returned to `pending_primary` because its failure
vocabulary recognized only legacy-manifest upgrade. Replaying the same
immutable intent could never succeed, while the operator guidance to correct
and retry could not be applied to that intent.

The semantic relationship contract is correct. A Handoff synthesized from a
Finding uses `Handoff --derived_from--> Finding`, or no relation when explicit
provenance is unnecessary. Widening `supports` would erase the distinction
between epistemic support and coordination provenance.

## Decision

### Shared target-neutral semantic preflight

`CognitionCaptureManifest` exposes one target-neutral semantic validator. It
checks the complete manifest semantics already enforced by Store delivery,
including supported schema, bounded and non-empty fields, Record/Knowledge/
Evidence construction, unique local identifiers, supported relation types,
endpoint kinds, labels, and duplicate relations.

The public `capture` route runs this validator immediately after canonical
manifest decoding and before project resolution, activation checks, journal
layout creation, intent admission, registry mutation, or Store access. An
invalid manifest therefore fails as `record_invalid` with zero control-plane
or Store writes. Target head/state guards remain delivery concerns and are not
introduced into `cognition_v2` admission.

Store delivery uses the same validator through the existing preparation path;
there is no second permissive semantic implementation.

### Historical immutable invalid intents

Already-admitted intents remain immutable. Recovery materializes the exact
target manifest and runs semantic preflight after read-only target guard
construction but before opening the Store for mutation.

If a non-legacy manifest fails deterministically as `record_invalid`, recovery
records a matching `delivery_failed` event with:

- `failure_code=semantic_manifest_invalid`; and
- `recovery_action=start_new_capture_with_corrected_payload`.

The derived terminal state is `semantic_manifest_invalid`. The original intent
and event chain remain authoritative and unchanged, the target Store is not
mutated, and replay reuses the existing failure without adding events. A
corrected payload uses a new Capture and may reference the old Capture ID in
its provenance; it never rewrites or resumes the invalid intent.

Legacy guarded manifests retain the existing
`legacy_manifest_upgrade_required` precedence and behavior. Unresolved
ownership still follows the ownership state machine until an exact binding can
anchor a delivery failure; semantic preflight does not authorize bootstrap or
recovery apply.

### Compatibility and authority

The journal envelope version remains unchanged because this decision adds one
`delivery_failed` enum value and one derived projection state without changing
event framing. Older binaries fail closed on the unknown value. The candidate
must therefore be installed before any live semantic-invalid terminalization,
and installation plus live recovery remain separately authorized operations.

This ADR does not authorize live registry or Capture mutation, binary
installation, Commit, Push, release, deployment, or a global capture Hook.

## Consequences

- Deterministic semantic mistakes cannot become new immutable journal intents.
- Historical invalid intents have an honest terminal state and actionable
  replacement path instead of an indefinitely retryable-looking state.
- Store delivery remains atomic and unchanged for valid manifests.
- `supports` retains its existing domain meaning; coordination summaries use
  `derived_from` or no relation.
- A terminal invalid intent remains part of audit history and is never deleted
  or rewritten.

## Verification

Isolated fixtures proved all of the following:

1. `Finding --supports--> Handoff` fails before journal layout creation and
   leaves the registry and Store byte-stable.
2. `Handoff --derived_from--> Finding` is admitted and delivered with exactly
   one `derived_from` relation.
3. A historically admitted invalid intent records `delivery_started` plus
   `semantic_manifest_invalid`, leaves the target Store byte-stable, and
   exposes the replacement action through status.
4. Reapplying that terminal recovery adds no event and performs no target
   write.

Full workspace format, lint, locked tests, schema validation, and CLI smoke
passed before local installation and bounded live recovery.
