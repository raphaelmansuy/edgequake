/**
 * @vitest-environment jsdom
 *
 * SPEC-185 — the add-provider form must ask for exactly what a shape can use.
 *
 * The reported failure was a Vertex workspace that could never answer: chat authenticated as a
 * Google identity with no access to the configured project, and the form had happily accepted a
 * Vertex "API key" that the client refuses. The security requirement now comes from the server
 * (`accepts_saved_key`), so the field disappears instead of accepting a value that cannot work.
 */
import i18n from "@/lib/i18n";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const SHAPES = [
  {
    id: "openai",
    label: "OpenAI",
    default_base_url: "https://api.openai.com/v1",
    hint: "API key required.",
    auth: "api_key",
    requires_key: true,
    accepts_saved_key: true,
    stores_secret: true,
  },
  {
    id: "ollama",
    label: "Ollama",
    default_base_url: "http://127.0.0.1:11434",
    hint: "Local. A key is optional.",
    auth: "optional_api_key",
    requires_key: false,
    accepts_saved_key: true,
    stores_secret: true,
  },
  {
    id: "vertexai",
    label: "Vertex AI",
    default_base_url: "https://aiplatform.googleapis.com",
    hint: "Uses Google Cloud credentials on the server.",
    auth: "server_google_identity",
    requires_key: false,
    accepts_saved_key: true,
    stores_secret: true,
    secret_kinds: ["server_identity", "service_account_json", "api_key"],
  },
];

function jsonResponse(body: unknown) {
  // `apiClient` reads `response.text()` then parses, so the mock must provide it; a `.json()`
  // stub leaves the query rejected and the card silently falls back to its built-in list.
  return Promise.resolve({
    ok: true,
    status: 200,
    headers: new Headers(),
    text: () => Promise.resolve(JSON.stringify(body)),
    json: () => Promise.resolve(body),
  } as unknown as Response);
}

async function renderCard() {
  const mod = await import("../connections-card");
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  });
  return render(
    <QueryClientProvider client={client}>
      <mod.ConnectionsCard />
    </QueryClientProvider>,
  );
}

describe("ConnectionsCard security requirements", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string) => {
        if (String(url).includes("/connections/shapes")) return jsonResponse(SHAPES);
        if (String(url).includes("/connections")) return jsonResponse([]);
        return jsonResponse({});
      }),
    );
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.resetModules();
  });

  it("offers an API key field while the key-bearing shape is selected", async () => {
    await renderCard();

    await waitFor(() => {
      expect(screen.getByLabelText(/^api key$/i)).toBeInTheDocument();
    });
  });

  it("lets Vertex store a service-account file and hides the field for the server identity", async () => {
    await renderCard();

    await waitFor(() => expect(screen.getByTestId("connection-shape")).toBeInTheDocument());
    const select = screen.getByTestId("connection-shape") as HTMLSelectElement;
    const options = Array.from(select.options).map((o) => o.value);
    expect(options).toContain("vertexai");
    await act(async () => {
      fireEvent.change(select, { target: { value: "vertexai" } });
    });

    await waitFor(() => {
      expect(screen.getByTestId("connection-credential-kind")).toBeInTheDocument();
    });
    expect(screen.getByTestId("connection-no-api-key").textContent).toMatch(/Google Cloud identity/i);
    expect(screen.queryByTestId("connection-service-account")).not.toBeInTheDocument();

    await act(async () => {
      fireEvent.change(screen.getByTestId("connection-credential-kind"), {
        target: { value: "service_account_json" },
      });
    });
    expect(screen.getByTestId("connection-service-account")).toBeInTheDocument();
    expect(screen.queryByLabelText(/^api key$/i)).not.toBeInTheDocument();
  });

  it("offers Rapid-MLX from the built-in list when the server is unreachable", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(() => Promise.reject(new Error("offline"))),
    );
    await renderCard();
    await waitFor(() =>
      expect(screen.getByTestId("connections-shapes-fallback")).toBeInTheDocument(),
    );
    const select = screen.getByTestId("connection-shape") as HTMLSelectElement;
    const rapid = Array.from(select.options).find((option) => option.value === "rapid-mlx");
    expect(rapid?.textContent).toBe("Rapid-MLX");
    await act(async () => {
      fireEvent.change(select, { target: { value: "rapid-mlx" } });
    });
    const base = screen.getByLabelText(/base url/i) as HTMLInputElement;
    expect(base.value).toBe("http://127.0.0.1:8000/v1");
    const key = screen.getByLabelText(/api key \(optional\)/i) as HTMLInputElement;
    expect(key).not.toBeRequired();
    expect(key.placeholder).toMatch(/leave empty/i);
  });

  it("lets an update keep a stored key when the shape omits requires_key", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string) => {
        if (String(url).includes("/connections/shapes")) {
          return jsonResponse([
            { id: "ollama", label: "Ollama", default_base_url: "http://127.0.0.1:11434", hint: "Local" },
          ]);
        }
        if (String(url).includes("/connections")) {
          return jsonResponse([
            {
              id: "11111111-1111-4111-8111-111111111111",
              slug: "local",
              display_name: "local",
              api_shape: "ollama",
              base_url: "http://127.0.0.1:11434",
              source: "db",
              key_configured: true,
            },
          ]);
        }
        return jsonResponse({});
      }),
    );
    await renderCard();
    await waitFor(() => expect(screen.getByTestId("connection-update-local")).toBeInTheDocument());
    expect(screen.getByLabelText(/^api key$/i)).toBeRequired();
    expect(screen.getByTestId("connection-save")).toBeDisabled();
    await act(async () => {
      fireEvent.click(screen.getByTestId("connection-update-local"));
    });
    expect(screen.getByLabelText(/^api key$/i)).not.toBeRequired();
    expect(screen.getByTestId("connection-save")).toBeEnabled();
  });

  it("saves on the deployment and does not send a tenant header", async () => {
    await renderCard();
    await waitFor(() => expect(screen.getByTestId("connection-shape")).toBeInTheDocument());
    await act(async () => {
      fireEvent.change(screen.getByTestId("connection-shape"), { target: { value: "ollama" } });
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("connection-save"));
    });

    await waitFor(() => {
      const save = vi.mocked(fetch).mock.calls.find((call) =>
        String(call[0]).includes("/connections?scope=deployment"),
      );
      expect(save, "save must name deployment scope").toBeTruthy();
      const headers = new Headers((save?.[1] as RequestInit | undefined)?.headers);
      expect(headers.get("x-tenant-id")).toBeNull();
    });
  });
});
