# Phase 4LP Bundle Profile Contract Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LP closes the ledger's Bundle profile contract gap for the current
local implementation by promoting the already implemented deterministic
directory artifact to a V1-local contract. It also records the external Store
boundary already implied by the confirmed architecture and current
implementation: direct canonical DAG apply from a different Store identity is
not implemented by the V1-local profile.

This evidence does not claim packaged Bundle archives, remote exchange,
streaming, cloud synchronization, live federation, external Store DAG
activation, or larger Store maturity.

## Authority Review

The review checked:

- [Knowledge Federation and Store Portability](../architecture/knowledge-federation-and-portability.md);
- [Physical Schema v0.1 Contract](../architecture/physical-schema-v0.1.md);
- [Implementation Contract v0.1](../architecture/implementation-contract-v0.1.md);
- ADR-0004 for Store portability and Bundle interchange;
- ADR-0123 through ADR-0128 for directory export, validation, preflight, and
  import attempt journaling;
- ADR-0150 through ADR-0159 for Branch integrity, same-Store apply, typed
  identity/Relation/Verification/Session/KnowledgeExposure/Checkpoint closure;
- ADR-0430 and Phase 4LO dogfood evidence; and
- the current core/CLI implementation around Bundle profile, validation,
  preflight, and apply.

The reviewed authorities agree on the current boundary:

- Bundle interchange must include manifest, canonical logical history,
  required immutable objects, format/schema metadata, and integrity metadata.
- The deterministic directory artifact is the current local implementation
  profile, not a final archive/container.
- Same-Store import compares Commit DAG ancestry and Branch refs; fast-forward
  can apply and divergence must not overwrite local state.
- Full canonical import is same-Store only in the current implementation
  boundary.
- Different Store identities are separate Stores, explicit forks, or selective
  Knowledge/Evidence adoption with ExternalObjectRef provenance. Direct
  canonical DAG merge from another Store is not implemented.

## Implementation Check

The implementation exposes:

```text
bundle_payload_export_profile=workvcs-local-payload-index-v1
bundle_payload_index_version=1
```

`workvcs bundle export-dir` writes:

```text
manifest.json
payload-index.json
payloads/<content-digest>.json
```

Validation parses canonical JSON, checks manifest/index target consistency,
checks payload counts and references, verifies every observed payload file
against the index, and validates manifest graph and Branch-head consistency
before target Store comparison.

Preflight reports `external_store_import_not_implemented` for different Store
identity artifacts. Apply only enters the write path when preflight reports
`same_store_fast_forward_ready`.

A focused external Store boundary check was run with two independently
initialized Stores:

```text
.work-governance/runtime/logs/phase-4lp-external-store-boundary-20260831T225634Z.log
external_store_boundary_result=PASS
source_store_relation=external_store
can_apply=false
action=external_store_import_not_implemented
recorded=true
applied=false
import_id=none
imported_commits=0
updated_branch_heads=0
```

## Change Made

The docs now include
[Bundle Local Directory Profile v0.1](../architecture/bundle-local-profile-v0.1.md),
which records the V1-local profile and open packaged/external boundaries.

One stale core error message was corrected from the old "task-only" apply scope
wording to the current same-Store apply scope wording. Later accepted ADRs had
already widened same-Store apply support beyond task-only Bundles.

## Remaining Gaps

- External Store canonical DAG activation remains unsupported by the V1-local
  profile.
- Packaged Bundle archive/container, compression, streaming, signatures, and
  exchange/access APIs remain Open.
- Missing Branch creation from imported Bundles remains unsupported.
- Larger Store portability performance evidence remains pending.
