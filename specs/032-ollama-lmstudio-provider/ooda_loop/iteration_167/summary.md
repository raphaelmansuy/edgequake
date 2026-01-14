# OODA Iteration 167 - Request Logging and Debugging

## Observe

### Focus
Verify that request logging is available for debugging.

### Investigation

**Logging Framework**:
- EdgeQuake uses `tracing` crate
- Structured logging throughout
- Environment variable controls log level

**Log Levels**:
- `RUST_LOG=debug` - Verbose debugging
- `RUST_LOG=info` - Normal operation
- `RUST_LOG=error` - Errors only

## Orient

### Logging Points

| Event | Log Level | Information |
|-------|-----------|-------------|
| Provider creation | info | Provider type, model |
| Request start | debug | Endpoint, model |
| Token streaming | trace | Token count |
| Request complete | info | Duration, tokens |
| Error | error | Full error details |

### Debug Configuration

```bash
# Enable debug logging for LLM crate
RUST_LOG=edgequake_llm=debug cargo run

# Enable trace for all edgequake crates
RUST_LOG=edgequake=trace cargo run
```

## Decide

**Status**: ✅ COMPLETE

Logging infrastructure is in place using `tracing`.

## Act

### Verified

- `tracing` crate used throughout
- Structured logging with spans
- Environment variable configuration
- Debug info available when needed

---
*Commit: docs(OODA 167): Verify request logging infrastructure*
