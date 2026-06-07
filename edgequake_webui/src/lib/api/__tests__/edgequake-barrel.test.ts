import { describe, expect, it } from "vitest";
import {
  checkHealth,
  getDocuments,
  getGraph,
  login,
  query,
} from "@/lib/api/edgequake";

describe("edgequake API barrel (UI-DRY-001)", () => {
  it("re-exports domain functions from split modules", () => {
    expect(typeof checkHealth).toBe("function");
    expect(typeof login).toBe("function");
    expect(typeof getDocuments).toBe("function");
    expect(typeof query).toBe("function");
    expect(typeof getGraph).toBe("function");
  });
});
