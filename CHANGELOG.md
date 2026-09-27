# Changelog

All notable public changes to WorkVCS will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and released versions will follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- ProjectRef v2 control-plane models, deterministic logical-project
  resolution, tool-neutral locator input, and a concrete desktop-project
  adapter that preserves semantic-project ownership ahead of Git and CWD.
- Read-only v1-to-v2 migration preview, digest-locked migration and rollback,
  independently activated v2 reads and journal admission, status-first
  recovery, and isolated fault-injection coverage.
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

### Remaining release gates

- Complete the first public release review.
- Select, tag, and publish the first public version.
