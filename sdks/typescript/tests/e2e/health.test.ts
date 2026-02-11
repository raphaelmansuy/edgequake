/**
 * E2E Tests: Health & System endpoints
 *
 * Tests the basic system health, readiness, and settings endpoints
 * against a live EdgeQuake backend.
 *
 * Run: EDGEQUAKE_E2E_URL=http://localhost:8080 npm test -- tests/e2e/
 */

import { beforeAll, describe, expect, it } from "vitest";
import { EdgeQuake } from "../../src/index.js";
import { E2E_ENABLED, createE2EClient } from "./helpers.js";

// WHY: Skip entire suite when no backend is available.
// This allows `npm test` to pass in CI without a running server.
const describeE2E = E2E_ENABLED ? describe : describe.skip;

describeE2E("E2E: Health & System", () => {
  let client: EdgeQuake;

  beforeAll(() => {
    client = createE2EClient()!;
  });

  it("should return healthy status", async () => {
    const health = await client.health();
    expect(health).toBeDefined();
    expect(health.status).toBe("healthy");
  }, 15_000);

  it("should include version in health response", async () => {
    const health = await client.health();
    expect(health.version).toBeTruthy();
    expect(typeof health.version).toBe("string");
  }, 15_000);

  it("should include storage_mode in health response", async () => {
    const health = await client.health();
    // WHY: EdgeQuake now requires PostgreSQL — memory mode removed
    expect(health.storage_mode).toBe("postgresql");
  }, 15_000);

  it("should include component status in health", async () => {
    const health = await client.health();
    expect(health.components).toBeDefined();
    expect(health.components.kv_storage).toBe(true);
    expect(health.components.vector_storage).toBe(true);
    expect(health.components.graph_storage).toBe(true);
    expect(health.components.llm_provider).toBe(true);
  }, 15_000);

  it("should report readiness", async () => {
    const ready = await client.ready();
    expect(ready).toBeDefined();
    // WHY: /ready returns plain text "OK", not JSON
    expect(ready).toBe("OK");
  }, 10_000);

  it("should report liveness", async () => {
    const live = await client.live();
    expect(live).toBeDefined();
    expect(live).toBe("OK");
  }, 10_000);

  it("should return provider status", async () => {
    // WHY: Actual SDK method is providerStatus(), not getProviderStatus()
    const status = await client.settings.providerStatus();
    expect(status).toBeDefined();
  }, 10_000);

  it("should list available providers", async () => {
    const providers = await client.settings.listProviders();
    expect(providers).toBeDefined();
  }, 10_000);
});
