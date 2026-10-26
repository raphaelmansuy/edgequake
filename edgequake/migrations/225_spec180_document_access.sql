-- SPEC-180: versioned document access and authoritative child inheritance.
ALTER TABLE public.documents ADD COLUMN IF NOT EXISTS label_version bigint NOT NULL DEFAULT 0;
ALTER TABLE public.mcp_upload_sessions ADD COLUMN IF NOT EXISTS access jsonb;

CREATE TABLE public.document_access_operations (
    operation_id uuid NOT NULL,
    workspace_id uuid NOT NULL REFERENCES public.workspaces(workspace_id) ON DELETE CASCADE,
    actor_id text NOT NULL,
    request_hash text NOT NULL,
    outcome jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(workspace_id, actor_id, operation_id)
);
CREATE INDEX document_access_operations_created ON public.document_access_operations(created_at);

-- New unlabeled rows in enforced workspaces never become broadly readable.
CREATE FUNCTION public.document_initial_access() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE existing public.documents%ROWTYPE;
BEGIN
    -- Worker INSERT ... ON CONFLICT must retain the authoritative saved label.
    SELECT * INTO existing FROM public.documents WHERE id=NEW.id AND workspace_id=NEW.workspace_id FOR SHARE;
    IF FOUND THEN
        NEW.classification_rank := existing.classification_rank;
        NEW.compartment_ids := existing.compartment_ids;
        NEW.uploaded_by := existing.uploaded_by;
        NEW.label_version := existing.label_version;
        RETURN NEW;
    END IF;
    IF EXISTS(SELECT 1 FROM public.workspace_clearance_settings
              WHERE workspace_id = NEW.workspace_id AND enforcement) THEN
        IF NEW.classification_rank IS NULL THEN
            NEW.classification_rank := 5;
        END IF;
        NEW.compartment_ids := COALESCE(NEW.compartment_ids, '{}');
        NEW.uploaded_by := COALESCE(NEW.uploaded_by, public.current_user_id());
        IF NEW.uploaded_by IS NULL THEN
            RAISE EXCEPTION 'Enforced document admission requires an uploader';
        END IF;
        NEW.label_version := GREATEST(NEW.label_version, 1);
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER document_initial_access BEFORE INSERT ON public.documents
    FOR EACH ROW EXECUTE FUNCTION public.document_initial_access();

-- Validate join scope/kind under reference-preserving locks. Names are not identity.
CREATE FUNCTION public.document_label_scope_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE doc_workspace uuid; tag_workspace uuid; tag_kind text;
BEGIN
    SELECT workspace_id INTO doc_workspace FROM public.documents WHERE id = NEW.document_id FOR SHARE;
    SELECT workspace_id, kind INTO tag_workspace, tag_kind FROM public.clearance_tags
        WHERE tag_id = NEW.tag_id FOR KEY SHARE;
    IF doc_workspace IS NULL OR tag_workspace IS DISTINCT FROM doc_workspace OR tag_kind <> 'compartment' THEN
        RAISE EXCEPTION 'Invalid document security tag' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER document_label_scope_guard BEFORE INSERT OR UPDATE ON public.document_labels
    FOR EACH ROW EXECUTE FUNCTION public.document_label_scope_guard();

-- All child writes derive access from a locked parent, even if stale metadata supplied it.
CREATE FUNCTION public.document_child_access() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE parent_id uuid; parent_rank smallint; parent_tags uuid[]; parent_uploader uuid;
BEGIN
    parent_id := NULLIF(to_jsonb(NEW)->>TG_ARGV[0], '')::uuid;
    IF parent_id IS NULL THEN
        -- PDF bytes are staged before the durable parent is allocated. The
        -- uploader-only quarantine applies from the very first stored byte.
        IF EXISTS(SELECT 1 FROM public.workspace_clearance_settings
                  WHERE workspace_id=NEW.workspace_id AND enforcement) THEN
            NEW.classification_rank := 5;
            NEW.compartment_ids := '{}';
            NEW.uploaded_by := COALESCE(NEW.uploaded_by, public.current_user_id());
            IF NEW.uploaded_by IS NULL THEN
                RAISE EXCEPTION 'Enforced staging requires an uploader';
            END IF;
        END IF;
        RETURN NEW;
    END IF;
    SELECT classification_rank, compartment_ids, uploaded_by
        INTO parent_rank, parent_tags, parent_uploader
        FROM public.documents WHERE id = parent_id AND workspace_id=NEW.workspace_id FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Document parent is unavailable' USING ERRCODE = '23503';
    END IF;
    NEW.classification_rank := parent_rank;
    NEW.compartment_ids := parent_tags;
    NEW.uploaded_by := parent_uploader;
    RETURN NEW;
END $$;
DO $$ DECLARE tbl text; parent_key text; BEGIN
    FOREACH tbl IN ARRAY ARRAY['chunks', 'document_originals', 'pdf_documents',
        'document_mm_assets', 'document_pages', 'graph_contributions', 'tasks'] LOOP
        parent_key := CASE WHEN tbl = 'graph_contributions' THEN 'source_document_id' ELSE 'document_id' END;
        EXECUTE format('CREATE TRIGGER z_document_child_access BEFORE INSERT OR UPDATE ON public.%I
            FOR EACH ROW EXECUTE FUNCTION public.document_child_access(%L)', tbl, parent_key);
    END LOOP;
END $$;

-- A vocabulary tag referenced only by a profile must not be deleted either.
CREATE FUNCTION public.clearance_tag_reference_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS(SELECT 1 FROM public.security_role_rules WHERE OLD.tag_id = ANY(tag_ids)) THEN
        RAISE EXCEPTION 'Security tag is used by an access profile' USING ERRCODE = '23503';
    END IF;
    RETURN OLD;
END $$;
CREATE TRIGGER clearance_tag_reference_guard BEFORE DELETE ON public.clearance_tags
    FOR EACH ROW EXECUTE FUNCTION public.clearance_tag_reference_guard();

-- Epoch writes serialize with label mutation's workspace lock. These tables already
-- define the policy; do not let an older client omit cache invalidation.
CREATE FUNCTION public.clearance_policy_changed() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE row_data jsonb; ws uuid; role uuid; member uuid;
BEGIN
    row_data := CASE WHEN TG_OP = 'DELETE' THEN to_jsonb(OLD) ELSE to_jsonb(NEW) END;
    ws := (row_data->>'workspace_id')::uuid;
    IF ws IS NULL AND TG_TABLE_NAME = 'security_role_rules' THEN
        role := (row_data->>'role_id')::uuid;
        SELECT workspace_id INTO ws FROM public.security_roles WHERE role_id = role;
    ELSIF ws IS NULL AND TG_TABLE_NAME = 'membership_security_roles' THEN
        member := (row_data->>'membership_id')::uuid;
        SELECT workspace_id INTO ws FROM public.memberships WHERE membership_id = member;
    END IF;
    IF ws IS NOT NULL THEN
        UPDATE public.workspace_clearance_settings SET policy_epoch = policy_epoch + 1 WHERE workspace_id = ws;
    END IF;
    RETURN NULL;
END $$;
DO $$ DECLARE tbl text; BEGIN
    FOREACH tbl IN ARRAY ARRAY['clearance_tags', 'security_roles', 'security_role_rules',
        'membership_security_roles', 'memberships'] LOOP
        EXECUTE format('CREATE TRIGGER clearance_policy_changed AFTER INSERT OR UPDATE OR DELETE ON public.%I
            FOR EACH ROW EXECUTE FUNCTION public.clearance_policy_changed()', tbl);
    END LOOP;
END $$;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.document_access_operations TO edgequake_tenant_access;

ALTER TABLE public.document_access_operations ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.document_access_operations FORCE ROW LEVEL SECURITY;
CREATE POLICY access_operation_scope ON public.document_access_operations
    FOR ALL TO edgequake_tenant_access
    USING (workspace_id = public.current_workspace_id() AND actor_id = public.current_user_id()::text)
    WITH CHECK (workspace_id = public.current_workspace_id() AND actor_id = public.current_user_id()::text);

CREATE FUNCTION public.document_propagate_access() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE tbl text; parent_key text; filter text; affected bigint; total_rows bigint := 0;
BEGIN
    IF OLD.classification_rank IS NOT DISTINCT FROM NEW.classification_rank
       AND OLD.compartment_ids IS NOT DISTINCT FROM NEW.compartment_ids THEN RETURN NEW; END IF;
    FOREACH tbl IN ARRAY ARRAY['chunks', 'document_originals', 'pdf_documents',
        'document_mm_assets', 'document_pages', 'graph_contributions', 'tasks'] LOOP
        parent_key := CASE WHEN tbl = 'graph_contributions' THEN 'source_document_id' ELSE 'document_id' END;
        filter := CASE WHEN tbl = 'tasks' THEN '$1::text' ELSE '$1' END;
        EXECUTE format('SELECT count(*) FROM (SELECT 1 FROM public.%I WHERE %I=%s LIMIT 500001) bounded',tbl,parent_key,filter)
            INTO affected USING NEW.id;
        total_rows := total_rows + affected;
        IF total_rows > 500000 THEN RAISE EXCEPTION 'ACCESS_CHANGE_TOO_LARGE' USING ERRCODE='23514'; END IF;
        EXECUTE format('UPDATE public.%I SET classification_rank=$2,compartment_ids=$3,uploaded_by=$4 WHERE %I=%s',tbl,parent_key,filter)
            USING NEW.id, NEW.classification_rank, NEW.compartment_ids, NEW.uploaded_by;
    END LOOP;
    RETURN NEW;
END $$;
CREATE TRIGGER document_propagate_access AFTER UPDATE OF classification_rank,compartment_ids ON public.documents
    FOR EACH ROW EXECUTE FUNCTION public.document_propagate_access();

-- Audit insertion is part of the access transaction and cannot bypass scope.
CREATE POLICY document_access_audit_insert ON public.audit_logs FOR INSERT TO edgequake_tenant_access
    WITH CHECK (tenant_id::text=public.current_tenant_id()::text
        AND workspace_id::text=public.current_workspace_id()::text
        AND user_id::text=public.current_user_id()::text
        AND event_category='document_clearance'
        AND event_action IN ('clearance.admission','clearance.relabel','clearance.downgrade'));
