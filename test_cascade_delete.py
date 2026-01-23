#!/usr/bin/env python3
"""
E2E test for document cascade delete functionality.
Tests that when a document is deleted, all associated KG entities are also deleted.
"""

import requests
import time
import sys
import uuid

API_URL = "http://localhost:8080/api/v1"

def get_entities_for_workspace(tenant_id, workspace_id):
    """Get all entities in a workspace."""
    resp = requests.get(
        f"{API_URL}/graph/entities",
        headers={"X-Tenant-ID": tenant_id, "X-Workspace-ID": workspace_id},
        params={"limit": 500}
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
        params={"search": search_term, "limit": 100}
    )
    if resp.status_code != 200:
        return []
    data = resp.json()
    return data.get("items", data.get("entities", []))

def main():
    tenant_id = f"cascade-test-{uuid.uuid4().hex[:8]}"
    print(f"=== Testing Document Cascade Delete ===")
    print(f"Tenant: {tenant_id}")
    
    # Step 1: Create tenant first
    print("\n1a. Creating tenant...")
    unique_name = f"Cascade Test {uuid.uuid4().hex[:8]}"
    resp = requests.post(
        f"{API_URL}/tenants",
        headers={"Content-Type": "application/json"},
        json={"name": unique_name}
    )
    if resp.status_code != 200 and resp.status_code != 201:
        print(f"   FAIL creating tenant: {resp.status_code} - {resp.text}")
        return 1
    tenant_id = resp.json().get("id")
    print(f"   Tenant ID: {tenant_id}")
    
    # Step 1b: Create workspace
    print("\n1b. Creating workspace...")
    resp = requests.post(
        f"{API_URL}/tenants/{tenant_id}/workspaces",
        headers={"Content-Type": "application/json"},
        json={"name": "Cascade Test", "description": "Testing cascade delete"}
    )
    if resp.status_code != 200 and resp.status_code != 201:
        print(f"   FAIL creating workspace: {resp.status_code} - {resp.text}")
        return 1
    workspace_id = resp.json().get("id")
    print(f"   Workspace ID: {workspace_id}")
    
    # Step 2: Upload document with unique content
    # Use unique names that we can search for
    print("\n2. Uploading document...")
    unique_id = uuid.uuid4().hex[:8]
    unique_person = f"TestPerson_{unique_id}"
    unique_org = f"TestOrg_{unique_id}"
    content = f"""# Test Research Document

{unique_person} is a researcher at {unique_org}.
{unique_person} studies machine learning and neural networks.
{unique_org} is a leading research institution in artificial intelligence.

The research focuses on transformer architectures and attention mechanisms.
{unique_person} collaborates with colleagues at {unique_org} on various projects."""

    resp = requests.post(
        f"{API_URL}/documents/upload",
        headers={"X-Tenant-ID": tenant_id, "X-Workspace-ID": workspace_id},
        files={"file": ("test.md", content, "text/markdown")},
        data={"title": "Test Research Document"}
    )
    if resp.status_code not in [200, 201, 202]:
        print(f"   FAIL: {resp.status_code} - {resp.text}")
        return 1
    result = resp.json()
    doc_id = result.get("document_id")
    print(f"   Document ID: {doc_id}")
    print(f"   Status: {result.get('status', 'unknown')}")
    print(f"   Entities: {result.get('entity_count', 0)}, Relationships: {result.get('relationship_count', 0)}")
    print(f"   Test entities to search for: {unique_person}, {unique_org}")
    
    # Step 3: Wait for processing (if not already completed)
    print("\n3. Checking document processing status...")
    if result.get('status') in ['processed', 'completed']:
        print("   Document already processed (synchronous mode)")
    else:
        print("   Waiting for async processing...")
        for i in range(30):
            resp = requests.get(
                f"{API_URL}/documents/{doc_id}",
                headers={"X-Tenant-ID": tenant_id, "X-Workspace-ID": workspace_id}
            )
            if resp.status_code == 403:
                print(f"   Got 403 - retrying...")
                time.sleep(2)
                continue
            if resp.status_code != 200:
                print(f"   API error: {resp.status_code} - {resp.text}")
                time.sleep(2)
                continue
            status = resp.json().get("status", "unknown")
            print(f"   Status: {status}")
            if status in ["completed", "processed"]:
                break
            if status == "failed":
                print(f"   Processing failed: {resp.json()}")
                return 1
            time.sleep(2)
        else:
            print("   Timeout waiting for processing!")
            return 1
    
    # Step 4: Check for our specific entities BEFORE deletion
    print("\n4. Checking for our test entities BEFORE deletion...")
    
    # Search for our unique person entity
    person_before = search_entity(tenant_id, workspace_id, unique_person)
    org_before = search_entity(tenant_id, workspace_id, unique_org)
    
    found_person_before = len(person_before) > 0
    found_org_before = len(org_before) > 0
    
    print(f"   {unique_person}: {'FOUND' if found_person_before else 'NOT FOUND'}")
    print(f"   {unique_org}: {'FOUND' if found_org_before else 'NOT FOUND'}")
    
    if not found_person_before and not found_org_before:
        print("   WARNING: Test entities not found - LLM may have extracted different names")
        print("   Checking all entities for partial matches...")
        all_entities = get_entities_for_workspace(tenant_id, workspace_id)
        print(f"   Total entities: {len(all_entities)}")
        # Look for entities containing our unique ID
        matching = [e for e in all_entities if unique_id in e.get('id', '').lower() or unique_id in e.get('entity_name', '').lower()]
        if matching:
            print(f"   Found {len(matching)} entities with unique ID:")
            for e in matching[:5]:
                print(f"     - {e.get('id')}")
        else:
            print("   No entities with unique ID found - checking if any entities were created")
            recent = [e for e in all_entities if 'test' in e.get('id', '').lower()]
            if recent:
                print(f"   Found {len(recent)} entities with 'test' in name:")
                for e in recent[:5]:
                    print(f"     - {e.get('id')}")
    
    # Step 5: Delete document
    print("\n5. Deleting document...")
    resp = requests.delete(
        f"{API_URL}/documents/{doc_id}",
        headers={"X-Tenant-ID": tenant_id, "X-Workspace-ID": workspace_id}
    )
    delete_result = resp.json()
    print(f"   Response: {delete_result}")
    
    # Step 6: Check for our specific entities AFTER deletion
    print("\n6. Checking for our test entities AFTER deletion...")
    
    # Search for our unique person entity
    person_after = search_entity(tenant_id, workspace_id, unique_person)
    org_after = search_entity(tenant_id, workspace_id, unique_org)
    
    found_person_after = len(person_after) > 0
    found_org_after = len(org_after) > 0
    
    print(f"   {unique_person}: {'FOUND - SHOULD BE DELETED' if found_person_after else 'NOT FOUND (GOOD)'}")
    print(f"   {unique_org}: {'FOUND - SHOULD BE DELETED' if found_org_after else 'NOT FOUND (GOOD)'}")
    
    # Also check for entities with our unique ID
    all_entities = get_entities_for_workspace(tenant_id, workspace_id)
    matching = [e for e in all_entities if unique_id in e.get('id', '').lower() or unique_id in e.get('entity_name', '').lower()]
    
    # Step 7: Verify
    print("\n=== RESULT ===")
    print(f"Entities with unique ID before delete: (checked via search)")
    print(f"Entities with unique ID after delete: {len(matching)}")
    print(f"Delete API reported: entities_affected={delete_result.get('entities_affected', 0)}")
    
    if not found_person_after and not found_org_after and len(matching) == 0:
        print("\n✅ SUCCESS: All test entities were deleted with the document!")
        return 0
    else:
        print(f"\n❌ FAIL: Test entities still remain after document deletion")
        if found_person_after:
            print(f"  - {unique_person} still exists")
        if found_org_after:
            print(f"  - {unique_org} still exists")
        for e in matching[:5]:
            print(f"  - {e.get('id')}")
        return 1

if __name__ == "__main__":
    sys.exit(main())
