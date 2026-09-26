//! PostgreSQL registry for immutable scope data bindings (SPEC-149).

use async_trait::async_trait;
use edgequake_storage_contracts::{
    AccessError, AccessResult, AccessScope, BindingRegistry, BindingRole, BindingState,
    DataBindingDescriptor, P0_REQUIRED_ROLES,
};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
struct BindingRow {
    binding_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Uuid,
    role: String,
    provider: String,
    config_ref: String,
    layout: String,
    physical_index: String,
    model_descriptor: Option<String>,
    generation: i64,
    state: String,
}

impl BindingRow {
    fn into_descriptor(self) -> AccessResult<DataBindingDescriptor> {
        let generation = u64::try_from(self.generation)
            .map_err(|_| AccessError::CorruptData("negative binding generation".into()))?;
        Ok(DataBindingDescriptor {
            binding_id: self.binding_id,
            scope: AccessScope::new(
                edgequake_storage_contracts::TenantId::new(self.tenant_id),
                edgequake_storage_contracts::WorkspaceId::new(self.workspace_id),
            ),
            role: BindingRole::parse(&self.role)?,
            provider: self.provider,
            config_ref: self.config_ref,
            layout: self.layout,
            physical_index: self.physical_index,
            model_descriptor: self.model_descriptor,
            generation,
            state: BindingState::parse(&self.state)?,
        })
    }
}

/// Deterministic P0 graph/vector binding provisioner.
#[derive(Clone)]
pub struct PgBindingRegistry {
    pool: PgPool,
}

impl PgBindingRegistry {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Stable UUID derived from scope + role so reboots do not create duplicates.
    pub fn deterministic_binding_id(scope: &AccessScope, role: BindingRole) -> Uuid {
        let name = format!(
            "edgequake:p0:{}:{}:{}",
            scope.tenant().into_uuid(),
            scope.workspace().into_uuid(),
            role.as_str()
        );
        Uuid::new_v5(&Uuid::NAMESPACE_OID, name.as_bytes())
    }

    fn p0_descriptor(scope: AccessScope, role: BindingRole) -> DataBindingDescriptor {
        Self::descriptor_for_role(scope, role)
    }

    /// Shared P0 descriptor construction for committer transaction provisioning.
    pub fn descriptor_for_role(scope: AccessScope, role: BindingRole) -> DataBindingDescriptor {
        let (provider, config_ref, layout, physical_index) = match role {
            BindingRole::Graph | BindingRole::GraphProjection => {
                ("age", "p0.age", "colocated", "age")
            }
            BindingRole::Vector | BindingRole::VectorProjection | BindingRole::Embedding => {
                ("pgvector", "p0.pgvector_colocated", "typed", "pgvector")
            }
        };
        DataBindingDescriptor {
            binding_id: Self::deterministic_binding_id(&scope, role),
            scope,
            role,
            provider: provider.into(),
            config_ref: config_ref.into(),
            layout: layout.into(),
            physical_index: physical_index.into(),
            model_descriptor: None,
            generation: 1,
            state: BindingState::Active,
        }
    }

    async fn upsert_descriptor(
        &self,
        descriptor: &DataBindingDescriptor,
    ) -> AccessResult<DataBindingDescriptor> {
        sqlx::query(
            r#"
            INSERT INTO public.data_bindings (
                binding_id, tenant_id, workspace_id, role, provider, config_ref,
                layout, physical_index, model_descriptor, generation, state
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (binding_id) DO NOTHING
            "#,
        )
        .bind(descriptor.binding_id)
        .bind(descriptor.scope.tenant().into_uuid())
        .bind(descriptor.scope.workspace().into_uuid())
        .bind(descriptor.role.as_str())
        .bind(&descriptor.provider)
        .bind(&descriptor.config_ref)
        .bind(&descriptor.layout)
        .bind(&descriptor.physical_index)
        .bind(descriptor.model_descriptor.as_deref())
        .bind(
            i64::try_from(descriptor.generation)
                .map_err(|_| AccessError::InvalidInput("binding generation exceeds i64".into()))?,
        )
        .bind(descriptor.state.as_str())
        .execute(&self.pool)
        .await
        .map_err(|error| AccessError::Unavailable(format!("upsert data binding: {error}")))?;
        Ok(descriptor.clone())
    }
}

#[async_trait]
impl BindingRegistry for PgBindingRegistry {
    async fn ensure_scope_bindings(
        &self,
        scope: &AccessScope,
        roles: &[BindingRole],
    ) -> AccessResult<Vec<DataBindingDescriptor>> {
        let roles = if roles.is_empty() {
            P0_REQUIRED_ROLES
        } else {
            roles
        };
        let mut out = Vec::with_capacity(roles.len());
        for role in roles {
            let descriptor = Self::p0_descriptor(*scope, *role);
            out.push(self.upsert_descriptor(&descriptor).await?);
        }
        Ok(out)
    }

    async fn get_binding(&self, binding_id: Uuid) -> AccessResult<Option<DataBindingDescriptor>> {
        let row = sqlx::query_as::<_, BindingRow>(
            r#"
            SELECT binding_id, tenant_id, workspace_id, role, provider, config_ref,
                   layout, physical_index, model_descriptor, generation, state
            FROM public.data_bindings
            WHERE binding_id = $1
            "#,
        )
        .bind(binding_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| AccessError::Unavailable(format!("load data binding: {error}")))?;
        row.map(BindingRow::into_descriptor).transpose()
    }

    async fn list_active(&self, scope: &AccessScope) -> AccessResult<Vec<DataBindingDescriptor>> {
        let rows = sqlx::query_as::<_, BindingRow>(
            r#"
            SELECT binding_id, tenant_id, workspace_id, role, provider, config_ref,
                   layout, physical_index, model_descriptor, generation, state
            FROM public.data_bindings
            WHERE tenant_id = $1 AND workspace_id = $2 AND state = 'active'
            ORDER BY role, binding_id
            "#,
        )
        .bind(scope.tenant().into_uuid())
        .bind(scope.workspace().into_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| AccessError::Unavailable(format!("list active bindings: {error}")))?;
        rows.into_iter().map(BindingRow::into_descriptor).collect()
    }
}
