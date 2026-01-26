#!/usr/bin/env python3
"""
E2E test for document cascade delete functionality.
Tests that when a document is deleted, all associated KG entities are also deleted.
"""

import sys
import time
import uuid

import requests

API_URL = "http://localhost:8080/api/v1"


def get_entities_for_workspace(tenant_id, workspace_id):
    """Get all entities in a workspace."""
    resp = requests.get(
        f"{API_URL}/graph/entities",
        headers={"X-Tenant-ID": tenant_id, "X-Workspace-ID": workspace_id},
        params={"limit": 500},
    )
    if resp.status_code != 200:
        return []
    data = resp.json()
    return data.get("items", data.get("entities", []))


def search_entity(tenant_id, workspace_id, search_term):
    """Search for entities by name."""
    resp = requests.get(
        f"{API_URL}/graph/entities",
        headers={"X-Tenant-ID": tenant_id, "X-Workspace-ID": workspace_id},
        params={"search": search_term, "limit": 100},
    )
    if resp.status_code != 200:
        return []
    data = resp.json()
    return data.get("items", data.get("entities", []))


def main():
    tenant_id = f"cascade-test-{uuid.uuid4().hex[:8]}"
    print(f"=== Testing Document Cascade Delete ===")
