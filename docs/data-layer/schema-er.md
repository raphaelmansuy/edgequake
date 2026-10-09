---
title: "Schema entity-relationship diagrams"
description: "Full Mermaid E/R diagrams of the EdgeQuake PostgreSQL schema, grouped by domain, generated from the migrations through schema train 169."
---

# Schema entity-relationship diagrams

This page is the full picture of the EdgeQuake relational schema. It is for operators who inspect the database and for developers who add columns or foreign keys.

Every diagram below was built from `edgequake/migrations/` through migration **169** (SPEC-163 provider connections). Column types are simplified for Mermaid (for example `TIMESTAMP WITH TIME ZONE` becomes `timestamptz`, and size arguments like `VARCHAR(64)` become `varchar`). Primary keys are marked `PK` and declared foreign keys are marked `FK`.

> How to read the diagrams: boxes are tables, lines are foreign keys, and `|o` / `}o` mean "zero or one / zero or many". Attributes inside a box are a representative subset when a table has many columns; the migration SQL is the authority for the full list.

Related pages: [PostgreSQL overview](./postgres.md) · [pgvector](./pgvector.md) · [Apache AGE](./age.md) · [Storage model](../architecture/storage-model.md) · [Upgrading](../operations/upgrading.md)

## Domain map

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
  T["Tenancy and identity"] --> D["Documents and chunks"]
  T --> S["SSO"]
  D --> P["PDF pipeline"]
  D --> G["Graph read models"]
  D --> E["Embeddings"]
  D --> J["Tasks"]
  J --> W["Durable writes"]
  T --> C["Conversations"]
  T --> O["Caches and ops"]
%% eq-classes
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class G,E eqLlm
class O eqStore
```

Start at **Tenancy and identity**. Everything else is scoped by a tenant and usually by a workspace.


## Tenancy and identity

Who owns what, and how a person signs in. Start at `tenants`: every other row in this diagram hangs off a tenant or a user.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
erDiagram
    workspaces }o--|| tenants : "tenant_id"
    users }o--|| tenants : "tenant_id"
    memberships }o--|| tenants : "tenant_id"
    memberships }o--|| workspaces : "workspace_id"
    memberships }o--|| users : "user_id"
    api_keys }o--|| users : "user_id"
    refresh_tokens }o--|| users : "user_id"
    tenants {
        uuid tenant_id PK
        varchar name
        varchar slug
        jsonb settings
        jsonb metadata
        boolean is_active
        timestamp created_at
        timestamp updated_at
        text description
        varchar plan
        int max_workspaces
        int max_users
    }
    workspaces {
        uuid workspace_id PK
        uuid tenant_id FK
        varchar name
        varchar slug
        text description
        jsonb settings
        jsonb metadata
        boolean is_active
        timestamp created_at
        timestamp updated_at
    }
    users {
        uuid user_id PK
        uuid tenant_id FK
        varchar email
        varchar username
        varchar display_name
        text password_hash
        varchar role
        boolean is_active
        timestamp last_login_at
        jsonb metadata
        timestamp created_at
        timestamp updated_at
        int failed_login_attempts
        timestamp locked_until
    }
    memberships {
        uuid membership_id PK
        uuid tenant_id FK
        uuid workspace_id FK
        uuid user_id FK
        varchar role
        boolean is_active
        timestamp joined_at
        jsonb metadata
    }
    api_keys {
        uuid key_id PK
        uuid user_id FK
        text key_hash
        varchar key_prefix
        varchar name
        text scopes
        varchar rate_limit_tier
        boolean is_active
        timestamp created_at
        timestamp last_used_at
        timestamp expires_at
        jsonb metadata
    }
    refresh_tokens {
        uuid token_id PK
        uuid user_id FK
        text token_hash
        timestamp expires_at
        boolean revoked
        timestamp revoked_at
        timestamp created_at
        text user_agent
        INET ip_address
        uuid family_id
        text status
    }
    jwt_jti_denylist {
        text jti PK
        timestamp expires_at
        timestamp revoked_at
        text reason
    }
    oauth_refresh_grants {
        text token_hash PK
        uuid family_id
        text client_id
        text resource
        text scope
        text user_id
        text role
        text tenant_id
        text workspace_id
        text status
        timestamp expires_at
        timestamp created_at
        timestamp updated_at
    }
    auth_handoff_codes {
        text code_hash PK
        uuid user_id
        uuid family_id
        text provider_slug
        uuid tenant_id
        uuid workspace_id
        text redirect_after
        timestamp expires_at
        timestamp created_at
    }
    tenant_lane_quota {
        uuid tenant_id PK
        text fairness_class PK
        float8 weight
        int max_concurrent
    }
    tenant_vruntime {
        uuid tenant_id PK
        text fairness_class PK
        float8 vruntime
    }
```

Read the boxes as tables and the arrows as foreign keys that point toward the parent. Tables in this section: `tenants`, `workspaces`, `users`, `memberships`, `api_keys`, `refresh_tokens`, `jwt_jti_denylist`, `oauth_refresh_grants`, `auth_handoff_codes`, `tenant_lane_quota`, `tenant_vruntime`.

## Single sign-on (OIDC)

External identity providers and the sessions they create. Read left to right: a provider links to federated identities, which open sessions.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
erDiagram
    federated_access_jti }o--|| federated_sessions : "family_id"
    identity_providers {
        uuid provider_id PK
        text slug
        text kind
        text display_name
        text issuer
        text client_id
        text client_secret_ref
        text redirect_uri
        text scopes
        boolean trust_email
        timestamp created_at
    }
    federated_identities {
        uuid federated_id PK
        uuid user_id FK
        text provider_slug
        text issuer
        text subject
        text email_at_link
        boolean email_verified_at_link
        timestamp linked_at
        timestamp last_login_at
    }
    federated_sessions {
        uuid family_id PK
        uuid user_id FK
        text provider_slug
        text issuer
        text subject
        text idp_sid
        uuid tenant_id
        uuid workspace_id
        timestamp created_at
        timestamp revoked_at
    }
    federated_access_jti {
        text jti PK
        uuid family_id FK
        timestamp expires_at
    }
    oidc_login_attempts {
        text state PK
        text provider_slug
        text pkce_verifier
        text nonce
        text organization_hint
        text redirect_after
        timestamp expires_at
        timestamp created_at
    }
    oidc_logout_jti {
        text issuer PK
        text jti PK
        timestamp expires_at
    }
```

Read the boxes as tables and the arrows as foreign keys that point toward the parent. Tables in this section: `identity_providers`, `federated_identities`, `federated_sessions`, `federated_access_jti`, `oidc_login_attempts`, `oidc_logout_jti`.

## Documents and chunks

Uploaded content and how it is split for retrieval. A document owns its chunks; `chunk_serving_state` is the visibility fence.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
erDiagram
    chunks }o--|| documents : "document_id"
    chunk_serving_state }o--|| chunks : "chunk_id"
    document_artifacts }o--|| documents : "document_id"
    document_originals }o--|| documents : "document_id"
    ingestion_dedup }o--|| documents : "document_id"
    pipeline_checkpoints }o--|| documents : "document_id"
    documents {
        uuid id PK
        uuid tenant_id FK
        uuid workspace_id FK
        text title
        text content
        varchar content_hash
        jsonb metadata
        text file_path
        bigint file_size_bytes
        varchar content_type
        timestamp created_at
    }
    chunks {
        uuid id PK
        uuid document_id FK
        uuid tenant_id FK
        uuid workspace_id FK
        text content
        int chunk_index
        int start_offset
        int end_offset
        int token_count
        vector embedding
        timestamp created_at
    }
    chunk_serving_state {
        uuid chunk_id PK
        text state
        int attempt_count
        jsonb last_error
        timestamp updated_at
    }
    document_artifacts {
        uuid document_id PK
        text kind PK
        jsonb payload
        timestamp created_at
        timestamp updated_at
    }
    document_originals {
        uuid document_id PK
        uuid workspace_id FK
        varchar filename
        varchar content_type
        bigint file_size_bytes
        bytea original_data
        timestamp created_at
    }
    ingestion_dedup {
        uuid id PK
        uuid workspace_id FK
        varchar content_hash
        text pipeline_version
        uuid document_id FK
        timestamp created_at
    }
    failed_chunks {
        uuid id PK
        varchar document_id
        uuid workspace_id
        uuid tenant_id
        int chunk_index
        varchar chunk_id
        text error_message
        boolean was_timeout
        int retry_attempts
        bigint processing_time_ms
    }
    pipeline_checkpoints {
        uuid document_id PK
        text kind PK
        jsonb payload
        timestamp created_at
        timestamp updated_at
    }
```

Read the boxes as tables and the arrows as foreign keys that point toward the parent. Tables in this section: `documents`, `chunks`, `chunk_serving_state`, `document_artifacts`, `document_originals`, `ingestion_dedup`, `failed_chunks`, `pipeline_checkpoints`.

## PDF pipeline

Binary PDFs, per-page geometry, layout regions, and rendered page images. Every PDF row points at a document.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
erDiagram
    pdf_document_blobs }o--|| pdf_documents : "pdf_id"
    page_layout_regions }o--|| document_pages : "page_id"
    pdf_documents {
        uuid pdf_id PK
        uuid workspace_id FK
        uuid document_id FK
        varchar filename
        varchar content_type
        bigint file_size_bytes
        varchar sha256_checksum
        int page_count
        bytea pdf_data
        varchar processing_status
        timestamp created_at
    }
    pdf_document_blobs {
        uuid pdf_id PK
        bytea pdf_data
        text markdown_content
        timestamp created_at
        timestamp updated_at
    }
    document_pages {
        uuid page_id PK
        uuid document_id FK
        uuid workspace_id FK
        int page_number
        float8 width_pt
        float8 height_pt
        smallint rotation
        jsonb cropbox_pdf
        int raster_width_px
        int raster_height_px
        text layout_model
        text layout_status
        timestamp created_at
        timestamp updated_at
    }
    document_page_states {
        uuid page_state_id PK
        uuid document_id FK
        uuid workspace_id FK
        int page_number
        text parse_status
        text parse_error
        int parse_attempts
        text parse_method
        text parse_model
        text raw_markdown
        timestamp created_at
    }
    page_layout_regions {
        uuid region_id PK
        uuid page_id FK
        uuid document_id FK
        uuid workspace_id FK
        text class
        text source
        jsonb bbox_pdf
        float4 confidence
        int reading_order
        text asset_path
        jsonb extra
        timestamp created_at
    }
    document_mm_assets {
        uuid document_id PK
        uuid workspace_id FK
        varchar asset_path PK
        varchar content_type
        bigint file_size_bytes
        bytea asset_data
        varchar asset_kind
        int page_num
        timestamp created_at
        varchar asset_id
    }
```

Read the boxes as tables and the arrows as foreign keys that point toward the parent. Tables in this section: `pdf_documents`, `pdf_document_blobs`, `document_pages`, `document_page_states`, `page_layout_regions`, `document_mm_assets`.

## Graph read models and lineage

Relational copies of the knowledge graph, plus the link tables that record which chunk produced which entity or relationship. The live graph itself lives in Apache AGE (see the AGE page).

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
erDiagram
    relationships }o--|| entities : "source_id"
    relationships }o--|| entities : "target_id"
    entities {
        uuid id PK
        uuid tenant_id FK
        uuid workspace_id FK
        text name
        text entity_type
        text description
        vector embedding
        uuid source_ids
        boolean is_manual
        timestamp manual_created_at
        timestamp created_at
    }
    relationships {
        uuid id PK
        uuid source_id FK
        uuid target_id FK
        uuid tenant_id FK
        uuid workspace_id FK
        text relation_type
        text description
        FLOAT weight
        text keywords
        uuid source_chunk_ids
        timestamp created_at
    }
    chunk_entity_links {
        text chunk_id PK
        text entity_name PK
        text workspace_id PK
        timestamp created_at
    }
    chunk_relation_links {
        text chunk_id PK
        text source_entity PK
        text target_entity PK
        text workspace_id PK
        timestamp created_at
    }
    graph_contributions {
        uuid tenant_id
        uuid workspace_id
        uuid fact_id
        bigint fact_revision
        uuid contribution_id PK
        uuid source_document_id
        bigint source_generation
        uuid source_chunk_id
        bytea payload_digest
        jsonb payload
    }
    graph_nodes {
        uuid id PK
        varchar graph_name
        text node_id
        varchar label
        jsonb properties
        uuid tenant_id
        uuid workspace_id
        timestamp created_at
        timestamp updated_at
    }
    graph_edges {
        uuid id PK
        varchar graph_name
        text source_node_id
        text target_node_id
        varchar label
        jsonb properties
        uuid tenant_id
        uuid workspace_id
        timestamp created_at
    }
```

Read the boxes as tables and the arrows as foreign keys that point toward the parent. Tables in this section: `entities`, `relationships`, `chunk_entity_links`, `chunk_relation_links`, `graph_contributions`, `graph_nodes`, `graph_edges`.

## Embeddings (pgvector)

Vectors for chunks, entities, relationships, and reports, keyed by embedding model. Manifests and projections track which model revision produced which vector.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
erDiagram
    chunk_embeddings }o--|| embedding_models : "model_id"
    entity_embeddings }o--|| embedding_models : "model_id"
    relationship_embeddings }o--|| embedding_models : "model_id"
    report_embeddings }o--|| embedding_models : "model_id"
    embedding_models {
        uuid id PK
        text name
        int dimensions
        text metric
        timestamp created_at
    }
    chunk_embeddings {
        uuid model_id PK
        uuid chunk_id PK
        uuid workspace_id FK
        halfvec embedding
        int dimensions
        timestamp created_at
    }
    entity_embeddings {
        uuid model_id PK
        uuid entity_id PK
        uuid workspace_id FK
        halfvec embedding
        int dimensions
        timestamp created_at
        text legacy_vector_id
    }
    relationship_embeddings {
        uuid model_id PK
        uuid relationship_id PK
        uuid workspace_id FK
        halfvec embedding
        int dimensions
        timestamp created_at
        text legacy_vector_id
    }
    report_embeddings {
        uuid model_id PK
        text report_id PK
        uuid workspace_id FK
        halfvec embedding
        int dimensions
        timestamp created_at
        text legacy_vector_id
    }
    embedding_manifests {
        uuid tenant_id PK
        uuid workspace_id PK
        uuid subject_id PK
        text family PK
        text model_revision PK
        bigint content_revision PK
        uuid physical_id
        int dimension
        text metric
        bytea digest
        text payload_ref
        bytea payload
    }
    embedding_projections {
        uuid tenant_id PK
        uuid workspace_id PK
        text family PK
        uuid subject_id PK
        text model_revision PK
        bigint content_revision PK
        uuid physical_id
        halfvec embedding
        int dimensions
        jsonb filter_payload
        bytea digest
    }
```

Read the boxes as tables and the arrows as foreign keys that point toward the parent. Tables in this section: `embedding_models`, `chunk_embeddings`, `entity_embeddings`, `relationship_embeddings`, `report_embeddings`, `embedding_manifests`, `embedding_projections`.

## Background tasks and jobs

Work the API queues for workers. `tasks` is partitioned by month; `task_events` and `attempts` are the audit trail.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
erDiagram
    tasks {
        uuid id PK
        uuid tenant_id FK
        uuid workspace_id FK
        varchar track_id PK
        varchar task_type
        varchar status
        int priority
        jsonb payload
        jsonb result
        text error_message
        timestamp created_at
    }
    task_events {
        BIGSERIAL id PK
        text task_id
        uuid job_id
        bigint seq
        text kind
        jsonb payload
        timestamp at
    }
    attempts {
        uuid id PK
        text task_track_id
        int attempt_no
        text worker_id
        uuid lease_token
        timestamp lease_expires_at
        timestamp started_at
        timestamp finished_at
        text outcome
        bigint fence_epoch
    }
    jobs {
        uuid id PK
        uuid tenant_id
        uuid workspace_id
        text operation
        text subject_kind
        text subject_id
        text idempotency_key
        text state
        timestamp created_at
    }
    ingest_batches {
        uuid tenant_id PK
        uuid workspace_id PK
        uuid document_id PK
        bigint generation PK
        int batch_ordinal PK
        bytea digest
        int expected_count
        text state
    }
```

Read the boxes as tables and the arrows as foreign keys that point toward the parent. Tables in this section: `tasks`, `task_events`, `attempts`, `jobs`, `ingest_batches`.

## Durable write path (SPEC-149)

Idempotent commits, object revisions, and projection events that feed downstream stores. Start at `mutation_requests` and follow the arrows to deliveries.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
erDiagram
    projection_event_items }o--|| projection_events : "event_id"
    projection_event_role_proofs }o--|| projection_events : "event_id"
    projection_deliveries }o--|| projection_events : "event_id"
    projection_deliveries }o--|| data_bindings : "binding_id"
    projection_visibility }o--|| data_bindings : "binding_id"
    projection_cleanup_intents }o--|| data_bindings : "binding_id"
    vector_provider_cutovers }o--|| data_bindings : "old_binding_id"
    vector_provider_cutovers }o--|| data_bindings : "new_binding_id"
    mutation_requests {
        uuid tenant_id PK
        uuid workspace_id PK
        text operation PK
        text idempotency_key PK
        bytea digest
        bytea receipt
        timestamp created_at
    }
    object_revisions {
        uuid tenant_id PK
        uuid workspace_id PK
        text kind PK
        uuid logical_id PK
        bigint revision PK
        text state
        uuid physical_id
        bytea digest
        text payload_ref
        bytea payload
        timestamp created_at
    }
    projection_events {
        uuid event_id PK
        uuid tenant_id
        uuid workspace_id
        text object_kind
        uuid object_id
        bigint object_revision
        int schema_version
        text operation
        text manifest_ref
        bytea digest
        timestamp created_at
    }
    projection_event_items {
        uuid event_id PK
        text role PK
        int ordinal PK
        text item_kind
        uuid record_id
        bigint record_revision
        bytea digest
        text logical_key
    }
    projection_event_role_proofs {
        uuid event_id PK
        text role PK
        bytea expected_digest
    }
    projection_deliveries {
        uuid event_id PK
        uuid binding_id PK
        text state
        timestamp next_attempt_at
        timestamp lease_until
        uuid lease_owner
        bigint epoch
        int attempts
        bytea receipt
        text provider_receipt
    }
    projection_visibility {
        uuid tenant_id PK
        uuid workspace_id PK
        text object_kind PK
        uuid object_id PK
        bigint object_revision PK
        uuid binding_id PK
        bytea completion_receipt
        bigint verified_generation
    }
    projection_cleanup_intents {
        uuid cleanup_manifest_id PK
        uuid binding_id PK
        uuid tenant_id
        uuid workspace_id
        uuid document_id
        bigint tombstone_revision
        text state
        timestamp created_at
    }
    outbox_events {
        uuid id PK
        text aggregate_type
        uuid aggregate_id
        text event_type
        jsonb payload
        timestamp created_at
        timestamp processed_at
        uuid workspace_id
        timestamp available_at
        int attempt_count
    }
    data_bindings {
        uuid binding_id PK
        uuid tenant_id
        uuid workspace_id
        text role
        text provider
        text config_ref
        text layout
        text physical_index
        text model_descriptor
        bigint generation
        text state
        timestamp created_at
    }
    vector_provider_cutovers {
        uuid cutover_id PK
        uuid tenant_id
        uuid workspace_id
        uuid old_binding_id FK
        uuid new_binding_id FK
        jsonb backfill_cursor
        text state
        timestamp created_at
        timestamp updated_at
    }
    compensation_quarantine {
        uuid entry_id PK
        uuid document_id FK
        uuid workspace_id FK
        text status
        timestamp next_attempt_at
        int attempt_count
        jsonb payload
        jsonb last_error
        timestamp created_at
        timestamp updated_at
    }
```

Read the boxes as tables and the arrows as foreign keys that point toward the parent. Tables in this section: `mutation_requests`, `object_revisions`, `projection_events`, `projection_event_items`, `projection_event_role_proofs`, `projection_deliveries`, `projection_visibility`, `projection_cleanup_intents`, `outbox_events`, `data_bindings`, `vector_provider_cutovers`, `compensation_quarantine`.

## Conversations

Chat history for the query UI. A conversation holds messages; folders group conversations.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
erDiagram
    conversations }o--|| folders : "folder_id"
    messages }o--|| conversations : "conversation_id"
    messages }o--|| messages : "parent_id"
    folders }o--|| folders : "parent_id"
    conversations {
        uuid conversation_id PK
        uuid tenant_id FK
        uuid workspace_id FK
        uuid user_id FK
        varchar title
        varchar mode
        boolean is_pinned
        boolean is_archived
        uuid folder_id FK
        varchar share_id
        jsonb meta
        timestamp created_at
        timestamp updated_at
    }
    messages {
        uuid message_id PK
        uuid conversation_id FK
        uuid parent_id FK
        varchar role
        text content
        varchar mode
        int tokens_used
        int duration_ms
        int thinking_time_ms
        jsonb context
        timestamp created_at
    }
    conversation_history {
        uuid id PK
        uuid conversation_id
        int message_index
        varchar role
        text content
        jsonb metadata
        uuid tenant_id
        uuid workspace_id
        timestamp created_at
    }
    folders {
        uuid folder_id PK
        uuid tenant_id FK
        uuid workspace_id FK
        uuid user_id FK
        varchar name
        uuid parent_id FK
        int position
        timestamp created_at
        timestamp updated_at
    }
```

Read the boxes as tables and the arrows as foreign keys that point toward the parent. Tables in this section: `conversations`, `messages`, `conversation_history`, `folders`.

## Caches, providers, and operations

Recomputable results, encrypted provider connections (SPEC-163), audit logs, and migration bookkeeping.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
erDiagram
    llm_cache {
        text cache_key PK
        text namespace PK
        jsonb value
        timestamp created_at
        timestamp updated_at
        timestamp expires_at
    }
    decision_cache {
        uuid workspace_id PK
        text key_hash PK
        uuid tenant_id
        text contract
        text model
        jsonb answer
        timestamp created_at
        timestamp last_used_at
    }
    decision_review {
        uuid review_id PK
        uuid workspace_id
        uuid document_id
        uuid tenant_id
        text chunk_id
        text kind
        text subject
        text label
        text object
        float4 score
        timestamp created_at
    }
    provider_connections {
        uuid id PK
        uuid tenant_id
        text slug
        text display_name
        text api_shape
        text locality
        text base_url
        text auth_scheme
        bytea api_key_ciphertext
        bytea api_key_nonce
        timestamp created_at
    }
    server_config {
        text key PK
        jsonb value
        timestamp updated_at
    }
    audit_logs {
        uuid id PK
        timestamp timestamp PK
        varchar tenant_id
        varchar workspace_id
        varchar user_id
        audit_event_type event_type
        varchar event_category
        varchar event_action
        varchar resource_type
        varchar resource_id
    }
    rls_audit_log {
        BIGSERIAL id PK
        timestamp event_time
        uuid tenant_id
        uuid workspace_id
        uuid user_id
        varchar action
        varchar table_name
        text record_id
        jsonb details
    }
    workspace_metrics_history {
        uuid id PK
        uuid workspace_id FK
        timestamp recorded_at
        text trigger_type
        bigint document_count
        bigint chunk_count
        bigint entity_count
        bigint relationship_count
        bigint embedding_count
        bigint storage_bytes
    }
    edgequake_schema_generation {
        text relation_name PK
        int generation
        timestamp retired_at
        text notes
        timestamp updated_at
    }
    edgequake_reconcile_state {
        text support_version PK
        text apply_sha384
        timestamp applied_at
        bigint duration_ms
        text outcome
    }
    eq_hot_ann_workspaces {
        text table_prefix PK
        text workspace_id PK
        timestamp created_at
    }
```

Read the boxes as tables and the arrows as foreign keys that point toward the parent. Tables in this section: `llm_cache`, `decision_cache`, `decision_review`, `provider_connections`, `server_config`, `audit_logs`, `rls_audit_log`, `workspace_metrics_history`, `edgequake_schema_generation`, `edgequake_reconcile_state`, `eq_hot_ann_workspaces`.

## What these diagrams leave out

- **Apache AGE graph.** Entities and relationships also exist as nodes and edges in the AGE graph (`ag_catalog`). Isolation there uses `tenant_id` / `workspace_id` properties, not foreign keys. See [age.md](./age.md).
- **Per-workspace vector tables.** Hot workspaces can get dedicated ANN tables (`eq_hot_ann_workspaces`). Those tables are created at runtime and are not listed above.
- **The `edgequake` schema.** Migration bookkeeping (`schema_compat`, `migration_run`, …) lives in a separate schema and is covered in [Upgrading](../operations/upgrading.md).
- **Views.** Five reporting views (`rate_limit_violations`, `recent_security_events`, `tenant_activity_summary`, `tenant_document_stats`, `tenant_entity_stats`) are not drawn.

## Regenerating

When you add a migration, update this page so the diagrams stay honest. The column lists and foreign keys should match the SQL in `edgequake/migrations/`. Prefer several domain diagrams over one giant diagram: Mermaid stays readable with about a dozen tables per figure.
