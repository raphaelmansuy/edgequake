# Decide

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`
Code commit under analysis: `d76fe803`
Iteration focus: Decide to keep provider comments next to assertions

## Prioritized Action

Decide to keep provider comments next to assertions.

## Decision Details

This is better than a generic module comment because the invariant is local to the surprising line.

## Success Criteria

- The touched contract is explicit in code or tests.
- Evidence references a real file or verified command.
- The decision reduces ambiguity, duplication, or flaky behavior.
