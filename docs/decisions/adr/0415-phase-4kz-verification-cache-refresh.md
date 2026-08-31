# ADR-0415: Phase 4KZ Verification Cache Refresh Recovery

Status: Accepted
Date: 2026-09-01

## Context

ADR-0414 added the high-level `workvcs verify` wrapper and proved that a
Resource-backed Acceptance Criterion can become immediately verified when the
wrapper records Evidence, a Resource Observation, a Verification, and a
branch-scoped applicability cache row at the same new WorkState commit.

That dogfood run exposed the next V1 gap: after a later semantic commit advances
the Work Branch head, the existing branch-aware projection must treat the old
cache row as stale. This is correct for safety, but the operator recovery path is
too manual: the lower-level `verification cache-record` command requires copying
all Resource stamp fields from the Verification resource basis.

## Decision

1. Add an explicit Verification applicability refresh operation that evaluates
   one Verification at the current branch head.
2. The refresh derives default observed Resource stamps from the Verification
   resource basis, but only when every Resource basis entry has a baseline
   Resource Observation.
3. The refresh delegates final branch-head checks, WorkState basis checks,
   Resource fingerprint comparison, and cache persistence to the existing
   `record_verification_applicability` semantics.
4. The refresh writes only derived cache rows. It does not create a new
   WorkStateCommit, Evidence object, Resource Observation, or Verification.
5. The CLI exposes the refresh as an ergonomic recovery command under the
   existing Verification cache command family.
6. `--expected-evaluated-commit`, when supplied, is a write-before precondition
   against the resolved current branch head. A mismatch fails without refreshing
   the cache.
7. Output reports the branch, Verification, evaluated commit, applicability,
   reason code, and stamp count needed by later `cache-show`, AC status, and
   handoff flows.

## Non-Goals

- No schema change.
- No automatic or background cache refresh.
- No Resource adapter, filesystem adapter, shell execution, path/glob
  normalization, or Resource re-observation.
- No multi-target or batch refresh.
- No weakening of branch-head sensitivity: a cache row remains valid only for
  the branch head at which it was evaluated.
- No change to Verification result, AC waiver, or effective-status semantics.

## Consequences

The local CLI now has a concrete stale-cache recovery loop for the common
dogfood path: run `verify`, make later semantic commits, then explicitly refresh
the Verification applicability cache at the new branch head when the original
baseline observations remain the operator-confirmed basis.

The command remains conservative. It cannot prove external Resource freshness by
itself; it only removes the mechanical copying burden for a deliberate refresh
using Resource observations already stored and validated in WorkVCS.

## Implementation Findings

- `workctl plan init` still requires an admission manifest in this checkout. The
  slice therefore records its governance plan as a repository document and avoids
  committing an active plan index that would point at ignored runtime state.
- Independent review found that treating `--expected-evaluated-commit` only as a
  post-write assertion would let a failed CLI command refresh cache state. The
  implementation now treats that flag as a Core write-before head precondition.
