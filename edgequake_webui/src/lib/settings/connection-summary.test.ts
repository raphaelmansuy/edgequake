import { describe, expect, it } from "vitest";
import {
  connectionCredentialLabel,
  connectionShapeLabel,
  connectionSourceLabel,
  connectionSummary,
  connectionSummaryParts,
} from "./connection-summary";

const base = {
  api_shape: "ollama",
  base_url: "http://localhost:11434",
  source: "user",
  key_configured: false,
  key_fingerprint: null,
  last_test_ok: null,
};

describe("connectionSourceLabel", () => {
  it("names the known sources in sentence case", () => {
    expect(connectionSourceLabel("user")).toBe("Saved by you");
    expect(connectionSourceLabel("env")).toBe("From environment");
  });

  it("passes unknown sources through unchanged", () => {
    expect(connectionSourceLabel("workspace")).toBe("workspace");
  });
});

describe("connectionShapeLabel", () => {
  it("names the shape instead of printing its slug", () => {
    expect(connectionShapeLabel("vertexai")).toBe("Vertex AI");
    expect(connectionShapeLabel("openai_chat")).toBe("OpenAI-compatible");
    expect(connectionShapeLabel("ollama")).toBe("Ollama");
  });

  it("prefers the server's label and tolerates a missing shape", () => {
    expect(connectionShapeLabel("vertexai", "Google Vertex AI")).toBe("Google Vertex AI");
    expect(connectionShapeLabel("")).toBe("");
    expect(connectionShapeLabel(undefined)).toBe("");
    expect(connectionShapeLabel("unheard-of")).toBe("unheard-of");
  });
});

describe("connectionCredentialLabel", () => {
  it("names the stored credential kind, never the fingerprint", () => {
    expect(connectionCredentialLabel("google_service_account")).toBe("Service account saved");
    expect(connectionCredentialLabel("bearer")).toBe("API key saved");
    expect(connectionCredentialLabel("google_api_key")).toBe("Vertex API key saved");
  });

  it("says nothing for a shape that stores no secret", () => {
    expect(connectionCredentialLabel("none")).toBe("");
    expect(connectionCredentialLabel(null)).toBe("");
    expect(connectionCredentialLabel(undefined)).toBe("");
  });
});

describe("connectionSummary", () => {
  it("joins shape, base url and source", () => {
    expect(connectionSummary(base)).toBe(
      "Ollama · http://localhost:11434 · Saved by you",
    );
  });

  it("never leaves a doubled separator when fields are blank", () => {
    const summary = connectionSummary({ ...base, api_shape: "", base_url: "  " });
    expect(summary).toBe("Saved by you");
    expect(summary).not.toContain("··");
    expect(summary).not.toMatch(/^\s*·/);
  });

  it("adds the key fingerprint only when a key is configured", () => {
    expect(connectionSummary({ ...base, key_configured: true, key_fingerprint: "sk-…a1" })).toContain(
      "key sk-…a1",
    );
    expect(connectionSummary({ ...base, key_configured: true, key_fingerprint: null })).toContain(
      "key set",
    );
    expect(connectionSummary(base)).not.toContain("key");
  });

  it("flags a failed last test", () => {
    expect(connectionSummary({ ...base, last_test_ok: false })).toMatch(/last test failed$/);
    expect(connectionSummary({ ...base, last_test_ok: true })).not.toContain("failed");
  });
});

describe("connectionSummaryParts", () => {
  it("returns no parts for an empty connection", () => {
    expect(
      connectionSummaryParts({
        api_shape: "",
        base_url: "",
        source: "",
        key_configured: false,
      }),
    ).toEqual([]);
  });
});
