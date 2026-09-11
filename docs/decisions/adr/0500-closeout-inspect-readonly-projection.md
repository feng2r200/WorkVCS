# ADR-0500: Read-only Closeout Inspection Projection

Status: Accepted / Implemented (current capability)
Date: 2026-09-11

## Context

Work-governance needs a cheap final inspection that can show the mechanical
state surrounding a bounded target without mutating WorkVCS or turning a
projection into a policy decision. The inspection must be safe to run after an
interrupted or disputed workflow and must provide evidence that it did not
change the Store.

The P0-4 implementation and focused validation establish this read-only
projection boundary as a current capability.

## Decision

The current command is an explicit-target inspection:

```text
workvcs closeout inspect [OPTIONS] --target-kind <TARGET_KIND> --target <TARGET> <STORE|--cwd <PATH>>
```

The request must carry `--target-kind goal|plan|task` and `--target TARGET`. It
resolves the target through either a project `--cwd PATH` binding or an
explicit `STORE` with `--branch BRANCH` or `--commit COMMIT`. It never
implicitly selects a Session, Branch, or target from ambient runtime state.

The inspector opens the Store with OS-level read-only protections and a
database `query_only` connection. The open path must reject migration,
initialization, schema repair, runtime writes, Work-State commits, receipts,
Claims, Session changes, and any other write. No schema change is required.

The projection expands only the target's direct mechanical neighborhood:

- a Task expands its own Acceptance Criteria, Verification Requirements,
  Verification judgments, and Evidence;
- a Plan expands direct Tasks while child Plans remain non-expanded;
- a Goal expands direct Plans and Tasks while child Goals remain non-expanded;
- active Sessions, Claims, Handoffs, and mechanical receipt states are
  aggregated as bounded runtime/provenance summaries.

The projection does not recursively expand unrelated descendants or infer
semantic completion. The default item budget is 50 and the hard maximum is
200. Ordering is stable and deterministic. When the budget trims output, the
result reports `truncated` and omitted-category/count information rather than
silently dropping items.

Runtime summaries are exact-target aggregates for active Sessions, Claims,
focused Handoff v1, and mechanical receipt states. If the selected commit is
not the current source for runtime coordination, current Sessions and Claims
are not projected as if historical; the result reports the corresponding
source gap. Receipt status is evaluated using the source commit time.

The output includes before/after proof captured around the inspection:

- Work Branch HEAD and Work-State digest before and after;
- the inspected target's relevant state digest before and after; and
- Store main database/WAL/SHM file metadata before and after the read;
- persisted content digest, size, and metadata for file-backed Evidence, when
  those facts exist in WorkVCS; and
- no dereference of arbitrary Evidence paths. If no file was actually checked,
  the projection reports no file check and does not fabricate metadata.

The proof is observational evidence, not a guarantee about concurrent writers
outside the inspection process. A detected change fails or is reported as
drift rather than being hidden.

## Policy boundary

Closeout inspect may report mechanical states such as active, blocked,
expired, consumed, stale, missing, or drifted. It must not output conclusions
that authorization is sufficient, quality is sufficient, work is ready,
complete, safe to push, safe to deploy, or otherwise approved by policy.
Those judgments remain work-governance responsibilities.

Running the inspection is zero-write: it creates no Plan, Session, Claim,
receipt, Handoff, checkpoint, event, or other WorkVCS state. ADR-0501 defines
the broader distinction between No-Plan and standalone cognition capture.

## Current capability boundary

Closeout inspect is a current capability. Its OS-level read-only open,
query-only enforcement, explicit target resolution, bounded projection, and
before/after proof are part of the implemented contract. Receipt `revoke` and
receipt projection into `context`/`why` remain deferred and are not blockers for
closeout.

## Non-goals

- No Store migration, initialization, repair, schema write, or runtime write.
- No implicit Session, Branch, target, policy, authorization, quality, ready,
  complete, push, or deploy selection/conclusion.
- No recursive full-graph dump, unbounded evidence expansion, or hidden
  truncation.
- No schema change and no new persistent closeout state.

## Consequences

Operators receive a stable, bounded mechanical snapshot with explicit target
and non-mutation evidence. The strict open and proof requirements make the
inspection safe for No-Plan use, while the absence of policy conclusions keeps
the ownership boundary between WorkVCS projection and work-governance judgment
explicit.
