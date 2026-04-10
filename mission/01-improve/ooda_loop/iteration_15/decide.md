# Decide

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`
Code commit under analysis: `d76fe803`
Iteration focus: Orient around reproducible provider selection

## Prioritized Action

Orient around reproducible provider selection.

## Decision Details

Credential leakage breaks first-principles isolation because the same source tree behaves differently on different developer machines.

## Success Criteria

- The touched contract is explicit in code or tests.
- Evidence references a real file or verified command.
- The decision reduces ambiguity, duplication, or flaky behavior.
