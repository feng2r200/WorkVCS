# ADR-0431: Phase 4LP Bundle Profile Contract

Status: Superseded by ADR-0506
Date: 2026-09-01

## Context

Phase 4LO dogfooded local copied-target Bundle portability, but the V1
readiness ledger still treated the Bundle container/profile contract and
external Store import/apply semantics as the next Open gap. The risk was that
future work would either continue adding narrow smoke assertions or over-claim
external portability beyond what the current implementation and accepted
architecture support.

Existing ADRs already define the deterministic local directory artifact,
payload index, validation, same-Store preflight/apply gate, same-Store apply
closure families, import attempt journal, and divergence refusal. They also
leave packaged archive/container details, remote exchange, streaming, and
direct external Store canonical DAG activation unfixed.

## Decision

Accept
[Bundle Local Directory Profile v0.1](../../architecture/bundle-local-profile-v0.1.md)
as the current V1-local Bundle profile contract.

The contract freezes the current local directory artifact for implementation
and operator use:

- `manifest.json`;
- `payload-index.json`;
- `payloads/<content-digest>.json`;
- payload-index profile `workvcs-local-payload-index-v1`;
- payload-index version `1`;
- fixed-point canonical JSON validation for manifest, index, and JSON
  payloads;
- payload digest/size/reference validation;
- manifest graph and exported Branch-head self-consistency checks;
- same-Store fast-forward apply only behind
  `same_store_fast_forward_ready`; and
- non-destructive refusal for divergence, missing Branch refs, unsupported
  same-Store shapes, and external Store artifacts.

Direct canonical DAG apply from a different Store identity is explicitly
outside the V1-local Bundle profile. External Store artifacts may validate as
self-consistent artifacts, but canonical history activation reports
`external_store_import_not_implemented`.

One stale implementation error message is updated to remove obsolete
"task-only" wording from the unsupported same-Store apply scope.

## Non-Goals

- No schema change.
- No Bundle command spelling change.
- No packaged archive/container format.
- No persisted exported Bundle byte stream.
- No compression, streaming, signing, exchange API, or access-control model.
- No external Store canonical DAG activation.
- No cross-Store live federation, remote subscription, or cloud sync.
- No missing Branch creation from imported Bundles.
- No larger-Store or release maturity claim.

## Consequences

The V1 readiness ledger no longer needs to track the local Bundle directory
profile as Open. The remaining portability work is now narrower: larger Store
portability evidence, missing Branch creation if V1 later requires it, and
external Store adoption/provenance work that does not pretend to directly merge
another Store's canonical DAG.

The implementation remains behavior-compatible. The only code change is an
operator-facing error-text correction for an unsupported manifest scope.
