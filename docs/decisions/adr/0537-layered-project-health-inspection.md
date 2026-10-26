# ADR-0537: Layered Project Health Inspection

Status: Accepted and implemented
Date: 2026-10-26

## Context

ADR-0518 defines `project health` as the complete, read-only control-plane
inspection. It validates the selected registry, every binding, routing and
journal-admission activation, declared capabilities, and optional current-CWD
resolution. That scope must remain intact: historical source archives and
derived search indexes are not control-plane authority and cannot silently
become prerequisites for ordinary Store work.

Operators nevertheless need one structured answer that distinguishes four
different questions:

1. whether the control plane is ready;
2. whether the selected current Store is valid;
3. whether explicitly selected historical source archives remain readable and
   how their original source files changed; and
4. whether explicitly selected current and historical lexical retrieval paths
   are usable.

A missing selection is not evidence of health. A disabled optional capability
is also not a reason to block a healthy control plane and current Store.

## Decision

The CLI adds a typed layered report with four top-level observations:

- `control_plane` adapts the existing ADR-0518 typed inspection result without
  parsing its key-value rendering or reimplementing its state machine;
- `current_store` records the selected ProjectRef, Store, Workspace, Branch,
  head commit and state digest, full integrity counters, and verified local
  content count;
- `historical_sources` inspects only explicitly selected immutable source
  archives using `Engine::inspect_source_archive` and preserves each archive's
  readback and original-source classification;
- `retrieval` probes explicitly selected current lexical search with unmanaged
  `Engine::search`, probes selected historical archives with
  `Engine::query_source_archive`, and reports semantic retrieval as
  `not_checked` until a pure read-only semantic inspection API is available.

Every layer and capability reports one of `healthy`, `degraded`, `blocked`, or
`not_checked`. The JSON result includes structured issues and recovery actions.
Recovery actions are descriptive only, are never automatic, and state whether
separate authorization is required.

Control-plane and current-Store layers are required for `normal_work_ready`.
Both must be `healthy`. A blocked required layer blocks the aggregate result;
an unexamined or degraded required layer makes the aggregate result degraded.

Historical sources and retrieval are optional. When they are not selected,
they remain explicitly `not_checked` and do not affect aggregate health. When
selected and unhealthy, they degrade the aggregate result but do not change
`normal_work_ready` for an otherwise healthy control plane and current Store.
This keeps optional evidence and retrieval gaps visible without redefining
ADR-0518 authority.

The layered inspection is strictly read-only. It does not write the Store,
registry, activation markers, journal, derived index, or immutable archive. It
does not refresh indexes, republish archives, activate capabilities, migrate
registries, or attempt repair.

## Consequences

- JSON consumers receive stable typed layers instead of inferring health from
  human-readable key-value text.
- `not_checked` prevents unselected optional surfaces from being misreported as
  healthy.
- Missing, moved, changed, unreadable, or damaged historical sources remain
  distinct from successful immutable archive readback.
- Missing or stale derived indexes return precise non-writing recovery advice;
  they do not disable valid native WorkVCS work.
- Semantic retrieval is not claimed healthy without a pure read-only verifier.

## Verification

Focused tests use real temporary WorkVCS Stores, full integrity validation,
real local-content validation, a real derived lexical index, and a real
immutable source archive. They verify:

- unselected optional layers remain `not_checked` while required healthy layers
  keep the aggregate healthy;
- an unselected required Store cannot be reported healthy or ready;
- a moved original source degrades historical-source currentness while archive
  readback and historical search remain healthy;
- one readable and one genuinely damaged archive retain separate success and
  failure observations in the same report;
- a missing selected lexical index yields a structured `missing_index` issue
  without creating the index or blocking otherwise healthy normal work;
- a blocked control plane blocks normal work; and
- Store, index, archive, and original-source bytes plus the inspected directory
  membership remain unchanged across the layered inspection.
