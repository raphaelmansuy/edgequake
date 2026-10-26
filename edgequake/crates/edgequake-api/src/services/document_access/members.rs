use super::*;
use serde::Serialize;

#[derive(Serialize, sqlx::FromRow)]
pub struct MemberChoice {
    pub user_id: Uuid,
    pub name: String,
}

/// A bounded steward lookup, separate from the administrative member directory.
pub async fn search_members(
    pool: &PgPool,
    actor: &AccessActor,
    query: &str,
) -> ApiResult<Vec<MemberChoice>> {
    let (mut tx, scope) = scoped_transaction(pool, actor, false).await?;
    if !scope.subject.steward {
        return Err(coded(
            StatusCode::FORBIDDEN,
            "ACCESS_CHANGE_FORBIDDEN",
            "Only security stewards can check a member",
        ));
    }
    let query = query.trim();
    if query.chars().count() < 2 {
        return Ok(Vec::new());
    }
    let pattern = format!(
        "%{}%",
        query
            .chars()
            .take(128)
            .collect::<String>()
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    let members=sqlx::query_as("SELECT DISTINCT u.user_id,COALESCE(NULLIF(u.display_name,''),u.username,u.email,'Workspace member') AS name FROM public.memberships m JOIN public.users u ON u.user_id=m.user_id WHERE m.workspace_id=$1 AND m.is_active AND u.is_active AND (u.username ILIKE $2 OR u.display_name ILIKE $2 OR u.email ILIKE $2) ORDER BY name,user_id LIMIT 20")
        .bind(actor.workspace).bind(pattern).fetch_all(&mut *tx).await.map_err(database)?;
    tx.commit().await.map_err(database)?;
    Ok(members)
}
