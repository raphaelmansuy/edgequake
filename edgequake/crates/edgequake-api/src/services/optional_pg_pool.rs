//! Cfg-safe optional Postgres pool handle for explicit data-access ports.
//!
//! `sqlx` is only linked when the `postgres` feature is on. Call sites and
//! helper signatures use [`OptionalPgPool`] so non-postgres builds still
//! compile while production paths pass `state.optional_pg_pool()`.

/// Optional borrow of a Postgres pool (or always-`None` without the feature).
#[cfg(feature = "postgres")]
pub type OptionalPgPool<'a> = Option<&'a sqlx::PgPool>;

/// Placeholder so `OptionalPgPool` exists without linking `sqlx`.
#[cfg(not(feature = "postgres"))]
#[derive(Debug, Clone, Copy)]
pub struct UnavailablePgPool;

/// Always `None` when Postgres is not compiled in.
#[cfg(not(feature = "postgres"))]
pub type OptionalPgPool<'a> = Option<&'a UnavailablePgPool>;

/// Explicit empty pool port (tests / non-postgres paths).
#[inline]
pub fn no_pg_pool<'a>() -> OptionalPgPool<'a> {
    None
}
