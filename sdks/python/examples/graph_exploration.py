#!/usr/bin/env python3
"""
Graph Exploration — EdgeQuake Python SDK

WHY: EdgeQuake's knowledge graph stores entities and relationships
extracted from documents. This example shows how to traverse it.

Requirements:
    - EdgeQuake server running on http://localhost:8080
    - EDGEQUAKE_API_KEY environment variable set
    - Documents uploaded and processed

Usage:
    export EDGEQUAKE_API_KEY="demo-key"
    python examples/graph_exploration.py
"""
import os

from edgequake import EdgequakeClient


def main():
    client = EdgequakeClient(
        api_key=os.environ.get("EDGEQUAKE_API_KEY", "demo-key"),
        base_url=os.environ.get("EDGEQUAKE_URL", "http://localhost:8080"),
    )

    # ── 1. Get graph overview ─────────────────────────────────

    graph = client.graph.get()
    print(f"Graph overview: {graph}")

    # ── 2. Search entities by keyword ─────────────────────────

    # WHY: Node search uses fuzzy matching on entity names and descriptions.
    nodes = client.graph.search_nodes(query="machine learning")
    print("\nSearch results:")
    for node in nodes or []:
        print(f"  {node['name']} ({node['entity_type']})")

    # ── 3. List entities ──────────────────────────────────────

    entities = client.graph.entities.list()
    entities_list = (
        entities.get("items", []) if isinstance(entities, dict) else entities
    )
    print(f"\nTotal entities: {len(entities_list)}")
    for entity in entities_list[:5]:
        desc = entity.get("description") or "(no description)"
        print(f"  {entity['name']} — {desc}")

    # ── 4. Get entity neighborhood ────────────────────────────

    # WHY: Neighborhood returns the entity plus all directly connected
    # entities (1-hop), useful for context expansion.
    if entities_list:
        first_entity = entities_list[0]
        neighborhood = client.graph.entities.neighborhood(first_entity["name"])
        print(f"\nNeighborhood of \"{first_entity['name']}\": {neighborhood}")

    # ── 5. List relationships ─────────────────────────────────

    relationships = client.graph.relationships.list()
    rel_list = (
        relationships.get("items", [])
        if isinstance(relationships, dict)
        else relationships
    )
    print(f"\nTotal relationships: {len(rel_list)}")
    for rel in rel_list[:5]:
        print(
            f"  {rel['source_name']} --[{rel['relationship_type']}]--> {rel['target_name']}"
        )

    # ── 6. Search labels ──────────────────────────────────────

    labels = client.graph.search_labels(query="PER")
    print(f"\nLabel search: {labels}")

    # ── 7. Popular labels ─────────────────────────────────────

    popular = client.graph.get_popular_labels()
    print(f"Popular labels: {popular}")


if __name__ == "__main__":
    main()
