/**
 * Unit tests for the EdgeQuake client class.
 *
 * @module tests/client.test
 */

import { describe, expect, it } from "vitest";
import { EdgeQuake } from "../../src/client.js";

describe("EdgeQuake client", () => {
  it("creates with default config", () => {
    const client = new EdgeQuake();
    expect(client).toBeInstanceOf(EdgeQuake);
    expect(client.baseUrl).toBe("http://localhost:8080");
  });

  it("creates with custom base URL", () => {
    const client = new EdgeQuake({ baseUrl: "http://custom:9090" });
    expect(client.baseUrl).toBe("http://custom:9090");
  });

  it("has all resource namespaces", () => {
    const client = new EdgeQuake();

    // Auth & users
    expect(client.auth).toBeDefined();
    expect(client.users).toBeDefined();
    expect(client.apiKeys).toBeDefined();

    // Documents
    expect(client.documents).toBeDefined();
    expect(client.documents.pdf).toBeDefined();

    // Query & chat
    expect(client.query).toBeDefined();
    expect(client.chat).toBeDefined();

    // Conversations
    expect(client.conversations).toBeDefined();
    expect(client.conversations.messages).toBeDefined();
    expect(client.folders).toBeDefined();
    expect(client.shared).toBeDefined();

    // Graph
    expect(client.graph).toBeDefined();
    expect(client.graph.entities).toBeDefined();
    expect(client.graph.relationships).toBeDefined();

    // Multi-tenancy
    expect(client.tenants).toBeDefined();
    expect(client.workspaces).toBeDefined();

    // Pipeline & tasks
    expect(client.tasks).toBeDefined();
    expect(client.pipeline).toBeDefined();
    expect(client.costs).toBeDefined();

    // Observability
    expect(client.lineage).toBeDefined();
    expect(client.chunks).toBeDefined();
    expect(client.provenance).toBeDefined();

    // Settings & models
    expect(client.settings).toBeDefined();
    expect(client.models).toBeDefined();
    expect(client.ollama).toBeDefined();
  });

  it("exposes transport for advanced usage", () => {
    const client = new EdgeQuake();
    expect(client.transport).toBeDefined();
    expect(typeof client.transport.request).toBe("function");
    expect(typeof client.transport.stream).toBe("function");
    expect(typeof client.transport.upload).toBe("function");
    expect(typeof client.transport.websocketUrl).toBe("function");
  });

  it("accepts API key in config", () => {
    const client = new EdgeQuake({ apiKey: "test-key-123" });
    expect(client).toBeInstanceOf(EdgeQuake);
  });

  it("accepts credentials in config", () => {
    const client = new EdgeQuake({
      credentials: { username: "admin", password: "secret" },
    });
    expect(client).toBeInstanceOf(EdgeQuake);
  });
});
