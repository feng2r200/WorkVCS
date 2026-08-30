# ADR-0177: Phase 4BV Why Relation Family Completion

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Early `why` slices kept coarse `Evolution` and `Epistemic` deferred-family
markers while individual relation writers and read paths were still missing.
Later slices added current relation rendering for Record-to-Record epistemic
and evolution edges, Record-to-Knowledge epistemic edges,
Knowledge-to-Knowledge supersession, and KnowledgeExposure provenance.

The remaining mismatch is metadata: `why` can render the currently implemented
confirmed relation families, but still reports the old coarse deferred markers
unconditionally.

## Decision

1. `why` no longer reports `Evolution` or `Epistemic` in
   `deferred_relation_families`.
2. The existing public enum variants remain for compatibility with callers that
   were compiled against earlier Phase 3 result shapes.
3. Existing relation edges keep their current canonical direction and ordering.
4. This slice does not add new relation kinds, generic arbitrary endpoints,
   transitive causal traversal, ranking, or CLI command spelling changes.

## Consequences

- Tools can treat an empty `deferred_relation_families` list as the current
  implementation state for relation-family coverage.
- Older clients that understand the enum variants remain source-compatible.

## Implementation Findings

- No frozen-contract contradiction was found.
- The closed status is scoped to relation families already implemented in the
  Engine facade. Product-open items such as cross-Store federation and generic
  exchange APIs remain outside this marker.
