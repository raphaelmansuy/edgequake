/**
 * E2E Tests — Conversations, Messages, Folders, Shared
 *
 * WHY: Validates conversation CRUD, message operations, folder management,
 * and sharing features against the live EdgeQuake backend.
 *
 * Requires: EDGEQUAKE_E2E_URL, EDGEQUAKE_TENANT_ID, EDGEQUAKE_USER_ID, EDGEQUAKE_WORKSPACE
 */

import { beforeAll, describe, expect, it } from "vitest";
import { EdgeQuake } from "../../src/index.js";
import {
  createE2EClient,
  E2E_ENABLED,
  E2E_TENANT_ID,
  E2E_USER_ID,
  testId,
} from "./helpers.js";

// WHY: Conversations/folders require X-Tenant-ID and X-User-ID headers.
// Skip all tests when these env vars are not set to avoid false failures.
const hasTenantUser = !!(E2E_TENANT_ID && E2E_USER_ID);
const describeE2E = E2E_ENABLED && hasTenantUser ? describe : describe.skip;

// ── Conversations ──────────────────────────────────────────

describeE2E("Conversations E2E", () => {
  let client: EdgeQuake;

  beforeAll(async () => {
    client = createE2EClient()!;
  }, 30_000);

  it("lists conversations with cursor-based pagination", async () => {
    const res = await client.conversations.list({ limit: 5 });
    expect(res).toBeDefined();
    expect(res.items).toBeDefined();
    expect(Array.isArray(res.items)).toBe(true);
    expect(res.pagination).toBeDefined();
    expect(typeof res.pagination.has_more).toBe("boolean");
  });

  it("creates, gets, updates, and deletes a conversation", async () => {
    const title = testId("conv");

    // Create
    const created = await client.conversations.create({
      title,
      mode: "hybrid",
    });
    expect(created).toBeDefined();
    expect(created.id).toBeDefined();
    expect(created.title).toBe(title);

    // Get (returns ConversationWithMessages)
    const detail = await client.conversations.get(created.id);
    expect(detail).toBeDefined();
    expect(detail.conversation.id).toBe(created.id);
    expect(detail.conversation.title).toBe(title);
    expect(Array.isArray(detail.messages)).toBe(true);

    // Update
    const updatedTitle = title + "-updated";
    const updated = await client.conversations.update(created.id, {
      title: updatedTitle,
      is_pinned: true,
    });
    expect(updated).toBeDefined();
    expect(updated.title).toBe(updatedTitle);
    expect(updated.is_pinned).toBe(true);

    // Delete
    await client.conversations.delete(created.id);

    // Verify deleted — should 404
    try {
      await client.conversations.get(created.id);
      expect.fail("Should have thrown NotFoundError");
    } catch (error: any) {
      expect(error.status).toBe(404);
    }
  }, 30_000);

  it("filters conversations by archived status", async () => {
    const res = await client.conversations.list({
      filter_archived: false,
      limit: 5,
    });
    expect(res).toBeDefined();
    expect(Array.isArray(res.items)).toBe(true);
  });

  it("filters conversations by mode", async () => {
    const res = await client.conversations.list({
      filter_mode: "hybrid",
      limit: 5,
    });
    expect(res).toBeDefined();
    expect(Array.isArray(res.items)).toBe(true);
  });
});

// ── Messages ──────────────────────────────────────────────

describeE2E("Messages E2E", () => {
  let client: EdgeQuake;
  let conversationId: string;

  beforeAll(async () => {
    client = createE2EClient()!;
    // Create a conversation to hold messages
    const conv = await client.conversations.create({
      title: testId("msg-conv"),
      mode: "hybrid",
    });
    conversationId = conv.id;
  }, 30_000);

  it("lists messages in a conversation (paginated)", async () => {
    const res = await client.conversations.messages.list(conversationId);
    expect(res).toBeDefined();
    expect(Array.isArray(res.items)).toBe(true);
    expect(res.pagination).toBeDefined();
  });

  it("creates a message in a conversation", async () => {
    const msg = await client.conversations.messages.create(conversationId, {
      role: "user",
      content: "Hello from SDK E2E test",
      stream: false,
    });
    expect(msg).toBeDefined();
    expect(msg.id).toBeDefined();
    expect(msg.content).toBe("Hello from SDK E2E test");
    expect(msg.role).toBe("user");
  });

  // Cleanup: delete the conversation after tests
  it("cleanup: delete test conversation", async () => {
    await client.conversations.delete(conversationId);
  });
});

// ── Folders ───────────────────────────────────────────────

describeE2E("Folders E2E", () => {
  let client: EdgeQuake;

  beforeAll(async () => {
    client = createE2EClient()!;
  }, 30_000);

  it("lists folders", async () => {
    const folders = await client.folders.list();
    expect(folders).toBeDefined();
    expect(Array.isArray(folders)).toBe(true);
  });

  it("creates, updates, and deletes a folder", async () => {
    const name = testId("folder");

    // Create
    const created = await client.folders.create({ name });
    expect(created).toBeDefined();
    expect(created.id).toBeDefined();
    expect(created.name).toBe(name);

    // Update
    const updatedName = name + "-updated";
    const updated = await client.folders.update(created.id, {
      name: updatedName,
    });
    expect(updated).toBeDefined();
    expect(updated.name).toBe(updatedName);

    // Delete
    await client.folders.delete(created.id);
  });
});

// ── Sharing ──────────────────────────────────────────────

describeE2E("Sharing E2E", () => {
  let client: EdgeQuake;
  let conversationId: string;

  beforeAll(async () => {
    client = createE2EClient()!;
    // Create a conversation to share
    const conv = await client.conversations.create({
      title: testId("share-conv"),
      mode: "hybrid",
    });
    conversationId = conv.id;
  }, 30_000);

  it("shares and unshares a conversation", async () => {
    // Share
    const shareRes = await client.conversations.share(conversationId);
    expect(shareRes).toBeDefined();
    expect(shareRes.share_id).toBeDefined();
    expect(typeof shareRes.share_id).toBe("string");

    // Access shared conversation
    const shared = await client.shared.get(shareRes.share_id);
    expect(shared).toBeDefined();

    // Unshare
    await client.conversations.unshare(conversationId);
  });

  // Cleanup
  it("cleanup: delete shared conversation", async () => {
    await client.conversations.delete(conversationId);
  });
});

// ── Bulk Operations ──────────────────────────────────────

describeE2E("Bulk Operations E2E", () => {
  let client: EdgeQuake;
  let convIds: string[] = [];

  beforeAll(async () => {
    client = createE2EClient()!;
    // Create 2 conversations for bulk ops
    for (let i = 0; i < 2; i++) {
      const conv = await client.conversations.create({
        title: testId(`bulk-${i}`),
        mode: "hybrid",
      });
      convIds.push(conv.id);
    }
  }, 30_000);

  it("bulk archives conversations", async () => {
    const res = await client.conversations.bulkArchive({
      conversation_ids: convIds,
      archive: true,
    });
    expect(res).toBeDefined();
    expect(typeof res.affected).toBe("number");
  });

  it("bulk unarchives conversations", async () => {
    const res = await client.conversations.bulkArchive({
      conversation_ids: convIds,
      archive: false,
    });
    expect(res).toBeDefined();
  });

  it("cleanup: bulk delete test conversations", async () => {
    const res = await client.conversations.bulkDelete({
      conversation_ids: convIds,
    });
    expect(res).toBeDefined();
    expect(typeof res.affected).toBe("number");
  });
});
