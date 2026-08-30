# ADR-0303: Phase 4GR Canonical Digest Domain Discovery CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs canonical digest` requires a supported semantic digest domain. The
confirmed V0.1 tool surface currently supports EntityVersion and
RelationVersion hashing domains, but users must know those strings beforehand.

## Decision

`workvcs canonical digest-domains` lists the semantic digest domains accepted by
`workvcs canonical digest`.

The command is store-independent and deterministic. It does not include
raw-byte ContentObject digesting because that is exposed through the separate
`content-digest` command.

## Consequences

Manual tooling can discover supported canonical semantic digest domains without
source inspection. The command does not add a new hashing domain or change
canonical digest behavior.
