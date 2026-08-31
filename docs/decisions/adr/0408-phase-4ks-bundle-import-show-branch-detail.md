# ADR-0408: Phase 4KS Bundle Import Show Branch Detail

Status: Accepted
Date: 2026-08-31

## Context

ADR-0407 added deterministic per-Branch head detail to Bundle preflight results
and to the live `bundle preflight-dir`, `bundle import-dir`, and
`bundle apply-dir` CLI outputs. Import attempts also persist the same detail in
`import_attempt_outcome.detail_json`, but `bundle import-show` currently exposes
only the persisted outcome, detail digest, and detail size.

That leaves a handoff gap after the import attempt id is the durable reference:
operators can verify that detail exists, but cannot recover the Branch id,
source head, target head, status, or common merge base from `import-show`
without re-running preflight against the original Bundle directory.

## Decision

Phase 4KS extends Bundle import attempt snapshots with a typed read-only
projection of the persisted `branch_head_details` array from the stored outcome
detail JSON.

The CLI `bundle import-show` output adds:

- `branch_head_detail_count`;
- itemized `branch_head_detail.N.*` fields matching ADR-0407's live preflight
  detail surface.

The command also accepts focused expectation flags for scripts:

- `--expected-branch-head-detail-count`;
- `--expected-first-branch-head-status`;
- `--expected-first-branch-head-source-head`;
- `--expected-first-branch-head-target-head`;
- `--expected-first-branch-head-merge-base`.

This is a persisted observability slice. It reads existing import attempt
outcome detail through the Engine and validates the stored canonical JSON before
rendering it. It does not change import recording, Bundle preflight
classification, Bundle application, Bundle manifest or payload format, database
schema, automatic merge behavior, Branch creation, divergent commit import, or
external Store import.

## Consequences

A recorded divergent Bundle import attempt now remains actionable after the
original Bundle directory is gone: `bundle import-show` can recover the source
head, target head, and merge base that blocked apply.

The safety boundary remains unchanged. Divergent same-Store Bundles stay
non-applied, and import-show remains a read-only query over already persisted
import attempt evidence.
