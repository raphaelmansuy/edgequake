//! Owned scopes and non-bypass temporary tables for typed ANN measurements.
use sqlx::PgPool;
use uuid::Uuid;

pub async fn seed(pool: &PgPool) -> (Uuid, Uuid) {
    let (tenant, workspace) = (Uuid::new_v4(), Uuid::new_v4());
    let slug = format!("typed-ann-{tenant}");
    sqlx::query("INSERT INTO public.tenants(tenant_id,name,slug) VALUES($1,$2,$2)")
        .bind(tenant)
        .bind(&slug)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO public.workspaces(workspace_id,tenant_id,name,slug) VALUES($1,$2,$3,$3)",
    )
    .bind(workspace)
    .bind(tenant)
    .bind(&slug)
    .execute(pool)
    .await
    .unwrap();
    (tenant, workspace)
}

pub async fn secure_temp_tables(pool: &PgPool) {
    let schema: String = sqlx::query_scalar(
        "SELECT quote_ident(nspname) FROM pg_namespace WHERE oid=pg_my_temp_schema()",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "GRANT USAGE ON SCHEMA {schema} TO edgequake_tenant_access"
    ))
    .execute(pool)
    .await
    .unwrap();
    for (table, tenant_column) in [
        ("embedding_models", false),
        ("chunks", true),
        ("entities", true),
        ("relationships", true),
        ("chunk_embeddings", false),
        ("entity_embeddings", false),
        ("relationship_embeddings", false),
        ("report_embeddings", false),
    ] {
        sqlx::query(&format!(
            "GRANT SELECT ON {schema}.{table} TO edgequake_tenant_access"
        ))
        .execute(pool)
        .await
        .unwrap();
        if table == "embedding_models" {
            continue;
        }
        let predicate = if tenant_column {
            "tenant_id=public.current_tenant_id() AND workspace_id=public.current_workspace_id()"
        } else {
            "workspace_id=public.current_workspace_id()"
        };
        for statement in [
            format!("ALTER TABLE {schema}.{table} ENABLE ROW LEVEL SECURITY"),
            format!("ALTER TABLE {schema}.{table} FORCE ROW LEVEL SECURITY"),
            format!("CREATE POLICY fixture_scope ON {schema}.{table} TO edgequake_tenant_access USING ({predicate}) WITH CHECK ({predicate})"),
        ] {
            sqlx::query(&statement).execute(pool).await.unwrap();
        }
    }
}

pub async fn cleanup(pool: &PgPool, workspace: Uuid) {
    let tenant: Uuid = sqlx::query_scalar(
        "DELETE FROM public.workspaces WHERE workspace_id=$1 RETURNING tenant_id",
    )
    .bind(workspace)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("DELETE FROM public.tenants WHERE tenant_id=$1")
        .bind(tenant)
        .execute(pool)
        .await
        .unwrap();
}
