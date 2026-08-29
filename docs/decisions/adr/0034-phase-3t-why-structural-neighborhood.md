# ADR-0034: Phase 3T Why Structural Neighborhood

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed relation vocabulary, query semantics, WorkState, and
  replay requirements.

## Context

The confirmed query semantics define `why` as explaining the current object
through structural, evolution, epistemic, and verification paths. Current code
has durable WorkState replay plus semantic read surfaces for primary
containment and structural references. Evolution and epistemic relations are
not implemented yet, and Verification relations exist but do not yet have a
general relation-neighborhood read API.

Phase 3R introduced structural `references` and explicitly left later `why`
and context consumers deferred. Phase 3S introduced `diff` and explicitly left
relation graph explanations deferred. The next safe slice is therefore a
narrow, read-only structural foundation for `why`, not the full deterministic
context resolver.

## Decision

1. Phase 3T introduces a read-only structural `why` query foundation in
   `workvcs-core`.
2. The public surface remains Engine-owned and semantic:
   `Engine::why(WhyQueryOptions)`.
3. Query targets support either an explicit Commit id or the current head of a
   Branch. Branch-head selectors resolve to their current WorkStateCommit
   before replay.
4. The queried subject Entity must be current in the selected WorkState. If it
   is absent from that WorkState, the query is rejected as a structured query
   error.
5. The result reports the original selector, resolved commit id, Workspace id,
   state digest, subject Entity id, subject EntityVersion id, adjacent relation
   edges, and deferred relation families.
6. Phase 3T relation edges include only currently implemented structural
   neighborhoods:

   ```text
   contains    primary containment, container -> child
   references  structural reference, referrer -> target
   ```

7. Each relation edge reports canonical direction plus whether it is incoming
   or outgoing relative to the queried subject.
8. Edges are ordered deterministically by relation kind, direction, endpoint
   kind/id, and Relation id.
9. The query uses replayed WorkState as authority and ignores projection rows,
   Event rows, and runtime state.
10. The query is read-only. It creates no Event, ChangeSet, ChangeOperation,
    WorkStateCommit, Branch HEAD movement, projection row, runtime row, Entity
    version, or Relation version.
11. The result explicitly marks the following confirmed `why` relation
    families as deferred in this slice:

    ```text
    evolution
    epistemic
    verification
    ```

12. Phase 3T does not implement full causal traversal, evolution relations,
    epistemic relations, Verification relation neighborhoods, scheduling
    readiness explanations, context ranking, budgeted context resolution,
    `next`, restore, merge, projection materialization, migrations, or business
    CLI commands.
13. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3T does not add a CLI `why` command.

## Consequences

- WorkVCS can now answer a deterministic structural explanation for a current
  Entity without mutating state or reading derived projections.
- Later `why` slices can extend the same Engine-owned result shape with
  Verification, evolution, and epistemic neighborhoods without exposing Store
  internals.
- Callers can distinguish an implemented structural explanation from deferred
  confirmed `why` families by inspecting the deferred-family list.

## Implementation Findings

- The confirmed `why` definition is broader than the currently implemented
  relation families. Phase 3T therefore returns a structural subset and marks
  evolution, epistemic, and verification as deferred instead of silently
  claiming full `why` support.
- Scheduling relations support readiness and `next`; they are not listed in
  the confirmed `why` sentence under query semantics. Phase 3T therefore does
  not include `depends_on` or `ordered_before` in the structural why result.
