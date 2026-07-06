-- public.audit_logs definition

-- Drop table

-- DROP TABLE audit_logs;

CREATE TABLE audit_logs (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	"timestamp" timestamptz DEFAULT now() NOT NULL,
	tenant_id varchar(255) NOT NULL,
	workspace_id varchar(255) NULL,
	user_id varchar(255) NULL,
	event_type public."audit_event_type" NOT NULL,
	event_category varchar(100) NOT NULL,
	event_action varchar(255) NOT NULL,
	resource_type varchar(100) NULL,
	resource_id varchar(255) NULL,
	"result" public."audit_result" NOT NULL,
	severity public."audit_severity" DEFAULT 'Medium'::audit_severity NOT NULL,
	ip_address inet NULL,
	user_agent text NULL,
	request_id varchar(100) NULL,
	session_id varchar(100) NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	error_message text NULL,
	retention_days int4 DEFAULT 90 NULL,
	archived bool DEFAULT false NULL,
	duration_ms int4 NULL,
	CONSTRAINT audit_logs_pkey PRIMARY KEY (id, "timestamp"),
	CONSTRAINT audit_logs_tenant_not_null CHECK ((tenant_id IS NOT NULL))
)
PARTITION BY RANGE ("timestamp");
CREATE INDEX idx_audit_logs_metadata_gin ON ONLY public.audit_logs USING gin (metadata);
CREATE INDEX idx_audit_logs_request_id ON ONLY public.audit_logs USING btree (request_id) WHERE (request_id IS NOT NULL);
CREATE INDEX idx_audit_logs_resource ON ONLY public.audit_logs USING btree (resource_type, resource_id, "timestamp" DESC) WHERE (resource_id IS NOT NULL);
CREATE INDEX idx_audit_logs_security ON ONLY public.audit_logs USING btree (event_type, result, "timestamp" DESC) WHERE ((result = ANY (ARRAY['Failure'::audit_result, 'Blocked'::audit_result])) OR (severity = ANY (ARRAY['High'::audit_severity, 'Critical'::audit_severity])));
CREATE INDEX idx_audit_logs_tenant_timestamp ON ONLY public.audit_logs USING btree (tenant_id, "timestamp" DESC);
CREATE INDEX idx_audit_logs_tenant_timestamp_perf ON ONLY public.audit_logs USING btree (tenant_id, "timestamp" DESC) WHERE (tenant_id IS NOT NULL);
CREATE INDEX idx_audit_logs_user_activity ON ONLY public.audit_logs USING btree (user_id, "timestamp" DESC) WHERE (user_id IS NOT NULL);
CREATE INDEX idx_audit_logs_workspace ON ONLY public.audit_logs USING btree (workspace_id, "timestamp" DESC) WHERE (workspace_id IS NOT NULL);


-- public.chunk_entity_links definition

-- Drop table

-- DROP TABLE chunk_entity_links;

CREATE TABLE chunk_entity_links (
	chunk_id text NOT NULL,
	entity_name text NOT NULL,
	workspace_id text NOT NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT chunk_entity_links_pkey PRIMARY KEY (chunk_id, entity_name, workspace_id)
);
CREATE INDEX idx_cel_chunk_id ON public.chunk_entity_links USING btree (chunk_id);
CREATE INDEX idx_cel_entity_workspace ON public.chunk_entity_links USING btree (entity_name, workspace_id);
CREATE INDEX idx_cel_workspace ON public.chunk_entity_links USING btree (workspace_id);


-- public.chunk_relation_links definition

-- Drop table

-- DROP TABLE chunk_relation_links;

CREATE TABLE chunk_relation_links (
	chunk_id text NOT NULL,
	source_entity text NOT NULL,
	target_entity text NOT NULL,
	workspace_id text NOT NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT chunk_relation_links_pkey PRIMARY KEY (chunk_id, source_entity, target_entity, workspace_id)
);
CREATE INDEX idx_crl_chunk_id ON public.chunk_relation_links USING btree (chunk_id);
CREATE INDEX idx_crl_source_target_workspace ON public.chunk_relation_links USING btree (source_entity, target_entity, workspace_id);
CREATE INDEX idx_crl_source_workspace ON public.chunk_relation_links USING btree (source_entity, workspace_id);
CREATE INDEX idx_crl_target_workspace ON public.chunk_relation_links USING btree (target_entity, workspace_id);


-- public.conversation_history definition

-- Drop table

-- DROP TABLE conversation_history;

CREATE TABLE conversation_history (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	conversation_id uuid NOT NULL,
	message_index int4 NOT NULL,
	"role" varchar(20) NOT NULL,
	"content" text NOT NULL,
	metadata jsonb NULL,
	tenant_id uuid NULL,
	workspace_id uuid NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT conversation_history_pkey PRIMARY KEY (id),
	CONSTRAINT unique_conversation_message UNIQUE (conversation_id, message_index),
	CONSTRAINT valid_role CHECK (((role)::text = ANY ((ARRAY['user'::character varying, 'assistant'::character varying, 'system'::character varying])::text[])))
);
CREATE INDEX idx_conversation_history_conversation_id ON public.conversation_history USING btree (conversation_id, message_index);
CREATE INDEX idx_conversation_history_created ON public.conversation_history USING btree (created_at DESC);
CREATE INDEX idx_conversation_history_tenant_workspace ON public.conversation_history USING btree (tenant_id, workspace_id);


-- public.eq_eq_default_kv definition

-- Drop table

-- DROP TABLE eq_eq_default_kv;

CREATE TABLE eq_eq_default_kv (
	"key" text NOT NULL,
	value jsonb NOT NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT eq_eq_default_kv_pkey PRIMARY KEY (key)
);
CREATE INDEX eq_eq_default_kv_reverse_key_idx ON public.eq_eq_default_kv USING btree (reverse(key) text_pattern_ops);
CREATE UNIQUE INDEX idx_kv_key ON public.eq_eq_default_kv USING btree (key);
CREATE INDEX idx_kv_updated_at ON public.eq_eq_default_kv USING btree (updated_at);

-- Table Triggers

create trigger eq_eq_default_kv_stats_insert_trg after
insert
    on
    public.eq_eq_default_kv for each row execute function eq_eq_default_kv_stats_insert();
create trigger eq_eq_default_kv_stats_delete_trg after
delete
    on
    public.eq_eq_default_kv for each row execute function eq_eq_default_kv_stats_delete();


-- public.eq_eq_default_kv_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_kv_stats;

CREATE TABLE eq_eq_default_kv_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_kv_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_kv_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_vectors;

CREATE TABLE eq_eq_default_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_vectors_content_tsv_idx ON public.eq_eq_default_vectors USING gin (content_tsv);
CREATE INDEX eq_eq_default_vectors_doc_id_idx ON public.eq_eq_default_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_vectors_embedding_idx ON public.eq_eq_default_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_vectors_tenant_ws_idx ON public.eq_eq_default_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_vectors for each row execute function eq_eq_default_vectors_stats_insert();
create trigger eq_eq_default_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_vectors for each row execute function eq_eq_default_vectors_stats_delete();


-- public.eq_eq_default_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_vectors_stats;

CREATE TABLE eq_eq_default_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_2acd35f8_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_2acd35f8_vectors;

CREATE TABLE eq_eq_default_ws_2acd35f8_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_2acd35f8_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_2acd35f8_vectors_doc_id_idx ON public.eq_eq_default_ws_2acd35f8_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_2acd35f8_vectors_embedding_idx ON public.eq_eq_default_ws_2acd35f8_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_2acd35f8_vectors_tenant_ws_idx ON public.eq_eq_default_ws_2acd35f8_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_2acd35f8_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_2acd35f8_vectors for each row execute function eq_eq_default_ws_2acd35f8_vectors_stats_insert();
create trigger eq_eq_default_ws_2acd35f8_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_2acd35f8_vectors for each row execute function eq_eq_default_ws_2acd35f8_vectors_stats_delete();


-- public.eq_eq_default_ws_2acd35f8_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_2acd35f8_vectors_stats;

CREATE TABLE eq_eq_default_ws_2acd35f8_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_2acd35f8_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_2acd35f8_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_2c6cc26e_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_2c6cc26e_vectors;

CREATE TABLE eq_eq_default_ws_2c6cc26e_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_2c6cc26e_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_2c6cc26e_vectors_doc_id_idx ON public.eq_eq_default_ws_2c6cc26e_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_2c6cc26e_vectors_embedding_idx ON public.eq_eq_default_ws_2c6cc26e_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_2c6cc26e_vectors_tenant_ws_idx ON public.eq_eq_default_ws_2c6cc26e_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_2c6cc26e_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_2c6cc26e_vectors for each row execute function eq_eq_default_ws_2c6cc26e_vectors_stats_insert();
create trigger eq_eq_default_ws_2c6cc26e_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_2c6cc26e_vectors for each row execute function eq_eq_default_ws_2c6cc26e_vectors_stats_delete();


-- public.eq_eq_default_ws_2c6cc26e_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_2c6cc26e_vectors_stats;

CREATE TABLE eq_eq_default_ws_2c6cc26e_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_2c6cc26e_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_2c6cc26e_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_6a7e449d_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_6a7e449d_vectors;

CREATE TABLE eq_eq_default_ws_6a7e449d_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_6a7e449d_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_6a7e449d_vectors_doc_id_idx ON public.eq_eq_default_ws_6a7e449d_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_6a7e449d_vectors_embedding_idx ON public.eq_eq_default_ws_6a7e449d_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_6a7e449d_vectors_tenant_ws_idx ON public.eq_eq_default_ws_6a7e449d_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_6a7e449d_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_6a7e449d_vectors for each row execute function eq_eq_default_ws_6a7e449d_vectors_stats_insert();
create trigger eq_eq_default_ws_6a7e449d_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_6a7e449d_vectors for each row execute function eq_eq_default_ws_6a7e449d_vectors_stats_delete();


-- public.eq_eq_default_ws_6a7e449d_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_6a7e449d_vectors_stats;

CREATE TABLE eq_eq_default_ws_6a7e449d_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_6a7e449d_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_6a7e449d_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_6b797029_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_6b797029_vectors;

CREATE TABLE eq_eq_default_ws_6b797029_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_6b797029_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_6b797029_vectors_doc_id_idx ON public.eq_eq_default_ws_6b797029_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_6b797029_vectors_embedding_idx ON public.eq_eq_default_ws_6b797029_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_6b797029_vectors_tenant_ws_idx ON public.eq_eq_default_ws_6b797029_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_6b797029_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_6b797029_vectors for each row execute function eq_eq_default_ws_6b797029_vectors_stats_insert();
create trigger eq_eq_default_ws_6b797029_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_6b797029_vectors for each row execute function eq_eq_default_ws_6b797029_vectors_stats_delete();


-- public.eq_eq_default_ws_6b797029_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_6b797029_vectors_stats;

CREATE TABLE eq_eq_default_ws_6b797029_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_6b797029_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_6b797029_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_07a5fb15_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_07a5fb15_vectors;

CREATE TABLE eq_eq_default_ws_07a5fb15_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_07a5fb15_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_07a5fb15_vectors_doc_id_idx ON public.eq_eq_default_ws_07a5fb15_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_07a5fb15_vectors_embedding_idx ON public.eq_eq_default_ws_07a5fb15_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_07a5fb15_vectors_tenant_ws_idx ON public.eq_eq_default_ws_07a5fb15_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_07a5fb15_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_07a5fb15_vectors for each row execute function eq_eq_default_ws_07a5fb15_vectors_stats_insert();
create trigger eq_eq_default_ws_07a5fb15_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_07a5fb15_vectors for each row execute function eq_eq_default_ws_07a5fb15_vectors_stats_delete();


-- public.eq_eq_default_ws_07a5fb15_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_07a5fb15_vectors_stats;

CREATE TABLE eq_eq_default_ws_07a5fb15_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_07a5fb15_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_07a5fb15_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_7c3ff8fc_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_7c3ff8fc_vectors;

CREATE TABLE eq_eq_default_ws_7c3ff8fc_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_7c3ff8fc_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_7c3ff8fc_vectors_content_tsv_idx ON public.eq_eq_default_ws_7c3ff8fc_vectors USING gin (content_tsv);
CREATE INDEX eq_eq_default_ws_7c3ff8fc_vectors_doc_id_idx ON public.eq_eq_default_ws_7c3ff8fc_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_7c3ff8fc_vectors_embedding_idx ON public.eq_eq_default_ws_7c3ff8fc_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_7c3ff8fc_vectors_tenant_ws_idx ON public.eq_eq_default_ws_7c3ff8fc_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_7c3ff8fc_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_7c3ff8fc_vectors for each row execute function eq_eq_default_ws_7c3ff8fc_vectors_stats_insert();
create trigger eq_eq_default_ws_7c3ff8fc_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_7c3ff8fc_vectors for each row execute function eq_eq_default_ws_7c3ff8fc_vectors_stats_delete();


-- public.eq_eq_default_ws_7c3ff8fc_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_7c3ff8fc_vectors_stats;

CREATE TABLE eq_eq_default_ws_7c3ff8fc_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_7c3ff8fc_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_7c3ff8fc_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_7f31c3af_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_7f31c3af_vectors;

CREATE TABLE eq_eq_default_ws_7f31c3af_vectors (
	id text NOT NULL,
	embedding public.halfvec NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_7f31c3af_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_7f31c3af_vectors_doc_id_idx ON public.eq_eq_default_ws_7f31c3af_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_7f31c3af_vectors_embedding_idx ON public.eq_eq_default_ws_7f31c3af_vectors USING hnsw (embedding halfvec_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_7f31c3af_vectors_tenant_ws_idx ON public.eq_eq_default_ws_7f31c3af_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_7f31c3af_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_7f31c3af_vectors for each row execute function eq_eq_default_ws_7f31c3af_vectors_stats_insert();
create trigger eq_eq_default_ws_7f31c3af_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_7f31c3af_vectors for each row execute function eq_eq_default_ws_7f31c3af_vectors_stats_delete();


-- public.eq_eq_default_ws_7f31c3af_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_7f31c3af_vectors_stats;

CREATE TABLE eq_eq_default_ws_7f31c3af_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_7f31c3af_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_7f31c3af_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_8c84b4d9_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_8c84b4d9_vectors;

CREATE TABLE eq_eq_default_ws_8c84b4d9_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_8c84b4d9_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_8c84b4d9_vectors_doc_id_idx ON public.eq_eq_default_ws_8c84b4d9_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_8c84b4d9_vectors_embedding_idx ON public.eq_eq_default_ws_8c84b4d9_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_8c84b4d9_vectors_tenant_ws_idx ON public.eq_eq_default_ws_8c84b4d9_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_8c84b4d9_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_8c84b4d9_vectors for each row execute function ag_catalog.eq_eq_default_ws_8c84b4d9_vectors_stats_insert();
create trigger eq_eq_default_ws_8c84b4d9_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_8c84b4d9_vectors for each row execute function ag_catalog.eq_eq_default_ws_8c84b4d9_vectors_stats_delete();


-- public.eq_eq_default_ws_8c84b4d9_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_8c84b4d9_vectors_stats;

CREATE TABLE eq_eq_default_ws_8c84b4d9_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_8c84b4d9_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_8c84b4d9_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_8c80935f_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_8c80935f_vectors;

CREATE TABLE eq_eq_default_ws_8c80935f_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_8c80935f_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_8c80935f_vectors_doc_id_idx ON public.eq_eq_default_ws_8c80935f_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_8c80935f_vectors_embedding_idx ON public.eq_eq_default_ws_8c80935f_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_8c80935f_vectors_tenant_ws_idx ON public.eq_eq_default_ws_8c80935f_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_8c80935f_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_8c80935f_vectors for each row execute function eq_eq_default_ws_8c80935f_vectors_stats_insert();
create trigger eq_eq_default_ws_8c80935f_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_8c80935f_vectors for each row execute function eq_eq_default_ws_8c80935f_vectors_stats_delete();


-- public.eq_eq_default_ws_8c80935f_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_8c80935f_vectors_stats;

CREATE TABLE eq_eq_default_ws_8c80935f_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_8c80935f_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_8c80935f_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_9eff2ef8_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_9eff2ef8_vectors;

CREATE TABLE eq_eq_default_ws_9eff2ef8_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_9eff2ef8_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_9eff2ef8_vectors_doc_id_idx ON public.eq_eq_default_ws_9eff2ef8_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_9eff2ef8_vectors_embedding_idx ON public.eq_eq_default_ws_9eff2ef8_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_9eff2ef8_vectors_tenant_ws_idx ON public.eq_eq_default_ws_9eff2ef8_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_9eff2ef8_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_9eff2ef8_vectors for each row execute function eq_eq_default_ws_9eff2ef8_vectors_stats_insert();
create trigger eq_eq_default_ws_9eff2ef8_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_9eff2ef8_vectors for each row execute function eq_eq_default_ws_9eff2ef8_vectors_stats_delete();


-- public.eq_eq_default_ws_9eff2ef8_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_9eff2ef8_vectors_stats;

CREATE TABLE eq_eq_default_ws_9eff2ef8_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_9eff2ef8_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_9eff2ef8_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_9fd97e1c_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_9fd97e1c_vectors;

CREATE TABLE eq_eq_default_ws_9fd97e1c_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_9fd97e1c_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_9fd97e1c_vectors_doc_id_idx ON public.eq_eq_default_ws_9fd97e1c_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_9fd97e1c_vectors_embedding_idx ON public.eq_eq_default_ws_9fd97e1c_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_9fd97e1c_vectors_tenant_ws_idx ON public.eq_eq_default_ws_9fd97e1c_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_9fd97e1c_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_9fd97e1c_vectors for each row execute function eq_eq_default_ws_9fd97e1c_vectors_stats_insert();
create trigger eq_eq_default_ws_9fd97e1c_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_9fd97e1c_vectors for each row execute function eq_eq_default_ws_9fd97e1c_vectors_stats_delete();


-- public.eq_eq_default_ws_9fd97e1c_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_9fd97e1c_vectors_stats;

CREATE TABLE eq_eq_default_ws_9fd97e1c_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_9fd97e1c_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_9fd97e1c_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_70c391bc_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_70c391bc_vectors;

CREATE TABLE eq_eq_default_ws_70c391bc_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_70c391bc_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_70c391bc_vectors_doc_id_idx ON public.eq_eq_default_ws_70c391bc_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_70c391bc_vectors_embedding_idx ON public.eq_eq_default_ws_70c391bc_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_70c391bc_vectors_tenant_ws_idx ON public.eq_eq_default_ws_70c391bc_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_70c391bc_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_70c391bc_vectors for each row execute function ag_catalog.eq_eq_default_ws_70c391bc_vectors_stats_insert();
create trigger eq_eq_default_ws_70c391bc_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_70c391bc_vectors for each row execute function ag_catalog.eq_eq_default_ws_70c391bc_vectors_stats_delete();


-- public.eq_eq_default_ws_70c391bc_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_70c391bc_vectors_stats;

CREATE TABLE eq_eq_default_ws_70c391bc_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_70c391bc_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_70c391bc_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_74b0edce_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_74b0edce_vectors;

CREATE TABLE eq_eq_default_ws_74b0edce_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_74b0edce_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_74b0edce_vectors_doc_id_idx ON public.eq_eq_default_ws_74b0edce_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_74b0edce_vectors_embedding_idx ON public.eq_eq_default_ws_74b0edce_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_74b0edce_vectors_tenant_ws_idx ON public.eq_eq_default_ws_74b0edce_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_74b0edce_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_74b0edce_vectors for each row execute function eq_eq_default_ws_74b0edce_vectors_stats_insert();
create trigger eq_eq_default_ws_74b0edce_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_74b0edce_vectors for each row execute function eq_eq_default_ws_74b0edce_vectors_stats_delete();


-- public.eq_eq_default_ws_74b0edce_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_74b0edce_vectors_stats;

CREATE TABLE eq_eq_default_ws_74b0edce_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_74b0edce_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_74b0edce_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_93d61f37_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_93d61f37_vectors;

CREATE TABLE eq_eq_default_ws_93d61f37_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_93d61f37_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_93d61f37_vectors_doc_id_idx ON public.eq_eq_default_ws_93d61f37_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_93d61f37_vectors_embedding_idx ON public.eq_eq_default_ws_93d61f37_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_93d61f37_vectors_tenant_ws_idx ON public.eq_eq_default_ws_93d61f37_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_93d61f37_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_93d61f37_vectors for each row execute function eq_eq_default_ws_93d61f37_vectors_stats_insert();
create trigger eq_eq_default_ws_93d61f37_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_93d61f37_vectors for each row execute function eq_eq_default_ws_93d61f37_vectors_stats_delete();


-- public.eq_eq_default_ws_93d61f37_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_93d61f37_vectors_stats;

CREATE TABLE eq_eq_default_ws_93d61f37_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_93d61f37_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_93d61f37_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_0175a534_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_0175a534_vectors;

CREATE TABLE eq_eq_default_ws_0175a534_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_0175a534_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_0175a534_vectors_doc_id_idx ON public.eq_eq_default_ws_0175a534_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_0175a534_vectors_embedding_idx ON public.eq_eq_default_ws_0175a534_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_0175a534_vectors_tenant_ws_idx ON public.eq_eq_default_ws_0175a534_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_0175a534_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_0175a534_vectors for each row execute function eq_eq_default_ws_0175a534_vectors_stats_insert();
create trigger eq_eq_default_ws_0175a534_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_0175a534_vectors for each row execute function eq_eq_default_ws_0175a534_vectors_stats_delete();


-- public.eq_eq_default_ws_0175a534_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_0175a534_vectors_stats;

CREATE TABLE eq_eq_default_ws_0175a534_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_0175a534_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_0175a534_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_231c4736_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_231c4736_vectors;

CREATE TABLE eq_eq_default_ws_231c4736_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_231c4736_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_231c4736_vectors_doc_id_idx ON public.eq_eq_default_ws_231c4736_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_231c4736_vectors_embedding_idx ON public.eq_eq_default_ws_231c4736_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_231c4736_vectors_tenant_ws_idx ON public.eq_eq_default_ws_231c4736_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_231c4736_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_231c4736_vectors for each row execute function eq_eq_default_ws_231c4736_vectors_stats_insert();
create trigger eq_eq_default_ws_231c4736_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_231c4736_vectors for each row execute function eq_eq_default_ws_231c4736_vectors_stats_delete();


-- public.eq_eq_default_ws_231c4736_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_231c4736_vectors_stats;

CREATE TABLE eq_eq_default_ws_231c4736_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_231c4736_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_231c4736_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_1186d051_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_1186d051_vectors;

CREATE TABLE eq_eq_default_ws_1186d051_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_1186d051_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_1186d051_vectors_doc_id_idx ON public.eq_eq_default_ws_1186d051_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_1186d051_vectors_embedding_idx ON public.eq_eq_default_ws_1186d051_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_1186d051_vectors_tenant_ws_idx ON public.eq_eq_default_ws_1186d051_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_1186d051_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_1186d051_vectors for each row execute function eq_eq_default_ws_1186d051_vectors_stats_insert();
create trigger eq_eq_default_ws_1186d051_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_1186d051_vectors for each row execute function eq_eq_default_ws_1186d051_vectors_stats_delete();


-- public.eq_eq_default_ws_1186d051_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_1186d051_vectors_stats;

CREATE TABLE eq_eq_default_ws_1186d051_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_1186d051_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_1186d051_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_6485f929_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_6485f929_vectors;

CREATE TABLE eq_eq_default_ws_6485f929_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_6485f929_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_6485f929_vectors_content_tsv_idx ON public.eq_eq_default_ws_6485f929_vectors USING gin (content_tsv);
CREATE INDEX eq_eq_default_ws_6485f929_vectors_doc_id_idx ON public.eq_eq_default_ws_6485f929_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_6485f929_vectors_embedding_idx ON public.eq_eq_default_ws_6485f929_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_6485f929_vectors_tenant_ws_idx ON public.eq_eq_default_ws_6485f929_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_6485f929_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_6485f929_vectors for each row execute function eq_eq_default_ws_6485f929_vectors_stats_delete();
create trigger eq_eq_default_ws_6485f929_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_6485f929_vectors for each row execute function eq_eq_default_ws_6485f929_vectors_stats_insert();


-- public.eq_eq_default_ws_6485f929_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_6485f929_vectors_stats;

CREATE TABLE eq_eq_default_ws_6485f929_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_6485f929_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_6485f929_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_16691ae0_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_16691ae0_vectors;

CREATE TABLE eq_eq_default_ws_16691ae0_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_16691ae0_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_16691ae0_vectors_content_tsv_idx ON public.eq_eq_default_ws_16691ae0_vectors USING gin (content_tsv);
CREATE INDEX eq_eq_default_ws_16691ae0_vectors_doc_id_idx ON public.eq_eq_default_ws_16691ae0_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_16691ae0_vectors_embedding_idx ON public.eq_eq_default_ws_16691ae0_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_16691ae0_vectors_tenant_ws_idx ON public.eq_eq_default_ws_16691ae0_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_16691ae0_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_16691ae0_vectors for each row execute function eq_eq_default_ws_16691ae0_vectors_stats_delete();
create trigger eq_eq_default_ws_16691ae0_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_16691ae0_vectors for each row execute function eq_eq_default_ws_16691ae0_vectors_stats_insert();


-- public.eq_eq_default_ws_16691ae0_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_16691ae0_vectors_stats;

CREATE TABLE eq_eq_default_ws_16691ae0_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_16691ae0_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_16691ae0_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_79185e65_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_79185e65_vectors;

CREATE TABLE eq_eq_default_ws_79185e65_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_79185e65_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_79185e65_vectors_doc_id_idx ON public.eq_eq_default_ws_79185e65_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_79185e65_vectors_embedding_idx ON public.eq_eq_default_ws_79185e65_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_79185e65_vectors_tenant_ws_idx ON public.eq_eq_default_ws_79185e65_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);


-- public.eq_eq_default_ws_93689c13_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_93689c13_vectors;

CREATE TABLE eq_eq_default_ws_93689c13_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_93689c13_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_93689c13_vectors_doc_id_idx ON public.eq_eq_default_ws_93689c13_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_93689c13_vectors_embedding_idx ON public.eq_eq_default_ws_93689c13_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_93689c13_vectors_tenant_ws_idx ON public.eq_eq_default_ws_93689c13_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_93689c13_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_93689c13_vectors for each row execute function eq_eq_default_ws_93689c13_vectors_stats_insert();
create trigger eq_eq_default_ws_93689c13_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_93689c13_vectors for each row execute function eq_eq_default_ws_93689c13_vectors_stats_delete();


-- public.eq_eq_default_ws_93689c13_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_93689c13_vectors_stats;

CREATE TABLE eq_eq_default_ws_93689c13_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_93689c13_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_93689c13_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_96521c24_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_96521c24_vectors;

CREATE TABLE eq_eq_default_ws_96521c24_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_96521c24_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_96521c24_vectors_doc_id_idx ON public.eq_eq_default_ws_96521c24_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_96521c24_vectors_embedding_idx ON public.eq_eq_default_ws_96521c24_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_96521c24_vectors_tenant_ws_idx ON public.eq_eq_default_ws_96521c24_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_96521c24_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_96521c24_vectors for each row execute function eq_eq_default_ws_96521c24_vectors_stats_insert();
create trigger eq_eq_default_ws_96521c24_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_96521c24_vectors for each row execute function eq_eq_default_ws_96521c24_vectors_stats_delete();


-- public.eq_eq_default_ws_96521c24_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_96521c24_vectors_stats;

CREATE TABLE eq_eq_default_ws_96521c24_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_96521c24_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_96521c24_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_430581b4_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_430581b4_vectors;

CREATE TABLE eq_eq_default_ws_430581b4_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_430581b4_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_430581b4_vectors_doc_id_idx ON public.eq_eq_default_ws_430581b4_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_430581b4_vectors_embedding_idx ON public.eq_eq_default_ws_430581b4_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_430581b4_vectors_tenant_ws_idx ON public.eq_eq_default_ws_430581b4_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_430581b4_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_430581b4_vectors for each row execute function eq_eq_default_ws_430581b4_vectors_stats_insert();
create trigger eq_eq_default_ws_430581b4_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_430581b4_vectors for each row execute function eq_eq_default_ws_430581b4_vectors_stats_delete();


-- public.eq_eq_default_ws_430581b4_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_430581b4_vectors_stats;

CREATE TABLE eq_eq_default_ws_430581b4_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_430581b4_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_430581b4_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_713434d4_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_713434d4_vectors;

CREATE TABLE eq_eq_default_ws_713434d4_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_713434d4_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_713434d4_vectors_doc_id_idx ON public.eq_eq_default_ws_713434d4_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_713434d4_vectors_embedding_idx ON public.eq_eq_default_ws_713434d4_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_713434d4_vectors_tenant_ws_idx ON public.eq_eq_default_ws_713434d4_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_713434d4_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_713434d4_vectors for each row execute function eq_eq_default_ws_713434d4_vectors_stats_insert();
create trigger eq_eq_default_ws_713434d4_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_713434d4_vectors for each row execute function eq_eq_default_ws_713434d4_vectors_stats_delete();


-- public.eq_eq_default_ws_713434d4_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_713434d4_vectors_stats;

CREATE TABLE eq_eq_default_ws_713434d4_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_713434d4_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_713434d4_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_1386071c_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_1386071c_vectors;

CREATE TABLE eq_eq_default_ws_1386071c_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_1386071c_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_1386071c_vectors_doc_id_idx ON public.eq_eq_default_ws_1386071c_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_1386071c_vectors_embedding_idx ON public.eq_eq_default_ws_1386071c_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_1386071c_vectors_tenant_ws_idx ON public.eq_eq_default_ws_1386071c_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_1386071c_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_1386071c_vectors for each row execute function eq_eq_default_ws_1386071c_vectors_stats_insert();
create trigger eq_eq_default_ws_1386071c_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_1386071c_vectors for each row execute function eq_eq_default_ws_1386071c_vectors_stats_delete();


-- public.eq_eq_default_ws_1386071c_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_1386071c_vectors_stats;

CREATE TABLE eq_eq_default_ws_1386071c_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_1386071c_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_1386071c_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_7119710e_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_7119710e_vectors;

CREATE TABLE eq_eq_default_ws_7119710e_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_7119710e_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_7119710e_vectors_doc_id_idx ON public.eq_eq_default_ws_7119710e_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_7119710e_vectors_embedding_idx ON public.eq_eq_default_ws_7119710e_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_7119710e_vectors_tenant_ws_idx ON public.eq_eq_default_ws_7119710e_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);


-- public.eq_eq_default_ws_a15c95b1_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_a15c95b1_vectors;

CREATE TABLE eq_eq_default_ws_a15c95b1_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_a15c95b1_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_a15c95b1_vectors_doc_id_idx ON public.eq_eq_default_ws_a15c95b1_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_a15c95b1_vectors_embedding_idx ON public.eq_eq_default_ws_a15c95b1_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_a15c95b1_vectors_tenant_ws_idx ON public.eq_eq_default_ws_a15c95b1_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_a15c95b1_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_a15c95b1_vectors for each row execute function eq_eq_default_ws_a15c95b1_vectors_stats_insert();
create trigger eq_eq_default_ws_a15c95b1_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_a15c95b1_vectors for each row execute function eq_eq_default_ws_a15c95b1_vectors_stats_delete();


-- public.eq_eq_default_ws_a15c95b1_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_a15c95b1_vectors_stats;

CREATE TABLE eq_eq_default_ws_a15c95b1_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_a15c95b1_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_a15c95b1_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_a2684dfa_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_a2684dfa_vectors;

CREATE TABLE eq_eq_default_ws_a2684dfa_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_a2684dfa_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_a2684dfa_vectors_doc_id_idx ON public.eq_eq_default_ws_a2684dfa_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_a2684dfa_vectors_embedding_idx ON public.eq_eq_default_ws_a2684dfa_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_a2684dfa_vectors_tenant_ws_idx ON public.eq_eq_default_ws_a2684dfa_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_a2684dfa_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_a2684dfa_vectors for each row execute function eq_eq_default_ws_a2684dfa_vectors_stats_insert();
create trigger eq_eq_default_ws_a2684dfa_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_a2684dfa_vectors for each row execute function ag_catalog.eq_eq_default_ws_a2684dfa_vectors_stats_delete();


-- public.eq_eq_default_ws_a2684dfa_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_a2684dfa_vectors_stats;

CREATE TABLE eq_eq_default_ws_a2684dfa_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_a2684dfa_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_a2684dfa_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_c915ea00_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_c915ea00_vectors;

CREATE TABLE eq_eq_default_ws_c915ea00_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_c915ea00_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_c915ea00_vectors_doc_id_idx ON public.eq_eq_default_ws_c915ea00_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_c915ea00_vectors_embedding_idx ON public.eq_eq_default_ws_c915ea00_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_c915ea00_vectors_tenant_ws_idx ON public.eq_eq_default_ws_c915ea00_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_c915ea00_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_c915ea00_vectors for each row execute function eq_eq_default_ws_c915ea00_vectors_stats_insert();
create trigger eq_eq_default_ws_c915ea00_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_c915ea00_vectors for each row execute function eq_eq_default_ws_c915ea00_vectors_stats_delete();


-- public.eq_eq_default_ws_c915ea00_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_c915ea00_vectors_stats;

CREATE TABLE eq_eq_default_ws_c915ea00_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_c915ea00_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_c915ea00_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_d87256f7_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_d87256f7_vectors;

CREATE TABLE eq_eq_default_ws_d87256f7_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_d87256f7_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_d87256f7_vectors_doc_id_idx ON public.eq_eq_default_ws_d87256f7_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_d87256f7_vectors_embedding_idx ON public.eq_eq_default_ws_d87256f7_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_d87256f7_vectors_tenant_ws_idx ON public.eq_eq_default_ws_d87256f7_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_d87256f7_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_d87256f7_vectors for each row execute function eq_eq_default_ws_d87256f7_vectors_stats_insert();
create trigger eq_eq_default_ws_d87256f7_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_d87256f7_vectors for each row execute function eq_eq_default_ws_d87256f7_vectors_stats_delete();


-- public.eq_eq_default_ws_d87256f7_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_d87256f7_vectors_stats;

CREATE TABLE eq_eq_default_ws_d87256f7_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_d87256f7_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_d87256f7_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_e3926fac_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_e3926fac_vectors;

CREATE TABLE eq_eq_default_ws_e3926fac_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_e3926fac_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_e3926fac_vectors_doc_id_idx ON public.eq_eq_default_ws_e3926fac_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_e3926fac_vectors_embedding_idx ON public.eq_eq_default_ws_e3926fac_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_e3926fac_vectors_tenant_ws_idx ON public.eq_eq_default_ws_e3926fac_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_e3926fac_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_e3926fac_vectors for each row execute function eq_eq_default_ws_e3926fac_vectors_stats_insert();
create trigger eq_eq_default_ws_e3926fac_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_e3926fac_vectors for each row execute function eq_eq_default_ws_e3926fac_vectors_stats_delete();


-- public.eq_eq_default_ws_e3926fac_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_e3926fac_vectors_stats;

CREATE TABLE eq_eq_default_ws_e3926fac_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_e3926fac_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_e3926fac_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_f6dcb4fc_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_f6dcb4fc_vectors;

CREATE TABLE eq_eq_default_ws_f6dcb4fc_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_f6dcb4fc_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_f6dcb4fc_vectors_doc_id_idx ON public.eq_eq_default_ws_f6dcb4fc_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_f6dcb4fc_vectors_embedding_idx ON public.eq_eq_default_ws_f6dcb4fc_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_f6dcb4fc_vectors_tenant_ws_idx ON public.eq_eq_default_ws_f6dcb4fc_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_f6dcb4fc_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_f6dcb4fc_vectors for each row execute function eq_eq_default_ws_f6dcb4fc_vectors_stats_insert();
create trigger eq_eq_default_ws_f6dcb4fc_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_f6dcb4fc_vectors for each row execute function eq_eq_default_ws_f6dcb4fc_vectors_stats_delete();


-- public.eq_eq_default_ws_f6dcb4fc_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_f6dcb4fc_vectors_stats;

CREATE TABLE eq_eq_default_ws_f6dcb4fc_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_f6dcb4fc_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_f6dcb4fc_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.eq_eq_default_ws_fb134461_vectors definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_fb134461_vectors;

CREATE TABLE eq_eq_default_ws_fb134461_vectors (
	id text NOT NULL,
	embedding public.vector NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	document_id text NULL,
	tenant_id text NULL,
	workspace_id text NULL,
	content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, COALESCE(metadata ->> 'content'::text, ''::text))) STORED NULL,
	CONSTRAINT eq_eq_default_ws_fb134461_vectors_pkey PRIMARY KEY (id)
);
CREATE INDEX eq_eq_default_ws_fb134461_vectors_doc_id_idx ON public.eq_eq_default_ws_fb134461_vectors USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX eq_eq_default_ws_fb134461_vectors_embedding_idx ON public.eq_eq_default_ws_fb134461_vectors USING hnsw (embedding vector_cosine_ops) WITH (m='16', ef_construction='32');
CREATE INDEX eq_eq_default_ws_fb134461_vectors_tenant_ws_idx ON public.eq_eq_default_ws_fb134461_vectors USING btree (tenant_id, workspace_id) WHERE (tenant_id IS NOT NULL);

-- Table Triggers

create trigger eq_eq_default_ws_fb134461_vectors_stats_insert_trg after
insert
    on
    public.eq_eq_default_ws_fb134461_vectors for each row execute function eq_eq_default_ws_fb134461_vectors_stats_insert();
create trigger eq_eq_default_ws_fb134461_vectors_stats_delete_trg after
delete
    on
    public.eq_eq_default_ws_fb134461_vectors for each row execute function eq_eq_default_ws_fb134461_vectors_stats_delete();


-- public.eq_eq_default_ws_fb134461_vectors_stats definition

-- Drop table

-- DROP TABLE eq_eq_default_ws_fb134461_vectors_stats;

CREATE TABLE eq_eq_default_ws_fb134461_vectors_stats (
	id int2 DEFAULT 1 NOT NULL,
	row_count int8 DEFAULT 0 NOT NULL,
	CONSTRAINT eq_eq_default_ws_fb134461_vectors_stats_id_check CHECK ((id = 1)),
	CONSTRAINT eq_eq_default_ws_fb134461_vectors_stats_pkey PRIMARY KEY (id)
);


-- public.failed_chunks definition

-- Drop table

-- DROP TABLE failed_chunks;

CREATE TABLE failed_chunks (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	document_id varchar(255) NOT NULL,
	workspace_id uuid NOT NULL,
	tenant_id uuid NULL,
	chunk_index int4 NOT NULL,
	chunk_id varchar(255) NOT NULL,
	error_message text NOT NULL,
	was_timeout bool DEFAULT false NOT NULL,
	retry_attempts int4 DEFAULT 0 NOT NULL,
	processing_time_ms int8 NULL,
	status varchar(32) DEFAULT 'pending'::character varying NOT NULL,
	failed_at timestamptz DEFAULT now() NOT NULL,
	retry_scheduled_at timestamptz NULL,
	last_retry_at timestamptz NULL,
	resolved_at timestamptz NULL,
	CONSTRAINT failed_chunks_document_id_chunk_index_failed_at_key UNIQUE (document_id, chunk_index, failed_at),
	CONSTRAINT failed_chunks_pkey PRIMARY KEY (id)
);
CREATE INDEX idx_failed_chunks_document_id ON public.failed_chunks USING btree (document_id);
CREATE INDEX idx_failed_chunks_retry_scheduled ON public.failed_chunks USING btree (retry_scheduled_at) WHERE (((status)::text = 'pending'::text) AND (retry_scheduled_at IS NOT NULL));
CREATE INDEX idx_failed_chunks_workspace_pending ON public.failed_chunks USING btree (workspace_id, status) WHERE ((status)::text = 'pending'::text);


-- public.rls_audit_log definition

-- Drop table

-- DROP TABLE rls_audit_log;

CREATE TABLE rls_audit_log (
	id bigserial NOT NULL,
	event_time timestamptz DEFAULT now() NULL,
	tenant_id uuid NULL,
	workspace_id uuid NULL,
	user_id uuid NULL,
	"action" varchar(50) NULL,
	table_name varchar(100) NULL,
	record_id text NULL,
	details jsonb NULL,
	CONSTRAINT rls_audit_log_pkey PRIMARY KEY (id)
);
CREATE INDEX idx_rls_audit_tenant ON public.rls_audit_log USING btree (tenant_id, event_time DESC);


-- public.server_config definition

-- Drop table

-- DROP TABLE server_config;

CREATE TABLE server_config (
	"key" text NOT NULL,
	value jsonb NOT NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT server_config_pkey PRIMARY KEY (key)
);


-- public.tenants definition

-- Drop table

-- DROP TABLE tenants;

CREATE TABLE tenants (
	tenant_id uuid DEFAULT gen_random_uuid() NOT NULL,
	"name" varchar(255) NOT NULL,
	slug varchar(100) NULL,
	settings jsonb DEFAULT '{}'::jsonb NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
	is_active bool DEFAULT true NOT NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT tenants_pkey PRIMARY KEY (tenant_id),
	CONSTRAINT tenants_slug_key UNIQUE (slug)
);
CREATE INDEX idx_tenants_active ON public.tenants USING btree (is_active) WHERE (is_active = true);
CREATE INDEX idx_tenants_slug ON public.tenants USING btree (slug) WHERE (slug IS NOT NULL);

-- Table Triggers

create trigger trigger_tenants_updated_at before
update
    on
    public.tenants for each row execute function update_tenants_updated_at();


-- public."_sqlx_migrations" definition

-- Drop table

-- DROP TABLE "_sqlx_migrations";

CREATE TABLE "_sqlx_migrations" (
	"version" int8 NOT NULL,
	description text NOT NULL,
	installed_on timestamptz DEFAULT now() NOT NULL,
	success bool NOT NULL,
	checksum bytea NOT NULL,
	execution_time int8 NOT NULL,
	CONSTRAINT "_sqlx_migrations_pkey" PRIMARY KEY (version)
);


-- public.users definition

-- Drop table

-- DROP TABLE users;

CREATE TABLE users (
	user_id uuid DEFAULT gen_random_uuid() NOT NULL,
	tenant_id uuid NOT NULL,
	email varchar(255) NULL,
	username varchar(100) NULL,
	display_name varchar(255) NULL,
	password_hash varchar(255) NULL,
	"role" varchar(50) DEFAULT 'user'::character varying NOT NULL,
	is_active bool DEFAULT true NOT NULL,
	last_login_at timestamptz NULL,
	metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	failed_login_attempts int4 DEFAULT 0 NOT NULL,
	locked_until timestamptz NULL,
	CONSTRAINT users_email_unique UNIQUE (tenant_id, email),
	CONSTRAINT users_pkey PRIMARY KEY (user_id),
	CONSTRAINT users_username_unique UNIQUE (tenant_id, username),
	CONSTRAINT valid_user_role CHECK (((role)::text = ANY ((ARRAY['admin'::character varying, 'user'::character varying, 'readonly'::character varying])::text[]))),
	CONSTRAINT users_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE CASCADE
);
CREATE INDEX idx_users_email ON public.users USING btree (email) WHERE (email IS NOT NULL);
CREATE INDEX idx_users_is_active ON public.users USING btree (is_active);
CREATE INDEX idx_users_role ON public.users USING btree (role);
CREATE INDEX idx_users_tenant ON public.users USING btree (tenant_id);
CREATE INDEX idx_users_tenant_email ON public.users USING btree (tenant_id, lower((email)::text));
CREATE INDEX idx_users_tenant_username ON public.users USING btree (tenant_id, lower((username)::text));
CREATE INDEX idx_users_username ON public.users USING btree (username);

-- Table Triggers

create trigger trigger_users_updated_at before
update
    on
    public.users for each row execute function update_users_updated_at();


-- public.workspaces definition

-- Drop table

-- DROP TABLE workspaces;

CREATE TABLE workspaces (
	workspace_id uuid DEFAULT gen_random_uuid() NOT NULL,
	tenant_id uuid NOT NULL,
	"name" varchar(255) NOT NULL,
	slug varchar(100) NULL,
	description text NULL,
	settings jsonb DEFAULT '{}'::jsonb NOT NULL,
	metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
	is_active bool DEFAULT true NOT NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT workspaces_pkey PRIMARY KEY (workspace_id),
	CONSTRAINT workspaces_slug_unique UNIQUE (tenant_id, slug),
	CONSTRAINT workspaces_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE CASCADE
);
CREATE INDEX idx_workspaces_active ON public.workspaces USING btree (is_active);
CREATE INDEX idx_workspaces_slug ON public.workspaces USING btree (tenant_id, slug) WHERE (slug IS NOT NULL);
CREATE INDEX idx_workspaces_tenant ON public.workspaces USING btree (tenant_id);

-- Table Triggers

create trigger trigger_workspaces_updated_at before
update
    on
    public.workspaces for each row execute function update_workspaces_updated_at();


-- public.api_keys definition

-- Drop table

-- DROP TABLE api_keys;

CREATE TABLE api_keys (
	key_id uuid DEFAULT gen_random_uuid() NOT NULL,
	user_id uuid NOT NULL,
	key_hash text NOT NULL,
	key_prefix varchar(20) NOT NULL,
	"name" varchar(255) NULL,
	scopes _text NULL,
	rate_limit_tier varchar(20) NULL,
	is_active bool DEFAULT true NULL,
	created_at timestamptz DEFAULT now() NULL,
	last_used_at timestamptz NULL,
	expires_at timestamptz NULL,
	metadata jsonb NULL,
	CONSTRAINT api_keys_pkey PRIMARY KEY (key_id),
	CONSTRAINT api_keys_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);
CREATE INDEX idx_api_keys_active ON public.api_keys USING btree (is_active);
CREATE INDEX idx_api_keys_prefix ON public.api_keys USING btree (key_prefix);
CREATE INDEX idx_api_keys_user ON public.api_keys USING btree (user_id);


-- public.documents definition

-- Drop table

-- DROP TABLE documents;

CREATE TABLE documents (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	tenant_id uuid NULL,
	workspace_id uuid NULL,
	title text DEFAULT 'Untitled'::text NOT NULL,
	"content" text NOT NULL,
	content_hash varchar(64) NULL,
	metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
	file_path text NULL,
	file_size_bytes int8 NULL,
	content_type varchar(100) NULL,
	status varchar(20) DEFAULT 'indexed'::character varying NOT NULL,
	track_id varchar(50) NULL,
	error_message text NULL,
	processing_time_ms int4 NULL,
	chunk_count int4 DEFAULT 0 NULL,
	entity_count int4 DEFAULT 0 NULL,
	relationship_count int4 DEFAULT 0 NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	cost_usd float8 NULL,
	input_tokens int8 NULL,
	output_tokens int8 NULL,
	total_tokens int8 NULL,
	CONSTRAINT documents_pkey PRIMARY KEY (id),
	CONSTRAINT documents_valid_status CHECK (((status)::text = ANY ((ARRAY['pending'::character varying, 'processing'::character varying, 'chunking'::character varying, 'extracting'::character varying, 'embedding'::character varying, 'indexing'::character varying, 'completed'::character varying, 'indexed'::character varying, 'failed'::character varying, 'partial_failure'::character varying, 'cancelled'::character varying])::text[]))),
	CONSTRAINT documents_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE CASCADE,
	CONSTRAINT documents_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id) ON DELETE CASCADE
);
CREATE INDEX idx_documents_content_hash ON public.documents USING btree (content_hash) WHERE (content_hash IS NOT NULL);
CREATE INDEX idx_documents_created_at ON public.documents USING btree (created_at DESC);
CREATE INDEX idx_documents_file_path ON public.documents USING btree (file_path);
CREATE INDEX idx_documents_status ON public.documents USING btree (status);
CREATE INDEX idx_documents_status_v2 ON public.documents USING btree (status);
CREATE INDEX idx_documents_tenant_status ON public.documents USING btree (tenant_id, status) INCLUDE (title, created_at, updated_at) WHERE (tenant_id IS NOT NULL);
CREATE INDEX idx_documents_tenant_title_search ON public.documents USING gin (to_tsvector('english'::regconfig, title)) WHERE (tenant_id IS NOT NULL);
CREATE INDEX idx_documents_tenant_workspace ON public.documents USING btree (tenant_id, workspace_id);
CREATE INDEX idx_documents_track_id ON public.documents USING btree (track_id);
CREATE UNIQUE INDEX idx_documents_workspace_content_hash_unique ON public.documents USING btree (workspace_id, content_hash) WHERE ((workspace_id IS NOT NULL) AND (content_hash IS NOT NULL) AND ((status)::text = 'indexed'::text));
CREATE INDEX idx_documents_workspace_hash_lookup ON public.documents USING btree (workspace_id, content_hash) WHERE (content_hash IS NOT NULL);

-- Table Triggers

create trigger trigger_documents_updated_at before
update
    on
    public.documents for each row execute function update_updated_at_column();
create trigger check_document_quota before
insert
    on
    public.documents for each row execute function check_workspace_quota();


-- public.entities definition

-- Drop table

-- DROP TABLE entities;

CREATE TABLE entities (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	tenant_id uuid NULL,
	workspace_id uuid NULL,
	"name" text NOT NULL,
	entity_type text NOT NULL,
	description text NULL,
	source_ids _uuid NULL,
	is_manual bool DEFAULT false NOT NULL,
	manual_created_at timestamptz NULL,
	manual_created_by varchar(255) NULL,
	last_manual_edit_at timestamptz NULL,
	last_manual_edit_by varchar(255) NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	source_chunk_ids _text DEFAULT '{}'::text[] NOT NULL,
	keywords _text DEFAULT '{}'::text[] NOT NULL,
	sync_status varchar(20) DEFAULT 'unsynced'::character varying NOT NULL,
	tsv tsvector GENERATED ALWAYS AS (to_tsvector('english'::regconfig, (((COALESCE(name, ''::text) || ' '::text) || COALESCE(entity_type, ''::text)) || ' '::text) || COALESCE(description, ''::text))) STORED NULL,
	description_history jsonb DEFAULT '[]'::jsonb NOT NULL,
	CONSTRAINT entities_pkey PRIMARY KEY (id),
	CONSTRAINT entities_unique_name UNIQUE NULLS NOT DISTINCT (tenant_id, workspace_id, name),
	CONSTRAINT entities_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE CASCADE,
	CONSTRAINT entities_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id) ON DELETE CASCADE
);
CREATE INDEX idx_entities_description_history ON public.entities USING gin (description_history jsonb_path_ops);
CREATE INDEX idx_entities_is_manual ON public.entities USING btree (is_manual);
CREATE INDEX idx_entities_manual_created_by ON public.entities USING btree (manual_created_by) WHERE (is_manual = true);
CREATE INDEX idx_entities_name ON public.entities USING btree (name);
CREATE INDEX idx_entities_source_chunk_ids ON public.entities USING gin (source_chunk_ids);
CREATE INDEX idx_entities_sync_status ON public.entities USING btree (sync_status) WHERE ((sync_status)::text <> 'synced'::text);
CREATE INDEX idx_entities_tenant_name_search ON public.entities USING btree (tenant_id, workspace_id, name) WHERE (tenant_id IS NOT NULL);
CREATE INDEX idx_entities_tenant_type ON public.entities USING btree (tenant_id, entity_type) WHERE (tenant_id IS NOT NULL);
CREATE INDEX idx_entities_tenant_workspace ON public.entities USING btree (tenant_id, workspace_id);
CREATE INDEX idx_entities_tsv ON public.entities USING gin (tsv);
CREATE INDEX idx_entities_type ON public.entities USING btree (entity_type);
CREATE INDEX idx_entities_type_workspace ON public.entities USING btree (entity_type, tenant_id, workspace_id) WHERE (workspace_id IS NOT NULL);

-- Table Triggers

create trigger trigger_entities_updated_at before
update
    on
    public.entities for each row execute function update_updated_at_column();


-- public.folders definition

-- Drop table

-- DROP TABLE folders;

CREATE TABLE folders (
	folder_id uuid DEFAULT gen_random_uuid() NOT NULL,
	tenant_id uuid NOT NULL,
	workspace_id uuid NULL,
	user_id uuid NOT NULL,
	"name" varchar(255) NOT NULL,
	parent_id uuid NULL,
	"position" int4 DEFAULT 0 NOT NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT folders_pkey PRIMARY KEY (folder_id),
	CONSTRAINT unique_folder_name_in_parent UNIQUE (tenant_id, user_id, parent_id, name),
	CONSTRAINT folders_parent_id_fkey FOREIGN KEY (parent_id) REFERENCES folders(folder_id) ON DELETE CASCADE,
	CONSTRAINT folders_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE CASCADE,
	CONSTRAINT folders_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE,
	CONSTRAINT folders_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id) ON DELETE SET NULL
);
CREATE INDEX idx_folders_parent ON public.folders USING btree (parent_id);
CREATE INDEX idx_folders_tenant_user ON public.folders USING btree (tenant_id, user_id);

-- Table Triggers

create trigger trigger_folders_updated_at before
update
    on
    public.folders for each row execute function update_folders_updated_at();


-- public.memberships definition

-- Drop table

-- DROP TABLE memberships;

CREATE TABLE memberships (
	membership_id uuid DEFAULT gen_random_uuid() NOT NULL,
	tenant_id uuid NOT NULL,
	workspace_id uuid NULL,
	user_id uuid NOT NULL,
	"role" varchar(50) DEFAULT 'member'::character varying NOT NULL,
	is_active bool DEFAULT true NOT NULL,
	joined_at timestamptz DEFAULT now() NOT NULL,
	metadata jsonb NULL,
	CONSTRAINT memberships_pkey PRIMARY KEY (membership_id),
	CONSTRAINT memberships_unique UNIQUE (user_id, tenant_id, workspace_id),
	CONSTRAINT valid_membership_role CHECK (((role)::text = ANY ((ARRAY['owner'::character varying, 'admin'::character varying, 'member'::character varying, 'readonly'::character varying])::text[]))),
	CONSTRAINT memberships_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE CASCADE,
	CONSTRAINT memberships_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE,
	CONSTRAINT memberships_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id) ON DELETE CASCADE
);
CREATE INDEX idx_memberships_tenant ON public.memberships USING btree (tenant_id);
CREATE INDEX idx_memberships_user ON public.memberships USING btree (user_id);
CREATE INDEX idx_memberships_workspace ON public.memberships USING btree (workspace_id);


-- public.pdf_documents definition

-- Drop table

-- DROP TABLE pdf_documents;

CREATE TABLE pdf_documents (
	pdf_id uuid DEFAULT gen_random_uuid() NOT NULL,
	workspace_id uuid NOT NULL,
	document_id uuid NULL,
	filename varchar(512) NOT NULL,
	content_type varchar(100) DEFAULT 'application/pdf'::character varying NOT NULL,
	file_size_bytes int8 NOT NULL,
	sha256_checksum varchar(64) NOT NULL,
	page_count int4 NULL,
	pdf_data bytea NOT NULL,
	processing_status varchar(50) DEFAULT 'pending'::character varying NOT NULL,
	extraction_method varchar(50) NULL,
	vision_model varchar(100) NULL,
	markdown_content text NULL,
	extraction_errors jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	processed_at timestamptz NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT pdf_documents_document_id_key UNIQUE (document_id),
	CONSTRAINT pdf_documents_pkey PRIMARY KEY (pdf_id),
	CONSTRAINT valid_checksum_format CHECK (((sha256_checksum)::text ~ '^[a-f0-9]{64}$'::text)),
	CONSTRAINT valid_extraction_method CHECK (((extraction_method IS NULL) OR ((extraction_method)::text = ANY ((ARRAY['text'::character varying, 'vision'::character varying, 'hybrid'::character varying, 'edgeparse'::character varying])::text[])))),
	CONSTRAINT valid_file_size CHECK (((file_size_bytes > 0) AND (file_size_bytes <= 104857600))),
	CONSTRAINT valid_page_count CHECK (((page_count IS NULL) OR (page_count > 0))),
	CONSTRAINT valid_processing_status CHECK (((processing_status)::text = ANY ((ARRAY['pending'::character varying, 'processing'::character varying, 'completed'::character varying, 'failed'::character varying])::text[]))),
	CONSTRAINT pdf_documents_document_id_fkey FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE,
	CONSTRAINT pdf_documents_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id) ON DELETE CASCADE
);
CREATE INDEX idx_pdf_documents_created ON public.pdf_documents USING btree (created_at DESC);
CREATE INDEX idx_pdf_documents_document_id ON public.pdf_documents USING btree (document_id) WHERE (document_id IS NOT NULL);
CREATE INDEX idx_pdf_documents_status ON public.pdf_documents USING btree (processing_status);
CREATE INDEX idx_pdf_documents_workspace ON public.pdf_documents USING btree (workspace_id);
CREATE UNIQUE INDEX idx_pdf_documents_workspace_checksum_unique ON public.pdf_documents USING btree (workspace_id, sha256_checksum);
CREATE INDEX idx_pdf_documents_workspace_status ON public.pdf_documents USING btree (workspace_id, processing_status, created_at DESC);

-- Table Triggers

create trigger set_updated_at before
update
    on
    public.pdf_documents for each row execute function trigger_set_updated_at();


-- public.refresh_tokens definition

-- Drop table

-- DROP TABLE refresh_tokens;

CREATE TABLE refresh_tokens (
	token_id uuid DEFAULT gen_random_uuid() NOT NULL,
	user_id uuid NOT NULL,
	token_hash text NOT NULL,
	expires_at timestamptz NOT NULL,
	revoked bool DEFAULT false NULL,
	revoked_at timestamptz NULL,
	created_at timestamptz DEFAULT now() NULL,
	user_agent text NULL,
	ip_address inet NULL,
	CONSTRAINT refresh_tokens_pkey PRIMARY KEY (token_id),
	CONSTRAINT refresh_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);
CREATE INDEX idx_refresh_tokens_active ON public.refresh_tokens USING btree (user_id, revoked, expires_at);
CREATE INDEX idx_refresh_tokens_expires ON public.refresh_tokens USING btree (expires_at);
CREATE UNIQUE INDEX idx_refresh_tokens_token_hash ON public.refresh_tokens USING btree (token_hash);
CREATE INDEX idx_refresh_tokens_user ON public.refresh_tokens USING btree (user_id);


-- public.relationships definition

-- Drop table

-- DROP TABLE relationships;

CREATE TABLE relationships (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	source_id uuid NOT NULL,
	target_id uuid NOT NULL,
	tenant_id uuid NULL,
	workspace_id uuid NULL,
	relation_type text NOT NULL,
	description text NULL,
	weight float4 DEFAULT 1.0 NULL,
	keywords _text NULL,
	source_chunk_ids _uuid NULL,
	is_manual bool DEFAULT false NOT NULL,
	manual_created_at timestamptz NULL,
	manual_created_by varchar(255) NULL,
	last_manual_edit_at timestamptz NULL,
	last_manual_edit_by varchar(255) NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	sync_status varchar(20) DEFAULT 'unsynced'::character varying NOT NULL,
	CONSTRAINT relationships_pkey PRIMARY KEY (id),
	CONSTRAINT relationships_unique UNIQUE NULLS NOT DISTINCT (tenant_id, workspace_id, source_id, target_id, relation_type),
	CONSTRAINT relationships_source_id_fkey FOREIGN KEY (source_id) REFERENCES entities(id) ON DELETE CASCADE,
	CONSTRAINT relationships_target_id_fkey FOREIGN KEY (target_id) REFERENCES entities(id) ON DELETE CASCADE,
	CONSTRAINT relationships_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE CASCADE,
	CONSTRAINT relationships_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id) ON DELETE CASCADE
);
CREATE INDEX idx_relationships_is_manual ON public.relationships USING btree (is_manual);
CREATE INDEX idx_relationships_manual_created_by ON public.relationships USING btree (manual_created_by) WHERE (is_manual = true);
CREATE INDEX idx_relationships_source ON public.relationships USING btree (source_id);
CREATE INDEX idx_relationships_source_chunk_ids ON public.relationships USING gin (source_chunk_ids);
CREATE INDEX idx_relationships_target ON public.relationships USING btree (target_id);
CREATE INDEX idx_relationships_tenant_type ON public.relationships USING btree (tenant_id, relation_type) WHERE (tenant_id IS NOT NULL);
CREATE INDEX idx_relationships_tenant_workspace ON public.relationships USING btree (tenant_id, workspace_id);
CREATE INDEX idx_relationships_type ON public.relationships USING btree (relation_type);
CREATE INDEX idx_relationships_workspace ON public.relationships USING btree (tenant_id, workspace_id) WHERE (workspace_id IS NOT NULL);

-- Table Triggers

create trigger trigger_relationships_updated_at before
update
    on
    public.relationships for each row execute function update_updated_at_column();


-- public.tasks definition

-- Drop table

-- DROP TABLE tasks;

CREATE TABLE tasks (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	tenant_id uuid DEFAULT '00000000-0000-0000-0000-000000000000'::uuid NOT NULL,
	workspace_id uuid DEFAULT '00000000-0000-0000-0000-000000000000'::uuid NOT NULL,
	track_id varchar(100) NOT NULL,
	task_type varchar(50) NOT NULL,
	status varchar(20) DEFAULT 'pending'::character varying NOT NULL,
	priority int4 DEFAULT 0 NOT NULL,
	payload jsonb DEFAULT '{}'::jsonb NOT NULL,
	"result" jsonb NULL,
	error_message text NULL,
	retry_count int4 DEFAULT 0 NOT NULL,
	max_retries int4 DEFAULT 3 NOT NULL,
	scheduled_at timestamptz NULL,
	started_at timestamptz NULL,
	completed_at timestamptz NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	consecutive_timeout_failures int4 DEFAULT 0 NOT NULL,
	circuit_breaker_tripped bool DEFAULT false NOT NULL,
	error jsonb NULL,
	CONSTRAINT tasks_pkey PRIMARY KEY (id),
	CONSTRAINT tasks_valid_status CHECK (((status)::text = ANY ((ARRAY['pending'::character varying, 'processing'::character varying, 'indexed'::character varying, 'failed'::character varying, 'cancelled'::character varying])::text[]))),
	CONSTRAINT valid_task_type CHECK (((task_type)::text = ANY ((ARRAY['upload'::character varying, 'insert'::character varying, 'scan'::character varying, 'reindex'::character varying, 'pdf_processing'::character varying])::text[]))),
	CONSTRAINT tasks_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE CASCADE,
	CONSTRAINT tasks_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id) ON DELETE CASCADE
);
CREATE INDEX idx_tasks_circuit_breaker ON public.tasks USING btree (circuit_breaker_tripped, status);
CREATE INDEX idx_tasks_consecutive_timeouts ON public.tasks USING btree (consecutive_timeout_failures) WHERE (consecutive_timeout_failures > 0);
CREATE INDEX idx_tasks_created ON public.tasks USING btree (created_at DESC);
CREATE INDEX idx_tasks_scheduled ON public.tasks USING btree (scheduled_at) WHERE ((status)::text = 'pending'::text);
CREATE INDEX idx_tasks_status ON public.tasks USING btree (status);
CREATE INDEX idx_tasks_status_type ON public.tasks USING btree (status, task_type);
CREATE INDEX idx_tasks_tenant_workspace ON public.tasks USING btree (tenant_id, workspace_id);
CREATE INDEX idx_tasks_tenant_workspace_status ON public.tasks USING btree (tenant_id, workspace_id, status) WHERE (tenant_id IS NOT NULL);
CREATE INDEX idx_tasks_tenant_workspace_type ON public.tasks USING btree (tenant_id, workspace_id, task_type);
CREATE INDEX idx_tasks_track_id ON public.tasks USING btree (track_id);
CREATE INDEX idx_tasks_type ON public.tasks USING btree (task_type);
CREATE INDEX idx_tasks_updated ON public.tasks USING btree (updated_at DESC);

-- Table Triggers

create trigger trigger_tasks_updated_at before
update
    on
    public.tasks for each row execute function update_updated_at_column();
create trigger normalize_edgequake_task_legacy_fields_trigger before
insert
    or
update
    on
    public.tasks for each row execute function normalize_edgequake_task_legacy_fields();


-- public.chunks definition

-- Drop table

-- DROP TABLE chunks;

CREATE TABLE chunks (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	document_id uuid NOT NULL,
	tenant_id uuid NULL,
	workspace_id uuid NULL,
	"content" text NOT NULL,
	chunk_index int4 NOT NULL,
	start_offset int4 NULL,
	end_offset int4 NULL,
	token_count int4 NULL,
	metadata jsonb DEFAULT '{}'::jsonb NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	char_start int4 NULL,
	char_end int4 NULL,
	page_start int4 NULL,
	page_end int4 NULL,
	embedding_id text NULL,
	CONSTRAINT chunks_pkey PRIMARY KEY (id),
	CONSTRAINT chunks_unique_doc_index UNIQUE (document_id, chunk_index),
	CONSTRAINT chunks_document_id_fkey FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE,
	CONSTRAINT chunks_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE CASCADE,
	CONSTRAINT chunks_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id) ON DELETE CASCADE
);
CREATE INDEX idx_chunks_created_at_brin ON public.chunks USING brin (created_at) WITH (pages_per_range='128');
CREATE INDEX idx_chunks_document ON public.chunks USING btree (document_id);
CREATE INDEX idx_chunks_embedding_id ON public.chunks USING btree (embedding_id) WHERE (embedding_id IS NOT NULL);
CREATE INDEX idx_chunks_page_span ON public.chunks USING btree (document_id, page_start, page_end) WHERE (page_start IS NOT NULL);
CREATE INDEX idx_chunks_tenant_workspace ON public.chunks USING btree (tenant_id, workspace_id);


-- public.conversations definition

-- Drop table

-- DROP TABLE conversations;

CREATE TABLE conversations (
	conversation_id uuid DEFAULT gen_random_uuid() NOT NULL,
	tenant_id uuid NOT NULL,
	workspace_id uuid NULL,
	user_id uuid NOT NULL,
	title varchar(500) DEFAULT 'New Conversation'::character varying NOT NULL,
	"mode" varchar(50) DEFAULT 'hybrid'::character varying NOT NULL,
	is_pinned bool DEFAULT false NOT NULL,
	is_archived bool DEFAULT false NOT NULL,
	folder_id uuid NULL,
	share_id varchar(64) NULL,
	meta jsonb DEFAULT '{}'::jsonb NOT NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT conversations_pkey PRIMARY KEY (conversation_id),
	CONSTRAINT conversations_share_id_key UNIQUE (share_id),
	CONSTRAINT valid_mode CHECK (((mode)::text = ANY ((ARRAY['local'::character varying, 'global'::character varying, 'hybrid'::character varying, 'naive'::character varying, 'mix'::character varying])::text[]))),
	CONSTRAINT conversations_folder_id_fkey FOREIGN KEY (folder_id) REFERENCES folders(folder_id) ON DELETE SET NULL,
	CONSTRAINT conversations_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE CASCADE,
	CONSTRAINT conversations_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE,
	CONSTRAINT conversations_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id) ON DELETE SET NULL
);
CREATE INDEX idx_conversations_archived ON public.conversations USING btree (tenant_id, user_id, is_archived, updated_at DESC);
CREATE INDEX idx_conversations_folder ON public.conversations USING btree (folder_id) WHERE (folder_id IS NOT NULL);
CREATE INDEX idx_conversations_pinned ON public.conversations USING btree (tenant_id, user_id, is_pinned) WHERE (is_pinned = true);
CREATE INDEX idx_conversations_share ON public.conversations USING btree (share_id) WHERE (share_id IS NOT NULL);
CREATE INDEX idx_conversations_tenant_user ON public.conversations USING btree (tenant_id, user_id, updated_at DESC);
CREATE INDEX idx_conversations_title_fts ON public.conversations USING gin (to_tsvector('english'::regconfig, (title)::text));
CREATE INDEX idx_conversations_workspace ON public.conversations USING btree (workspace_id, updated_at DESC) WHERE (workspace_id IS NOT NULL);

-- Table Triggers

create trigger trigger_conversations_updated_at before
update
    on
    public.conversations for each row execute function update_conversations_updated_at();


-- public.messages definition

-- Drop table

-- DROP TABLE messages;

CREATE TABLE messages (
	message_id uuid DEFAULT gen_random_uuid() NOT NULL,
	conversation_id uuid NOT NULL,
	parent_id uuid NULL,
	"role" varchar(20) NOT NULL,
	"content" text NOT NULL,
	"mode" varchar(50) NULL,
	tokens_used int4 NULL,
	duration_ms int4 NULL,
	thinking_time_ms int4 NULL,
	context jsonb NULL,
	is_error bool DEFAULT false NOT NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT messages_pkey PRIMARY KEY (message_id),
	CONSTRAINT valid_role CHECK (((role)::text = ANY ((ARRAY['user'::character varying, 'assistant'::character varying, 'system'::character varying])::text[]))),
	CONSTRAINT messages_conversation_id_fkey FOREIGN KEY (conversation_id) REFERENCES conversations(conversation_id) ON DELETE CASCADE,
	CONSTRAINT messages_parent_id_fkey FOREIGN KEY (parent_id) REFERENCES messages(message_id) ON DELETE SET NULL
);
CREATE INDEX idx_messages_content_fts ON public.messages USING gin (to_tsvector('english'::regconfig, content));
CREATE INDEX idx_messages_conversation ON public.messages USING btree (conversation_id, created_at);
CREATE INDEX idx_messages_parent ON public.messages USING btree (parent_id) WHERE (parent_id IS NOT NULL);

-- Table Triggers

create trigger trigger_messages_updated_at before
update
    on
    public.messages for each row execute function update_messages_updated_at();
create trigger trigger_update_conversation_on_message after
insert
    on
    public.messages for each row execute function update_conversation_on_message();
