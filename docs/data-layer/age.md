# Apache AGE data-layer operations (`DATA-AGE-*`)


> **IMP-031-01 (2026-07):** `get_nodes_by_ids` / `get_node` / `has_node` use native `UNNEST` + UNIQUE node_id — **O(K log N)** one RT. Cypher IN removed from request path.

Cypher + native SQL over AGE label tables. Sources: [AGE docs](https://age.apache.org/age-manual/master/index.html), AGE 1.8.0.

## DATA-AGE-GRAPH-HAS-NODE-025

<a id="data-age-graph-has-node-025"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-HAS-NODE-025` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | HAS-NODE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:30` |
| **Entry** | `GraphStorage::has_node` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_HAS_NODE_025` |
| **Benchmark** | [benchmarks/025.md](./benchmarks/025.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-GET-NODE-026

<a id="data-age-graph-get-node-026"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-GET-NODE-026` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | GET-NODE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:33` |
| **Entry** | `GraphStorage::get_node` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_GET_NODE_026` |
| **Benchmark** | [benchmarks/026.md](./benchmarks/026.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-NODE-DEGREE-027

<a id="data-age-graph-node-degree-027"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-NODE-DEGREE-027` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | NODE-DEGREE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:37` |
| **Entry** | `GraphStorage::node_degree` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_NODE_DEGREE_027` |
| **Benchmark** | [benchmarks/027.md](./benchmarks/027.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-NODE-DEGREES-BATCH-028

<a id="data-age-graph-node-degrees-batch-028"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-NODE-DEGREES-BATCH-028` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | NODE-DEGREES-BATCH |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:41` |
| **Entry** | `GraphStorage::node_degrees_batch` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_NODE_DEGREES_BATCH_028` |
| **Benchmark** | [benchmarks/028.md](./benchmarks/028.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-GET-ALL-NODES-029

<a id="data-age-graph-get-all-nodes-029"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-GET-ALL-NODES-029` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | GET-ALL-NODES |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:45` |
| **Entry** | `GraphStorage::get_all_nodes` |
| **Type** | R |
| **Transactional** | Y |
| **Notes** | FORBIDDEN request path |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_GET_ALL_NODES_029` |
| **Benchmark** | [benchmarks/029.md](./benchmarks/029.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-GET-NODES-BY-IDS-030

<a id="data-age-graph-get-nodes-by-ids-030"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-GET-NODES-BY-IDS-030` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | GET-NODES-BY-IDS |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:49` |
| **Entry** | `GraphStorage::get_nodes_by_ids` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_GET_NODES_BY_IDS_030` |
| **Benchmark** | [benchmarks/030.md](./benchmarks/030.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-GET-NODES-BATCH-031

<a id="data-age-graph-get-nodes-batch-031"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-GET-NODES-BATCH-031` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | GET-NODES-BATCH |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:53` |
| **Entry** | `GraphStorage::get_nodes_batch` |
| **Type** | R |
| **Transactional** | Y |
| **Notes** | native SQL preferred |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_GET_NODES_BATCH_031` |
| **Benchmark** | [benchmarks/031.md](./benchmarks/031.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-GET-EDGES-FOR-NODES-BATCH-032

<a id="data-age-graph-get-edges-for-nodes-batch-032"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-GET-EDGES-FOR-NODES-BATCH-032` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | GET-EDGES-FOR-NODES-BATCH |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:57` |
| **Entry** | `GraphStorage::get_edges_for_nodes_batch` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_GET_EDGES_FOR_NODES_BATCH_032` |
| **Benchmark** | [benchmarks/032.md](./benchmarks/032.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-HAS-EDGE-033

<a id="data-age-graph-has-edge-033"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-HAS-EDGE-033` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | HAS-EDGE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:68` |
| **Entry** | `GraphStorage::has_edge` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_HAS_EDGE_033` |
| **Benchmark** | [benchmarks/033.md](./benchmarks/033.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-GET-EDGE-034

<a id="data-age-graph-get-edge-034"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-GET-EDGE-034` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | GET-EDGE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:72` |
| **Entry** | `GraphStorage::get_edge` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_GET_EDGE_034` |
| **Benchmark** | [benchmarks/034.md](./benchmarks/034.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-GET-NODE-EDGES-035

<a id="data-age-graph-get-node-edges-035"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-GET-NODE-EDGES-035` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | GET-NODE-EDGES |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:76` |
| **Entry** | `GraphStorage::get_node_edges` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_GET_NODE_EDGES_035` |
| **Benchmark** | [benchmarks/035.md](./benchmarks/035.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-GET-INCIDENT-EDGES-BATCH-036

<a id="data-age-graph-get-incident-edges-batch-036"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-GET-INCIDENT-EDGES-BATCH-036` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | GET-INCIDENT-EDGES-BATCH |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:80` |
| **Entry** | `GraphStorage::get_incident_edges_batch` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_GET_INCIDENT_EDGES_BATCH_036` |
| **Benchmark** | [benchmarks/036.md](./benchmarks/036.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-GET-ALL-EDGES-037

<a id="data-age-graph-get-all-edges-037"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-GET-ALL-EDGES-037` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | GET-ALL-EDGES |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:92` |
| **Entry** | `GraphStorage::get_all_edges` |
| **Type** | R |
| **Transactional** | Y |
| **Notes** | FORBIDDEN request path |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_GET_ALL_EDGES_037` |
| **Benchmark** | [benchmarks/037.md](./benchmarks/037.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-GET-KNOWLEDGE-GRAPH-038

<a id="data-age-graph-get-knowledge-graph-038"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-GET-KNOWLEDGE-GRAPH-038` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | GET-KNOWLEDGE-GRAPH |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:96` |
| **Entry** | `GraphStorage::get_knowledge_graph` |
| **Type** | R |
| **Transactional** | Y |
| **Notes** | bounded expand |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_GET_KNOWLEDGE_GRAPH_038` |
| **Benchmark** | [benchmarks/038.md](./benchmarks/038.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-GET-POPULAR-LABELS-039

<a id="data-age-graph-get-popular-labels-039"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-GET-POPULAR-LABELS-039` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | GET-POPULAR-LABELS |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:108` |
| **Entry** | `GraphStorage::get_popular_labels` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_GET_POPULAR_LABELS_039` |
| **Benchmark** | [benchmarks/039.md](./benchmarks/039.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-SEARCH-LABELS-040

<a id="data-age-graph-search-labels-040"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-SEARCH-LABELS-040` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | SEARCH-LABELS |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:118` |
| **Entry** | `GraphStorage::search_labels` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_SEARCH_LABELS_040` |
| **Benchmark** | [benchmarks/040.md](./benchmarks/040.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-SEARCH-NODES-041

<a id="data-age-graph-search-nodes-041"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-SEARCH-NODES-041` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | SEARCH-NODES |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:129` |
| **Entry** | `GraphStorage::search_nodes` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_SEARCH_NODES_041` |
| **Benchmark** | [benchmarks/041.md](./benchmarks/041.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-GET-NEIGHBORS-042

<a id="data-age-graph-get-neighbors-042"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-GET-NEIGHBORS-042` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | GET-NEIGHBORS |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:141` |
| **Entry** | `GraphStorage::get_neighbors` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_GET_NEIGHBORS_042` |
| **Benchmark** | [benchmarks/042.md](./benchmarks/042.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-GET-POPULAR-NODES-DEGREE-043

<a id="data-age-graph-get-popular-nodes-degree-043"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-GET-POPULAR-NODES-DEGREE-043` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | GET-POPULAR-NODES-DEGREE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:152` |
| **Entry** | `GraphStorage::get_popular_nodes_with_degree` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_GET_POPULAR_NODES_DEGREE_043` |
| **Benchmark** | [benchmarks/043.md](./benchmarks/043.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-GET-EDGES-FOR-NODE-SET-044

<a id="data-age-graph-get-edges-for-node-set-044"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-GET-EDGES-FOR-NODE-SET-044` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | GET-EDGES-FOR-NODE-SET |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:170` |
| **Entry** | `GraphStorage::get_edges_for_node_set` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_GET_EDGES_FOR_NODE_SET_044` |
| **Benchmark** | [benchmarks/044.md](./benchmarks/044.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-UPSERT-NODE-045

<a id="data-age-graph-upsert-node-045"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-UPSERT-NODE-045` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | UPSERT-NODE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:184` |
| **Entry** | `GraphStorage::upsert_node` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_UPSERT_NODE_045` |
| **Benchmark** | [benchmarks/045.md](./benchmarks/045.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-UPSERT-NODES-BATCH-046

<a id="data-age-graph-upsert-nodes-batch-046"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-UPSERT-NODES-BATCH-046` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | UPSERT-NODES-BATCH |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:191` |
| **Entry** | `GraphStorage::upsert_nodes_batch` |
| **Type** | W |
| **Transactional** | Y |
| **Notes** | native ON CONFLICT |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_UPSERT_NODES_BATCH_046` |
| **Benchmark** | [benchmarks/046.md](./benchmarks/046.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-DELETE-NODE-047

<a id="data-age-graph-delete-node-047"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-DELETE-NODE-047` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | DELETE-NODE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:198` |
| **Entry** | `GraphStorage::delete_node` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_DELETE_NODE_047` |
| **Benchmark** | [benchmarks/047.md](./benchmarks/047.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-DELETE-NODES-BATCH-048

<a id="data-age-graph-delete-nodes-batch-048"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-DELETE-NODES-BATCH-048` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | DELETE-NODES-BATCH |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:202` |
| **Entry** | `GraphStorage::delete_nodes_batch` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_DELETE_NODES_BATCH_048` |
| **Benchmark** | [benchmarks/048.md](./benchmarks/048.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-DELETE-NODE-SCOPED-049

<a id="data-age-graph-delete-node-scoped-049"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-DELETE-NODE-SCOPED-049` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | DELETE-NODE-SCOPED |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:206` |
| **Entry** | `GraphStorage::delete_node_scoped` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_DELETE_NODE_SCOPED_049` |
| **Benchmark** | [benchmarks/049.md](./benchmarks/049.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-UPSERT-EDGE-050

<a id="data-age-graph-upsert-edge-050"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-UPSERT-EDGE-050` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | UPSERT-EDGE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:216` |
| **Entry** | `GraphStorage::upsert_edge` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_UPSERT_EDGE_050` |
| **Benchmark** | [benchmarks/050.md](./benchmarks/050.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-UPSERT-EDGES-BATCH-051

<a id="data-age-graph-upsert-edges-batch-051"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-UPSERT-EDGES-BATCH-051` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | UPSERT-EDGES-BATCH |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:225` |
| **Entry** | `GraphStorage::upsert_edges_batch` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_UPSERT_EDGES_BATCH_051` |
| **Benchmark** | [benchmarks/051.md](./benchmarks/051.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-DELETE-EDGE-052

<a id="data-age-graph-delete-edge-052"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-DELETE-EDGE-052` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | DELETE-EDGE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:232` |
| **Entry** | `GraphStorage::delete_edge` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_DELETE_EDGE_052` |
| **Benchmark** | [benchmarks/052.md](./benchmarks/052.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-DELETE-EDGES-BATCH-053

<a id="data-age-graph-delete-edges-batch-053"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-DELETE-EDGES-BATCH-053` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | DELETE-EDGES-BATCH |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:236` |
| **Entry** | `GraphStorage::delete_edges_batch` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_DELETE_EDGES_BATCH_053` |
| **Benchmark** | [benchmarks/053.md](./benchmarks/053.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-DELETE-EDGE-SCOPED-054

<a id="data-age-graph-delete-edge-scoped-054"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-DELETE-EDGE-SCOPED-054` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | DELETE-EDGE-SCOPED |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:240` |
| **Entry** | `GraphStorage::delete_edge_scoped` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_DELETE_EDGE_SCOPED_054` |
| **Benchmark** | [benchmarks/054.md](./benchmarks/054.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-CLEAR-055

<a id="data-age-graph-clear-055"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-CLEAR-055` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | CLEAR |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:251` |
| **Entry** | `GraphStorage::clear` |
| **Type** | W |
| **Transactional** | Y |
| **Notes** | ADMIN |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_CLEAR_055` |
| **Benchmark** | [benchmarks/055.md](./benchmarks/055.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-CLEAR-WORKSPACE-056

<a id="data-age-graph-clear-workspace-056"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-CLEAR-WORKSPACE-056` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | CLEAR-WORKSPACE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:255` |
| **Entry** | `GraphStorage::clear_workspace` |
| **Type** | W |
| **Transactional** | Y |
| **Notes** | ADMIN |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_CLEAR_WORKSPACE_056` |
| **Benchmark** | [benchmarks/056.md](./benchmarks/056.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-NODE-COUNT-057

<a id="data-age-graph-node-count-057"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-NODE-COUNT-057` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | NODE-COUNT |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:263` |
| **Entry** | `GraphStorage::node_count` |
| **Type** | R |
| **Transactional** | Y |
| **Notes** | O(N) exact |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_NODE_COUNT_057` |
| **Benchmark** | [benchmarks/057.md](./benchmarks/057.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-EDGE-COUNT-058

<a id="data-age-graph-edge-count-058"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-EDGE-COUNT-058` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | EDGE-COUNT |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:266` |
| **Entry** | `GraphStorage::edge_count` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_EDGE_COUNT_058` |
| **Benchmark** | [benchmarks/058.md](./benchmarks/058.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-NODE-COUNT-FAST-059

<a id="data-age-graph-node-count-fast-059"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-NODE-COUNT-FAST-059` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | NODE-COUNT-FAST |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:270` |
| **Entry** | `GraphStorage::node_count_fast` |
| **Type** | R |
| **Transactional** | Y |
| **Notes** | reltuples |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_NODE_COUNT_FAST_059` |
| **Benchmark** | [benchmarks/059.md](./benchmarks/059.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-EDGE-COUNT-FAST-060

<a id="data-age-graph-edge-count-fast-060"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-EDGE-COUNT-FAST-060` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | EDGE-COUNT-FAST |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:274` |
| **Entry** | `GraphStorage::edge_count_fast` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_EDGE_COUNT_FAST_060` |
| **Benchmark** | [benchmarks/060.md](./benchmarks/060.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-NODE-COUNT-BY-WORKSPACE-061

<a id="data-age-graph-node-count-by-workspace-061"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-NODE-COUNT-BY-WORKSPACE-061` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | NODE-COUNT-BY-WORKSPACE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:278` |
| **Entry** | `GraphStorage::node_count_by_workspace` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_NODE_COUNT_BY_WORKSPACE_061` |
| **Benchmark** | [benchmarks/061.md](./benchmarks/061.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-EDGE-COUNT-BY-WORKSPACE-062

<a id="data-age-graph-edge-count-by-workspace-062"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-EDGE-COUNT-BY-WORKSPACE-062` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | EDGE-COUNT-BY-WORKSPACE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:286` |
| **Entry** | `GraphStorage::edge_count_by_workspace` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_EDGE_COUNT_BY_WORKSPACE_062` |
| **Benchmark** | [benchmarks/062.md](./benchmarks/062.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-DISTINCT-NODE-TYPE-COUNT-063

<a id="data-age-graph-distinct-node-type-count-063"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-DISTINCT-NODE-TYPE-COUNT-063` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | DISTINCT-NODE-TYPE-COUNT |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:290` |
| **Entry** | `GraphStorage::distinct_node_type_count_by_workspace` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_DISTINCT_NODE_TYPE_COUNT_063` |
| **Benchmark** | [benchmarks/063.md](./benchmarks/063.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-NODE-COUNT-BY-SOURCE-PREFIX-064

<a id="data-age-graph-node-count-by-source-prefix-064"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-NODE-COUNT-BY-SOURCE-PREFIX-064` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | NODE-COUNT-BY-SOURCE-PREFIX |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:298` |
| **Entry** | `GraphStorage::node_count_by_source_prefix` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_NODE_COUNT_BY_SOURCE_PREFIX_064` |
| **Benchmark** | [benchmarks/064.md](./benchmarks/064.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-NODE-COUNTS-BY-SOURCE-PREFIXES-065

<a id="data-age-graph-node-counts-by-source-prefixes-065"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-NODE-COUNTS-BY-SOURCE-PREFIXES-065` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | NODE-COUNTS-BY-SOURCE-PREFIXES |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:302` |
| **Entry** | `GraphStorage::node_counts_by_source_prefixes` |
| **Type** | R |
| **Transactional** | Y |
| **Notes** | batched list reconcile |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_NODE_COUNTS_BY_SOURCE_PREFIXES_065` |
| **Benchmark** | [benchmarks/065.md](./benchmarks/065.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-LIST-NODES-FILTERED-066

<a id="data-age-graph-list-nodes-filtered-066"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-LIST-NODES-FILTERED-066` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | LIST-NODES-FILTERED |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:313` |
| **Entry** | `GraphStorage::list_nodes_filtered` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_LIST_NODES_FILTERED_066` |
| **Benchmark** | [benchmarks/066.md](./benchmarks/066.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-LIST-EDGES-FILTERED-067

<a id="data-age-graph-list-edges-filtered-067"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-LIST-EDGES-FILTERED-067` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | LIST-EDGES-FILTERED |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:321` |
| **Entry** | `GraphStorage::list_edges_filtered` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_LIST_EDGES_FILTERED_067` |
| **Benchmark** | [benchmarks/067.md](./benchmarks/067.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-FIND-NODES-BY-SOURCE-PREFIXES-068

<a id="data-age-graph-find-nodes-by-source-prefixes-068"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-FIND-NODES-BY-SOURCE-PREFIXES-068` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | FIND-NODES-BY-SOURCE-PREFIXES |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:330` |
| **Entry** | `GraphStorage::find_nodes_by_source_prefixes` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_FIND_NODES_BY_SOURCE_PREFIXES_068` |
| **Benchmark** | [benchmarks/068.md](./benchmarks/068.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-FIND-EDGES-BY-SOURCE-PREFIXES-069

<a id="data-age-graph-find-edges-by-source-prefixes-069"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-FIND-EDGES-BY-SOURCE-PREFIXES-069` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | FIND-EDGES-BY-SOURCE-PREFIXES |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:339` |
| **Entry** | `GraphStorage::find_edges_by_source_prefixes` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_FIND_EDGES_BY_SOURCE_PREFIXES_069` |
| **Benchmark** | [benchmarks/069.md](./benchmarks/069.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-FIND-EDGE-BY-RELATIONSHIP-ID-070

<a id="data-age-graph-find-edge-by-relationship-id-070"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-FIND-EDGE-BY-RELATIONSHIP-ID-070` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | FIND-EDGE-BY-RELATIONSHIP-ID |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:348` |
| **Entry** | `GraphStorage::find_edge_by_relationship_id` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_FIND_EDGE_BY_RELATIONSHIP_ID_070` |
| **Benchmark** | [benchmarks/070.md](./benchmarks/070.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-CYPHER-EXEC-071

<a id="data-age-graph-cypher-exec-071"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-CYPHER-EXEC-071` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | CYPHER-EXEC |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers/cypher_exec.rs:1` |
| **Entry** | `execute_cypher / cypher_query` |
| **Type** | R/W |
| **Transactional** | Y |
| **Notes** | AGE session wrapper |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_CYPHER_EXEC_071` |
| **Benchmark** | [benchmarks/071.md](./benchmarks/071.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-LIFECYCLE-ENSURE-INDEXES-072

<a id="data-age-graph-lifecycle-ensure-indexes-072"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-LIFECYCLE-ENSURE-INDEXES-072` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | LIFECYCLE-ENSURE-INDEXES |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers/graph_lifecycle.rs:1` |
| **Entry** | `ensure_indexes` |
| **Type** | DDL |
| **Transactional** | N |
| **Notes** | boot-time index reconcile |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_LIFECYCLE_ENSURE_INDEXES_072` |
| **Benchmark** | [benchmarks/072.md](./benchmarks/072.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-COPY-LOAD-VERTICES-073

<a id="data-age-graph-copy-load-vertices-073"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-COPY-LOAD-VERTICES-073` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | COPY-LOAD-VERTICES |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/age_csv_loader.rs:1` |
| **Entry** | `load_vertices_from_csv` |
| **Type** | W |
| **Transactional** | Y |
| **Notes** | COPY bulk |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_COPY_LOAD_VERTICES_073` |
| **Benchmark** | [benchmarks/073.md](./benchmarks/073.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-GRAPH-SESSION-LOAD-AGE-074

<a id="data-age-graph-session-load-age-074"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-GRAPH-SESSION-LOAD-AGE-074` |
| **Engine** | AGE |
| **Domain** | GRAPH |
| **Operation** | SESSION-LOAD-AGE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers/session.rs:1` |
| **Entry** | `set_age_session / search_path` |
| **Type** | SESSION |
| **Transactional** | Y |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_GRAPH_SESSION_LOAD_AGE_074` |
| **Benchmark** | [benchmarks/074.md](./benchmarks/074.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-AGE-WORKSPACE-GET-STATS-157

<a id="data-age-workspace-get-stats-157"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-AGE-WORKSPACE-GET-STATS-157` |
| **Engine** | AGE |
| **Domain** | WORKSPACE |
| **Operation** | GET-STATS |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/workspace_ops.rs:421` |
| **Entry** | `pg_get_workspace_stats` |
| **Type** | R |
| **Transactional** | Y |
| **Notes** | secondary: PG |
| **Variables** | N=nodes, E=edges, K=batch, depth=hops, branch=avg degree |
| **Time** | O(K log N) batch ID; O(branch^depth) expand; O(N) full scan FORBIDDEN |
| **Space** | O(K) or O(branch^depth) |
| **I/O** | property index / edge ends |
| **Failure mode** | cartesian expansion / timeout / OOM on unbounded MATCH |
| **Tests** | `data_layer_*` containing `DATA_AGE_WORKSPACE_GET_STATS_157` |
| **Benchmark** | [benchmarks/157.md](./benchmarks/157.md) |

**Limits**

- Native writes preferred (UNIQUE node_id)
- Cypher MERGE debug-only
- Traversal must be depth-bounded
- No native graph index types — use PG btree/GIN on properties

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.
