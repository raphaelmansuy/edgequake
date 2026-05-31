import { describe, expect, it } from "vitest";
import { getProviderIconColorClass } from "@/components/providers/provider-icon";

describe("ProviderIcon (UI-DRY-007)", () => {
  it("maps known providers to stable color classes", () => {
    expect(getProviderIconColorClass("openai")).toBe("text-green-600");
    expect(getProviderIconColorClass("ollama")).toBe("text-blue-600");
    expect(getProviderIconColorClass("lmstudio")).toBe("text-purple-600");
    expect(getProviderIconColorClass("mock")).toBe("text-gray-500");
  });

  it("is case-insensitive", () => {
    expect(getProviderIconColorClass("OpenAI")).toBe("text-green-600");
  });

  it("falls back for unknown providers", () => {
    expect(getProviderIconColorClass(undefined)).toBe("text-muted-foreground");
    expect(getProviderIconColorClass("unknown-vendor")).toBe(
      "text-muted-foreground",
    );
  });
});
