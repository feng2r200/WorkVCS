# Changelog

All notable public changes to WorkVCS will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and released versions will follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

This section is the draft change set for the planned first public version
`v0.1.0`. The planned distribution is source-only; the version is not tagged
or published, binary archives are deferred, and the Rust packages remain
unpublished. See the [v0.1.0 source-only release-notes draft](docs/releases/v0.1.0.md)
and the [candidate dependency audit record](docs/provenance/v0.1.0-source-release-audit.md).

### Added

- ProjectRef v2 control-plane models, deterministic logical-project
  resolution, tool-neutral locator input, and a concrete desktop-project
  adapter that preserves semantic-project ownership ahead of Git and CWD.
- Read-only v1-to-v2 migration preview, digest-locked migration and rollback,
  independently activated v2 reads and journal admission, status-first
  recovery, and isolated fault-injection coverage.
- Digest-locked, same-registry-lineage atomic refresh for stale read-routing
  and journal-admission markers after a registry revision advances.
- Target-neutral `cognition_v2` capture with one canonical primary Record,
  idempotent delivery receipts, CaptureGroup secondary references, and
  Store-free secondary recall.
- A 93-row ProjectRef acceptance matrix and bounded live local canary evidence.
- Structured `--rationale-json` input for Task transitions that require a
  terminal rationale.
- A complete Simplified Chinese README with bidirectional language navigation.
- Public project overview, architecture map, contributor guidance, security
  policy, support policy, community health files, and continuous integration.
- Apache-2.0 licensing for the repository and Rust packages.
- Standard `--version` / `-V` CLI output.

### Changed

- The bundled Agent Skill now supports an explicit higher-priority
  required-participation policy: WorkVCS is selected at task start and
  reconciled before closeout while Plan admission and all mutation authority
  gates remain independent. The public default remains value-gated.
- The bundled Agent Skill now keeps routine same-target delivery and exact
  readback with the task that created an operation, while historical open
  inventories require semantic-currentness and counterfactual-effect
  classification instead of user monitoring or batch replay.
- Project discovery and durable capture no longer equate the ambient CWD with
  logical ownership; resolution follows explicit ProjectRef, verified semantic
  project, repository, then CWD precedence and fails closed on ambiguity.
- The public `capture` route admits registry-v2 intents without caller-selected
  Store targets; target delivery remains a separate guarded recovery action.
- Public-facing maturity language now distinguishes the bounded locally
  release-ready V1 implementation from the unreleased `0.1.0` package version.

### Safety

- Registry replacement, routing activation, journal admission, recovery, and
  rollback use explicit snapshot digests, ordered locks, atomic replacement,
  and fail-closed recovery probes rather than implicit fallback or dual write.
- Activation refresh additionally locks the exact installed stale-marker
  digest and rejects wrong-lineage, equal/newer-revision, invalid, symlinked,
  absent, or concurrently changed markers.

### Remaining release gates

- Recheck the exact final code and documentation candidate after review-only
  changes, including the recorded dependency and license-policy decisions.
- Keep the first release source-only; binary archives and crates.io publication
  remain deferred unless a later scope decision changes them.
- With separate authorization, select the final commit, create the `v0.1.0`
  tag, push it, and publish the source-only GitHub Release. No tag or release
  exists yet.
