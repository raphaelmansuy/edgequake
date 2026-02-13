"""Tests for lineage, provenance, chunk lineage, and metadata types.

WHY: OODA-17 — Ensure Python SDK covers all lineage/metadata fields
from the API surface. Tests Pydantic model serialization/deserialization,
optional field handling, and field-level validation.
"""

from __future__ import annotations

import pytest
from edgequake.types.graph import (
    Entity,
    EntityCreate,
    EntityDetail,
    EntityUpdate,
    Relationship,
    RelationshipCreate,
)
from edgequake.types.operations import (
    ChunkDetail,
    ChunkLineageInfo,
    DocumentFullLineage,
    LineageEdge,
    LineageGraph,
    LineageNode,
    ProvenanceRecord,
    ProviderStatus,
)


# ─────────────────────── Entity Lineage Fields ───────────────────────


class TestEntityLineageFields:
    """Entity model includes lineage metadata: source_id, metadata, timestamps."""

    def test_entity_with_source_id(self) -> None:
        e = Entity(name="ALICE", entity_type="PERSON", source_id="doc-1")
        assert e.source_id == "doc-1"

    def test_entity_with_metadata(self) -> None:
        e = Entity(
            name="ALICE",
            entity_type="PERSON",
            metadata={"confidence": 0.95, "extraction_model": "gpt-4o"},
        )
        assert e.metadata is not None
        assert e.metadata["confidence"] == 0.95
        assert e.metadata["extraction_model"] == "gpt-4o"

    def test_entity_with_timestamps(self) -> None:
        e = Entity(
            name="ALICE",
            entity_type="PERSON",
            created_at="2026-01-15T10:00:00Z",
            updated_at="2026-01-16T12:00:00Z",
        )
        assert e.created_at == "2026-01-15T10:00:00Z"
        assert e.updated_at == "2026-01-16T12:00:00Z"

    def test_entity_with_degree_and_source_count(self) -> None:
        e = Entity(name="ALICE", entity_type="PERSON", degree=5, source_count=3)
        assert e.degree == 5
        assert e.source_count == 3

    def test_entity_from_api_response_alias(self) -> None:
        """Entity name can come as entity_name from API."""
        e = Entity.model_validate({"entity_name": "BOB", "entity_type": "PERSON"})
        assert e.name == "BOB"

    def test_entity_with_properties(self) -> None:
        e = Entity(
            name="ALICE",
            entity_type="PERSON",
            properties={"role": "researcher", "affiliation": "MIT"},
        )
        assert e.properties is not None
        assert e.properties["role"] == "researcher"

    def test_entity_all_fields_none(self) -> None:
        """All optional fields default to None."""
        e = Entity()
        assert e.name is None
        assert e.entity_type is None
        assert e.metadata is None
        assert e.source_id is None
        assert e.created_at is None
        assert e.degree is None


# ─────────────────────── EntityCreate with Metadata ───────────────────────


class TestEntityCreateMetadata:
    """EntityCreate carries source_id and serializes entity_name correctly."""

    def test_create_with_source_id(self) -> None:
        ec = EntityCreate(
            name="DATA_SCIENCE",
            entity_type="TECHNOLOGY",
            description="Field of study",
            source_id="doc-research",
        )
        assert ec.source_id == "doc-research"

    def test_create_serializes_entity_name(self) -> None:
        """SDK field is `name` but API expects `entity_name`."""
        ec = EntityCreate(
            name="DATA_SCIENCE",
            entity_type="TECHNOLOGY",
        )
        data = ec.model_dump(by_alias=True)
        assert "entity_name" in data
        assert data["entity_name"] == "DATA_SCIENCE"

    def test_create_with_properties(self) -> None:
        ec = EntityCreate(
            name="ALICE",
            entity_type="PERSON",
            properties={"confidence": 0.95, "merged_count": 3},
        )
        assert ec.properties is not None
        assert ec.properties["confidence"] == 0.95


# ─────────────────────── EntityDetail ───────────────────────


class TestEntityDetail:
    """EntityDetail extends Entity with neighbors, relationships, sources."""

    def test_detail_with_neighbors(self) -> None:
        ed = EntityDetail(
            name="ALICE",
            entity_type="PERSON",
            neighbors=[Entity(name="BOB", entity_type="PERSON")],
        )
        assert ed.neighbors is not None
        assert len(ed.neighbors) == 1
        assert ed.neighbors[0].name == "BOB"

    def test_detail_inherits_metadata_fields(self) -> None:
        ed = EntityDetail(
            name="ALICE",
            entity_type="PERSON",
            source_id="doc-1",
            metadata={"test": True},
            created_at="2026-01-15T10:00:00Z",
        )
        assert ed.source_id == "doc-1"
        assert ed.metadata is not None
        assert ed.created_at is not None


# ─────────────────────── LineageGraph ───────────────────────


class TestLineageGraph:
    """LineageGraph contains nodes, edges, and root_id."""

    def test_empty_graph(self) -> None:
        g = LineageGraph()
        assert len(g.nodes) == 0
        assert len(g.edges) == 0
        assert g.root_id is None

    def test_graph_with_nodes_and_edges(self) -> None:
        g = LineageGraph(
            nodes=[
                LineageNode(id="n1", name="ALICE", node_type="entity"),
                LineageNode(id="n2", name="doc-1", node_type="document"),
            ],
            edges=[
                LineageEdge(source="n1", target="n2", relationship="extracted_from"),
            ],
            root_id="n1",
        )
        assert len(g.nodes) == 2
        assert g.nodes[0].name == "ALICE"
        assert g.nodes[0].node_type == "entity"
        assert len(g.edges) == 1
        assert g.edges[0].relationship == "extracted_from"
        assert g.root_id == "n1"

    def test_lineage_node_with_properties(self) -> None:
        n = LineageNode(
            id="n1",
            name="ALICE",
            properties={"degree": 5, "source_count": 3},
        )
        assert n.properties is not None
        assert n.properties["degree"] == 5

    def test_lineage_edge_with_metadata(self) -> None:
        e = LineageEdge(
            source="n1",
            target="n2",
            relationship="extracted_from",
            metadata={"confidence": 0.9, "chunk_id": "c1"},
        )
        assert e.metadata is not None
        assert e.metadata["confidence"] == 0.9


# ─────────────────────── DocumentFullLineage ───────────────────────


class TestDocumentFullLineage:
    """DocumentFullLineage has metadata and lineage data."""

    def test_minimal(self) -> None:
        fl = DocumentFullLineage(document_id="doc-1")
        assert fl.document_id == "doc-1"
        assert fl.metadata is None
        assert fl.lineage is None

    def test_with_metadata_and_lineage(self) -> None:
        fl = DocumentFullLineage(
            document_id="doc-1",
            metadata={
                "title": "Research Paper",
                "author": "Dr. Smith",
                "tags": ["AI", "NLP"],
            },
            lineage={
                "entities_extracted": 15,
                "relationships_extracted": 8,
                "pipeline_version": "1.2.0",
            },
        )
        assert fl.metadata is not None
        assert fl.metadata["title"] == "Research Paper"
        assert fl.metadata["tags"] == ["AI", "NLP"]
        assert fl.lineage is not None
        assert fl.lineage["entities_extracted"] == 15

    def test_model_validate_from_dict(self) -> None:
        data = {
            "document_id": "doc-1",
            "metadata": {"key": "value"},
            "lineage": {"step": 1},
        }
        fl = DocumentFullLineage.model_validate(data)
        assert fl.document_id == "doc-1"
        assert fl.metadata == {"key": "value"}


# ─────────────────────── ChunkLineageInfo ───────────────────────


class TestChunkLineageInfo:
    """ChunkLineageInfo includes position, parent doc, and entity info."""

    def test_minimal(self) -> None:
        cli = ChunkLineageInfo(chunk_id="c1")
        assert cli.chunk_id == "c1"
        assert cli.document_id is None
        assert cli.entity_count is None

    def test_full_fields(self) -> None:
        cli = ChunkLineageInfo(
            chunk_id="c1",
            document_id="doc-1",
            document_name="Paper.pdf",
            document_type="pdf",
            index=3,
            start_line=42,
            end_line=60,
            start_offset=1200,
            end_offset=1800,
            token_count=150,
            content_preview="Alice works at MIT...",
            entity_count=3,
            relationship_count=2,
            entity_names=["ALICE", "MIT", "BOB"],
            document_metadata={"author": "Dr. Smith"},
        )
        assert cli.start_line == 42
        assert cli.end_line == 60
        assert cli.entity_count == 3
        assert len(cli.entity_names) == 3
        assert cli.document_metadata is not None
        assert cli.document_metadata["author"] == "Dr. Smith"
        assert cli.document_type == "pdf"
        assert cli.token_count == 150

    def test_entity_names_default_empty(self) -> None:
        cli = ChunkLineageInfo(chunk_id="c1")
        assert cli.entity_names == []


# ─────────────────────── ProvenanceRecord ───────────────────────


class TestProvenanceRecordLineage:
    """ProvenanceRecord includes entity_name, confidence, extraction_method."""

    def test_full_provenance(self) -> None:
        pr = ProvenanceRecord(
            entity_id="e1",
            entity_name="ALICE",
            document_id="doc-1",
            document_title="Research Paper",
            chunk_id="c1",
            extraction_method="llm",
            confidence=0.95,
            created_at="2026-01-15T10:00:00Z",
        )
        assert pr.entity_name == "ALICE"
        assert pr.confidence == 0.95
        assert pr.extraction_method == "llm"
        assert pr.document_title == "Research Paper"

    def test_minimal_provenance(self) -> None:
        pr = ProvenanceRecord()
        assert pr.entity_id is None
        assert pr.confidence is None

    def test_confidence_zero_is_valid(self) -> None:
        pr = ProvenanceRecord(confidence=0.0)
        assert pr.confidence == 0.0

    def test_model_validate_from_api(self) -> None:
        data = {
            "entity_id": "e1",
            "document_id": "d1",
            "confidence": 0.85,
            "extraction_method": "regex",
        }
        pr = ProvenanceRecord.model_validate(data)
        assert pr.confidence == 0.85
        assert pr.extraction_method == "regex"


# ─────────────────────── Relationship with Metadata ───────────────────────


class TestRelationshipMetadata:
    """Relationships carry source_id for lineage tracking."""

    def test_relationship_create_with_source_id(self) -> None:
        rc = RelationshipCreate(
            source="ALICE",
            target="BOB",
            relationship_type="KNOWS",
            weight=0.8,
            description="Research collaboration",
            source_id="doc-1",
        )
        assert rc.source_id == "doc-1"
        assert rc.weight == 0.8

    def test_relationship_properties(self) -> None:
        r = Relationship(
            source="ALICE",
            target="BOB",
            relationship_type="WORKS_AT",
        )
        assert r.source == "ALICE"
        assert r.target == "BOB"


# ─────────────────────── ProviderStatus ───────────────────────


class TestProviderStatusLineage:
    """ProviderStatus includes embedding info for lineage tracking."""

    def test_provider_status_fields(self) -> None:
        ps = ProviderStatus(
            current_provider="openai",
            current_model="gpt-4o",
            embedding_provider="ollama",
            embedding_model="nomic-embed-text",
            status="healthy",
        )
        assert ps.current_provider == "openai"
        assert ps.embedding_provider == "ollama"
        assert ps.embedding_model == "nomic-embed-text"


# ─────────────────────── Edge Cases ───────────────────────


class TestLineageEdgeCases:
    """Edge cases for lineage/metadata types."""

    def test_entity_metadata_nested_objects(self) -> None:
        e = Entity(
            name="ALICE",
            metadata={
                "source_line_range": [10, 15],
                "extraction": {"model": "gpt-4o", "gleaning": 2},
            },
        )
        assert e.metadata is not None
        assert e.metadata["source_line_range"] == [10, 15]
        assert e.metadata["extraction"]["model"] == "gpt-4o"

    def test_document_full_lineage_empty_metadata(self) -> None:
        fl = DocumentFullLineage(document_id="d1", metadata={})
        assert fl.metadata == {}

    def test_chunk_lineage_info_zero_counts(self) -> None:
        cli = ChunkLineageInfo(
            chunk_id="c1",
            entity_count=0,
            relationship_count=0,
            entity_names=[],
        )
        assert cli.entity_count == 0
        assert cli.relationship_count == 0

    def test_lineage_graph_single_node_no_edges(self) -> None:
        g = LineageGraph(
            nodes=[LineageNode(id="n1", name="ORPHAN")],
            edges=[],
        )
        assert len(g.nodes) == 1
        assert len(g.edges) == 0

    def test_provenance_record_serialization_roundtrip(self) -> None:
        pr = ProvenanceRecord(
            entity_id="e1",
            document_id="d1",
            confidence=0.9,
        )
        data = pr.model_dump()
        pr2 = ProvenanceRecord.model_validate(data)
        assert pr2.entity_id == "e1"
        assert pr2.confidence == 0.9

    def test_entity_create_roundtrip(self) -> None:
        ec = EntityCreate(
            name="TEST",
            entity_type="CONCEPT",
            description="Test entity",
            source_id="manual",
        )
        data = ec.model_dump(by_alias=True)
        assert data["entity_name"] == "TEST"
        assert data["source_id"] == "manual"
