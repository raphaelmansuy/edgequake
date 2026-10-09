# 01 — First principles

1. **Env remains valid.** Process-wide env providers keep working and appear as read-only "env connections".
2. **Keys never round-trip.** Write-only; masked fingerprint on read; redacted Debug.
3. **Health never lies.** Unavailable local servers are unavailable.
4. **Locality is a property of the server, not a brand name.** oMLX and llama.cpp get the same timeouts as Ollama.
5. **One documented path.** Docker quickstart, source `make dev`, Helm. Everything else is an alias.
6. **Upgrade is SPEC-150.** Connections are expand-only schema. Serve never migrates.
7. **Loopback by default.** Quickstart publishes 127.0.0.1. Public bind is explicit.
