# Decide

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`
Code commit under analysis: `d76fe803`
Iteration focus: Decide to harden provider env cleanup

## Prioritized Action

Decide to harden provider env cleanup.

## Decision Details

The alternative, weakening assertions to accept whatever provider appears, would hide the determinism problem instead of fixing it.

## Success Criteria

- The touched contract is explicit in code or tests.
- Evidence references a real file or verified command.
- The decision reduces ambiguity, duplication, or flaky behavior.
