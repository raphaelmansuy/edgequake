# OODA-26 Orient: Background Task Implementation Options

## Option 1: Tokio Spawn + Interval

Simple approach using tokio::spawn with interval:

```rust
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
        record_all_workspace_metrics(&state).await;
    }
});
```

Pros: Simple, built-in to tokio
Cons: No graceful shutdown, crude scheduling

## Option 2: External Scheduler (cron)

Let external cron job call API endpoint.

Pros: Decoupled, flexible
Cons: Requires deployment config, doesn't work standalone

## Option 3: Add Manual Trigger Endpoint First

Create `POST /api/v1/workspaces/{id}/metrics-snapshot` that can be:

- Called manually by users
- Called by external cron
- Called by internal scheduler later

Pros: Immediate value, testable, building block
Cons: No automatic scheduling yet

## Decision: Option 3 (Manual Trigger Endpoint)

This provides:

1. Immediate debugging value
2. Test coverage foundation
3. Building block for future scheduler
4. User control
