# ADR-0244: Phase 4EK Session List Focus Filter CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs session list` already supports lifecycle, workspace, and branch
filters. The rendered session projection also includes the current
`focus_entity_id`, but the CLI did not allow callers to narrow sessions by that
focus.

The engine session list projection already includes focus state. This slice does
not alter session lifecycle, focus mutation semantics, runnable projection, or
storage schema.

## Decision

`workvcs session list` accepts:

- `--focus <ENTITY_ID>`

The filter parses the entity id canonically and keeps only sessions whose
current focus entity matches that id. Sessions with no current focus never match
the filter.

When combined with lifecycle, workspace, or branch filters, all selected
conditions must match.

## Consequences

Operators can locate sessions currently focused on a Task, Goal, Plan, or other
entity using the existing session projection.

This remains a CLI-side read filter and does not expand the Engine query
surface.
