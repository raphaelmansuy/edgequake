# Installing Apache AGE

Sources: [AGE Manual → Installation](https://age.apache.org/age-manual/master/intro/setup.html),
[Apache AGE GitHub](https://github.com/apache/age).

## Build from source

The repo has one branch per supported Postgres major. Per the upstream
`git ls-remote`, branches **PG11** through **PG18** exist, and `master`
tracks the latest (currently AGE 1.7.0, targeting Postgres 18 — see the
[AGE README](https://github.com/apache/age#readme) and `age.control`'s
`default_version = '1.7.0'`). Pick the branch that matches your server.

```bash
git clone https://github.com/apache/age.git
cd age
git checkout PG17                    # or PG16, PG15, ... PG18
make PG_CONFIG=/usr/local/pgsql/bin/pg_config
sudo make PG_CONFIG=/usr/local/pgsql/bin/pg_config install
```

## Enable per database

```sql
CREATE EXTENSION IF NOT EXISTS age CASCADE;
LOAD 'age';
SET search_path = ag_catalog, "$user", public;
```

Three things to remember:

1. `CREATE EXTENSION` is **once per database**.
2. `LOAD 'age'` is **once per session/connection** before any AGE call.
3. `search_path` must include `ag_catalog` so the `cypher()` function and
   `agtype` operators are resolved without schema qualification.

Source: [AGE Manual → Setup](https://age.apache.org/age-manual/master/intro/setup.html).

## EdgeQuake does this for you

[edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs#L135](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs):

```rust
sqlx::query("CREATE EXTENSION IF NOT EXISTS age CASCADE")
    .execute(pool).await;  // warn-on-fail (AGE is optional)

sqlx::query("LOAD 'age'").execute(&mut *conn).await?;
sqlx::query(r#"SET search_path = ag_catalog, "$user", public"#)
    .execute(&mut *conn).await?;
```

EdgeQuake re-runs `LOAD 'age'` for each pooled connection because the
extension state is **per backend**, not per session.

## Docker

Apache publishes images under [`apache/age`](https://hub.docker.com/r/apache/age/).
EdgeQuake's quickstart Compose stack uses a custom image that combines
`pgvector` + `age` because both are needed.

## Verifying

```sql
SELECT extversion FROM pg_extension WHERE extname = 'age';
SELECT * FROM ag_catalog.ag_graph;   -- empty until you create_graph()
```

## EdgeQuake's defensive availability check

[edgequake/migrations/013_add_age_graph.sql](../../../../edgequake/migrations/013_add_age_graph.sql)
ships a `is_age_available()` PL/pgSQL function so application code can
branch on whether AGE was installed at deploy time.

```sql
SELECT public.is_age_available();   -- boolean
```
