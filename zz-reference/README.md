# `zz-reference/` — Grounded Reference for EdgeQuake Storage

> **Code is law.** Every fact in this tree is anchored to an official source:
> the upstream README/manual, or a file in this repository. If you cannot
> find a link, treat the claim as unverified.

EdgeQuake's PostgreSQL adapter rests on two extensions:

```
+-----------------------------------+
|         EdgeQuake API             |
+-----------------+-----------------+
|  pgvector       |  Apache AGE     |
|  (embeddings)   |  (graph nodes)  |
+-----------------+-----------------+
|         PostgreSQL 13 - 17        |
+-----------------------------------+
```

This reference exists to give engineers the **shortest path to a correct
implementation or optimization**.

## Layout

| Folder                             | Purpose                                  |
| ---------------------------------- | ---------------------------------------- |
| [001-pgvector/](001-pgvector/)     | Vector similarity search extension       |
| [002-apache-age/](002-apache-age/) | openCypher graph extension               |
| [zz-rawlinks.md](zz-rawlinks.md)   | Single source of truth for official URLs |

Each technology folder follows the same shape:

```
NNN-topic/
  README.md                  <- index + 30s overview
  000-mental-model.md        <- the one-screen picture (read first)
  001-why/                   <- 5-Whys + first principles
  002-fundamentals/          <- types, install, core API
  003-indexing/ or cypher/   <- the heavy lifting
  004-...                    <- performance / SQL integration
  005-edgequake-usage/       <- how this repo actually uses it
  006-faq/                   <- short, grounded answers
```

## Conventions

- File names: `NNN-kebab-case.md` (numbered for stable ordering)
- Folder names: `NNN-kebab-case/`
- Every non-trivial claim cites either the upstream doc URL or a path in
  `edgequake/` (`crates/...`, `migrations/...`, `docker/...`)
- ASCII diagrams use only `+`, `-`, `|`, `<`, `>`, `*` for portability
- No marketing language — only facts you can act on

## Authoritative sources

- pgvector: <https://github.com/pgvector/pgvector> (v0.8.2 at time of writing)
- Apache AGE: <https://age.apache.org/age-manual/master/index.html>
- EdgeQuake adapter: [edgequake/crates/edgequake-storage/src/adapters/postgres/](../edgequake/crates/edgequake-storage/src/adapters/postgres/)
