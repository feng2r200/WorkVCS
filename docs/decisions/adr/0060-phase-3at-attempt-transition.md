# ADR-0060: Phase 3AT Attempt Terminal Transitions

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Attempt lifecycle.

## Context

Phase 3AS added explicit running Attempt Records. The confirmed Record
lifecycle requires Attempt transitions from `running` to `succeeded`, `failed`,
or `inconclusive`, and terminal Attempts must never reopen.

## Decision

1. Phase 3AT extends `RecordStatus` with `succeeded`, `failed`, and
   `inconclusive`.
2. Attempt transitions are explicit semantic Record operations.
3. The only Attempt transition graph in this slice is:

   ```text
   running -> succeeded | failed | inconclusive
   ```

4. Terminal Attempt Records cannot transition again.
5. The CLI exposes `record attempt-status ... --status
   succeeded|failed|inconclusive --rationale <text>`.
6. This slice does not implement one-shot approach/result creation, automatic
   Task execution coupling, Handoff records, or typed causal relations from
   Attempts to Findings.

## Consequences

- Attempt Records now satisfy the confirmed lightweight lifecycle.
- Later slices can add richer Attempt payloads or causal relations without
  changing the terminal-state rule.

## Implementation Findings

- The existing Record transition machinery was sufficient after dispatching by
  Record kind.
