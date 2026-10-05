//! Scope-specialized lookups that keep the composite entity index usable in
//! custom and generic prepared plans. NULL scopes still mean NULL, never all.

fn scope_predicate(workspace_present: bool, tenant_present: bool) -> String {
    let workspace = if workspace_present {
        "workspace_id = $2"
    } else {
        "workspace_id IS NULL AND $2::uuid IS NULL"
    };
    let tenant = if tenant_present {
        "tenant_id = $3"
    } else {
        "tenant_id IS NULL AND $3::uuid IS NULL"
    };
    // Keep all three bind slots typed even when a scope is absent.
    format!("{workspace} AND {tenant}")
}

pub(super) fn entity_lookup_sql(workspace_present: bool, tenant_present: bool) -> String {
    let scope = scope_predicate(workspace_present, tenant_present);
    format!(
        "SELECT id FROM entities WHERE {scope}
         AND name = ANY(ARRAY[$1, ($2::uuid)::text || '::' || $1])
         ORDER BY CASE WHEN name = $1 THEN 0 ELSE 1 END LIMIT 1"
    )
}

pub(super) fn entity_batch_lookup_sql(workspace_present: bool, tenant_present: bool) -> String {
    let scope = scope_predicate(workspace_present, tenant_present);
    format!("SELECT id, name FROM entities WHERE {scope} AND name = ANY($1)")
}
