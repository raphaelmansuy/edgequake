# EdgeQuake E2E Test Cases: Document Management & Multi-Tenancy

This document formalizes the end-to-end (E2E) test cases for EdgeQuake, focusing on document ingestion, multi-tenant/multi-workspace isolation, and UI state management.

## 1. Document Upload & Processing

### TC-DOC-001: Single Document Upload (Manual)

**Description:** Verify that a user can manually upload a single document and it is correctly processed.
**Pre-conditions:** User is logged in and has selected a workspace.
**Steps:**

1. Navigate to the **Documents** page.
2. Click on the **Upload** area or button.
3. Select a file (e.g., `test_upload.txt`).
4. Wait for the upload to complete.
5. Verify the document appears in the list with status `Processing` then `Completed`.
   **Expected Results:**

- Document is visible in the document list.
- Status transitions from `Pending` -> `Processing` -> `Completed`.
- Entity count is greater than 0 (if content contains entities).

### TC-DOC-002: Batch Document Upload

**Description:** Verify that multiple documents can be uploaded simultaneously.
**Pre-conditions:** User is logged in and has selected a workspace.
**Steps:**

1. Navigate to the **Documents** page.
2. Drag and drop 3-5 files into the upload area.
3. Monitor the progress of each file.
   **Expected Results:**

- All files are listed in the upload queue.
- Each file is processed independently.
- All files eventually reach `Completed` status.

### TC-DOC-003: Directory Scan Ingestion

**Description:** Verify the "Scan Directory" feature for bulk ingestion.
**Pre-conditions:** A directory with multiple documents exists on the server or is accessible via the API.
**Steps:**

1. Navigate to the **Documents** page.
2. Click **Scan Directory**.
3. Provide the path to the directory.
4. Click **Start Scan**.
   **Expected Results:**

- System identifies all supported files in the directory.
- Documents are added to the processing queue.
- Progress is reflected in the UI.

---

## 2. Multi-Tenancy & Workspace Isolation

### TC-MT-001: Cross-Tenant Data Isolation

**Description:** Verify that data uploaded to Tenant A is not accessible or searchable by Tenant B.
**Pre-conditions:** Two tenants (Tenant_A, Tenant_B) and two workspaces (WS_A, WS_B) are created.
**Steps:**

1. **Context A:** Select Tenant_A / WS_A.
2. Upload `document_A.txt` with content: "The secret code for Project Alpha is 12345."
3. Wait for processing to complete.
4. **Context B:** Switch to Tenant_B / WS_B.
5. Navigate to the **Query** page.
6. Ask: "What is the secret code for Project Alpha?"
7. **Context A:** Switch back to Tenant_A / WS_A.
8. Ask the same question: "What is the secret code for Project Alpha?"
   **Expected Results:**

- **In Context B:** The system should respond that it doesn't know or provide a generic answer not containing "12345".
- **In Context A:** The system should correctly retrieve the information and answer "12345".

### TC-MT-002: Cross-Workspace Isolation (Same Tenant)

**Description:** Verify that data is isolated between workspaces within the same tenant.
**Pre-conditions:** One tenant (Tenant_A) with two workspaces (WS_Alpha, WS_Beta).
**Steps:**

1. **Context Alpha:** Select Tenant_A / WS_Alpha.
2. Upload `alpha_info.txt`: "Project Alpha is led by Sarah Chen."
3. **Context Beta:** Select Tenant_A / WS_Beta.
4. Upload `beta_info.txt`: "Project Beta is led by Michael Ross."
5. **Query Alpha:** In WS_Alpha, ask "Who leads Project Beta?"
6. **Query Beta:** In WS_Beta, ask "Who leads Project Alpha?"
   **Expected Results:**

- **Query Alpha:** Should NOT find Michael Ross.
- **Query Beta:** Should NOT find Sarah Chen.
- Each workspace maintains its own Knowledge Graph and Vector Index.

---

## 3. UI State Refresh & Context Switching

### TC-UI-001: Dashboard Refresh on Context Change

**Description:** Verify that the Dashboard statistics update immediately when switching tenant/workspace.
**Pre-conditions:**

- Tenant_A / WS_A has 10 documents.
- Tenant_B / WS_B has 0 documents.
  **Steps:**

1. Select Tenant_A / WS_A.
2. Observe the "Documents" count on the Dashboard (should be 10).
3. Switch to Tenant_B / WS_B using the sidebar selector.
   **Expected Results:**

- The Dashboard "Documents" count should immediately update to 0.
- "Recent Activity" should clear or show activity only for Tenant_B.

### TC-UI-002: Knowledge Graph Visualization Isolation

**Description:** Verify the graph view only shows entities from the current workspace.
**Steps:**

1. In WS_A, upload a document about "Quantum Computing".
2. In WS_B, upload a document about "Ancient Rome".
3. Navigate to **Knowledge Graph** in WS_A.
4. Switch to WS_B.
   **Expected Results:**

- In WS_A, the graph shows "Quantum Computing" related nodes.
- After switching to WS_B, the graph clears and shows "Ancient Rome" related nodes.

### TC-UI-003: Query Conversation Reset on Context Change

**Description:** Verify that the active chat conversation is cleared when switching tenant/workspace to prevent cross-context information leakage in the UI.
**Steps:**

1. In WS_A, ask "What is Project Alpha?".
2. Receive response.
3. Switch to WS_B.
   **Expected Results:**

- The chat window should be cleared (Empty State shown).
- The conversation history for WS_A should NOT be visible in WS_B.
- _Note: This ensures that a user doesn't accidentally see sensitive data from another workspace in the current view._

---

## 4. Security & Negative Testing

### TC-SEC-001: Unauthorized Workspace Access

**Description:** Verify that manually changing the `X-Workspace-ID` header to a workspace the user doesn't have access to results in an error.
**Steps:**

1. Intercept an API request (e.g., `/api/v1/documents`).
2. Change the `X-Workspace-ID` header to a known ID from another tenant.
3. Send the request.
   **Expected Results:**

- API returns `403 Forbidden` or `404 Not Found`.
- No data from the unauthorized workspace is returned.

---

## 5. Example Content for Testing

### Document A (Project Alpha)

```text
Project Alpha Overview
Date: 2025-10-12
Lead: Sarah Chen
Location: Sector 7G
Security Code: ALPHA-99-X
Project Alpha is a classified initiative focused on developing next-generation edge computing nodes.
The primary goal is to reduce latency to under 1ms for industrial IoT applications.
```

### Document B (Project Beta)

```text
Project Beta Status Report
Date: 2025-11-05
Lead: Michael Ross
Location: Research Lab B
Budget: $2.5M
Project Beta explores the use of graph neural networks for predictive maintenance in smart factories.
It is entirely independent of Project Alpha.
```

### Document C (General Knowledge)

```text
EdgeQuake Documentation
EdgeQuake is a RAG framework that uses Knowledge Graphs.
It supports multi-tenancy and workspace isolation.
Users can upload TXT, MD, and JSON files.
```

---

## 6. Implementation Notes & Known Gaps

During the formalization of these test cases, a technical audit of the codebase revealed the following:

### 1. Isolation Mechanisms

- **Core Level:** The `TenantRAGManager` in `edgequake-core` provides **perfect isolation** by maintaining separate `EdgeQuake` instances (each with its own storage) for each tenant/workspace combination. This is the recommended path for production.
- **API Level:** Some current API handlers in `edgequake-api` use a shared `AppState` and rely on metadata-based filtering. While this works for graph nodes, it currently has a **known gap** in vector search results (chunks), which are not yet filtered by tenant ID in the shared index.

### 2. Entity Normalization

- The system automatically normalizes entity names (e.g., `PROJECT_ALPHA` becomes `PROJECTALPHA`). Test cases should account for this when verifying exact matches in the retrieved context.

### 3. UI State Management

- The UI correctly uses TanStack Query keys that include `tenantId` and `workspaceId`, ensuring that data is re-fetched and the UI state is refreshed upon context switching.
- Active conversations are managed via `useChatStore` and should be explicitly cleared on context change to satisfy **TC-UI-003**.

### 4. Security Recommendation

- It is highly recommended to use the `TenantRAGManager` for all multi-tenant deployments to ensure physical isolation of data files and storage instances, rather than relying solely on metadata filtering in a shared instance.
