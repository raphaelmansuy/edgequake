/** Entity and relationship API types. */

export interface Entity {
  /** Unique entity ID. */
  id: string;
  /** Entity name (used as label for display). */
  entity_name: string;
  /** Entity type (e.g., PERSON, ORGANIZATION). */
  entity_type: string;
  /** Human-readable description. */
  description?: string;
  /** Source document ID. */
  source_id?: string;
  /** Node degree (number of connections). */
  degree?: number;
  /** Additional metadata. */
  metadata?: Record<string, unknown>;
  /** ISO timestamp of creation. */
  created_at?: string;
  /** ISO timestamp of last update. */
  updated_at?: string;

  // Legacy fields for backward compatibility
  /** @deprecated Use entity_name instead. */
  label?: string;
  /** @deprecated Use metadata instead. */
  properties?: Record<string, unknown>;
  /** @deprecated Use source_id instead. */
  source_ids?: string[];
  /** @deprecated Aliases are now in metadata. */
  aliases?: string[];
}

export interface MergeEntitiesRequest {
  source_entity: string;
  target_entity: string;
  merge_strategy?: string;
  metadata?: Record<string, unknown>;
}

export interface MergeEntitiesResponse {
  status?: string;
  message?: string;
  merged_entity: Entity;
  merged_count?: number;
  merge_details?: {
    source_entity_id: string;
    target_entity_id: string;
    relationships_merged: number;
    duplicate_relationships_removed: number;
    description_strategy: string;
    metadata_strategy: string;
  };
}

// Relationship types
/** Relationship returned from the API - matches backend RelationshipResponse. */
export interface Relationship {
  /** Unique relationship ID. */
  id: string;
  /** Source entity ID. */
  src_id: string;
  /** Target entity ID. */
  tgt_id: string;
  /** Relationship type/label. */
  relation_type: string;
  /** Keywords describing the relationship. */
  keywords?: string;
  /** Weight/strength of the relationship. */
  weight?: number;
  /** Human-readable description. */
  description?: string;
  /** Source document ID. */
  source_id?: string;
  /** ISO timestamp of creation. */
  created_at?: string;

  // Legacy fields for backward compatibility
  /** @deprecated Use src_id instead. */
  source_entity_id?: string;
  /** @deprecated Use tgt_id instead. */
  target_entity_id?: string;
  /** @deprecated Use relation_type instead. */
  relationship_type?: string;
  /** @deprecated Use source_id instead. */
  source_ids?: string[];
  /** @deprecated Use metadata in Entity instead. */
  properties?: Record<string, unknown>;
}
