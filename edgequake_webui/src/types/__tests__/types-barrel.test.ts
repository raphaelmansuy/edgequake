import { describe, expect, it } from "vitest";
import {
  Document,
  GraphNode,
  LoginRequest,
  QueryMode,
  QUERY_MODES,
  ServerConversation,
  Tenant,
  Workspace,
  isQueryMode,
} from "@/types";

describe("types barrel (UI-DRY-006)", () => {
  it("re-exports domain types from split modules", () => {
    expect(typeof isQueryMode).toBe("function");
    expect(QUERY_MODES.length).toBeGreaterThan(0);

    const _graph: GraphNode = { id: "1", label: "A", node_type: "PERSON" };
    const _doc: Document = { id: "d1" };
    const _login: LoginRequest = { username: "u", password: "p" };
    const _tenant: Tenant = { id: "t1", name: "T", created_at: "" };
    const _ws: Workspace = { id: "w1", tenant_id: "t1", name: "W", created_at: "" };
    const _conv: ServerConversation = {
      id: "c1",
      tenant_id: "t1",
      user_id: "u1",
      title: "Test",
      mode: "local" as QueryMode,
      is_pinned: false,
      is_archived: false,
      message_count: 0,
      meta: {},
      created_at: "",
      updated_at: "",
    };

    expect(_graph.id).toBe("1");
    expect(_doc.id).toBe("d1");
    expect(_login.username).toBe("u");
    expect(_tenant.name).toBe("T");
    expect(_ws.name).toBe("W");
    expect(_conv.title).toBe("Test");
  });
});
