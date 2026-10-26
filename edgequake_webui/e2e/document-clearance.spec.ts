import { expect, test, type Page, type Route } from "@playwright/test";
import { prepareMembersPage } from "./spec169/helpers/members-mock";
import { applyTheme, setViewport, shot } from "./spec174/helpers/shots";
import { mockSettingsAccess } from "./spec175/helpers/access-mock";

const HIDDEN = "Restricted atlas payroll";
const WS = "spec155-ws-001";
/** Bob Martin in the SPEC-169 members fixture. */
const BOB_ID = "00000000-0000-4000-8000-000000000002";
const BASE = `**/workspaces/${WS}/clearance`;

type Rule = {
  effect: string;
  match_kind: string;
  rank: number | null;
  tag_ids: string[];
};

/** Stateful Access API. Saves stick, so every test can check what the UI really sent. */
function accessState() {
  return {
    enforcement: false,
    roles: [
      { id: "role-hr", name: "HR reader", steward: false, rules: [rule("classification_at_most", "allow", 2), rule("any_of", "allow", null, ["tag-hr"])] },
      { id: "role-rd", name: "R&D reader", steward: false, rules: [] as Rule[] },
    ] as { id: string; name: string; steward: boolean; rules: Rule[] }[],
    tags: [
      { id: "tag-hr", kind: "compartment", key: "hr", value: "HR", rank: null },
      { id: "tag-rd", kind: "compartment", key: "rd", value: "R&D", rank: null },
      { id: "tag-atlas", kind: "compartment", key: "atlas", value: "Atlas", rank: null },
    ],
    memberRoles: { [BOB_ID]: ["role-hr"] } as Record<string, string[]>,
    ruleWrites: [] as { roleId: string; rules: unknown[] }[],
    memberWrites: [] as { userId: string; roleIds: string[] }[],
    labelWrites: [] as { documentId: string; body: unknown }[],
  };
}

function rule(match_kind: string, effect: string, rank: number | null, tag_ids: string[] = []): Rule {
  return { effect, match_kind, rank, tag_ids };
}

const json = (route: Route, data: unknown) => route.fulfill({ json: data });

async function accessRoutes(page: Page, state: ReturnType<typeof accessState>) {
  const sessionUser={id:'spec155-user-001',user_id:'spec155-user-001',tenant_id:'spec155-tenant-001',username:'demo',email:'demo@example.test',role:'admin',is_active:true};
  await page.route('**/api/v1/auth/refresh',route=>json(route,{access_token:'spec174-ui-fixture',token_type:'Bearer',expires_in:3600,user:sessionUser}));
  await page.route('**/api/v1/auth/me',route=>json(route,sessionUser));
  await mockSettingsAccess(page,'platform_admin');

  await page.route(BASE, async (route) => {
    if (route.request().method() === "GET") {
      await json(route, {
        enforcement: state.enforcement,
        policy_epoch: 0,
        steward: true,
        rank: "restricted",
        can_manage: true,
      });
      return;
    }
    state.enforcement = true;
    await json(route, { enforcement: true });
  });
  await page.route(`${BASE}/roles`, (route) => json(route, state.roles));
  await page.route(`${BASE}/tags`, (route) => json(route, state.tags));
  await page.route(`${BASE}/roles/*/rules`, async (route) => {
    const roleId = route.request().url().split("/roles/")[1].split("/")[0];
    const body = route.request().postDataJSON() as { rules: { effect: string; match_kind: string; rank?: number; tag_ids?: string[] }[] };
    state.ruleWrites.push({ roleId, rules: body.rules });
    const role = state.roles.find((item) => item.id === roleId);
    if (role) {
      role.rules = body.rules.map((item) => rule(item.match_kind, item.effect, item.rank ?? null, item.tag_ids ?? []));
    }
    await json(route, { ok: true });
  });
  await page.route(`${BASE}/simulate`, (route) =>
    json(route, { titles: ["Public handbook"], count: 1 }),
  );
  await page.route(`**/workspaces/${WS}/members/*/clearance-roles`, async (route) => {
    const userId = route.request().url().split("/members/")[1].split("/")[0];
    if (route.request().method() === "GET") {
      await json(route, { role_ids: state.memberRoles[userId] ?? [] });
      return;
    }
    const body = route.request().postDataJSON() as { role_ids: string[] };
    state.memberWrites.push({ userId, roleIds: body.role_ids });
    state.memberRoles[userId] = body.role_ids;
    await json(route, { ok: true });
  });
}

const LIBRARY_DOCUMENT = {
  id: "doc-public",
  title: "Public handbook",
  status: "completed",
  classification: "internal",
  compartments: [{ id: "tag-hr", key: "hr", value: "HR" }],
};

async function documentRoutes(page: Page, state: ReturnType<typeof accessState>) {
  await page.route("**/api/v1/documents**", async (route) => {
    const url = route.request().url();
    if (url.includes(HIDDEN) || url.includes("hidden-doc")) {
      await route.fulfill({ status: 404, json: { kind: "not_found", message: "Document not found" } });
      return;
    }
    if (route.request().method() !== "GET" || url.includes("/doc-public")) {
      await route.fallback();
      return;
    }
    await json(route, {
      documents: [LIBRARY_DOCUMENT],
      total: 1,
      page: 1,
      page_size: 50,
      total_pages: 1,
      status_counts: { pending: 0, processing: 0, completed: 1, failed: 0 },
    });
  });
  await page.route("**/api/v1/documents/doc-public", (route) =>
    route.request().method() === "GET" ? json(route, LIBRARY_DOCUMENT) : route.fallback(),
  );
  const capabilities={can_edit:true,can_downgrade:true,can_manage_vocabulary:true,can_check_member:false};
  await page.route(`${BASE}/assignment-options?*`,route=>json(route,{enforcement:true,policy_epoch:1,break_glass:false,can_upload:true,capabilities,classifications:['public','internal','confidential','restricted'],tags:state.tags,next_cursor:null}));
  await page.route("**/api/v1/documents/doc-public/access",route=>json(route,{document_id:'doc-public',access:{classification:'internal',compartment_ids:['tag-hr'],expected_policy_epoch:1},label_version:1,policy_epoch:1,enforcement:true,capabilities,tags:state.tags.filter(tag=>tag.id==='tag-hr')}));
  await page.route(`${BASE}/preview`,route=>json(route,{allowed:true,policy_epoch:1,items:[{allowed:true,requires_reason:false,change_kind:'strengthen',reason:null}]}));
  await page.route("**/api/v1/documents/doc-public/labels", async (route) => {
    state.labelWrites.push({ documentId: "doc-public", body: route.request().postDataJSON() });
    await route.fulfill({ status: 204, body: "" });
  });
}

test.describe("SPEC-174 document clearance", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("access settings: card, profiles, preview", async ({ page }) => {
    const state = accessState();
    await prepareMembersPage(page);
    await accessRoutes(page, state);
    await setViewport(page, 1440);
    await page.goto("/settings/workspace/access", { waitUntil: "domcontentloaded" });
    const card = page.getByTestId("document-labels-card");
    await expect(card).toBeVisible({ timeout: 30_000 });
    await card.scrollIntoViewIfNeeded();
    await expect(card.getByText("Access profiles")).toBeVisible();
    await expect(page.getByTestId("profile-role-hr")).toBeVisible();
    await expect(card.getByText("Internal · HR", { exact: true })).toBeVisible();
    await expect(card.getByText("No access yet")).toBeVisible();
    await shot(page, "access-card-off");

    await applyTheme(page, "dark");
    await card.scrollIntoViewIfNeeded();
    await shot(page, "access-card-off", { theme: "dark" });
    await applyTheme(page, "light");

    await setViewport(page, 390);
    await card.scrollIntoViewIfNeeded();
    await shot(page, "access-card-off", { width: 390 });
    await setViewport(page, 1440);

    await page.getByTestId("document-labels-switch").click();
    const dialog = page.getByTestId("clearance-enable-dialog");
    await expect(dialog).toContainText("Members without an access profile will see an empty library.");
    await shot(page, "enable-dialog");
    await page.getByRole("button", { name: "Cancel" }).click();

    await page.getByTestId("profile-role-hr").click();
    const editor = page.getByTestId("profile-editor-role-hr");
    await expect(editor).toBeVisible();
    await expect(editor.getByRole("heading", { name: "HR reader" })).toBeVisible();
    await shot(page, "profile-editor");
    await editor.getByRole("checkbox", { name: "R&D" }).click();
    await page.keyboard.press("Escape");
    await expect(editor).toHaveCount(0);
    await expect.poll(() => state.ruleWrites.length).toBe(1);
    expect(state.ruleWrites[0].roleId).toBe("role-hr");
    await expect(card.getByText("Internal · HR, R&D", { exact: true })).toBeVisible();

    await page.getByTestId("clearance-vocabulary").click();
    await expect(page.getByTestId("vocabulary-editor")).toBeVisible();
    await shot(page, "vocabulary-popover");
    await page.keyboard.press("Escape");

    await page.getByTestId("profile-role-hr").click();
    await page.getByTestId("profile-preview").click();
    const sheet = page.getByTestId("clearance-preview-sheet");
    await expect(sheet).toContainText("Public handbook");
    await expect(sheet.getByRole("heading", { name: "HR reader" })).toBeVisible();
    await expect(page.getByTestId("clearance-preview-count")).toHaveText("1 document");
    await expect(page.getByText(HIDDEN)).toHaveCount(0);
    await shot(page, "preview-sheet");
  });

  test("member access profiles save to the person", async ({ page }) => {
    const state = accessState();
    await prepareMembersPage(page);
    await accessRoutes(page, state);
    await setViewport(page, 1440);
    await page.goto("/members", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("members-table")).toBeVisible({ timeout: 30_000 });
    await page
      .locator("[data-testid^=member-row-]")
      .filter({ has: page.getByText("Bob Martin", { exact: true }) })
      .getByRole("button", { name: "Bob Martin", exact: true })
      .click();
    await expect(page.getByTestId("member-detail-sheet")).toBeVisible();
    const access = page.getByTestId("detail-clearance");
    await expect(access.getByTestId("member-access-profile")).toHaveText("HR reader");
    await access.getByTestId("member-access-profile").click();
    const editor = page.getByTestId("member-access-editor");
    await expect(editor).toBeVisible();
    await shot(page, "member-access-editor");
    await editor.getByRole("checkbox", { name: "R&D reader" }).click();
    await page.keyboard.press("Escape");
    await expect(editor).toHaveCount(0);
    await expect.poll(() => state.memberWrites.length).toBe(1);
    expect(state.memberWrites[0]).toEqual({ userId: BOB_ID, roleIds: ["role-hr", "role-rd"] });
    await expect(access.getByTestId("member-access-profile")).toHaveText("HR reader, R&D reader");
  });

  test("document access can be changed by an owner", async ({ page }) => {
    const state = accessState();
    state.enforcement = true;
    await prepareMembersPage(page);
    await accessRoutes(page, state);
    await documentRoutes(page, state);
    await setViewport(page, 1440);
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    await expect(page.getByText("Public handbook")).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("document-access-token")).toContainText("Internal");
    await page.getByText("Public handbook", { exact: true }).click();
    const access = page.getByTestId("access-property");
    await expect(access).toHaveText("Internal · HR");
    await access.click();
    const editor = page.getByTestId("document-access-editor");
    await expect(editor).toBeVisible();
    await shot(page, "document-access-editor");
    await editor.getByRole("combobox", { name: "Sensitivity" }).click();
    await page.getByRole("option", { name: "Confidential" }).click();
    expect(state.labelWrites).toHaveLength(0);
    await editor.getByRole('button',{name:'Save access',exact:true}).click();
    await expect(editor).toHaveCount(0);
    await expect.poll(() => state.labelWrites.length).toBe(1);
    expect(state.labelWrites[0].body).toEqual({ classification: "confidential", compartment_ids: ["tag-hr"],expected_policy_epoch:1 });
  });

  test("hidden title is absent from the library", async ({ page }) => {
    const state = accessState();
    await prepareMembersPage(page);
    await accessRoutes(page, state);
    await documentRoutes(page, state);
    await setViewport(page, 1440);
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    await expect(page.getByText("Public handbook")).toBeVisible({ timeout: 30_000 });
    await expect(page.getByText(HIDDEN)).toHaveCount(0);
    await expect(page.getByTestId("document-access-token")).toContainText("Internal");
    await shot(page, "document-table-token");
    await page.goto("/documents/hidden-doc", { waitUntil: "domcontentloaded" });
    await expect(page.getByRole("heading", { name: "Document not found" })).toBeVisible({ timeout: 30_000 });
    await expect(page.getByText(HIDDEN)).toHaveCount(0);
    await shot(page, "hidden-id-not-found");
  });
});
