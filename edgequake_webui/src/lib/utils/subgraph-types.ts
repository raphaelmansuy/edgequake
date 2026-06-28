/**
 * API subgraph DTOs (SPEC-028 FP-028-09) — mirrors backend SubgraphBundle.
 */

export interface EntityLineage {
  source_chunk_ids?: string[];
  source_document_id?: string;
  source_file_path?: string;
}

export interface RelationshipLineage {
  source_chunk_id?: string;
  source_document_id?: string;
  source_file_path?: string;
}

export interface ContextEntityApi {
  id: string;
  name: string;
  entity_type: string;
  description: string;
  score: number;
  degree: number;
  lineage?: EntityLineage;
}

export interface ContextRelationshipApi {
  id: string;
  source: string;
  target: string;
  relation_type: string;
  description: string;
  score: number;
  lineage?: RelationshipLineage;
}

export interface SubgraphBundle {
  entities: ContextEntityApi[];
  relationships: ContextRelationshipApi[];
}
