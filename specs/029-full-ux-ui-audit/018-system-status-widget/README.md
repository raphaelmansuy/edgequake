# 018 — System Status Widget Audit

**First Principle: Delight** — A healthy system should be quietly beautiful, not invisibly nothing.

## Current State

When all systems are healthy, the system status shows a compact inline badge:
```
✓ All systems operational · ollama
```

This is good (minimal, not distracting) but could be elevated with a tiny illustrated status indicator.

## Issue SS-01 · Healthy State Is Anonymous

The compact healthy state has no visual identity — it blends with other secondary text in the dashboard footer area. Users might not even notice it's a live system health indicator.

## Proposed Improvement

Add a subtle animated pulse to the green dot (already on the "API" indicator in the header) and improve the healthy badge with a more polished micro-illustration using SVG.

The badge should feel like the "green dot" status indicators used in:
- Vercel (small animated green orb for deployments)
- Linear (status indicators for connection health)
- Slack (user presence indicators)

## Reference
- [Vercel Status Indicators](https://vercel.com/status)
- [Linear Connection Status](https://linear.app)
