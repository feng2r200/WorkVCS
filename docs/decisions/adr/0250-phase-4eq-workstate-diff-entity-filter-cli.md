# ADR-0250: Phase 4EQ WorkState Diff Entity Filter CLI

Status: Accepted
Date: 2026-08-30

## Context

ADR-0249 added `workvcs diff` filtering by target kind and change kind. CLI
users also need to ask whether one known Entity changed between two WorkState
targets without scanning the full entity change list.

## Decision

`workvcs diff` accepts optional `--entity <ENTITY_ID>`.

The CLI parses the id through the canonical EntityId parser, computes the full
diff through `Engine::diff`, retains only matching entity changes, and clears
relation changes because the filter addresses an Entity identity.

`--entity` can be combined with `--target-kind` and `--change-kind`; all
supplied filters must match.

## Consequences

Users can directly inspect a known Entity's WorkState membership/version change
between two commits or branch heads.

This slice does not change WorkState diff semantics, does not inspect semantic
entity payloads, does not add relation filtering, and does not change the
Engine diff contract.
