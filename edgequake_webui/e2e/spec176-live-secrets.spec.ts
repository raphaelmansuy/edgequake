/**
 * SPEC-176 live stack. Skipped unless Playwright is pointed at a running server.
 * A save uses a closed local port so the test never calls a real model provider.
 */
import { expect, test } from "@playwright/test";
import { skipUnlessLiveStack } from "./helpers/live-stack";

test.describe("SPEC-176 live secret backend", () => {
  test("shapes exist, and a key save either stores or refuses without Infisical @spec176", async ({ request }) => {
    skipUnlessLiveStack();
    const shapes = await request.get("/api/v1/connections/shapes");
    expect(shapes.status(), "shapes must be routed, not 404").not.toBe(404);
    if (shapes.status() === 401 || shapes.status() === 403) return;

    expect(shapes.status()).toBe(200);
    const body = (await shapes.json()) as Array<{ id: string }>;
    expect(body.map((shape) => shape.id)).toEqual(
      expect.arrayContaining(["openai", "anthropic", "ollama", "openrouter", "mistral"]),
    );

    const slug = `spec176-${Date.now()}`;
    const created = await request.post("/api/v1/connections?scope=deployment", {
      data: {
        slug,
        display_name: slug,
        api_shape: "openai_chat",
        base_url: "http://127.0.0.1:9",
        api_key: "sk-spec176-not-a-real-key",
        locality: "local",
        allow_private_network: true,
      },
    });
    if (created.status() === 400) {
      const text = await created.text();
      expect(text).toContain("EDGEQUAKE_SECRETS_KEY");
      return;
    }
    expect(created.status()).toBe(201);
    const saved = (await created.json()) as { id: string; api_key?: string; secret_backend?: string };
    expect(saved.api_key).toBeUndefined();
    expect(saved.secret_backend).toBe("database");

    const probe = await request.post(`/api/v1/connections/${saved.id}/test?scope=deployment`);
    expect(probe.status()).toBeLessThan(500);
    await request.delete(`/api/v1/connections/${saved.id}?scope=deployment&confirm=true`);
  });
});
