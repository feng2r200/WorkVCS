# ADR-0160: Phase 4BE Bundle Event Provenance Apply

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

ADR-0003 defines Events as semantic provenance associated with ChangeSets,
Sessions, and other operations; they are not replay truth. Same-Store Bundle
apply already restores the commit, ChangeSet, ChangeOperation, membership, typed
identity, relation, verification, session, knowledge exposure, and checkpoint
closures, but it did not carry the corresponding Event rows.

## Decision

1. Bundle export now includes an `events` closure for Event rows referenced by
   the exported commit closure's ChangeSets.
2. The Event manifest ref records event id, optional workspace id, optional
   changeset id, optional session id, event kind, occurrence timestamp, and the
   digest/size of `event.payload_json`.
3. Event payload JSON is exported through the payload index with role
   `event_payload`.
4. Same-Store Bundle apply restores Event rows after the ChangeSet/commit
   closure is inserted, reusing matching existing rows and rejecting immutable
   conflicts.
5. Event coverage remains same-Workspace/same-Store scoped; referenced
   ChangeSets and Sessions must be present in the imported closure.

## Non-Goals

- This slice does not implement cross-Store import, Event remapping, remote
  transport, replay from Events, or Event mutation APIs.
- KnowledgeExposure transitions with non-null `event_id` remain outside the
  active writer-generated test path; they are validated for closure coverage but
  are not introduced by this slice.

## Consequences

- Same-Store fast-forward Bundle imports now preserve semantic Event provenance
  for imported commits.
- Import attempt outcome details and CLI apply output report `imported_events`.
- Event payloads add one payload reference per exported Event while sharing
  payload files by digest with existing JSON payloads when bytes are identical.

## Implementation Findings

- Existing semantic write paths create ChangeSet-linked Event rows, but current
  KnowledgeExposure lifecycle transitions still store `event_id = NULL`.
