/**
 * @vitest-environment jsdom
 *
 * Editing a provider from Deployment › Providers must save the fields the card shows,
 * including the Google Cloud project a Vertex query needs.
 */
import i18n from "@/lib/i18n";
import type { DeploymentProvider } from "@/lib/api/edgequake/provider-offer";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ProviderEditForm } from "../provider-edit-form";

function provider(overrides: Partial<DeploymentProvider> = {}): DeploymentProvider {
  return {
    slug: "vertexai",
    display_name: "Vertex AI",
    origin: "fleet",
    api_shape: "vertexai",
    base_url: "https://aiplatform.googleapis.com",
    offered: true,
    withdrawn: false,
    probe: "ok",
    key_configured: false,
    auth: "server_google_identity",
    accepts_saved_key: true,
    connection_id: "11111111-1111-1111-1111-111111111111",
    models: [],
    ...overrides,
  };
}

function jsonResponse() {
  return Promise.resolve({
    ok: true,
    status: 200,
    headers: new Headers(),
    text: () => Promise.resolve("{}"),
    json: () => Promise.resolve({}),
  } as unknown as Response);
}

function renderForm(row: DeploymentProvider) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <ProviderEditForm row={row} onClose={() => undefined} />
    </QueryClientProvider>,
  );
}

describe("ProviderEditForm", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    vi.stubGlobal("fetch", vi.fn(() => jsonResponse()));
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("saves a Google Cloud project onto the existing connection", async () => {
    renderForm(provider());
    const project = screen.getByTestId("edit-project-vertexai");
    expect(screen.getByTestId("edit-save-vertexai")).toBeDisabled();
    fireEvent.change(project, { target: { value: "mobioos-506015" } });
    expect(screen.getByTestId("edit-save-vertexai")).toBeEnabled();
    fireEvent.click(screen.getByTestId("edit-save-vertexai"));

    await waitFor(() => {
      const save = vi.mocked(fetch).mock.calls.find((call) =>
        String(call[0]).includes("/connections/11111111-1111-1111-1111-111111111111?scope=deployment"),
      );
      expect(save).toBeTruthy();
      const body = JSON.parse(String((save?.[1] as RequestInit).body));
      expect(body.google_project).toBe("mobioos-506015");
      expect(body.display_name).toBe("Vertex AI");
      expect((save?.[1] as RequestInit).method).toBe("PUT");
    });
  });

  it("creates a saved connection when an environment provider has none", async () => {
    renderForm(
      provider({
        slug: "openai",
        display_name: "OpenAI",
        origin: "env",
        api_shape: "openai_chat",
        auth: "api_key",
        base_url: "https://api.openai.com/v1",
        connection_id: null,
      }),
    );
    expect(screen.queryByTestId("edit-project-openai")).not.toBeInTheDocument();
    fireEvent.change(screen.getByTestId("edit-name-openai"), { target: { value: "OpenAI production" } });
    fireEvent.click(screen.getByTestId("edit-save-openai"));

    await waitFor(() => {
      const save = vi.mocked(fetch).mock.calls.find((call) =>
        String(call[0]).includes("/connections?scope=deployment"),
      );
      expect(save).toBeTruthy();
      const body = JSON.parse(String((save?.[1] as RequestInit).body));
      expect(body.display_name).toBe("OpenAI production");
      expect(body.slug).toBe("openai");
      expect(body.google_project).toBeUndefined();
      expect((save?.[1] as RequestInit).method).toBe("POST");
    });
  });
});
