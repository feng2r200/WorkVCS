# ADR-0523: Semantic-Versioned Delivery and Cleanup

Status: Accepted; implemented and locally validated
Date: 2026-10-10

## Context

WorkVCS gained runtime compatibility checks, a stable-entry gate, package
identity diagnostics, and historical-artifact cleanup while the Cargo packages
and installed CLI continued to report `0.1.0`. A build commit, timestamp, or
digest made individual binaries distinguishable, but did not communicate the
compatibility level of the delivered CLI. The same failure mode exists in
work-governance: the plugin manifests carried changing `+codex.<timestamp>`
metadata while their semantic version remained `2.0.0`.

## Decision

1. Each project has one semantic version authority. WorkVCS uses the Cargo
   workspace package version; work-governance uses `pyproject.toml`. Package,
   plugin, and installation manifests must match that authority.
2. Any optimization that changes runtime behavior, a Skill contract, CLI
   behavior, schema or migration handling, packaging, installation, release
   workflow, or validation policy is release-impacting and must bump the
   semantic version. Compatible fixes and operational improvements use patch;
   compatible capabilities use minor; incompatible CLI, registry, or state
   contracts use major. Pure wording, formatting, or historical-record edits
   may keep the version unchanged.
3. Git commits, build timestamps, binary digests, and local package directory
   names are build identity and provenance only. They cannot substitute for a
   semantic version bump.
4. CI compares the current version with the change base and fails closed when
   release-impacting paths changed without a higher version. Packaging exposes
   the semantic version in its manifest and runtime status continues to expose
   the same version through the CLI.
5. Outside WorkVCS development, only the installed stable WorkVCS entrypoint
   may read or write WorkVCS state. After installation and adoption, dated
   package copies and old plugin-cache versions are removed through exact,
   digest/name-verified cleanup; the active stable version and a temporary
   development candidate are the only allowed local exceptions, and the
   candidate must be removed when development ends.

## Consequences

- A release identity cannot silently remain unchanged after a meaningful
  optimization.
- Version changes become reviewable and machine-enforced instead of depending
  on a user remembering to request them.
- Historical evidence keeps its original version and is not rewritten to look
  like the current release.
- Cleanup remains bounded and auditable; it does not delete source, registry,
  Store, or WorkVCS semantic records.

## Validation

- The current WorkVCS source packages report `0.2.0` and package manifests
  include that version.
- `scripts/validate-version-bump.sh` rejects unchanged versions for
  release-impacting diffs and accepts the current bump.
- CI invokes the gate with the exact change base.
- Stable installation and exact historical-artifact cleanup are verified after
  the versioned package is built.
