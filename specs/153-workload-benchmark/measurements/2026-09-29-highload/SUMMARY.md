> **Superseded** by [`2026-09-29-highload-push`](../2026-09-29-highload-push/EdgeQuake-Higher-User-Workload-Report.pdf): exact GCP host documented, VM telemetry, sustained (H5) test, vector charts. This earlier run had no per-level timestamps or VM inventory, so its PDF is kept as-is.

# SPEC-153 highload run — 2026-09-29-highload

- **Result:** PASS
- **PDF:** `EdgeQuake-Higher-User-Workload-Report.pdf` (topology + latency/attainment/goodput/open-loop/FAQ charts)
- **Machines:** `machines.json` (client Apple M4 Max vs SUT demo.edgequake.com v0.28.3)
- **H1 knee:** C=8 attainment=1.0 (status=measured)
- **H2 knee:** λ=0.25/s attainment=0.933 (status=measured)
- **H3:** FAQ cache hit 100% at C=16 and C=32
- **Protocol:** `specs/153-workload-benchmark/07-highload-protocol.md`
- **Safety:** no ingest/delete; API key scrubbed from run dir
