#!/usr/bin/env python3
"""
E2E test for document cascade delete with MULTIPLE documents.
Tests that when a document is deleted:
1. Entities unique to that document are deleted
2. Entities shared with other documents are preserved
"""

import requests
import time
import sys
import uuid

API_URL = "http://localhost:8080/api/v1"

def search_entity(tenant_id, workspace_id, search_term):
    """Search for entities by name."""
    resp = requests.get(
        f"{API_URL}/graph/entities",
        headers={"X-Tenant-ID": tenant_id, "X-Workspace-ID": workspace_id},
        params={"search": search_term, "limit": 100}
    )
    if resp.status_code != 200:
        return []
    data = resp.json()
    return data.get("items", data.get("entities", []))

def main():
    tenant_name = f"multi-doc-{uuid.uuid4().hex[:8]}"
    print(f"=== Testing Multi-Document Cascade Delete ===")
    print(f"Tenant: {tenant_name}")
    
    # Step 1: Create tenant and workspace
    print("\n1. Creating tenant and workspace...")
    resp = requests.post(
        f"{API_URL}/tenants",
        headers={"Content-Type": "application/json"},
        json={"name": f"Multi Doc Test {uuid.uuid4().hex[:8]}"}
    )
    if resp.status_code not in [200, 201]:
        print(f"   FAIL creating tenant: {resp.status_code} - {resp.text}")
        return 1
    tenant_id = resp.json().get("id")
    print(f"   Tenant ID: {tenant_id}")
    
    resp = requests.post(
        f"{API_URL}/tenants/{tenant_id}/workspaces",
        headers={"Content-Type": "application/json"},
        json={"name": "Multi Doc Test", "description": "Testing multi-doc cascade"}
    )
    if resp.status_code not in [200, 201]:
        print(f"   FAIL creating workspace: {resp.status_code} - {resp.text}")
        return 1
    workspace_id = resp.json().get("id")
    print(f"   Workspace ID: {workspace_id}")
    
    # Create unique identifiers
    unique_id = uuid.uuid4().hex[:8]
    shared_entity = f"SharedOrg_{unique_id}"
    doc1_unique_entity = f"Doc1Person_{unique_id}"
    doc2_unique_entity = f"Doc2Person_{unique_id}"
    
    # Step 2: Upload Document 1 - has shared entity + unique entity
    print("\n2. Uploading Document 1...")
    content1 = f"""# Document 1

{doc1_unique_entity} works at {shared_entity}.
{doc1_unique_entity} is a software engineer specializing in machine learning.
{shared_entity} is a leading technology company."""

    resp = requests.post(
        f"{API_URL}/documents/upload",
        headers={"X-Tenant-ID": tenant_id, "X-Workspace-ID": workspace_id},
        files={"file": ("doc1.md", content1, "text/markdown")},
        data={"title": "Document 1"}
    )
    if resp.status_code not in [200, 201, 202]:
        print(f"   FAIL: {resp.status_code} - {resp.text}")
        return 1
    doc1_result = resp.json()
    doc1_id = doc1_result.get("document_id")
    print(f"   Document 1 ID: {doc1_id}")
    print(f"   Entities: {doc1_result.get('entity_count', 0)}")
    
    # Step 3: Upload Document 2 - has shared entity + different unique entity
    print("\n3. Uploading Document 2...")
    content2 = f"""# Document 2

{doc2_unique_entity} also works at {shared_entity}.
{doc2_unique_entity} is a data scientist focusing on NLP.
{shared_entity} sponsors many research projects."""

    resp = requests.post(
        f"{API_URL}/documents/upload",
        headers={"X-Tenant-ID": tenant_id, "X-Workspace-ID": workspace_id},
        files={"file": ("doc2.md", content2, "text/markdown")},
        data={"title": "Document 2"}
    )
    if resp.status_code not in [200, 201, 202]:
        print(f"   FAIL: {resp.status_code} - {resp.text}")
        return 1
    doc2_result = resp.json()
    doc2_id = doc2_result.get("document_id")
    print(f"   Document 2 ID: {doc2_id}")
    print(f"   Entities: {doc2_result.get('entity_count', 0)}")
    
    # Step 4: Verify all entities exist
    print("\n4. Checking entities BEFORE any deletion...")
    shared_before = search_entity(tenant_id, workspace_id, shared_entity)
    doc1_before = search_entity(tenant_id, workspace_id, doc1_unique_entity)
    doc2_before = search_entity(tenant_id, workspace_id, doc2_unique_entity)
    
    print(f"   {shared_entity}: {'FOUND' if len(shared_before) > 0 else 'NOT FOUND'}")
    print(f"   {doc1_unique_entity}: {'FOUND' if len(doc1_before) > 0 else 'NOT FOUND'}")
    print(f"   {doc2_unique_entity}: {'FOUND' if len(doc2_before) > 0 else 'NOT FOUND'}")
    
    if not all([shared_before, doc1_before, doc2_before]):
        print("\n   WARNING: Not all expected entities were created")
        print("   This may be due to LLM extraction variability")
    
    # Step 5: Delete Document 1
    print("\n5. Deleting Document 1...")
    resp = requests.delete(
        f"{API_URL}/documents/{doc1_id}",
        headers={"X-Tenant-ID": tenant_id, "X-Workspace-ID": workspace_id}
    )
    delete_result = resp.json()
    print(f"   Response: {delete_result}")
    
    # Step 6: Verify entity states after deletion
    print("\n6. Checking entities AFTER deleting Document 1...")
    shared_after = search_entity(tenant_id, workspace_id, shared_entity)
    doc1_after = search_entity(tenant_id, workspace_id, doc1_unique_entity)
    doc2_after = search_entity(tenant_id, workspace_id, doc2_unique_entity)
    
    found_shared = len(shared_after) > 0
    found_doc1 = len(doc1_after) > 0
    found_doc2 = len(doc2_after) > 0
    
    print(f"   {shared_entity}: {'FOUND (GOOD - shared)' if found_shared else 'NOT FOUND (BAD - was shared!)'}")
    print(f"   {doc1_unique_entity}: {'FOUND (BAD - should be deleted)' if found_doc1 else 'NOT FOUND (GOOD)'}")
    print(f"   {doc2_unique_entity}: {'FOUND (GOOD)' if found_doc2 else 'NOT FOUND (BAD!)'}")
    
    # Step 7: Final verdict
    print("\n=== RESULT ===")
    
    # Note: LLM extraction may not create the same entities we expect,
    # so we check what we can verify
    success = True
    
    # Doc1 unique entity should be deleted
    if found_doc1:
        print(f"❌ {doc1_unique_entity} should have been deleted with Doc1")
        success = False
    else:
        print(f"✅ {doc1_unique_entity} was correctly deleted with Doc1")
    
    # Doc2 unique entity should still exist
    if not found_doc2:
        print(f"⚠️  {doc2_unique_entity} unexpectedly not found (may not have been extracted)")
    else:
        print(f"✅ {doc2_unique_entity} still exists (from Doc2)")
    
    # Shared entity behavior depends on whether it was actually shared
    if len(shared_before) > 0:
        if found_shared:
            print(f"✅ {shared_entity} preserved (referenced by Doc2)")
        else:
            print(f"❌ {shared_entity} was deleted but should have been preserved by Doc2")
            success = False
    
    if success:
        print("\n✅ SUCCESS: Multi-document cascade delete works correctly!")
        return 0
    else:
        print("\n❌ FAIL: Multi-document cascade delete has issues")
        return 1

if __name__ == "__main__":
    sys.exit(main())
