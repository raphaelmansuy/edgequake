# Version matrix (PG16 / PG17 / PG18)

Default status after Phase 0–4 inventory: **pending live EXPLAIN capture** unless covered by existing e2e_spec061 matrix.

Stack pins: PG16/17/18 images · pgvector 0.8.5 · AGE 1.7+/1.8.0.

| Ref ID | PG16 | PG17 | PG18 | Behavioral deltas |
|---|---|---|---|---|
| `DATA-PGVEC-VECTORS-ANN-QUERY-001` | pending | pending | pending | iterative_scan ≥0.8; PG18 async I/O may cut heap fetch latency |
| `DATA-PGVEC-VECTORS-ANN-QUERY-FILTERED-002` | pending | pending | pending | iterative_scan ≥0.8; PG18 async I/O may cut heap fetch latency |
| `DATA-PG-VECTORS-TEXT-SEARCH-FILTERED-003` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PGVEC-VECTORS-UPSERT-BATCH-004` | pending | pending | pending | iterative_scan ≥0.8; PG18 async I/O may cut heap fetch latency |
| `DATA-PG-VECTORS-DELETE-BY-ID-005` | pending | pending | pending |  |
| `DATA-PG-VECTORS-DELETE-ENTITY-006` | pending | pending | pending |  |
| `DATA-PG-VECTORS-DELETE-ENTITIES-BATCH-007` | pending | pending | pending |  |
| `DATA-PG-VECTORS-DELETE-ENTITY-RELATIONS-008` | pending | pending | pending |  |
| `DATA-PG-VECTORS-GET-BY-ID-009` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-VECTORS-GET-BY-IDS-010` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-VECTORS-COUNT-011` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-VECTORS-IS-EMPTY-012` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-VECTORS-PING-013` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-VECTORS-CLEAR-014` | pending | pending | pending |  |
| `DATA-PG-VECTORS-CLEAR-WORKSPACE-015` | pending | pending | pending |  |
| `DATA-PG-VECTORS-DELETE-BY-DOCUMENT-016` | pending | pending | pending |  |
| `DATA-PGVEC-VECTORS-WARMUP-ANN-017` | pending | pending | pending | iterative_scan ≥0.8; PG18 async I/O may cut heap fetch latency |
| `DATA-PGVEC-VECTORS-DDL-CREATE-TABLE-018` | pending | pending | pending | iterative_scan ≥0.8; PG18 async I/O may cut heap fetch latency |
| `DATA-PGVEC-VECTORS-DDL-ENSURE-ANN-INDEX-019` | pending | pending | pending | iterative_scan ≥0.8; PG18 async I/O may cut heap fetch latency |
| `DATA-PGVEC-VECTORS-DDL-PARTIAL-HNSW-020` | pending | pending | pending | iterative_scan ≥0.8; PG18 async I/O may cut heap fetch latency |
| `DATA-PG-VECTORS-DDL-ENSURE-FTS-021` | pending | pending | pending |  |
| `DATA-PGVEC-VECTORS-SESSION-SEARCH-TUNING-022` | pending | pending | pending | iterative_scan ≥0.8; PG18 async I/O may cut heap fetch latency |
| `DATA-PG-VECTORS-WS-DROP-TABLE-023` | pending | pending | pending |  |
| `DATA-PGVEC-VECTORS-DIM-RECONCILE-024` | pending | pending | pending | iterative_scan ≥0.8; PG18 async I/O may cut heap fetch latency |
| `DATA-AGE-GRAPH-HAS-NODE-025` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-GET-NODE-026` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-NODE-DEGREE-027` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-NODE-DEGREES-BATCH-028` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-GET-ALL-NODES-029` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-GET-NODES-BY-IDS-030` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-GET-NODES-BATCH-031` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-GET-EDGES-FOR-NODES-BATCH-032` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-HAS-EDGE-033` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-GET-EDGE-034` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-GET-NODE-EDGES-035` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-GET-INCIDENT-EDGES-BATCH-036` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-GET-ALL-EDGES-037` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-GET-KNOWLEDGE-GRAPH-038` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-GET-POPULAR-LABELS-039` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-SEARCH-LABELS-040` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-SEARCH-NODES-041` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-GET-NEIGHBORS-042` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-GET-POPULAR-NODES-DEGREE-043` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-GET-EDGES-FOR-NODE-SET-044` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-UPSERT-NODE-045` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-UPSERT-NODES-BATCH-046` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-DELETE-NODE-047` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-DELETE-NODES-BATCH-048` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-DELETE-NODE-SCOPED-049` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-UPSERT-EDGE-050` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-UPSERT-EDGES-BATCH-051` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-DELETE-EDGE-052` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-DELETE-EDGES-BATCH-053` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-DELETE-EDGE-SCOPED-054` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-CLEAR-055` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-CLEAR-WORKSPACE-056` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-NODE-COUNT-057` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-EDGE-COUNT-058` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-NODE-COUNT-FAST-059` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-EDGE-COUNT-FAST-060` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-NODE-COUNT-BY-WORKSPACE-061` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-EDGE-COUNT-BY-WORKSPACE-062` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-DISTINCT-NODE-TYPE-COUNT-063` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-NODE-COUNT-BY-SOURCE-PREFIX-064` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-NODE-COUNTS-BY-SOURCE-PREFIXES-065` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-LIST-NODES-FILTERED-066` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-LIST-EDGES-FILTERED-067` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-FIND-NODES-BY-SOURCE-PREFIXES-068` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-FIND-EDGES-BY-SOURCE-PREFIXES-069` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-FIND-EDGE-BY-RELATIONSHIP-ID-070` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-CYPHER-EXEC-071` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-LIFECYCLE-ENSURE-INDEXES-072` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-COPY-LOAD-VERTICES-073` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-AGE-GRAPH-SESSION-LOAD-AGE-074` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-PG-KV-GET-BY-ID-075` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-KV-GET-BY-IDS-076` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-KV-GET-BY-IDS-ORDERED-077` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-KV-FILTER-KEYS-078` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-KV-UPSERT-079` | pending | pending | pending |  |
| `DATA-PG-KV-DELETE-080` | pending | pending | pending |  |
| `DATA-PG-KV-COUNT-081` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-KV-IS-EMPTY-082` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-KV-PING-083` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-KV-COUNT-EMBEDDED-CHUNKS-084` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-KV-KEYS-WITH-PREFIX-085` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-KV-KEYS-WITH-PREFIX-LIMITED-086` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-KV-KEYS-WITH-SUFFIX-087` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-KV-KEYS-WITH-SUFFIX-LIMITED-088` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-KV-KEYS-089` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-KV-CLEAR-090` | pending | pending | pending |  |
| `DATA-PG-KV-TRANSITION-IF-STATUS-091` | pending | pending | pending |  |
| `DATA-PG-KV-DDL-CREATE-TABLE-092` | pending | pending | pending |  |
| `DATA-PG-PDF-STORE-093` | pending | pending | pending |  |
| `DATA-PG-PDF-GET-094` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-PDF-UPDATE-MARKDOWN-095` | pending | pending | pending |  |
| `DATA-PG-PDF-UPDATE-STATUS-096` | pending | pending | pending |  |
| `DATA-PG-PDF-LINK-TO-DOCUMENT-097` | pending | pending | pending |  |
| `DATA-PG-PDF-LIST-098` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-PDF-DELETE-099` | pending | pending | pending |  |
| `DATA-PG-PDF-CLEAR-MARKDOWN-100` | pending | pending | pending |  |
| `DATA-PG-DOCS-ENSURE-RECORD-101` | pending | pending | pending |  |
| `DATA-PG-DOCS-UPDATE-STATS-102` | pending | pending | pending |  |
| `DATA-PG-DOCS-TOUCH-STATUS-103` | pending | pending | pending |  |
| `DATA-PG-DOCS-DELETE-RECORD-104` | pending | pending | pending |  |
| `DATA-PG-PDF-COUNT-105` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-DOCS-LIST-SUMMARIES-106` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-DOCS-DELETE-WORKSPACE-107` | pending | pending | pending |  |
| `DATA-PG-ORIGINAL-STORE-108` | pending | pending | pending |  |
| `DATA-PG-MM-ASSET-STORE-109` | pending | pending | pending |  |
| `DATA-PG-CONV-CREATE-110` | pending | pending | pending |  |
| `DATA-PG-CONV-GET-111` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-CONV-UPDATE-112` | pending | pending | pending |  |
| `DATA-PG-CONV-DELETE-113` | pending | pending | pending |  |
| `DATA-PG-CONV-LIST-114` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-CONV-SHARE-115` | pending | pending | pending |  |
| `DATA-PG-CONV-UNSHARE-116` | pending | pending | pending |  |
| `DATA-PG-CONV-GET-SHARED-117` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-CONV-MSG-CREATE-118` | pending | pending | pending |  |
| `DATA-PG-CONV-MSG-UPDATE-119` | pending | pending | pending |  |
| `DATA-PG-CONV-MSG-GET-120` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-CONV-MSG-DELETE-121` | pending | pending | pending |  |
| `DATA-PG-CONV-MSG-LIST-122` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-CONV-FOLDER-CREATE-123` | pending | pending | pending |  |
| `DATA-PG-CONV-FOLDER-LIST-124` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-CONV-FOLDER-UPDATE-125` | pending | pending | pending |  |
| `DATA-PG-CONV-FOLDER-GET-126` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-CONV-FOLDER-DELETE-127` | pending | pending | pending |  |
| `DATA-PG-CONV-BULK-DELETE-128` | pending | pending | pending |  |
| `DATA-PG-CONV-BULK-ARCHIVE-129` | pending | pending | pending |  |
| `DATA-PG-CONV-BULK-MOVE-130` | pending | pending | pending |  |
| `DATA-PG-TASKS-CREATE-131` | pending | pending | pending |  |
| `DATA-PG-TASKS-GET-132` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-TASKS-TOUCH-133` | pending | pending | pending |  |
| `DATA-PG-TASKS-UPDATE-134` | pending | pending | pending |  |
| `DATA-PG-TASKS-DELETE-135` | pending | pending | pending |  |
| `DATA-PG-TASKS-LIST-136` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-TASKS-STATS-137` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-TASKS-FIND-ACTIVE-PDF-138` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-TASKS-FIND-ACTIVE-INGEST-139` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-TASKS-CLAIM-NEXT-140` | pending | pending | pending |  |
| `DATA-PG-TASKS-REFRESH-LEASE-141` | pending | pending | pending |  |
| `DATA-PG-TASKS-RELEASE-CLAIM-142` | pending | pending | pending |  |
| `DATA-PG-TASKS-QUEUE-METRICS-143` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-TASKS-TOTAL-COUNT-144` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-TENANT-CREATE-145` | pending | pending | pending |  |
| `DATA-PG-TENANT-GET-146` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-TENANT-GET-BY-SLUG-147` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-TENANT-UPDATE-148` | pending | pending | pending |  |
| `DATA-PG-TENANT-DELETE-149` | pending | pending | pending |  |
| `DATA-PG-TENANT-LIST-150` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-WORKSPACE-CREATE-151` | pending | pending | pending |  |
| `DATA-PG-WORKSPACE-GET-152` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-WORKSPACE-GET-BY-SLUG-153` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-WORKSPACE-UPDATE-154` | pending | pending | pending |  |
| `DATA-PG-WORKSPACE-DELETE-155` | pending | pending | pending |  |
| `DATA-PG-WORKSPACE-LIST-156` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-AGE-WORKSPACE-GET-STATS-157` | pending | pending | pending | AGE 1.7+ auto id indexes; property UNIQUE still required |
| `DATA-PG-MEMBERSHIP-ADD-158` | pending | pending | pending |  |
| `DATA-PG-MEMBERSHIP-GET-USER-159` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-MEMBERSHIP-GET-TENANT-160` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-MEMBERSHIP-UPDATE-ROLE-161` | pending | pending | pending |  |
| `DATA-PG-MEMBERSHIP-REMOVE-162` | pending | pending | pending |  |
| `DATA-PG-MEMBERSHIP-CHECK-TENANT-163` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-MEMBERSHIP-CHECK-WORKSPACE-164` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-MEMBERSHIP-GET-ROLE-165` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-QUOTA-UPDATE-TENANT-166` | pending | pending | pending |  |
| `DATA-PG-METRICS-RECORD-SNAPSHOT-167` | pending | pending | pending |  |
| `DATA-PG-METRICS-GET-HISTORY-168` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-AUTH-SYNC-USER-169` | pending | pending | pending |  |
| `DATA-PG-AUTH-ENSURE-DEFAULT-TENANT-WS-170` | pending | pending | pending |  |
| `DATA-PG-AUTH-SYNC-MEMBERSHIP-171` | pending | pending | pending |  |
| `DATA-PG-AUTH-VERIFY-MEMBERSHIP-172` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-AUTH-LOAD-USER-173` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-AUTH-FIND-USER-BY-LOGIN-174` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-AUTH-LIST-USERS-175` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-AUTH-DELETE-USER-176` | pending | pending | pending |  |
| `DATA-PG-SESSION-PERSIST-REFRESH-177` | pending | pending | pending |  |
| `DATA-PG-SESSION-LOAD-REFRESH-178` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-SESSION-REVOKE-REFRESH-179` | pending | pending | pending |  |
| `DATA-PG-SESSION-PERSIST-API-KEY-180` | pending | pending | pending |  |
| `DATA-PG-SESSION-LIST-API-KEYS-181` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-SESSION-FIND-API-KEY-PREFIX-182` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-SESSION-REVOKE-API-KEY-183` | pending | pending | pending |  |
| `DATA-PG-ENTITY-UPSERT-184` | pending | pending | pending |  |
| `DATA-PG-ENTITY-REMOVE-SOURCES-185` | pending | pending | pending |  |
| `DATA-PG-LINEAGE-RECORD-ENTITY-LINK-186` | pending | pending | pending |  |
| `DATA-PG-LINEAGE-RECORD-RELATION-LINK-187` | pending | pending | pending |  |
| `DATA-PG-LINEAGE-RECORD-RELATION-LINKS-BATCH-188` | pending | pending | pending |  |
| `DATA-PG-LINEAGE-RECORD-ENTITY-LINKS-BATCH-189` | pending | pending | pending |  |
| `DATA-PG-LINEAGE-APPEND-DESC-HISTORY-190` | pending | pending | pending |  |
| `DATA-PG-LINEAGE-LOAD-DOC-FROM-CHUNKS-191` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-FAILED-CHUNKS-INSERT-192` | pending | pending | pending |  |
| `DATA-PG-FAILED-CHUNKS-LIST-193` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-FAILED-CHUNKS-MARK-STATUS-194` | pending | pending | pending |  |
| `DATA-PG-RLS-SET-TENANT-CONTEXT-195` | pending | pending | pending |  |
| `DATA-PG-RLS-CLEAR-TENANT-CONTEXT-196` | pending | pending | pending |  |
| `DATA-PG-POOL-ACQUIRE-CONNECT-197` | pending | pending | pending |  |
| `DATA-PG-AUDIT-WRITE-EVENT-198` | pending | pending | pending |  |
| `DATA-PG-AUDIT-QUERY-LOGS-199` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-CONFIG-LOAD-LLM-DEFAULTS-200` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-CONFIG-SAVE-LLM-DEFAULTS-201` | pending | pending | pending |  |
| `DATA-PG-CONFIG-LOAD-PRIORITY-MODE-202` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-CONFIG-SAVE-PRIORITY-MODE-203` | pending | pending | pending |  |
| `DATA-PG-KEYWORDS-CACHE-GET-204` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-KEYWORDS-CACHE-SET-205` | pending | pending | pending |  |
| `DATA-PG-KEYWORDS-CACHE-DELETE-206` | pending | pending | pending |  |
| `DATA-PG-KEYWORDS-CACHE-INIT-207` | pending | pending | pending |  |
| `DATA-PG-STATS-ENSURE-ROW-COUNT-208` | pending | pending | pending |  |
| `DATA-PG-ID-ALLOCATE-DOCUMENT-209` | pending | pending | pending |  |
| `DATA-PG-INSPECT-CHECK-EXTENSIONS-210` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-INSPECT-CHECK-TABLES-211` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-INSPECT-CHECK-INVARIANTS-212` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-INSPECT-APPLY-REPAIR-213` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIGRATE-RUNNER-214` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-INIT-BASE-215` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-TASKS-TABLE-216` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-CONVERSATION-TABLE-217` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-AUDIT-LOG-218` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-RLS-POLICIES-219` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-AGE-GRAPH-220` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-FULLTEXT-SEARCH-221` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-FAILED-CHUNKS-222` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-PDF-DOCUMENTS-223` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-VECTOR-BTREE-INDEXES-224` | pending | pending | pending | PG18 skip scan may use more composite btrees |
| `DATA-PG-SCHEMA-MIG-SOURCE-IDS-GIN-225` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-CQRS-ENTITIES-226` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-CHUNK-LINEAGE-227` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-AGE-INDEXES-CONSOLIDATE-228` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-HNSW-OPTIMIZE-229` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-HALFVEC-EMBEDDINGS-230` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-DOCUMENT-ORIGINALS-231` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-MM-ASSETS-232` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-TASK-LEASE-233` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-MERGE-GRAPH-PROPS-234` | pending | pending | pending |  |
| `DATA-PG-SCHEMA-MIG-EQ-ID-DENORM-235` | pending | pending | pending |  |
