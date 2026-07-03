//! Per-migration reconcile hooks (SRP — one module per migration family).

use sqlx::PgPool;

/// Execute bootstrap `apply.sql` scripts (multi-statement; not compatible with `sqlx::query`).
pub(super) async fn execute_bootstrap_apply_sql(
    pool: &PgPool,
    sql: &str,
) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(sql).execute(pool).await?;
    Ok(())
}

mod m038;
mod m040;
mod m041;
mod m042;
mod m043;
mod m044;
mod m045;
mod m046;
mod m047;
mod m048;
mod m049;
mod m050;
mod m051;
mod m052;
mod m053;
mod m054;
mod m055;
mod m056;
mod m057;
mod m058;
mod m059;
mod m060;
mod m061;
mod m062;
mod m063;
mod m064;
mod m065;
mod m078;

pub(super) use m038::reconcile_migration_038;
pub(super) use m040::reconcile_migration_040_background;
pub(super) use m041::reconcile_migration_041;
pub(super) use m042::reconcile_migration_042;
pub(super) use m043::reconcile_migration_043;
pub(super) use m044::reconcile_migration_044;
pub(super) use m045::reconcile_migration_045;
pub(super) use m046::reconcile_migration_046;
pub(super) use m047::reconcile_migration_047;
pub(super) use m048::reconcile_migration_048;
pub(super) use m049::reconcile_migration_049;
pub(super) use m050::reconcile_migration_050;
pub(super) use m051::reconcile_migration_051;
pub(super) use m052::reconcile_migration_052;
pub(super) use m053::reconcile_migration_053;
pub(super) use m054::reconcile_migration_054;
pub(super) use m055::reconcile_migration_055;
pub(super) use m056::reconcile_migration_056;
pub(super) use m057::reconcile_migration_057;
pub(super) use m058::reconcile_migration_058;
pub(super) use m059::reconcile_migration_059;
pub(super) use m060::reconcile_migration_060;
pub(super) use m061::reconcile_migration_061;
pub(super) use m062::reconcile_migration_062;
pub(super) use m063::reconcile_migration_063;
pub(super) use m064::reconcile_migration_064;
pub(super) use m065::reconcile_migration_065;
pub(super) use m078::{reconcile_migration_078, repair_migration_078_checksum_if_needed};
