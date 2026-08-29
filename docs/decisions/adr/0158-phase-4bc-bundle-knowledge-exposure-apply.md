# ADR-0158: Phase 4BC Bundle KnowledgeExposure Apply

Status: Accepted
Date: 2026-08-30

## Context

ADR-0147 added Bundle export closure for Store-local KnowledgeSpace and
KnowledgeExposure provenance referenced by exported RelationVersion endpoints.
Until this slice, same-Store Bundle apply still rejected any manifest containing
those KnowledgeExposure closure arrays.

That left adopted Knowledge portable enough to inspect, but not yet portable
through the local fast-forward apply path.

## Decision

Phase 4BC moves local KnowledgeExposure closure into same-Store Bundle apply.
The apply document now parses and validates:

- `knowledge_spaces`;
- `knowledge_exposures`;
- `knowledge_exposure_local_sources`;
- `knowledge_exposure_transitions`;
- `knowledge_exposure_source_statuses`.

Apply imports those rows before RelationVersion rows so relations can point at
KnowledgeExposure object identities. Existing rows remain immutable: a matching
row is reused and any content mismatch fails the import.

The imported transition history must be a single local-source chain with one
initial transition and one head. `knowledge_exposure_current` is rebuilt from
that chain head. Transition `event_id` remains outside this same-Store apply
slice because Bundle export does not yet carry Event closure.

## Consequences

- A same-Store Bundle containing adopted Knowledge and its
  `derived_from` KnowledgeExposure relation can now preflight and apply.
- Apply result and CLI output report KnowledgeExposure closure import counts.
- Source Knowledge entity versions are allowed in same-Store apply as the local
  source payload for the exposure, but external KnowledgeExposure sources remain
  outside scope.
- Checkpoint candidates remain exported but not restored by apply in this slice.

## Implementation Findings

- KnowledgeExposure current projection is not exported as an authority row. It
  is rebuildable from the validated transition chain head during apply.
- The existing Bundle export closure for KnowledgeExposure only covers local
  sources, so same-Store apply intentionally rejects external or Event-backed
  exposure transition material until their closures exist.
