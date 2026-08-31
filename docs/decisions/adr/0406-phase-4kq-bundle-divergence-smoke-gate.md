# ADR-0406: Phase 4KQ Bundle Divergence Smoke Gate

Status: Accepted
Date: 2026-08-31

## Context

ADR-0404 added repository-level smoke coverage for same-Store Bundle
fast-forward apply. ADR-0405 extended that gate to prove checkpoint candidates
survive the same Bundle transport path.

The current Bundle preflight implementation already counts diverged exported
Branch heads through `branch_heads_diverged`, and apply refuses any import whose
preflight cannot apply. However, a same-Store divergence is still reported with
the generic `same_store_import_not_implemented` action. That makes the
last-write-wins prohibition observable only through count fields and adjacent
knowledge, not through the primary action string used by CLI smoke gates and
import journals.

## Decision

Phase 4KQ makes same-Store exported Branch head divergence an explicit
non-apply action:

- `same_store_divergence_detected`

The action applies when Bundle preflight is valid, the Bundle source Store is
the same Store, the incoming target Commit is not already present locally, and
at least one exported Branch head diverges from the local Branch head. The
preflight result keeps `import_required=true`, `can_apply=false`, and the
existing branch-head count fields.

The repository CLI smoke workflow adds a separate Bundle divergence gate. It
creates a source Store, copies it after an older Branch head, advances the
source Branch to the exported head, independently advances the copied target
Store from the same older head, exports the source Bundle directory, validates
and preflights it against the target Store with one diverged Branch head,
records an import attempt with the explicit divergence outcome, runs
`apply-dir` without `--require-applied`, and verifies the target Branch head and
target-only Task remain current.

This is classification and process-level acceptance coverage. It does not
implement missing Branch creation, automatic conflict resolution, divergent
merge import, external Store import, runtime recovery after import, Bundle
format changes, schema changes, or last-write-wins behavior.

## Consequences

Same-Store Bundle divergence becomes visible through the primary preflight,
import-attempt, and apply outcome, making the refusal path easier to assert in
scripts and easier to interpret in import journals.

The smoke workflow now proves both sides of the same-Store Bundle boundary:
fast-forward imports can apply, while divergent imports are classified,
non-applied, and leave the target Branch unchanged.

Missing Branch creation, conflict materialization, merge-from-Bundle workflows,
external Store import, and runtime recovery remain separate decisions.
