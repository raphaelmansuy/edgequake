# Decide

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`
Code commit under analysis: `d76fe803`
Iteration focus: Decide to keep explicit ProcessingStats fixtures

## Prioritized Action

Decide to keep explicit ProcessingStats fixtures.

## Decision Details

This preserves signal by using mutation only when mutation is what the test cares about.

## Success Criteria

- The touched contract is explicit in code or tests.
- Evidence references a real file or verified command.
- The decision reduces ambiguity, duplication, or flaky behavior.
