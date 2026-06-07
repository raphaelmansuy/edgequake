# `001-why/` — Why pgvector

See [001-five-whys.md](001-five-whys.md) for the full reasoning chain.

TL;DR:

> EdgeQuake stores embeddings **alongside** the entities, relationships,
> documents, and tenants that produced them. Keeping vectors out of Postgres
> would force two-system consistency, two backup paths, two security models,
> and two query languages — for the same data.
