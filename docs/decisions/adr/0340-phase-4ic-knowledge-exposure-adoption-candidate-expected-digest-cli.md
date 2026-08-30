# ADR-0340: Phase 4IC Knowledge Exposure Adoption Candidate Expected Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs store knowledge-exposure-adoption-candidate` renders the source
Knowledge state digest that would be adopted. Automation needs to assert that
the candidate still refers to the expected source Knowledge version before
calling adopt.

## Decision

`workvcs store knowledge-exposure-adoption-candidate` accepts optional
`--expected-source-knowledge-state-digest HEX`.

The CLI reads the adoption candidate through the Engine facade, parses the
expected digest through the core `Digest` parser, and returns
`source_knowledge_state_matches_expected=true` only when the candidate source
Knowledge state digest matches. A mismatch returns `DigestInvalid`.

## Consequences

Adoption scripts can fail fast when an exposure candidate points to an
unexpected source Knowledge version. The command does not change exposure
creation, freshness checks, adoption, relation creation, or canonical digest
semantics.
