# ADR-0228: Phase 4DU Knowledge Space Name List Filter CLI

Status: Accepted

Date: 2026-08-30

## Context

KnowledgeSpace names are already confirmed as exact, case-sensitive,
Store-local unique identifiers. The CLI can create, show, and list Knowledge
Spaces, but list output can only be limited by count. Operators often know the
name and need to resolve or verify the matching KnowledgeSpace without scanning
all Store-level federation containers.

KnowledgeSpace remains outside Workspace WorkState.

## Decision

1. Extend `KnowledgeSpaceListOptions` with an optional exact `name` filter.
2. Validate the filter with the existing KnowledgeSpace name rules.
3. Add `--name NAME` to `workvcs store knowledge-space-list`.
4. Preserve limit behavior and apply the limit after filtering.
5. Keep name matching exact and case-sensitive.

## Non-Goals

- This slice does not add fuzzy search, prefix search, or case-insensitive
  matching.
- This slice does not change KnowledgeSpace uniqueness rules.
- This slice does not create KnowledgeExposure or federation sync behavior.
- This slice does not make KnowledgeSpace part of WorkState.

## Consequences

- CLI users can resolve a KnowledgeSpace by confirmed exact name through the
  Engine facade.
- Federation setup workflows can verify target KnowledgeSpace existence without
  lower-level storage inspection.
- The change stays within the Store-local federation boundary.
