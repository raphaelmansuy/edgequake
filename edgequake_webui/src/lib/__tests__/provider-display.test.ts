import { describe, expect, it } from "bun:test";
import { getProviderAuthLabel } from "@/lib/provider-display";

describe("getProviderAuthLabel", () => {
  it("returns Identity (ADC) for vertexai", () => {
    expect(getProviderAuthLabel("vertexai")).toBe("Identity (ADC)");
    expect(getProviderAuthLabel("vertexai", "oauth2_identity")).toBe("Identity (ADC)");
  });

  it("returns null for providers without special auth", () => {
    expect(getProviderAuthLabel("openai")).toBeNull();
  });
});
