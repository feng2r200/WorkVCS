# ADR-0504: WorkVCS Skill Tree Install Integrity

Status: Accepted
Date: 2026-09-12

## Context

The WorkVCS package copies the complete `skills/workvcs` directory but records
and verifies only the digest of `SKILL.md`. References, agents metadata,
templates, or future scripts can therefore be missing, modified, or augmented
while packaging and installation still report success.

The Skill is the caller-facing contract for configuration, recording,
recovery, Evidence, and multi-Agent coordination. Verifying only its entry file
does not prove the installed capability that Codex actually loads.

## Decision

Packaging creates a deterministic SHA-256 manifest from the source WorkVCS
Skill tree for every regular file, sorted by relative path. Symlinks and other
special entries are rejected rather than copied outside the integrity contract.
Because the manifest is line-oriented, relative paths containing control
characters are rejected instead of being represented ambiguously. Enumeration,
sorting, and every file or manifest hash are fail-closed; an unreadable file or
invalid digest aborts creation and verification.
The package manifest records the Skill file count, tree-manifest path, and
digest of that tree manifest.

Each invocation uses a collision-resistant artifact path and refuses any
pre-existing artifact or archive. Before archiving, packaging verifies the
packaged tree against the source-generated manifest and rejects missing,
modified, or extra files. Explicit Skill
installation copies atomically, verifies the installed tree against the same
manifest, and reports the installed tree digest and file count. The generated
tree manifest remains package metadata and is not installed as part of the
Skill itself.

Any future change to the WorkVCS Skill directory participates automatically;
callers do not maintain a handwritten file allowlist.

## Consequences

- Package and install success covers the complete capability, not only its
  entrypoint.
- A changed reference or unexpected file produces a deterministic failure.
- WorkVCS remains the owner of its independently installed Skill; no governance
  Plugin dependency is introduced.
