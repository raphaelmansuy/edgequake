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
    accepts_saved_key: false,
    stores_secret: false,
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

  it("explains the identity requirement instead of asking for a key Vertex cannot use", async () => {
    await renderCard();

    // The server's list marks every shape, so the card knows Vertex takes no saved key. Selecting
    // it must replace the password field with the real requirement, not accept a dead value.
    await waitFor(() => expect(screen.getByTestId("connection-shape")).toBeInTheDocument());
    const select = screen.getByTestId("connection-shape") as HTMLSelectElement;
    // The shape list must actually contain Vertex, otherwise the assertion below would pass for
    // the wrong reason (a fallback list, not the server's contract).
    const options = Array.from(select.options).map((o) => o.value);
    expect(options).toContain("vertexai");
    await act(async () => {
      fireEvent.change(select, { target: { value: "vertexai" } });
    });

    await waitFor(() => {
      expect(screen.getByTestId("connection-no-api-key")).toBeInTheDocument();
    });
    expect(screen.queryByLabelText(/^api key$/i)).not.toBeInTheDocument();
    expect(screen.getByTestId("connection-no-api-key").textContent).toMatch(
      /GOOGLE_CLOUD_PROJECT|Google Cloud identity/i,
    );
  });
});
