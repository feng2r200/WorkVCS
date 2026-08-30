# ADR-0251: Phase 4ER WorkState Diff Relation Filter CLI

Status: Accepted
Date: 2026-08-30

## Context

ADR-0250 added exact Entity filtering for `workvcs diff`. WorkState changes also
include Relation membership/version changes, and CLI users need the same direct
inspection path for one known Relation id.

## Decision

`workvcs diff` accepts optional `--relation <RELATION_ID>`.

`--entity` and `--relation` are mutually exclusive. The CLI parses relation ids
through the canonical RelationId parser, computes the full diff through
`Engine::diff`, retains only matching relation changes, and clears entity
changes because the filter addresses a Relation identity.

`--relation` can be combined with `--target-kind` and `--change-kind`; all
supplied filters must match.

## Consequences

Users can directly inspect a known Relation's WorkState membership/version
change between two commits or branch heads.

This slice does not change WorkState diff semantics, does not inspect semantic
relation payloads, and does not change the Engine diff contract.
