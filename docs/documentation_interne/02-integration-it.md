---
title: "EdgeQuake — Guide d'intégration IT"
version: "0.26.9"
audience: "Équipes d'exploitation, DBA, supervision, sécurité opérationnelle"
---

# EdgeQuake — Guide d'intégration IT

> **Produit** : EdgeQuake v0.26.9 · **Schéma base** : migrations jusqu'à **149**
> **Documents liés** : [Déploiement technique](01-deploiement-technique.md) · [Deep dive architecture & algorithme](03-deep-dive-architecture-algorithme.md)

Ce guide s'adresse aux équipes IT qui **exploitent** EdgeQuake au quotidien. Il
couvre les procédures d'exploitation, la supervision, la sauvegarde, la mise à jour et
le rollback.

---

## Sommaire

1. [Modèle d'exploitation et responsabilités](#1-modèle-dexploitation-et-responsabilités)
2. [Procédures d'exploitation](#2-procédures-dexploitation)
3. [Monitoring](#3-monitoring)
4. [Sauvegarde et restauration](#4-sauvegarde-et-restauration)
5. [Mise à jour](#5-mise-à-jour)
6. [Rollback](#6-rollback)
7. [Runbooks d'incident](#7-runbooks-dincident)
8. [Capacité et dimensionnement](#8-capacité-et-dimensionnement)
9. [Checklists](#9-checklists)

---

## 1. Modèle d'exploitation et responsabilités

### 1.1 Ce qu'il faut retenir avant tout

Trois faits structurent toute l'exploitation d'EdgeQuake :

| #     | Fait                                                                  | Conséquence opérationnelle                                                                              |
| ----- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| **1** | **Tout l'état est dans PostgreSQL.** Aucun état applicatif hors base. | Sauvegarder la base = sauvegarder le système. L'API et l'UI sont remplaçables à chaud.                  |
| **2** | **L'API ne migre jamais la base.**                                    | La migration est un acte d'exploitation explicite et ordonnancé. Ne jamais compter sur un auto-upgrade. |
| **3** | **Certaines migrations sont irréversibles.**                          | Après application, le rollback n'existe plus : seule la restauration de sauvegarde ramène en arrière.   |

### 1.2 Répartition indicative des responsabilités

| Activité                            | Exploitation | DBA   | Sécurité | Métier |
| ----------------------------------- | ------------ | ----- | -------- | ------ |
| Démarrage / arrêt / redémarrage     | **R**        | C     | I        | I      |
| Supervision et astreinte            | **R**        | C     | I        | I      |
| Sauvegarde et tests de restauration | C            | **R** | I        | I      |
| Application des migrations          | C            | **R** | I        | A      |
| Mise à jour de version              | **R**        | C     | C        | A      |
| Rotation des secrets                | **R**        | C     | **A**    | I      |
| Gestion des comptes et des rôles    | **R**        | I     | **A**    | C      |
| Choix du fournisseur LLM            | C            | I     | **A**    | **R**  |

_R = réalise, A = approuve, C = consulté, I = informé._

---

## 2. Procédures d'exploitation

### 2.1 Démarrage

```bash
# Ordre imposé par les conditions de santé : PostgreSQL → API → Web UI
docker compose -f docker-compose.prod.yml up -d
docker compose -f docker-compose.prod.yml ps
```

> `docker-compose.prod.yml` désigne, dans tout ce guide, le fichier de production
> de l'exploitant — dérivé du modèle `docker-compose.quickstart.yml` avec les
> durcissements du [document 01 §7](01-deploiement-technique.md#7-configuration-sécurité).

Validation du démarrage :

```bash
curl -sf http://API:8080/live   || echo "processus non démarré"
curl -sf http://API:8080/ready  || echo "non apte à servir le trafic"
curl -s  http://API:8080/health | jq '.status, .version'
```

> **`/ready` renvoie 503 au démarrage ?** Ce n'est pas nécessairement une anomalie :
> la sonde couvre la migration, l'état du stockage, la présence des index ANN et la
> pression de la file. Consulter `/health` pour connaître la cause exacte
> (§ [7.1](#71-ready-répond-503)).

### 2.2 Arrêt

```bash
# Arrêt propre : les tâches en cours terminent leur bail, les baux ne sont pas renouvelés
docker compose -f docker-compose.prod.yml stop api
docker compose -f docker-compose.prod.yml down          # arrêt complet
```

L'arrêt est **sûr en cours d'ingestion** : les tâches non terminées restent en état
`Pending`/`Processing` en base ; leur bail expire et un worker les reprend au
redémarrage. Aucune perte de travail admis.

> Ne **jamais** utiliser `down -v` : l'option supprime le volume PostgreSQL, donc
> l'intégralité des données.

### 2.3 Redémarrage

```bash
docker compose -f docker-compose.prod.yml restart api
```

Un redémarrage de l'API seule est non destructif et n'exige aucune fenêtre de
maintenance si la file est faible. En cas de file chargée, laisser d'abord se vider :

```bash
curl -s http://API:8080/api/v1/pipeline/queue-metrics | jq
```

### 2.4 Consultation de l'état

| Besoin                                  | Commande                                                               |
| --------------------------------------- | ---------------------------------------------------------------------- |
| Version en service                      | `curl -s http://API:8080/version`                                      |
| Santé détaillée                         | `curl -s http://API:8080/health \| jq`                                 |
| File de tâches                          | `curl -s http://API:8080/api/v1/pipeline/queue-metrics \| jq`          |
| Activité du pipeline                    | `curl -s http://API:8080/api/v1/pipeline/status \| jq`                 |
| Santé des fournisseurs LLM              | `curl -s http://API:8080/api/v1/models/health \| jq`                   |
| Intégrité du stockage                   | `curl -s http://API:8080/api/v1/admin/storage/inspect \| jq` _(admin)_ |
| Statut du schéma                        | `edgequake migrate status`                                             |
| Plan de migration (avant MAJ)           | `edgequake migrate plan`                                               |
| Feu vert avant suppression irréversible | `edgequake migrate guard [--family <f>]`                               |

### 2.5 Gestion de la file d'ingestion

```bash
# Lister les tâches
curl -s http://API:8080/api/v1/tasks | jq

# Annuler une ingestion (annulation durable — état terminal « Cancelled »)
curl -X POST http://API:8080/api/v1/tasks/{track_id}/cancel

# Relancer une tâche en échec
curl -X POST http://API:8080/api/v1/tasks/{track_id}/retry

# Débloquer des documents restés en cours après un incident
curl -X POST http://API:8080/api/v1/documents/recover-stuck
```

L'annulation est durable : l'interface affiche **Stopping…** jusqu'à l'état terminal
`Cancelled` (et non `Failed`). Les tâches `Pending` survivent au redémarrage du
processus grâce au mécanisme claim/lease PostgreSQL.

### 2.6 Purge et rétention

| Objet                | Rétention par défaut           | Réglage                         |
| -------------------- | ------------------------------ | ------------------------------- |
| Tâches terminales    | **30 jours**                   | `EDGEQUAKE_TASK_RETENTION_DAYS` |
| Documents et graphe  | illimitée                      | Suppression métier via l'API    |
| Journaux applicatifs | selon la politique de collecte | Configuration du collecteur     |

Suppression métier avec évaluation d'impact préalable :

```bash
curl -s http://API:8080/api/v1/documents/{id}/deletion-impact | jq
curl -X DELETE http://API:8080/api/v1/documents/{id}
```

L'évaluation d'impact indique les entités et relations qui deviendront orphelines —
à consulter avant toute suppression de masse.

### 2.7 Rotation des secrets

| Secret                  | Procédure                                                        | Impact                                                      |
| ----------------------- | ---------------------------------------------------------------- | ----------------------------------------------------------- |
| `JWT_SECRET`            | Mise à jour puis redémarrage de l'API                            | **Invalide toutes les sessions** — reconnexion utilisateurs |
| Clé du fournisseur LLM  | Mise à jour puis redémarrage                                     | Les tâches en vol échouent puis sont reprises               |
| Mot de passe PostgreSQL | `ALTER ROLE` puis mise à jour de `DATABASE_URL` puis redémarrage | Coupure brève                                               |
| Clés d'API applicatives | `DELETE /api/v1/api-keys/{id}` puis création                     | Impacte les intégrations concernées                         |

Planifier les rotations de `JWT_SECRET` hors heures ouvrées.

---

## 3. Monitoring

### 3.1 Sondes de santé

| Endpoint   | Sémantique                                   | Usage       | Action sur échec                                                     |
| ---------- | -------------------------------------------- | ----------- | -------------------------------------------------------------------- |
| `/live`    | Le processus répond                          | _Liveness_  | Redémarrer le conteneur                                              |
| `/ready`   | Apte à recevoir du trafic (200 / **503**)    | _Readiness_ | **Retirer du répartiteur de charge** — ne pas redémarrer aveuglément |
| `/health`  | Diagnostic détaillé : `healthy` / `degraded` | Supervision | Analyser la cause                                                    |
| `/metrics` | Exposition Prometheus (texte)                | Collecte    | —                                                                    |

Ce qui fait basculer `/ready` en 503 :

- schéma de base non aligné avec le binaire (migration requise) ;
- composant de stockage indisponible (KV, vecteurs ou graphe) ;
- **index ANN (HNSW) manquant** — posture _fail-closed_ : plutôt refuser le trafic
  que servir des résultats silencieusement dégradés ;
- pression excessive de la file de tâches.

Ce qui fait basculer `/health` en `degraded` : un composant de stockage en défaut, une
migration en état dégradé, ou une file saturée.

> **Règle d'astreinte** : un `503` sur `/ready` avec un `/live` à 200 n'est **pas** un
> plantage. Redémarrer ne corrige rien et fait perdre le diagnostic. Lire `/health`
> d'abord.

### 3.2 Métriques Prometheus

Collecte : `GET http://API:8080/metrics` (format texte Prometheus, sans
authentification — **à filtrer au réseau**, cf. [01 §7.5](01-deploiement-technique.md#75-endpoints-non-authentifiés--à-filtrer-au-réseau)).

**Métriques de service**

| Métrique                                  | Type        | Usage                               |
| ----------------------------------------- | ----------- | ----------------------------------- |
| `edgequake_http_requests_total`           | compteur    | Trafic, taux d'erreur par code      |
| `edgequake_http_request_duration_seconds` | histogramme | Latence API (p50/p95/p99)           |
| `edgequake_query_requests_total`          | compteur    | Volume d'interrogations             |
| `edgequake_query_duration_seconds`        | histogramme | Latence RAG bout en bout            |
| `edgequake_query_arm_duration_seconds`    | histogramme | Latence par branche de récupération |

**Métriques d'ingestion**

| Métrique                                         | Type        | Usage                                             |
| ------------------------------------------------ | ----------- | ------------------------------------------------- |
| `edgequake_task_queue_pending`                   | jauge       | **Profondeur de file — indicateur clé**           |
| `edgequake_task_queue_processing`                | jauge       | Tâches en cours de traitement                     |
| `edgequake_task_queue_failed`                    | jauge       | Tâches en échec                                   |
| `edgequake_task_transitions_total`               | compteur    | Transitions d'état                                |
| `edgequake_document_processing_total`            | compteur    | Documents traités                                 |
| `edgequake_document_processing_duration_seconds` | histogramme | Durée par document                                |
| `edgequake_ingest_stage_duration_seconds`        | histogramme | Durée par étape (chunk / extract / embed / store) |
| `edgequake_ingestion_failures_total`             | compteur    | Échecs d'ingestion                                |
| `edgequake_extract_retry_total`                  | compteur    | Reprises d'extraction (troncature LLM)            |

**Métriques LLM et coûts**

| Métrique                                        | Type        | Usage                                 |
| ----------------------------------------------- | ----------- | ------------------------------------- |
| `edgequake_llm_requests_total`                  | compteur    | Appels par fournisseur, taux d'erreur |
| `edgequake_llm_request_duration_seconds`        | histogramme | Latence fournisseur                   |
| `edgequake_provider_slots_inflight`             | jauge       | Concurrence en vol                    |
| `edgequake_provider_slot_hold_duration_seconds` | histogramme | Attente d'un créneau                  |
| `edgequake_ollama_network_error_total`          | compteur    | Erreurs réseau Ollama on-premise      |

**Métriques de stockage et de qualité**

| Métrique                                       | Type        | Usage                                   |
| ---------------------------------------------- | ----------- | --------------------------------------- |
| `edgequake_db_pool_connections`                | jauge       | Saturation du pool                      |
| `edgequake_storage_errors_total`               | compteur    | Erreurs de stockage                     |
| `edgequake_storage_op_duration_seconds`        | histogramme | Latence des opérations                  |
| `edgequake_vector_ann_index_missing`           | jauge       | **Index HNSW absent — bloque `/ready`** |
| `edgequake_vector_dim_mismatch_rejected_total` | compteur    | Incohérence de dimension d'embedding    |
| `edgequake_storage_drift_critical`             | jauge       | Dérive critique du stockage             |
| `edgequake_graph_quality_nodes` / `_edges`     | jauge       | Taille du graphe                        |
| `edgequake_graph_quality_orphan_rate`          | jauge       | Taux de nœuds orphelins (qualité)       |
| `edgequake_graph_quality_sparse`               | jauge       | Graphe anormalement clairsemé           |
| `edgequake_rate_limit_exceeded_total`          | compteur    | Dépassements de quota                   |

### 3.3 Seuils d'alerte proposés

| Alerte              | Condition                                               | Sévérité     | Action                                             |
| ------------------- | ------------------------------------------------------- | ------------ | -------------------------------------------------- |
| `EdgeQuakeDown`     | `/live` injoignable > 2 min                             | **Critique** | Redémarrer, escalader                              |
| `EdgeQuakeNotReady` | `/ready` = 503 > 5 min                                  | **Critique** | Lire `/health`, cf. §7.1                           |
| `EdgeQuakeDegraded` | `/health.status == "degraded"` > 10 min                 | Majeure      | Identifier le composant                            |
| `AnnIndexMissing`   | `edgequake_vector_ann_index_missing > 0`                | **Critique** | Reconstruire l'index, cf. §7.4                     |
| `QueueBacklog`      | `edgequake_task_queue_pending > 500` pendant 15 min     | Majeure      | Vérifier le fournisseur LLM, augmenter les workers |
| `QueueStalled`      | `pending > 0` et `processing == 0` pendant 10 min       | **Critique** | Workers bloqués — cf. §7.3                         |
| `HighErrorRate`     | ratio HTTP 5xx > 5 % sur 5 min                          | Majeure      | Analyser les journaux                              |
| `LlmErrorRate`      | erreurs `edgequake_llm_requests_total` > 10 %           | Majeure      | Vérifier le fournisseur, le quota, la clé          |
| `HighQueryLatency`  | p95 `edgequake_query_duration_seconds` > 5 s            | Mineure      | Ajuster le mode, vérifier les index                |
| `DbPoolSaturation`  | `edgequake_db_pool_connections` > 90 % de la taille max | Majeure      | Élargir le pool ou réduire la concurrence          |
| `StorageDrift`      | `edgequake_storage_drift_critical > 0`                  | **Critique** | `admin/storage/inspect` puis `repair`              |
| `GraphOrphanRate`   | `edgequake_graph_quality_orphan_rate > 0.3`             | Mineure      | Qualité d'extraction à revoir                      |
| `BackupTooOld`      | dernière sauvegarde > 24 h                              | **Critique** | Voir §4                                            |

### 3.4 Journaux

Format structuré, niveau piloté par `RUST_LOG`.

| Environnement       | Valeur recommandée                       |
| ------------------- | ---------------------------------------- |
| Production          | `RUST_LOG=info`                          |
| Diagnostic ciblé    | `RUST_LOG=info,edgequake_pipeline=debug` |
| Diagnostic requêtes | `RUST_LOG=info,edgequake_query=debug`    |
| Diagnostic SQL      | `RUST_LOG=info,sqlx=debug`               |

> `RUST_LOG=debug` global en production **inonde** la collecte et dégrade les
> performances. Toujours cibler un crate.

**À router vers le SIEM** : les événements d'audit (`Authentication`, `Authorization`,
`SecurityViolation`, `RateLimitExceeded`, `ConfigChange`, `DataExport`, `TenantAccess`,
`WorkspaceAccess`), avec leur résultat (`Success` / `Failure` / `Blocked` / `Warning`)
et leur sévérité (`Low` → `Critical`).

Sur Loki/ELK, indexer a minima : `trace_id`, `tenant_id`, `workspace_id`, `track_id`,
`level`.

### 3.5 Traçage distribué

EdgeQuake émet des traces OpenTelemetry (OTLP/HTTP), avec des spans GenAI dédiés
(`rag.retrieval`) permettant d'analyser chaque branche de récupération.

```bash
OTEL_EXPORTER_OTLP_ENDPOINT=http://collecteur:4318
# Ou vers Langfuse (SPEC-124)
EDGEQUAKE_LANGFUSE_ENABLED=true
LANGFUSE_BASE_URL=https://langfuse.intra.{client}
LANGFUSE_PUBLIC_KEY=...
LANGFUSE_SECRET_KEY=...
```

Détail : [../OBSERVABILITY.md](../OBSERVABILITY.md) et
[../operations/monitoring.md](../operations/monitoring.md).

> ⚠️ **Langfuse — compatibilité de version** : l'export exige l'endpoint OTLP
> `/api/public/otel/v1/traces`, **absent des Langfuse antérieurs à la 3.22x**
> (404 constaté en 3.1). `export_active: true` n'atteste que de la présence des
> clés, jamais de l'arrivée des traces. Procédure et diagnostic :
> [04-langfuse-kubernetes.md](04-langfuse-kubernetes.md).

### 3.6 Supervision PostgreSQL

À superviser comme toute base critique, avec deux points spécifiques :

| Point                  | Requête / métrique               | Seuil                       |
| ---------------------- | -------------------------------- | --------------------------- |
| Connexions             | `pg_stat_activity`               | < 80 % de `max_connections` |
| Taille de la base      | `pg_database_size('edgequake')`  | Croissance et espace libre  |
| Requêtes lentes        | `pg_stat_statements`             | À analyser au-delà de 1 s   |
| **Gonflement (bloat)** | tables de chunks et d'embeddings | Autovacuum à surveiller     |
| **Index HNSW**         | présence et validité             | Conditionne `/ready`        |

---

## 4. Sauvegarde et restauration

### 4.1 Périmètre

**Un seul objet à sauvegarder : la base PostgreSQL.** Elle contient les documents
originaux, le markdown, les chunks, les embeddings, le graphe, les identités, l'audit
et la file de tâches. Les conteneurs API et UI sont sans état et se reconstruisent
depuis les images.

À sauvegarder par ailleurs, hors base : le fichier de configuration
(`docker-compose*.yml`, variables d'environnement **hors secrets**) et les secrets,
gérés dans le coffre d'entreprise.

### 4.2 Objectifs proposés

| Indicateur            | Cible    | Mécanisme                                 |
| --------------------- | -------- | ----------------------------------------- |
| **RPO**               | ≤ 15 min | Archivage WAL continu                     |
| **RTO**               | ≤ 2 h    | Restauration + redémarrage des conteneurs |
| Rétention quotidienne | 30 jours | Sauvegarde complète                       |
| Rétention mensuelle   | 12 mois  | Archive                                   |

_À arbitrer selon la criticité retenue par le métier._

### 4.3 Sauvegarde logique (référence)

```bash
# Format personnalisé, compressé, parallélisable à la restauration
pg_dump -Fc -Z6 \
  --dbname="$DATABASE_URL" \
  --file="/backup/edgequake-$(date +%Y%m%dT%H%M%SZ).dump"

# Contrôle d'intégrité de l'archive (obligatoire — une sauvegarde non vérifiée n'existe pas)
pg_restore --list /backup/edgequake-*.dump > /dev/null && echo "archive valide"
```

**Point de vigilance Apache AGE** : le graphe réside dans des schémas dédiés créés par
l'extension. Une sauvegarde `pg_dump` **de la base entière** les inclut. Ne jamais
restreindre le dump à `--schema=public` — le graphe serait perdu.

### 4.4 Sauvegarde physique (recommandée en production)

```bash
# Sauvegarde de base + archivage WAL continu → PITR
pg_basebackup -D /backup/base -Ft -z -P --dbname="$ADMIN_DATABASE_URL"
```

Configuration PostgreSQL : `archive_mode = on`, `archive_command` vers un stockage
distant. C'est le seul mécanisme qui tient un RPO de 15 minutes.

En déploiement conteneurisé mono-nœud, un instantané du volume
`edgequake-pg-data` est acceptable **à condition que le conteneur soit arrêté ou que
l'instantané soit cohérent au niveau du système de fichiers**. Un instantané à chaud
d'un volume actif sans cohérence garantie n'est **pas** une sauvegarde fiable.

### 4.5 Restauration

```bash
# 1. Arrêter les applicatifs (surtout pas la base)
docker compose stop api frontend

# 2. Restaurer
createdb edgequake_restore
pg_restore --dbname="postgres://…/edgequake_restore" --jobs=4 \
  /backup/edgequake-20260817T020000Z.dump

# 3. Vérifier les extensions et le schéma
psql "…/edgequake_restore" -c "SELECT extname, extversion FROM pg_extension
                               WHERE extname IN ('vector','age');"

# 4. Vérifier l'alignement schéma / binaire
DATABASE_URL="…/edgequake_restore" edgequake migrate status

# 5. Basculer DATABASE_URL puis redémarrer
docker compose up -d api frontend
curl -sf http://API:8080/ready
```

### 4.6 Test de restauration — obligatoire

> Une sauvegarde jamais restaurée est une hypothèse, pas une garantie.

Cadence proposée : **restauration complète en environnement de recette une fois par
trimestre**, avec consignation :

| Contrôle                              | Attendu                                            |
| ------------------------------------- | -------------------------------------------------- |
| Durée de restauration                 | Conforme au RTO                                    |
| `/ready` après restauration           | 200                                                |
| Nombre de documents                   | Identique à la source                              |
| Nombre de nœuds et d'arêtes du graphe | Identique (`/health` ou métriques `graph_quality`) |
| Interrogation de contrôle             | Réponse cohérente avec sources                     |

### 4.7 Sauvegarde avant opération sensible

Sauvegarde **obligatoire** avant : toute mise à jour de version comportant des
migrations, toute exécution de `migrate --confirm-drop`, toute suppression de masse,
toute montée de version majeure de PostgreSQL.

---

## 5. Mise à jour

### 5.1 Principe

Le binaire et le schéma de base ont un contrat de version strict. À chaque
démarrage, l'API vérifie l'alignement ; en cas d'écart, elle **refuse de démarrer avec
le code de sortie 78** (`EX_CONFIG`). Ce code est distinct d'un plantage, ce qui
permet à un orchestrateur de brancher automatiquement sur « migration requise ».

Séquence canonique : **sauvegarde → `migrate dry-run` → `migrate` → démarrage des
nouveaux binaires**.

### 5.2 Procédure standard (mise à jour mineure ou correctif)

```bash
# 1. Fenêtre de maintenance ouverte, trafic dérouté
# 2. Attendre le vidage de la file d'ingestion
curl -s http://API:8080/api/v1/pipeline/queue-metrics | jq '.pending'

# 3. Arrêter les applicatifs (la base reste en service)
docker compose stop api frontend

# 4. SAUVEGARDE — non négociable
pg_dump -Fc --dbname="$DATABASE_URL" --file="/backup/pre-upgrade-$(date +%Y%m%d).dump"
pg_restore --list /backup/pre-upgrade-*.dump > /dev/null   # vérification

# 5. Récupérer les nouvelles images
docker compose pull

# 6. Simulation de migration : liste les migrations en attente
#    et signale explicitement les suppressions irréversibles
docker run --rm -e DATABASE_URL="$DATABASE_URL" \
  registry.intra.{client}/edgequake/edgequake:<nouvelle-version> migrate dry-run

# 7. Application
docker run --rm -e DATABASE_URL="$DATABASE_URL" \
  registry.intra.{client}/edgequake/edgequake:<nouvelle-version> migrate

# 8. Démarrage
docker compose up -d

# 9. Recette (cf. §9.2)
curl -sf http://API:8080/ready && curl -s http://API:8080/version
```

**Ne jamais démarrer la nouvelle API avant l'étape 7.** Elle sortirait en 78, sans
dommage mais sans service.

### 5.3 Migrations irréversibles

Certaines migrations suppriment définitivement des structures héritées :

| Migration | Objet supprimé                                          | Réversibilité                         |
| --------- | ------------------------------------------------------- | ------------------------------------- |
| **125**   | magasin KV `eq_*_kv`                                    | **Aucune** — restauration seulement   |
| **126**   | tables vectorielles héritées                            | **Aucune**                            |
| **131**   | tables vectorielles de flotte                           | **Aucune**                            |
| **142**   | assertion de bascule (échoue si des résidus subsistent) | Différée tant qu'il reste des résidus |

> **Depuis v0.26.1 (SPEC-137)** : les migrations **144 à 149 sont classées
> « SAFE SCHEMA »** — extensibles, sans suppression. La migration **149**
> (`tasks.document_id`) est un `ADD COLUMN IF NOT EXISTS` + `CREATE INDEX IF NOT
EXISTS` : **réversible et rejouable**. Les seules suppressions irréversibles
> restent 125 / 126 / 131 / 142, héritées de la bascule SPEC-091.

Ces dernières exigent le drapeau explicite `--confirm-drop` :

```bash
edgequake migrate guard          # doit être GREEN avant toute suppression
edgequake migrate --confirm-drop # --drop-confirm accepté comme alias (v0.26.1+)
```

**Trois évolutions de la CLI en v0.26.1 à connaître :**

| Évolution                                                                 | Effet en exploitation                                                     |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| `--drop-confirm` accepté comme alias de `--confirm-drop`                  | Les deux orthographes fonctionnent — plus d'échec sur une faute de frappe |
| Tout drapeau `--*` inconnu fait **sortir en erreur**                      | Une option mal orthographiée n'est plus ignorée silencieusement           |
| Motifs d'abandon classifiés (Wave D / W4 / IW2 / 142 / checksum / verrou) | Le message indique la cause réelle, plus systématiquement `pg_locks`      |

> ⚠️ **Piège de mise à jour** : l'image GHCR **`0.26.0` embarque l'ancienne CLI**.
> Pour toute opération de migration, utiliser le binaire **`0.26.1`** — y compris
> pour rattraper des suppressions 125/126/131 restées en attente.

> **Règle** : ne jamais passer `--confirm-drop` sans une sauvegarde **vérifiée** de
> moins d'une heure. Passé ce point, le retour arrière n'est possible que par
> restauration complète.

Le mécanisme est protégé : la migration 125 exécute une purge conservatrice puis un
garde-fou de lignes durables. En cas de résidu non migré, **elle avorte** et la base
reste dans son état pré-suppression pour cette exécution.

### 5.4 Mise à jour depuis une version ≤ 0.22.0

Chemin particulier (bascule relationnelle SPEC-091), à dérouler en maintenance
planifiée :

```
sauvegarde → migrate dry-run → migrate (extensibles)
           → migrate --confirm-drop → migrate (applique 142 différée)
           → démarrage de l'API
```

Runbook complet :
[../operations/spec091-upgrade-from-v0.22.0.md](../operations/spec091-upgrade-from-v0.22.0.md)
et [../operations/migrate-to-0.23.md](../operations/migrate-to-0.23.md).

### 5.5 Mise à jour en multi-réplique

Aucune migration ne doit s'exécuter pendant qu'un mélange d'anciennes et de nouvelles
versions écrit en base.

```
1. Arrêt des écritures (ou passage en lecture seule)
2. Retrait de tous les réplicas du répartiteur
3. Sauvegarde
4. Migration — une seule exécution, depuis un poste d'administration
5. Déploiement de tous les réplicas dans la nouvelle version
6. Remise progressive dans le répartiteur, /ready comme critère
```

Une mise à jour progressive sans coupure n'est envisageable **que** pour une version
sans migration de schéma (`migrate dry-run` renvoie « aucune migration en attente »).

### 5.6 Notes de version à consulter systématiquement

| Version cible            | Document                                                                                                                                              |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| **0.26.9** _(courante)_   | **Pas de note de mise à jour dédiée** — SPEC-148 hébergement GCP (Terraform + Compose), SPEC-149 progression temps réel (WebSocket `subscribe`, SSE authentifié). **Sans nouvelle migration** ; schéma **149** inchangé depuis 0.26.0 |
| 0.26.7 – 0.26.8           | **Pas de note de mise à jour dédiée** — correctifs intermédiaires, **sans nouvelle migration** |
| 0.26.5                    | [../operations/upgrade-to-0.26.5.md](../operations/upgrade-to-0.26.5.md) — SPEC-145 traces Langfuse complètes (`EDGEQUAKE_LANGFUSE_IO_MAX_BYTES`), **sans nouvelle migration** |
| 0.26.4                    | [../operations/upgrade-to-0.26.4.md](../operations/upgrade-to-0.26.4.md) — SPEC-144 Next 16.3.3, listes, distroless, **sans nouvelle migration** |
| 0.26.3                    | [../operations/upgrade-to-0.26.3.md](../operations/upgrade-to-0.26.3.md) — SPEC-139 moteur mid-cutover, **sans nouvelle migration** |
| 0.26.2                   | [../operations/upgrade-to-0.26.2.md](../operations/upgrade-to-0.26.2.md) — Langfuse 3.1, K8s, SSE, **sans nouvelle migration**                         |
| 0.26.1                   | [../operations/upgrade-to-0.26.1.md](../operations/upgrade-to-0.26.1.md) — patch CLI migrate, **sans nouvelle migration**                             |
| 0.26.0                   | [../operations/upgrade-to-0.26.0.md](../operations/upgrade-to-0.26.0.md) — **migration 149**                                                          |
| 0.25.0                   | [../operations/upgrade-to-0.25.0.md](../operations/upgrade-to-0.25.0.md)                                                                              |
| 0.24.4 / 0.24.3 / 0.24.2 | [upgrade-to-0.24.4](../operations/upgrade-to-0.24.4.md) · [0.24.3](../operations/upgrade-to-0.24.3.md) · [0.24.2](../operations/upgrade-to-0.24.2.md) |
| 0.23.x depuis ≤ 0.22     | [migrate-to-0.23.md](../operations/migrate-to-0.23.md)                                                                                                |

---

## 6. Rollback

### 6.1 Matrice de réversibilité — à lire avant toute mise à jour

| Situation                                                                              | Rollback possible ? | Procédure                                                                                                                        |
| -------------------------------------------------------------------------------------- | ------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| Mise à jour **sans** migration de schéma                                               | **Oui, immédiat**   | Redéployer le tag précédent                                                                                                      |
| Migrations appliquées, **toutes extensibles** (dont **144–149**, classées SAFE SCHEMA) | **Oui**             | Redéployer l'ancien binaire ; il tolère un schéma en avance tant qu'aucune suppression n'est intervenue — **valider en recette** |
| Migration **125 / 126 / 131** appliquée                                                | **Non**             | **Restauration de sauvegarde uniquement**                                                                                        |
| Migration **142** appliquée                                                            | **Non**             | Restauration uniquement                                                                                                          |
| Corruption ou perte de données                                                         | —                   | Restauration + rejeu depuis l'archivage WAL                                                                                      |

### 6.2 Rollback applicatif (sans migration)

```bash
docker compose stop api frontend
# Repositionner le tag précédent dans le fichier compose
docker compose up -d
curl -sf http://API:8080/ready && curl -s http://API:8080/version
```

Durée typique : quelques minutes.

### 6.3 Rollback par restauration (migration irréversible appliquée)

```bash
# 1. Arrêt applicatif — la base cible ne doit plus être écrite
docker compose stop api frontend

# 2. Restauration de la sauvegarde pré-mise-à-jour (§4.5)
#    Restaurer dans une base neuve, ne pas écraser en place tant que
#    le diagnostic n'est pas clos.

# 3. Rejeu WAL jusqu'à l'instant précédant la migration, si PITR disponible
#    recovery_target_time = '<horodatage juste avant migrate>'

# 4. Bascule de DATABASE_URL vers la base restaurée

# 5. Redéploiement de l'ANCIENNE version applicative
docker compose up -d

# 6. Recette complète (§9.2) + inventaire des données ingérées
#    entre la sauvegarde et l'incident → à réinjecter
```

**Perte de données à prévoir** : tout ce qui a été ingéré entre la sauvegarde et le
rollback. D'où l'exigence d'un gel des écritures pendant la fenêtre de mise à jour
(§5.5) : elle réduit cette fenêtre à zéro.

### 6.4 Critères de décision

| Symptôme après mise à jour                       | Décision                                                    |
| ------------------------------------------------ | ----------------------------------------------------------- |
| `/ready` en 503, cause « migration »             | Ce n'est pas un rollback : appliquer la migration manquante |
| Taux d'erreur 5xx élevé, données intactes        | Rollback applicatif (§6.2)                                  |
| Résultats de recherche incohérents, index absent | Reconstruire l'index avant d'envisager un rollback (§7.4)   |
| Données absentes ou corrompues                   | Restauration (§6.3) — **escalade immédiate**                |

---

## 7. Runbooks d'incident

### 7.1 `/ready` répond 503

```bash
curl -s http://API:8080/health | jq
```

| Cause dans `/health`                               | Action                                               |
| -------------------------------------------------- | ---------------------------------------------------- |
| Migration requise / bootstrap non prêt             | `edgequake migrate` puis redémarrage                 |
| Composant de stockage KO (`kv`, `vector`, `graph`) | Vérifier PostgreSQL, les extensions, la connectivité |
| Index ANN manquant                                 | §7.4                                                 |
| Pression de la file                                | §7.3                                                 |

Si le processus est sorti au démarrage : vérifier le code de sortie.
**78** = configuration ou schéma (`docker inspect --format='{{.State.ExitCode}}' edgequake-api`).
**1** = contrôle de sécurité au démarrage bloquant (`JWT_SECRET`, CORS — cf.
[01 §7.1](01-deploiement-technique.md#71-contrôles-bloquants-au-démarrage)).

### 7.2 L'ingestion échoue ou reste bloquée

```bash
curl -s http://API:8080/api/v1/models/health | jq        # fournisseur LLM joignable ?
curl -s http://API:8080/api/v1/tasks | jq '.[] | select(.status=="failed")'   # statuts en minuscules
docker logs edgequake-api --tail 200 | grep -iE 'error|timeout'
```

Causes fréquentes :

| Cause                               | Signature                                 | Remède                                                                            |
| ----------------------------------- | ----------------------------------------- | --------------------------------------------------------------------------------- |
| Fournisseur LLM injoignable / quota | `edgequake_llm_requests_total` en erreur  | Vérifier clé, quota, réseau sortant                                               |
| Délai LLM dépassé                   | `edgequake_extract_retry_total` en hausse | Augmenter `EDGEQUAKE_CHUNK_TIMEOUT_SECS` (défaut 180 s ; 600 s pour un LLM local) |
| Contexte Ollama insuffisant         | Extractions vides ou tronquées            | Relever `OLLAMA_CONTEXT_LENGTH`                                                   |
| Documents bloqués après incident    | tâches `Processing` sans progression      | `POST /api/v1/documents/recover-stuck`                                            |
| Chunks en échec isolés              | —                                         | `GET /documents/{id}/failed-chunks` puis `POST /documents/{id}/retry-chunks`      |

### 7.3 File saturée ou à l'arrêt

```bash
curl -s http://API:8080/api/v1/pipeline/queue-metrics | jq
```

| Diagnostic                        | Action                                                                                                                                           |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `pending` élevé, `processing` > 0 | Fonctionnement nominal sous charge — augmenter `EDGEQUAKE_TASK_MAX_WORKERS` (défaut **4**) ou `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` (défaut 16) |
| `pending` > 0, `processing` = 0   | Workers bloqués — redémarrer l'API ; les baux expirés seront repris                                                                              |
| `failed` en hausse continue       | Cause systémique — voir §7.2                                                                                                                     |

Attention : augmenter les workers accroît la pression sur le fournisseur LLM et sur
le pool PostgreSQL. Monter par paliers en surveillant
`edgequake_provider_slots_inflight` et `edgequake_db_pool_connections`.

### 7.4 Index ANN manquant

Symptômes : `/ready` en 503, `edgequake_vector_ann_index_missing > 0`.

```bash
curl -s http://API:8080/api/v1/admin/storage/inspect | jq        # admin
curl -X POST http://API:8080/api/v1/admin/ann/warmup             # préchauffage
# Si l'index est réellement absent : reconstruction des embeddings de l'espace
curl -X POST http://API:8080/api/v1/workspaces/{ws}/rebuild-embeddings
```

Le comportement est _fail-closed_ **par conception** : sans index HNSW, la recherche
vectorielle dégraderait silencieusement la qualité des réponses. Le refus de trafic
est préférable à une réponse fausse.

### 7.5 Latence d'interrogation élevée

| Piste                            | Vérification                                                                             |
| -------------------------------- | ---------------------------------------------------------------------------------------- |
| Latence du fournisseur LLM       | `edgequake_llm_request_duration_seconds`                                                 |
| Branche de récupération coûteuse | `edgequake_query_arm_duration_seconds` par branche                                       |
| Requêtes PostgreSQL lentes       | `pg_stat_statements`                                                                     |
| Mode inadapté                    | `hybrid` est le plus complet mais le plus coûteux — `local` ou `naive` suffisent souvent |

Voir [../operations/performance-tuning.md](../operations/performance-tuning.md).

### 7.6 Dérive de stockage

```bash
curl -s  http://API:8080/api/v1/admin/storage/inspect | jq
curl -X POST http://API:8080/api/v1/admin/storage/repair          # après analyse
curl -X POST http://API:8080/api/v1/admin/entities/reconcile      # réconciliation du graphe
```

Ne lancer `repair` qu'après lecture du rapport d'inspection et sauvegarde récente.

### 7.7 « Connection Lost — Real-time updates unavailable » dans l'interface

Bandeau affiché quand la connexion WebSocket de progression est absente. Le
traitement lui-même n'est **pas** affecté : l'ingestion continue côté serveur, seule
la progression en direct est perdue. Vérifier dans l'ordre :

| # | Contrôle | Commande / vérification | Cause si KO |
|---|---|---|---|
| 1 | Le proxy relaie l'`Upgrade` | `curl -i -H 'Connection: Upgrade' -H 'Upgrade: websocket' https://UI/ws/pipeline/progress` → `101` attendu | Proxy sans `proxy_set_header Upgrade` (doc 01 §6.3) |
| 2 | Le jeton passe | En mode authentifié, le navigateur transmet le jeton en query string (`?token=…`) — un WAF qui dépouille les query strings des routes `/ws/*` provoque un 401 | Filtrage WAF (doc 01 §6.5) |
| 3 | L'URL d'API est joignable **depuis le poste** | `EDGEQUAKE_API_URL` résolvable côté navigateur, pas seulement depuis le conteneur | Flux F5, doc 01 §6.2 |
| 4 | Le délai d'inactivité du proxy | `proxy_read_timeout` ≥ 600 s | Coupure silencieuse sur ingestion longue |
| 5 | La progression côté serveur avance | `curl -s http://API:8080/api/v1/tasks/{track_id} \| jq .status` | Si `processing` progresse, le défaut est purement côté transport |

Un bandeau **transitoire** juste après une connexion ou un rafraîchissement de jeton
est normal : depuis SPEC-149 la connexion est reconstruite avec les nouvelles
informations d'authentification, puis les abonnements sont rétablis par l'interface.

---

## 8. Capacité et dimensionnement

### 8.1 Leviers de réglage

| Variable                               | Défaut | Effet                           | Contrainte                     |
| -------------------------------------- | ------ | ------------------------------- | ------------------------------ |
| `EDGEQUAKE_TASK_MAX_WORKERS`           | **4**  | Tâches d'ingestion en parallèle | Pression LLM et pool PG        |
| `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` | 16     | Appels LLM simultanés           | Quota fournisseur              |
| `EDGEQUAKE_CHUNK_TIMEOUT_SECS`         | 180    | Délai LLM par chunk             | Monter à 600 pour un LLM local |
| `EDGEQUAKE_PDF_CONCURRENCY`            | —      | Conversions PDF simultanées     | CPU                            |
| `EDGEQUAKE_PDF_VISION_JOBS`            | —      | Tâches vision simultanées       | Coût et quota                  |
| `EDGEQUAKE_MAX_UPLOAD_BYTES`           | —      | Taille maximale d'un dépôt      | Aligner avec le reverse proxy  |
| `EDGEQUAKE_MAX_BATCH_UPLOAD_FILES`     | —      | Fichiers par lot                | —                              |
| `EDGEQUAKE_TASK_RETENTION_DAYS`        | **30** | Rétention des tâches terminales | Volumétrie                     |
| `EDGEQUAKE_GRAPH_QUERY_TIMEOUT_SECS`   | —      | Délai des requêtes de graphe    | —                              |

### 8.2 Ordre de réglage recommandé

1. **Mesurer d'abord** : identifier le facteur limitant via
   `edgequake_ingest_stage_duration_seconds` (chunk / extract / embed / store).
2. Si c'est **extract** : le LLM est le goulot → augmenter la concurrence _si le quota
   le permet_, ou changer de modèle.
3. Si c'est **store** : PostgreSQL est le goulot → pool, index, E/S disque.
4. Si c'est **embed** : envisager un nœud d'embeddings dédié
   (`OLLAMA_EMBEDDING_HOST`).

> Augmenter les workers sans mesure préalable déplace le goulot sans améliorer le
> débit, et accroît le taux d'erreur côté fournisseur.

### 8.3 Croissance du stockage

Estimation : **volume brut des documents × 3 à 4**. Superviser
`pg_database_size('edgequake')` et alerter à 75 % de l'espace disponible. Le poste le
plus dynamique est le couple embeddings + index HNSW.

---

## 9. Checklists

### 9.1 Mise en service

- [ ] Images répliquées dans le registre interne, tags figés (pas de `latest`)
- [ ] Extensions PostgreSQL installées aux versions attendues
- [ ] `edgequake migrate` exécuté **avant** le premier démarrage de l'API
- [ ] `EDGEQUAKE_DEV_MODE=false`, `EDGEQUAKE_AUTH_ENABLED=true`
- [ ] `EDGEQUAKE_STRICT_STARTUP=1`
- [ ] `JWT_SECRET` ≥ 32 octets, issu du coffre
- [ ] `EDGEQUAKE_CORS_ORIGINS` renseigné avec les origines réelles
- [ ] Administrateur d'amorçage créé, mot de passe par défaut changé
- [ ] TLS terminé au reverse proxy, WebSocket et SSE relayés sans tampon
- [ ] `/metrics`, `/health`, `/ready`, `/live`, `/admin/*` filtrés au réseau
- [ ] Sauvegarde planifiée **et** première restauration testée
- [ ] Métriques collectées, alertes du §3.3 armées
- [ ] Audit routé vers le SIEM
- [ ] Fournisseur LLM validé au regard de la classification des données
- [ ] Recette [01 §9](01-deploiement-technique.md#9-recette-post-déploiement) déroulée et consignée

### 9.2 Après chaque mise à jour

- [ ] `curl /version` → version attendue
- [ ] `curl /ready` → 200
- [ ] `curl /health | jq .status` → `healthy`
- [ ] `edgequake migrate status` → aucune migration en attente
- [ ] Ingestion de bout en bout d'un document de test → `completed`
- [ ] Interrogation de contrôle → réponse avec sources
- [ ] Contrôle d'accès : appel non authentifié → **401**
- [ ] Volumétrie documents / nœuds / arêtes cohérente avec l'avant-mise-à-jour
- [ ] Aucune alerte nouvelle après 30 min d'observation

### 9.3 Revue mensuelle

- [ ] Espace disque PostgreSQL et tendance de croissance
- [ ] Fraîcheur et intégrité des sauvegardes
- [ ] Purge des tâches terminales effective
- [ ] Coûts LLM (`/api/v1/costs/summary`) au regard du budget
- [ ] Comptes et clés d'API : révocation des accès obsolètes
- [ ] Correctifs de sécurité disponibles pour les images
- [ ] Qualité du graphe : `edgequake_graph_quality_orphan_rate`

### 9.4 Revue trimestrielle

- [ ] **Test de restauration complet** en environnement de recette
- [ ] Revue des seuils d'alerte au regard des incidents survenus
- [ ] Revue du dimensionnement au regard de la croissance
- [ ] Rotation des secrets
- [ ] Revue des dépendances et de la version PostgreSQL supportée

---

## 10. Références

| Sujet                                                 | Document                                                                           |
| ----------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Architecture déployée, prérequis, réseau, sécurité    | [01-deploiement-technique.md](01-deploiement-technique.md)                         |
| Fonctionnement interne et algorithmique               | [03-deep-dive-architecture-algorithme.md](03-deep-dive-architecture-algorithme.md) |
| Supervision détaillée                                 | [../operations/monitoring.md](../operations/monitoring.md)                         |
| Réglage des performances                              | [../operations/performance-tuning.md](../operations/performance-tuning.md)         |
| Catalogue des variables                               | [../operations/configuration.md](../operations/configuration.md)                   |
| Annulation et équité d'ingestion                      | [../ingestion-cancel-and-fairness.md](../ingestion-cancel-and-fairness.md)         |
| Durcissement de l'authentification                    | [../operations/runtime-auth-hardening.md](../operations/runtime-auth-hardening.md) |
| Observabilité et traçage                              | [../OBSERVABILITY.md](../OBSERVABILITY.md)                                         |
| **Langfuse : compatibilité de version et Kubernetes** | [04-langfuse-kubernetes.md](04-langfuse-kubernetes.md)                             |
