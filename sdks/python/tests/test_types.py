"""Tests for Pydantic type models — ensures serialization/deserialization works."""

from __future__ import annotations

from edgequake.types.auth import LoginRequest, TokenResponse, UserInfo
from edgequake.types.chat import (
    ChatChoice,
    ChatCompletionRequest,
    ChatCompletionResponse,
    ChatMessage,
    ChatUsage,
)
from edgequake.types.conversations import ConversationInfo, FolderInfo, Message
from edgequake.types.documents import (
    DocumentSummary,
    ListDocumentsResponse,
    UploadDocumentResponse,
)
from edgequake.types.graph import Entity, GraphNode, GraphResponse, Relationship
from edgequake.types.operations import (
    ChunkDetail,
    CostSummary,
    ModelInfo,
    PipelineStatus,
    ProvenanceRecord,
    TaskInfo,
)
from edgequake.types.query import QueryRequest, QueryResponse, SourceReference
from edgequake.types.shared import HealthResponse
from edgequake.types.workspaces import WorkspaceInfo, WorkspaceStats


class TestHealthResponse:
    def test_basic(self) -> None:
        h = HealthResponse(status="healthy", version="0.1.0")
        assert h.status == "healthy"

    def test_extra_fields_allowed(self) -> None:
        h = HealthResponse.model_validate({"status": "ok", "custom_field": True})
        assert h.status == "ok"


class TestDocumentTypes:
    def test_upload_response(self) -> None:
        r = UploadDocumentResponse(
            document_id="doc-1", status="processing", message="OK"
        )
        assert r.document_id == "doc-1"

    def test_document_summary(self) -> None:
        ds = DocumentSummary(id="doc-1", status="completed")
        assert ds.id == "doc-1"
        assert ds.status == "completed"

    def test_list_documents_response(self) -> None:
        lr = ListDocumentsResponse(documents=[])
        assert len(lr.documents) == 0


class TestQueryTypes:
    def test_query_request(self) -> None:
        qr = QueryRequest(query="test query")
        assert qr.query == "test query"
        assert qr.mode == "hybrid"  # default

    def test_query_request_with_mode(self) -> None:
        qr = QueryRequest(query="test", mode="hybrid")
        assert qr.mode == "hybrid"

    def test_source_reference(self) -> None:
        sr = SourceReference(score=0.95)
        assert sr.score == 0.95

    def test_query_response(self) -> None:
        qr = QueryResponse(answer="The answer is 42.", sources=[])
        assert "42" in qr.answer


class TestChatTypes:
    def test_chat_message(self) -> None:
        m = ChatMessage(role="user", content="Hello")
        assert m.role == "user"

    def test_chat_completion_request(self) -> None:
        r = ChatCompletionRequest(message="Hi")
        assert r.message == "Hi"

    def test_chat_completion_response(self) -> None:
        r = ChatCompletionResponse(
            conversation_id="conv-1",
            content="Hi there!",
        )
        assert r.content == "Hi there!"

    def test_chat_choice(self) -> None:
        c = ChatChoice(
            index=0,
            message=ChatMessage(role="assistant", content="Hi!"),
            finish_reason="stop",
        )
        assert c.message.content == "Hi!"

    def test_chat_usage(self) -> None:
        u = ChatUsage(prompt_tokens=5, completion_tokens=3, total_tokens=8)
        assert u.total_tokens == 8


class TestGraphTypes:
    def test_graph_node(self) -> None:
        n = GraphNode(id="n1", label="PERSON", properties={"name": "Alice"})
        assert n.label == "PERSON"

    def test_graph_response(self) -> None:
        r = GraphResponse(nodes=[], edges=[])
        assert len(r.nodes) == 0

    def test_entity(self) -> None:
        e = Entity(name="ALICE", entity_type="PERSON")
        assert e.name == "ALICE"

    def test_relationship(self) -> None:
        r = Relationship(source="ALICE", target="BOB", relationship_type="KNOWS")
        assert r.relationship_type == "KNOWS"
        assert r.source == "ALICE"


class TestAuthTypes:
    def test_login_request(self) -> None:
        lr = LoginRequest(username="admin", password="secret")
        assert lr.username == "admin"

    def test_token_response(self) -> None:
        tr = TokenResponse(
            access_token="jwt",
            refresh_token="ref",
            token_type="bearer",
            expires_in=3600,
        )
        assert tr.access_token == "jwt"

    def test_user_info(self) -> None:
        u = UserInfo(id="u1", username="admin")
        assert u.username == "admin"


class TestWorkspaceTypes:
    def test_workspace_info(self) -> None:
        w = WorkspaceInfo(id="ws-1", name="Default")
        assert w.name == "Default"

    def test_workspace_stats(self) -> None:
        s = WorkspaceStats(
            workspace_id="ws-1",
            document_count=10,
            entity_count=50,
            relationship_count=30,
            chunk_count=100,
        )
        assert s.entity_count == 50


class TestConversationTypes:
    def test_conversation_info(self) -> None:
        c = ConversationInfo(id="c1", title="Chat")
        assert c.title == "Chat"

    def test_message(self) -> None:
        m = Message(id="m1", role="user", content="Hello")
        assert m.content == "Hello"

    def test_folder_info(self) -> None:
        f = FolderInfo(id="f1", name="Work")
        assert f.name == "Work"


class TestOperationTypes:
    def test_task_info(self) -> None:
        t = TaskInfo(track_id="t1", status="running")
        assert t.status == "running"

    def test_pipeline_status(self) -> None:
        p = PipelineStatus(status="idle")
        assert p.status == "idle"

    def test_cost_summary(self) -> None:
        c = CostSummary(total_cost_usd=1.5, total_tokens=10000)
        assert c.total_cost_usd == 1.5

    def test_chunk_detail(self) -> None:
        c = ChunkDetail(id="ch1", document_id="doc-1", content="text")
        assert c.content == "text"

    def test_provenance_record(self) -> None:
        p = ProvenanceRecord(chunk_id="ch1", document_id="doc-1")
        assert p.chunk_id == "ch1"

    def test_model_info(self) -> None:
        m = ModelInfo(id="gpt-4", name="GPT-4")
        assert m.name == "GPT-4"
