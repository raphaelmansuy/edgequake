# Upload Performance Testing - Progressive Load

## Overview

This directory contains comprehensive end-to-end performance tests for document upload functionality in EdgeQuake. The tests use a **progressive load testing** methodology, starting with low load and incrementally increasing to identify system behavior under stress.

## Test Files

### 1. Frontend E2E Tests (Playwright)
**File:** `edgequake_webui/e2e/upload-performance-progressive.spec.ts`

Tests document upload through the web UI and API, measuring:
- Response time distributions (min, max, avg, P50, P90, P95, P99)
- Throughput (documents per second)
- Error rates under load
- System recovery characteristics

### 2. Backend API Tests (Rust)
**File:** `edgequake/crates/edgequake-api/tests/e2e_upload_performance.rs`

Tests the backend API directly with precise timing and concurrency control.

## Test Phases

Both test suites implement the same progressive load strategy:

| Phase | Description | Documents | Concurrency | Content Size | Purpose |
|-------|-------------|-----------|-------------|--------------|---------|
| **0. Warmup** | Baseline measurement | 1 | 1 | Small | Establish single-request latency |
| **1. Light Load** | Low concurrency | 10 | 5 | Small | Verify basic concurrent handling |
| **2. Medium Load** | Moderate stress | 20 | 10 | Medium | Test normal operating conditions |
| **3. Heavy Load** | High concurrency | 50 | 25 | Medium | Identify scaling limits |
| **4. Stress Load** | Maximum stress | 100 | 50 | Large | Find breaking points |
| **5. Recovery** | Post-stress test | 10 | 5 | Small | Verify system recovery |
| **6. Mixed Size** | Varied content | 25 | 1 | Mixed | Real-world simulation |

## Running the Tests

### Prerequisites

1. **Backend services running:**
   ```bash
   # Start PostgreSQL (if using persistent storage)
   make db-start
   
   # Start backend API
   make backend
   ```

2. **Frontend service running (for Playwright tests):**
   ```bash
   cd edgequake_webui
   npm run dev -- --port 3001
   ```

### Running Playwright E2E Tests

```bash
cd edgequake_webui

# Run all performance tests
npx playwright test upload-performance-progressive.spec.ts

# Run with UI (watch mode)
npx playwright test upload-performance-progressive.spec.ts --ui

# Run specific phase
npx playwright test upload-performance-progressive.spec.ts -g "Phase 1"

# Run and generate HTML report
npx playwright test upload-performance-progressive.spec.ts --reporter=html
npx playwright show-report
```

### Running Rust Backend Tests

```bash
cd edgequake/crates/edgequake-api

# Run performance tests (they are ignored by default)
cargo test --test e2e_upload_performance -- --ignored --nocapture

# Run specific test
cargo test --test e2e_upload_performance test_progressive_load_performance -- --ignored --nocapture

# Run sustained load test
cargo test --test e2e_upload_performance test_sustained_load -- --ignored --nocapture
```

### Running with Custom Configuration

**Frontend (environment variables):**
```bash
# Use external backend
PLAYWRIGHT_BASE_URL=http://localhost:8080 npx playwright test upload-performance-progressive.spec.ts

# Adjust test parameters (edit spec file)
# - Modify phase concurrency levels
# - Change document counts
# - Adjust content sizes
```

**Backend (test configuration):**
```bash
# Run with different storage backend
DATABASE_URL=postgresql://user:pass@localhost/edgequake cargo test --test e2e_upload_performance -- --ignored --nocapture

# Use mock LLM (faster, no API costs)
# Default behavior - no OPENAI_API_KEY set

# Use real OpenAI (slower, costs money)
export OPENAI_API_KEY=sk-your-key-here
cargo test --test e2e_upload_performance -- --ignored --nocapture
```

## Understanding Results

### Key Metrics

**Response Time:**
- **Min/Max:** Range of observed latencies
- **Average:** Mean response time
- **P50 (Median):** Half of requests complete faster
- **P90/P95/P99:** 90%/95%/99% of requests complete faster

**Throughput:**
- Documents processed per second
- Indicates system capacity

**Error Rate:**
- Percentage of failed requests
- Should remain low (<10%) under normal load
- May increase under stress load (acceptable up to ~30%)

### Expected Results

**Healthy System:**
- ✅ Warmup: <5s average latency
- ✅ Light Load: <10s P95, <10% error rate
- ✅ Medium Load: <15s average, <20% error rate
- ✅ Heavy Load: >0.5 docs/sec throughput, <30% error rate
- ✅ Recovery: Within 50% of baseline performance

**Problem Indicators:**
- ❌ High error rates in light/medium load (>20%)
- ❌ Throughput drops to near zero
- ❌ No recovery after stress test
- ❌ Memory/CPU exhaustion (check system metrics)

### Sample Output

```
╔════════════════════════════════════════════════════════════════════════╗
║ Phase 1: Light Load                                                    ║
╠════════════════════════════════════════════════════════════════════════╣
║ Concurrency:          5          │ Total Uploads:      10         ║
║ Success:              10         │ Failures:           0          ║
║ Error Rate:           0.00%      │ Throughput:         2.34/s     ║
╠════════════════════════════════════════════════════════════════════════╣
║ Response Times (ms)                                                    ║
║ Min:                  1234       │ Max:                3456       ║
║ Average:              2145       │ P50 (Median):       2100       ║
║ P90:                  2890       │ P95:                3123       ║
║ P99:                  3401       │ Total Duration:     4279       ║
╚════════════════════════════════════════════════════════════════════════╝
```

## Test Reports

### Playwright Reports

**Location:** `edgequake_webui/test-results/upload-performance-report.txt`

Generated automatically after test run. Contains:
- All phase metrics
- Comparative analysis table
- Recovery analysis

**View HTML Report:**
```bash
cd edgequake_webui
npx playwright show-report
```

### Rust Test Output

**Console Output:** Printed to terminal with `--nocapture` flag

**CI Integration:** Can be parsed from test output for automated performance tracking

## Customizing Tests

### Adjust Load Parameters

**Playwright (`upload-performance-progressive.spec.ts`):**

```typescript
// Change phase configuration
const phases = [
  { name: "Light", uploads: 20, concurrency: 10 },  // Increase load
  { name: "Heavy", uploads: 100, concurrency: 50 }, // More stress
];

// Modify content sizes
function generateTestContent(size: "small" | "medium" | "large" | "xlarge") {
  // Add new size variant
}
```

**Rust (`e2e_upload_performance.rs`):**

```rust
// Adjust phase parameters
let results = execute_concurrent_uploads(
    100,  // count: more documents
    50,   // concurrency: higher parallelism
    ContentSize::Large  // size
).await;

// Add new content size
enum ContentSize {
    Small,
    Medium,
    Large,
    XLarge,  // New size
}
```

### Add Custom Phases

**Example: Burst Load Test**

Already implemented in Playwright test - see `test("Burst Load - Rapid Uploads with Pauses")`.

**Example: Sustained Load Test**

Already implemented in Rust test - see `test_sustained_load()`.

## Performance Baselines

### Reference Hardware

Tests should establish baselines for your deployment environment:

**Development (Local):**
- MacBook Pro M1/M2: ~2-3 docs/sec sustained
- Desktop (16GB RAM, SSD): ~1-2 docs/sec sustained

**Production (Cloud):**
- 2 vCPU, 4GB RAM: ~1-2 docs/sec sustained
- 4 vCPU, 8GB RAM: ~3-5 docs/sec sustained
- 8 vCPU, 16GB RAM: ~8-12 docs/sec sustained

### Factors Affecting Performance

1. **LLM Provider:**
   - Mock provider: Very fast, no network latency
   - OpenAI API: Network latency + rate limits
   - Local Ollama: Fast but CPU-intensive

2. **Storage Backend:**
   - Memory: Fastest, no persistence
   - PostgreSQL: Persistent, network overhead

3. **Document Size:**
   - Small (<1KB): Minimal processing time
   - Medium (2-5KB): Standard processing
   - Large (>10KB): LLM chunking + more tokens

4. **Concurrency:**
   - Higher concurrency = more throughput but higher latency
   - Optimal concurrency depends on CPU cores and I/O capacity

## Troubleshooting

### Tests Timing Out

**Increase timeouts in Playwright:**
```typescript
test.setTimeout(120000); // 2 minutes per test
```

**Increase timeouts in Rust:**
```rust
#[tokio::test]
#[timeout(120000)] // Requires tokio-test crate
async fn test_progressive_load_performance() { ... }
```

### High Error Rates

**Check backend logs:**
```bash
# Backend terminal should show errors
# Look for: connection pool exhausted, out of memory, etc.
```

**Verify services running:**
```bash
# Check PostgreSQL
psql -h localhost -U postgres -d edgequake -c "SELECT 1;"

# Check backend health
curl http://localhost:8080/health
```

### Memory Issues

**Monitor system resources:**
```bash
# macOS
top -pid $(pgrep -f edgequake-api)

# Linux
htop -p $(pgrep -f edgequake-api)
```

**Adjust test concurrency:**
- Reduce concurrent uploads if memory usage is high
- Increase timeout between phases to allow garbage collection

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Performance Tests

on:
  schedule:
    - cron: '0 2 * * *'  # Nightly at 2 AM
  workflow_dispatch:

jobs:
  performance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Start services
        run: |
          docker-compose up -d postgres
          make backend &
          sleep 10
      
      - name: Run Rust performance tests
        run: |
          cd edgequake/crates/edgequake-api
          cargo test --test e2e_upload_performance -- --ignored --nocapture
      
      - name: Run Playwright performance tests
        run: |
          cd edgequake_webui
          npm install
          npx playwright install --with-deps
          npx playwright test upload-performance-progressive.spec.ts
      
      - name: Upload reports
        uses: actions/upload-artifact@v3
        if: always()
        with:
          name: performance-reports
          path: |
            edgequake_webui/test-results/
            edgequake_webui/playwright-report/
```

## Best Practices

1. **Run on consistent hardware** - Performance varies significantly across machines
2. **Isolate test environment** - Close other applications during tests
3. **Warm up the system** - First few requests may be slower (JIT, caches, etc.)
4. **Test regularly** - Track performance trends over time
5. **Document baselines** - Record expected performance for your environment
6. **Monitor resources** - Track CPU, memory, disk I/O during tests
7. **Vary test data** - Use different document sizes and content types
8. **Test recovery** - Verify system returns to normal after stress

## Contributing

When adding new performance tests:

1. Follow the progressive load pattern
2. Document expected performance characteristics
3. Include clear success criteria
4. Add metrics visualization if possible
5. Update this README with new test descriptions

## Related Documentation

- [AGENTS.md](../../AGENTS.md) - Repository guidelines and development workflow
- [Production LLM Integration](../../docs/production-llm-integration.md) - LLM configuration
- [E2E Testing Guide](../../docs/e2e-testing.md) - General E2E testing practices
- [API Documentation](../../edgequake/crates/edgequake-api/README.md) - Backend API reference
