# Decide

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`
Code commit under analysis: `d76fe803`
Iteration focus: Orient around test-harness drift

## Prioritized Action

Orient around test-harness drift.

## Decision Details

Tests that depend on local machine credentials or hardcoded provider dimensions are especially risky because they become flaky outside CI.

## Success Criteria

- The touched contract is explicit in code or tests.
- Evidence references a real file or verified command.
- The decision reduces ambiguity, duplication, or flaky behavior.
