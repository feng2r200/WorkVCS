# ADR-0331: Phase 4HT Resource Observation Show Expected Detail Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs resource observation-show` renders a Resource Observation and, when
present, its detail content digest. Resource-backed verification tooling needs
a direct assertion that an observation contains the expected raw detail content.

## Decision

`workvcs resource observation-show` accepts optional
`--expected-detail-content-digest HEX`.

The CLI reads the Resource Observation through the Engine facade, parses the
expected digest through the core `Digest` parser, and returns
`detail_content_matches_expected=true` only when the observation has detail
content with the expected digest. A missing detail object or mismatched digest
returns `DigestInvalid`.

## Consequences

Scripts can fail fast when an Observation id resolves to the wrong captured
detail content. The command does not change observation recording, fingerprint
calculation, list filtering, resource applicability, or verification semantics.
