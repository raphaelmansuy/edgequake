# Upload Performance Testing - Quick Reference

## 🚀 Quick Start

```bash
# Run all performance tests (recommended)
./scripts/run-upload-performance-tests.sh both

# Run quick smoke test (5 minutes)
./scripts/run-upload-performance-tests.sh quick

# Run frontend tests only
./scripts/run-upload-performance-tests.sh frontend

# Run backend tests only
./scripts/run-upload-performance-tests.sh backend
```

## 📊 Test Phases

| Phase | Load | Docs | Concurrency | Size | Duration |
|-------|------|------|-------------|------|----------|
| 0. Warmup | Baseline | 1 | 1 | Small | ~5s |
| 1. Light | Low | 10 | 5 | Small | ~30s |
| 2. Medium | Normal | 20 | 10 | Medium | ~1m |
| 3. Heavy | High | 50 | 25 | Medium | ~2m |
| 4. Stress | Max | 100 | 50 | Large | ~5m |
| 5. Recovery | Post-stress | 10 | 5 | Small | ~30s |
| 6. Mixed | Real-world | 25 | 1 | Mixed | ~2m |

**Total Duration:** ~15-20 minutes (full suite)  
**Quick Mode:** ~5 minutes (warmup + light load only)

## 🎯 Success Criteria

| Metric | Healthy | Warning | Critical |
|--------|---------|---------|----------|
| Warmup Avg | < 5s | 5-10s | > 10s |
| Light P95 | < 10s | 10-15s | > 15s |
| Error Rate (Light) | < 10% | 10-20% | > 20% |
| Error Rate (Medium) | < 20% | 20-30% | > 30% |
| Throughput (Heavy) | > 0.5/s | 0.3-0.5/s | < 0.3/s |
| Recovery | ±50% | ±50-100% | > 100% |

## 📈 Key Metrics Explained

- **P50 (Median):** Half of requests are faster
- **P90:** 90% of requests are faster
- **P95:** 95% of requests are faster (SLA threshold)
- **P99:** 99% of requests are faster
- **Throughput:** Documents processed per second
- **Error Rate:** % of failed requests

## 🛠️ Manual Execution

### Frontend (Playwright)

```bash
cd edgequake_webui

# Install dependencies
npm install
npx playwright install --with-deps chromium

# Run all tests
npx playwright test upload-performance-progressive.spec.ts

# Run specific phase
npx playwright test upload-performance-progressive.spec.ts -g "Phase 1"

# View HTML report
npx playwright show-report
```

### Backend (Rust)

```bash
cd edgequake/crates/edgequake-api

# Run all tests
cargo test --test e2e_upload_performance -- --ignored --nocapture

# Run specific test
cargo test --test e2e_upload_performance test_progressive_load_performance -- --ignored --nocapture

# Run sustained load test
cargo test --test e2e_upload_performance test_sustained_load -- --ignored --nocapture
```

## 🔍 Troubleshooting

### Tests Timeout
```bash
# Increase Playwright timeout
test.setTimeout(120000); // in spec file

# Check services are running
curl http://localhost:8080/health  # Backend
curl http://localhost:3001         # Frontend
```

### High Error Rates
```bash
# Check backend logs
tail -f /tmp/edgequake-backend.log

# Check database connection
psql -h localhost -U postgres -d edgequake -c "SELECT 1;"

# Reduce concurrency
# Edit test files and reduce concurrent upload counts
```

### Out of Memory
```bash
# Monitor system resources
top -pid $(pgrep -f edgequake-api)

# Reduce test load
./scripts/run-upload-performance-tests.sh quick
```

## 📁 Output Files

### Playwright Reports
- **Console:** Real-time formatted output
- **Text Report:** `edgequake_webui/test-results/upload-performance-report.txt`
- **HTML Report:** `edgequake_webui/playwright-report/index.html`

### Rust Test Output
- **Console:** Formatted tables and metrics (with `--nocapture`)
- **CI/CD:** Parse from test output for automation

## 🎨 Sample Output

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

## 🔗 Resources

- **Full Documentation:** [docs/upload-performance-testing.md](../docs/upload-performance-testing.md)
- **Repository Guidelines:** [AGENTS.md](../AGENTS.md)
- **API Documentation:** [edgequake/crates/edgequake-api/README.md](../edgequake/crates/edgequake-api/README.md)

## ⚙️ Configuration

### Environment Variables

```bash
# Frontend tests
export PLAYWRIGHT_BASE_URL=http://localhost:3001

# Backend tests (optional - uses mock by default)
export OPENAI_API_KEY=sk-your-key-here
export DATABASE_URL=postgresql://user:pass@localhost/edgequake
```

### Customize Test Parameters

**Edit test files to adjust:**
- Number of uploads per phase
- Concurrency levels
- Document content sizes
- Timeout values
- Success criteria thresholds

## 🚦 CI/CD Integration

```yaml
# .github/workflows/performance.yml
- name: Run Performance Tests
  run: |
    ./scripts/run-upload-performance-tests.sh both
    
- name: Upload Reports
  uses: actions/upload-artifact@v3
  with:
    name: performance-reports
    path: |
      edgequake_webui/test-results/
      edgequake_webui/playwright-report/
```

---

**Need help?** Check the [full documentation](../docs/upload-performance-testing.md) or [task log](../TASK-LOGS/2026-01-20-e2e-performance-upload-progressive-load.md).
