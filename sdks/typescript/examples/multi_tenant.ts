/**
 * Multi-Tenant — EdgeQuake TypeScript SDK
 *
 * WHY: EdgeQuake supports multi-tenant isolation — each tenant has
 * separate workspaces, documents, and knowledge graphs. This example
 * shows tenant/workspace CRUD and context switching.
 *
 * Usage:
 *   npx tsx examples/multi_tenant.ts
 */
import { EdgeQuake } from "@edgequake/sdk";

async function main() {
  // ── 1. Admin client (no tenant scope) ─────────────────────

  const admin = new EdgeQuake({
    baseUrl: process.env.EDGEQUAKE_URL ?? "http://localhost:8080",
    apiKey: process.env.EDGEQUAKE_API_KEY ?? "demo-key",
  });

  // ── 2. Create a tenant ────────────────────────────────────

  const tenant = await admin.tenants.create({
    name: "Acme Corporation",
    slug: "acme-corp",
  });
  console.log(`Created tenant: ${tenant.id} (${tenant.name})`);

  // ── 3. Create workspaces within the tenant ────────────────

  // WHY: Workspaces isolate document collections and graphs
  // within a single tenant. Use for teams, projects, or environments.
  const prodWorkspace = await admin.tenants.createWorkspace(tenant.id, {
    name: "Production",
    slug: "prod",
  });
  console.log(`Created workspace: ${prodWorkspace.id} (${prodWorkspace.name})`);

  const devWorkspace = await admin.tenants.createWorkspace(tenant.id, {
    name: "Development",
    slug: "dev",
  });
  console.log(`Created workspace: ${devWorkspace.id} (${devWorkspace.name})`);

  // ── 4. Scoped client (tenant + workspace) ─────────────────

  // WHY: Creating a scoped client adds X-Tenant-Id and X-Workspace-Id
  // headers to every request, ensuring data isolation.
  const scopedClient = new EdgeQuake({
    baseUrl: process.env.EDGEQUAKE_URL ?? "http://localhost:8080",
    apiKey: process.env.EDGEQUAKE_API_KEY ?? "demo-key",
    tenantId: tenant.id,
    workspaceId: prodWorkspace.id,
  });

  // All operations now scoped to this tenant + workspace
  const doc = await scopedClient.documents.upload({
    content: "Tenant-scoped document for Acme Corp production workspace.",
    title: "Acme Production Doc",
  });
  console.log(`\nUploaded to production workspace: ${doc.document_id}`);

  // ── 5. List workspaces ────────────────────────────────────

  const workspaces = await admin.tenants.listWorkspaces(tenant.id);
  console.log(`\nWorkspaces for tenant ${tenant.name}:`);
  for (const ws of workspaces) {
    console.log(`  ${ws.id}: ${ws.name} (${ws.slug})`);
  }

  // ── 6. Workspace stats ────────────────────────────────────

  const stats = await admin.workspaces.stats(prodWorkspace.id);
  console.log(`\nProduction workspace stats:`, stats);

  // ── 7. Cleanup ────────────────────────────────────────────

  await admin.workspaces.delete(devWorkspace.id);
  await admin.workspaces.delete(prodWorkspace.id);
  await admin.tenants.delete(tenant.id);
  console.log("\nCleaned up tenant and workspaces");
}

main().catch(console.error);
