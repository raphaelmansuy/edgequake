#!/usr/bin/env python3
"""
Multi-Tenant — EdgeQuake Python SDK

WHY: EdgeQuake supports multi-tenant isolation — each tenant has
separate workspaces, documents, and knowledge graphs. This example
shows tenant/workspace CRUD and context switching.

Requirements:
    - EdgeQuake server running on http://localhost:8080
    - EDGEQUAKE_API_KEY environment variable set
    - Admin privileges

Usage:
    export EDGEQUAKE_API_KEY="demo-key"
    python examples/multi_tenant.py
"""
import os

from edgequake import EdgequakeClient


def main():
    # ── 1. Admin client (no tenant scope) ─────────────────────

    admin = EdgequakeClient(
        api_key=os.environ.get("EDGEQUAKE_API_KEY", "demo-key"),
        base_url=os.environ.get("EDGEQUAKE_URL", "http://localhost:8080"),
    )

    # ── 2. Create a tenant ────────────────────────────────────

    tenant = admin.tenants.create(
        name="Acme Corporation",
        slug="acme-corp",
    )
    print(f"Created tenant: {tenant['id']} ({tenant['name']})")

    # ── 3. Create workspaces within the tenant ────────────────

    # WHY: Workspaces isolate document collections and graphs
    # within a single tenant. Use for teams, projects, or environments.
    prod_workspace = admin.tenants.create_workspace(
        tenant["id"],
        name="Production",
        slug="prod",
    )
    print(f"Created workspace: {prod_workspace['id']} ({prod_workspace['name']})")

    dev_workspace = admin.tenants.create_workspace(
        tenant["id"],
        name="Development",
        slug="dev",
    )
    print(f"Created workspace: {dev_workspace['id']} ({dev_workspace['name']})")

    # ── 4. Scoped client (tenant + workspace) ─────────────────

    # WHY: Creating a scoped client adds X-Tenant-Id and X-Workspace-Id
    # headers to every request, ensuring data isolation.
    scoped_client = EdgequakeClient(
        api_key=os.environ.get("EDGEQUAKE_API_KEY", "demo-key"),
        base_url=os.environ.get("EDGEQUAKE_URL", "http://localhost:8080"),
        tenant_id=tenant["id"],
        workspace_id=prod_workspace["id"],
    )

    # All operations now scoped to this tenant + workspace
    doc = scoped_client.documents.upload(
        content="Tenant-scoped document for Acme Corp production workspace.",
        title="Acme Production Doc",
    )
    print(f"\nUploaded to production workspace: {doc['document_id']}")

    # ── 5. List workspaces ────────────────────────────────────

    workspaces = admin.tenants.list_workspaces(tenant["id"])
    workspace_list = (
        workspaces.get("items", []) if isinstance(workspaces, dict) else workspaces
    )
    print(f"\nWorkspaces for tenant {tenant['name']}:")
    for ws in workspace_list:
        print(f"  {ws['id']}: {ws['name']} ({ws['slug']})")

    # ── 6. Workspace stats ────────────────────────────────────

    stats = admin.workspaces.stats(prod_workspace["id"])
    print(f"\nProduction workspace stats: {stats}")

    # ── 7. Cleanup ────────────────────────────────────────────

    admin.workspaces.delete(dev_workspace["id"])
    admin.workspaces.delete(prod_workspace["id"])
    admin.tenants.delete(tenant["id"])
    print("\nCleaned up tenant and workspaces")


if __name__ == "__main__":
    main()
