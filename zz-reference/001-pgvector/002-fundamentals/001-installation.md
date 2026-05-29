# Installing pgvector

Source: [pgvector README → Installation](https://github.com/pgvector/pgvector#installation).

## Supported Postgres versions

The upstream README states: *"supports any version supported by Postgres
(currently Postgres 13+)"*. Docker images are published for `pg13`
through the latest supported major; check
[pgvector Docker tags](https://hub.docker.com/r/pgvector/pgvector/tags)
for the exact list at install time.

## Method 1 — official Docker image (recommended for dev)

```bash
docker pull pgvector/pgvector:pg17
```

Tags follow the pattern `pgN-bookworm` / `pgN-trixie` for N in 13..18.
Source: [pgvector README → Docker](https://github.com/pgvector/pgvector#docker).

EdgeQuake bundles this in [docker-compose.quickstart.yml](../../../../docker-compose.quickstart.yml).

## Method 2 — package manager

```bash
# Debian / Ubuntu (replace 17 with your PG major)
sudo apt install postgresql-17-pgvector

# Homebrew (only postgresql@17 and @18 formulas)
brew install pgvector
```

Source: [pgvector README → APT](https://github.com/pgvector/pgvector#apt),
[Homebrew](https://github.com/pgvector/pgvector#homebrew).

## Method 3 — from source

```bash
cd /tmp
git clone --branch v0.8.2 https://github.com/pgvector/pgvector.git
cd pgvector
make
sudo make install
```

If you have multiple Postgres installations, point at the right one:

```bash
export PG_CONFIG=/Library/PostgreSQL/17/bin/pg_config
sudo --preserve-env=PG_CONFIG make install
```

Source: [pgvector README → Postgres Location](https://github.com/pgvector/pgvector#postgres-location).

## Enabling the extension

```sql
CREATE EXTENSION IF NOT EXISTS vector;
```

This is exactly what EdgeQuake runs at pool init:

> [edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs#L124](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs)

```rust
sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
    .execute(pool)
    .await
```

## Verifying the install

```sql
SELECT extversion FROM pg_extension WHERE extname = 'vector';
-- 0.8.2
```

## Upgrading

```sql
ALTER EXTENSION vector UPDATE;
```

Source: [pgvector README → Upgrading](https://github.com/pgvector/pgvector#upgrading).

## Common install failures

| Symptom                                         | Cause                             | Fix                                                                 |
| ----------------------------------------------- | --------------------------------- | ------------------------------------------------------------------- |
| `fatal error: postgres.h: No such file`         | `postgresql-server-dev-N` missing | `apt install postgresql-server-dev-17`                              |
| `Illegal instruction` after copying binary      | Built with `-march=native`        | Rebuild with `make OPTFLAGS=""`                                     |
| `Failed to create vector extension` (EdgeQuake) | Extension files not installed     | Install the OS package; see EdgeQuake error path in `connection.rs` |
