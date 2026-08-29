# ADR-0150: Phase 4AU Bundle Branch Integrity

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0148 added exported Branch head refs to Bundle manifests, and ADR-0149 made
preflight classify local Branch heads as already present, missing,
fast-forward, or diverged. That comparison assumes the manifest's target,
commit closure, and exported Branch head refs are internally consistent before
any local Store state is inspected.

## Decision

1. Phase 4AU validates Bundle manifest self-consistency during import preflight
   directory validation.
2. The manifest target Commit must appear in `commit_closure`, and its
   `state_digest` must match the target `state_digest`.
3. Commit ids in `commit_closure` must be unique.
4. Every parent Commit referenced by a manifest Commit must also appear in the
   exported commit closure.
5. Exported Branch ids must be unique.
6. Every exported Branch head must belong to the target Workspace.
7. Every exported Branch head Commit must appear in the exported commit closure,
   and the Branch head digest must match that Commit's closure digest.
8. Invalid manifests remain invalid Bundle directories and return
   `invalid_bundle_directory`.
9. This slice does not ingest canonical rows, create or move Branch refs,
   activate imports, resolve divergence, or implement cross-Store import.

## Consequences

- Preflight no longer lets a structurally valid JSON manifest reach local
  Branch comparison when the exported Branch head refs contradict the exported
  commit graph.
- Later import activation can rely on one checked manifest graph without
  repeating these structural checks.

## Implementation Findings

- None.
