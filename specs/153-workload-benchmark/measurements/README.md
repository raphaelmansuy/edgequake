# measurements/

Run artifacts for SPEC-153. One directory per `run_id`.

```text
  measurements/<run_id>/
    env.json
    machines.json          # highload: client + SUT inventory
    charts/*.png           # highload: matplotlib diagrams
    preflight-health.json
    provider/… or system/…
    summary.json
    SUMMARY.md
    *.pdf
```

See [04-protocol](../04-protocol.md) §8 and [07-highload-protocol](../07-highload-protocol.md).

**Demo pin reminder:** every Provider run against live demo must echo

`tenant_id=00000000-0000-0000-0000-000000000002`  
`workspace_id=00000000-0000-0000-0000-000000000003`  
`base_url=https://demo.edgequake.com`

| run_id | Meaning |
|--------|---------|
| [2026-09-29-demo-preflight](2026-09-29-demo-preflight/) | Live demo preflight + MCP L7 smoke on pinned tenant (no knee) |
| [2026-09-29-demo-query-knee](2026-09-29-demo-query-knee/) | Provider Q&A capacity sweep + business PDF (cold vs cache) |
| [2026-09-29-full-hybrid](2026-09-29-full-hybrid/) | Full hybrid suite + EdgeQuake-Full-Workload-Report.pdf |
| [2026-09-29-highload](2026-09-29-highload/) | *Superseded by `-push`.* Higher concurrent users H1–H3 + charted PDF + machines.json |
| [2026-09-29-highload-push](2026-09-29-highload-push/) | Pushed run: H1–H4 past the knee + H5 sustained ladder, exact GCP VM (`gcp-server.json`), VM telemetry, vector charts + 300 DPI PNGs |
