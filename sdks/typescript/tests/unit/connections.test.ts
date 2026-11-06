import { beforeEach, describe, expect, it } from "vitest";
import { ConnectionsResource } from "../../src/resources/connections.js";
import type { HttpTransport } from "../../src/transport/types.js";
import { createMockTransport } from "../helpers/mock-transport.js";

const connection = {
  id: "11111111-1111-1111-1111-111111111111",
  slug: "my-ollama",
  display_name: "Ollama",
  api_shape: "ollama",
  locality: "local",
  base_url: "http://127.0.0.1:11434",
  auth_scheme: "none",
  key_configured: false,
  timeout_secs: 120,
  allow_private_network: true,
  source: "db",
};

const probe = {
  ok: true,
  kind: "ok",
  latency_ms: 12,
  message: "ok",
  models: ["gemma3"],
  chat_ok: true,
  embed_ok: false,
  list_ok: true,
};

describe("ConnectionsResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let connections: ConnectionsResource;

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/connections": { body: [connection] },
      "POST /api/v1/connections": { body: connection },
      "PUT /api/v1/connections/11111111-1111-1111-1111-111111111111": {
        body: connection,
      },
      "DELETE /api/v1/connections/11111111-1111-1111-1111-111111111111": {
        body: null,
      },
      "POST /api/v1/connections/11111111-1111-1111-1111-111111111111/test": {
        body: probe,
      },
      "POST /api/v1/providers/test": { body: probe },
    });
    connections = new ConnectionsResource(mock as unknown as HttpTransport);
  });

  it("lists connections", async () => {
    const rows = await connections.list();
    expect(mock.lastRequest?.method).toBe("GET");
    expect(mock.lastRequest?.path).toBe("/api/v1/connections");
    expect(rows[0]?.slug).toBe("my-ollama");
  });

  it("creates a connection", async () => {
    await connections.create({
      slug: "my-ollama",
      display_name: "Ollama",
      api_shape: "ollama",
      base_url: "http://127.0.0.1:11434",
      api_key: "secret",
      allow_private_network: false,
    });
    expect(mock.lastRequest?.method).toBe("POST");
    expect(mock.lastRequest?.path).toBe("/api/v1/connections?scope=deployment");
    expect(mock.lastRequest?.body).toMatchObject({
      slug: "my-ollama",
      api_key: "secret",
      allow_private_network: false,
    });
  });

  it("updates and deletes a connection", async () => {
    const id = connection.id;
    await connections.update(id, {
      slug: "my-ollama",
      display_name: "Ollama",
      api_shape: "ollama",
      base_url: "http://127.0.0.1:11434",
    });
    expect(mock.lastRequest?.method).toBe("PUT");
    expect(mock.lastRequest?.path).toBe(`/api/v1/connections/${id}?scope=deployment`);

    await connections.delete(id);
    expect(mock.lastRequest?.method).toBe("DELETE");
    expect(mock.lastRequest?.path).toBe(`/api/v1/connections/${id}?scope=deployment`);
  });

  it("probes a stored connection and an unsaved provider", async () => {
    const stored = await connections.test(connection.id);
    expect(mock.lastRequest?.method).toBe("POST");
    expect(mock.lastRequest?.path).toBe(
      `/api/v1/connections/${connection.id}/test?scope=deployment`,
    );
    expect(stored.kind).toBe("ok");

    await connections.testProvider({
      shape: "ollama",
      base_url: "http://127.0.0.1:11434",
      model: "gemma3",
      expected_dimension: 768,
    });
    expect(mock.lastRequest?.path).toBe("/api/v1/providers/test");
    expect(mock.lastRequest?.body).toEqual({
      shape: "ollama",
      base_url: "http://127.0.0.1:11434",
      model: "gemma3",
      expected_dimension: 768,
    });
  });
});
