-- Run only against the disposable edgequake_spec180 database.
\set ON_ERROR_STOP on
SELECT current_database() = 'edgequake_spec180' AS test_database \gset
\if :test_database
BEGIN;
SET LOCAL search_path = public;
INSERT INTO tenants(tenant_id,name,slug) VALUES('00000000-0000-0000-0000-000000000002','Northstar research','spec180') ON CONFLICT DO NOTHING;
INSERT INTO workspaces(workspace_id,tenant_id,name,slug) VALUES('00000000-0000-0000-0000-000000000003','00000000-0000-0000-0000-000000000002','Research library','research-library') ON CONFLICT(workspace_id) DO UPDATE SET tenant_id=EXCLUDED.tenant_id;
DO $$ DECLARE tbl text; BEGIN
 FOREACH tbl IN ARRAY ARRAY['documents','chunks','memberships','clearance_tags','security_roles'] LOOP
  EXECUTE format('UPDATE public.%I SET workspace_id=$1 WHERE workspace_id=$2',tbl)
   USING '00000000-0000-0000-0000-000000000003'::uuid,'18000000-0000-4000-8000-000000000002'::uuid;
 END LOOP;
END $$;
INSERT INTO users(user_id,tenant_id,username,email,password_hash,role)
SELECT id::uuid,'00000000-0000-0000-0000-000000000002'::uuid,name,name||'@spec180.test','$argon2id$v=19$m=19456,t=2,p=1$7qFmLwDcGWJcj5hle0NHYA$Qv9+W7JgDZv1weg0tIrKtnwqMntax9FjdYGSv+KQ8C8','user'
FROM (VALUES
 ('18000000-0000-4000-8000-000000000011','spec180owner'),
 ('18000000-0000-4000-8000-000000000012','spec180writer'),
 ('18000000-0000-4000-8000-000000000013','spec180readonly')) AS fixture(id,name)
ON CONFLICT(user_id) DO UPDATE SET tenant_id=EXCLUDED.tenant_id;
UPDATE public.documents SET tenant_id='00000000-0000-0000-0000-000000000002' WHERE workspace_id='00000000-0000-0000-0000-000000000003';
UPDATE public.chunks SET tenant_id='00000000-0000-0000-0000-000000000002' WHERE workspace_id='00000000-0000-0000-0000-000000000003';
UPDATE public.memberships SET tenant_id='00000000-0000-0000-0000-000000000002' WHERE workspace_id='00000000-0000-0000-0000-000000000003';
INSERT INTO memberships(tenant_id,workspace_id,user_id,role)
SELECT '00000000-0000-0000-0000-000000000002','00000000-0000-0000-0000-000000000003',id::uuid,role
FROM (VALUES ('18000000-0000-4000-8000-000000000011','owner'),('18000000-0000-4000-8000-000000000012','member'),('18000000-0000-4000-8000-000000000013','readonly')) AS fixture(id,role)
ON CONFLICT DO NOTHING;
INSERT INTO workspace_clearance_settings(workspace_id,enforcement) VALUES('00000000-0000-0000-0000-000000000003',true) ON CONFLICT DO NOTHING;
INSERT INTO memberships(tenant_id,user_id,role)
SELECT tenant_id,user_id,role FROM memberships WHERE workspace_id='00000000-0000-0000-0000-000000000003'
ON CONFLICT DO NOTHING;
INSERT INTO clearance_tags(tag_id,workspace_id,kind,key,value) VALUES
('18000000-0000-4000-8000-000000000021','00000000-0000-0000-0000-000000000003','compartment','department','Finance'),
('18000000-0000-4000-8000-000000000022','00000000-0000-0000-0000-000000000003','compartment','department','Legal'),
('18000000-0000-4000-8000-000000000023','00000000-0000-0000-0000-000000000003','compartment','project','Atlas') ON CONFLICT DO NOTHING;
INSERT INTO security_roles(role_id,workspace_id,name,steward) VALUES
('18000000-0000-4000-8000-000000000031','00000000-0000-0000-0000-000000000003','Security stewards',true),
('18000000-0000-4000-8000-000000000032','00000000-0000-0000-0000-000000000003','Finance writers',false) ON CONFLICT DO NOTHING;
INSERT INTO security_role_rules(role_id,position,effect,match_kind,rank)
SELECT '18000000-0000-4000-8000-000000000032',0,'allow','classification_at_most',3
WHERE NOT EXISTS(SELECT 1 FROM security_role_rules WHERE role_id='18000000-0000-4000-8000-000000000032');
INSERT INTO security_role_rules(role_id,position,effect,match_kind,tag_ids)
SELECT '18000000-0000-4000-8000-000000000032',1,'allow','any_of',ARRAY['18000000-0000-4000-8000-000000000021'::uuid]
WHERE NOT EXISTS(SELECT 1 FROM security_role_rules WHERE role_id='18000000-0000-4000-8000-000000000032' AND position=1);
INSERT INTO membership_security_roles(membership_id,role_id)
SELECT membership_id, CASE WHEN user_id='18000000-0000-4000-8000-000000000012' THEN '18000000-0000-4000-8000-000000000032'::uuid ELSE '18000000-0000-4000-8000-000000000031'::uuid END
FROM memberships WHERE workspace_id='00000000-0000-0000-0000-000000000003' ON CONFLICT DO NOTHING;
INSERT INTO documents(id,tenant_id,workspace_id,title,content,status,classification_rank,compartment_ids,uploaded_by,label_version) VALUES
('18000000-0000-4000-8000-000000000041','00000000-0000-0000-0000-000000000002','00000000-0000-0000-0000-000000000003','Quarterly financial outlook','Financial outlook for the next quarter.','completed',2,ARRAY['18000000-0000-4000-8000-000000000021'::uuid],'18000000-0000-4000-8000-000000000011',1),
('18000000-0000-4000-8000-000000000042','00000000-0000-0000-0000-000000000002','00000000-0000-0000-0000-000000000003','Atlas partnership agreement','Agreement for the Atlas partnership.','completed',3,ARRAY['18000000-0000-4000-8000-000000000022'::uuid,'18000000-0000-4000-8000-000000000023'::uuid],'18000000-0000-4000-8000-000000000011',1),
('18000000-0000-4000-8000-000000000043','00000000-0000-0000-0000-000000000002','00000000-0000-0000-0000-000000000003','New acquisition notes','Needs a security review.','completed',5,'{}','18000000-0000-4000-8000-000000000011',1)
ON CONFLICT DO NOTHING;
INSERT INTO document_labels(document_id,tag_id) SELECT id,unnest(compartment_ids) FROM documents WHERE workspace_id='00000000-0000-0000-0000-000000000003' ON CONFLICT DO NOTHING;
INSERT INTO chunks(id,document_id,tenant_id,workspace_id,content,chunk_index) VALUES('18000000-0000-4000-8000-000000000051','18000000-0000-4000-8000-000000000041','00000000-0000-0000-0000-000000000002','00000000-0000-0000-0000-000000000003','Financial outlook for the next quarter.',0) ON CONFLICT DO NOTHING;
COMMIT;
\else
\echo 'Refusing to seed a non-SPEC-180 database'
\quit 1
\endif
