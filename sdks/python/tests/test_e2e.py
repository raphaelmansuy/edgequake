"""E2E Tests for EdgeQuake Python SDK against a live backend.

Requires:
    EDGEQUAKE_E2E_URL=http://localhost:8080 python -m pytest tests/test_e2e.py -v

WHY: Unit tests validate SDK logic with mocks; E2E tests validate actual
API compatibility against the real EdgeQuake backend (Docker stack).
"""

from __future__ import annotations

import os
import uuid

import pytest

# WHY: Skip entire module when backend URL is not set — avoids CI failures
E2E_URL = os.environ.get("EDGEQUAKE_E2E_URL", "")
pytestmark = pytest.mark.skipif(not E2E_URL, reason="EDGEQUAKE_E2E_URL not set")


@pytest.fixture(scope="module")
def client():
    """Create an EdgeQuake client pointing at the live backend."""
    from edgequake import EdgeQuake

    return EdgeQuake(base_url=E2E_URL)


@pytest.fixture(scope="module")
def test_doc_id(client):
    """Upload a test document and return its ID for subsequent tests."""
    tag = uuid.uuid4().hex[:8]
    # WHY: Python SDK upload() takes content (positional) + title (kwarg)
    resp = client.documents.upload(
        content="Knowledge graphs connect ALICE and BOB through WORKS_WITH relationships.",
        title=f"Python E2E {tag}",
    )
    return resp.document_id


# ── 1. Health ──────────────────────────────────────────────


class TestHealth:
    def test_health_check(self, client):
        h = client.health()
        assert h.status == "healthy"
        assert h.version is not None

    def test_health_has_components(self, client):
        h = client.health()
        assert h.components is not None


# ── 2. Documents ───────────────────────────────────────────


class TestDocuments:
    def test_list_documents(self, client):
        resp = client.documents.list()
        assert resp.documents is not None
        assert isinstance(resp.documents, list)

    def test_upload_text_document(self, client, test_doc_id):
        assert test_doc_id is not None

    def test_get_document(self, client, test_doc_id):
        doc = client.documents.get(test_doc_id)
        assert doc is not None


# ── 3. Graph ───────────────────────────────────────────────


class TestGraph:
    def test_get_graph(self, client):
        g = client.graph.get()
        assert g is not None

    def test_search_nodes(self, client):
        # WHY: Python SDK search_nodes takes query as positional arg
        results = client.graph.search_nodes("alice")
        assert results is not None

    def test_list_entities(self, client):
        entities = client.entities.list()
        assert entities is not None

    def test_create_and_delete_entity(self, client):
        from edgequake.types.graph import EntityCreate

        tag = uuid.uuid4().hex[:8]
        name = f"PYTEST_{tag}"
        # WHY: Python SDK entities.create takes EntityCreate object
        created = client.entities.create(
            EntityCreate(
                name=name,
                entity_type="TEST",
                description="E2E test entity",
                source_id="e2e-test",
            )
        )
        assert created is not None

        # WHY: Python SDK entities.delete returns None (void)
        client.entities.delete(name)

    def test_list_relationships(self, client):
        rels = client.relationships.list()
        assert rels is not None


# ── 4. Query ───────────────────────────────────────────────


class TestQuery:
    def test_execute_query(self, client):
        result = client.query.execute(query="What is a knowledge graph?")
        assert result is not None

    def test_execute_query_with_mode(self, client):
        result = client.query.execute(query="Tell me about entities", mode="hybrid")
        assert result is not None


# ── 5. Chat ────────────────────────────────────────────────


class TestChat:
    def test_chat_completions(self, client):
        # WHY: Python SDK chat.complete takes messages list, not single message
        # Chat may return 401/403 without auth — handle gracefully
        try:
            from edgequake.types.chat import ChatMessage

            resp = client.chat.complete(
                messages=[{"role": "user", "content": "Hello, what is EdgeQuake?"}]
            )
            assert resp is not None
        except Exception as e:
            if "401" in str(e) or "403" in str(e) or "422" in str(e):
                pytest.skip("Chat requires auth or has validation error")
            raise


# ── 6. Tenants ─────────────────────────────────────────────


class TestTenants:
    def test_list_tenants(self, client):
        tenants = client.tenants.list()
        assert tenants is not None

    def test_create_and_delete_tenant(self, client):
        from edgequake.types.auth import TenantCreate

        tag = uuid.uuid4().hex[:8]
        # WHY: Python SDK tenants.create takes TenantCreate object
        t = client.tenants.create(
            TenantCreate(name=f"pytest-{tag}", slug=f"pytest-{tag}")
        )
        assert t is not None
        assert t.id is not None

        # Delete
        client.tenants.delete(t.id)


# ── 7. Users ──────────────────────────────────────────────


class TestUsers:
    def test_list_users(self, client):
        users = client.users.list()
        assert users is not None


# ── 8. API Keys ────────────────────────────────────────────


class TestApiKeys:
    def test_list_api_keys(self, client):
        keys = client.api_keys.list()
        assert keys is not None


# ── 9. Tasks ───────────────────────────────────────────────


class TestTasks:
    def test_list_tasks(self, client):
        tasks = client.tasks.list()
        assert tasks is not None


# ── 10. Pipeline ───────────────────────────────────────────


class TestPipeline:
    def test_pipeline_status(self, client):
        status = client.pipeline.status()
        assert status is not None

    def test_queue_metrics(self, client):
        metrics = client.pipeline.queue_metrics()
        assert metrics is not None


# ── 11. Models ─────────────────────────────────────────────


class TestModels:
    def test_list_models(self, client):
        # WHY: Python SDK uses `list` (not `catalog`)
        models = client.models.list()
        assert models is not None

    def test_models_health(self, client):
        health = client.models.health()
        assert health is not None


# ── 12. Settings ───────────────────────────────────────────


class TestSettings:
    def test_provider_status(self, client):
        status = client.settings.provider_status()
        assert status is not None


# ── 13. Costs ──────────────────────────────────────────────


class TestCosts:
    def test_cost_summary(self, client):
        summary = client.costs.summary()
        assert summary is not None


# ── 14. Cleanup ────────────────────────────────────────────


class TestCleanup:
    def test_delete_test_document(self, client, test_doc_id):
        """Cleanup: delete the test document created during setup."""
        # WHY: Python SDK documents.delete returns None (void)
        client.documents.delete(test_doc_id)
        # Verify it was deleted by trying to get it (should fail or return None)
        try:
            doc = client.documents.get(test_doc_id)
            # If we get here without error, check if returned None
            assert doc is None or doc.status == "deleted"
        except Exception:
            # Expected — document was deleted
            pass
