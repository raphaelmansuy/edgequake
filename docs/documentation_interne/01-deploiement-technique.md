---
title: "EdgeQuake — Documentation technique de déploiement"
version: "0.26.9"
audience: "Architectes, ingénieurs infrastructure, RSSI"
---

# EdgeQuake — Documentation technique de déploiement

> **Produit** : EdgeQuake v0.26.9 · **Schéma base** : migrations 001 → **149**
> **Documents liés** : [Intégration IT](02-integration-it.md) · [Deep dive architecture & algorithme](03-deep-dive-architecture-algorithme.md)

---

## 1. Objet et périmètre

Ce document décrit **ce qui est installé, où, avec quoi, et comment les composants
communiquent** pour un déploiement EdgeQuake en environnement maîtrisé.

Il couvre :

| Section | Contenu                                                                 |
| ------- | ----------------------------------------------------------------------- |
| §2      | Architecture déployée (topologie physique et logique)                   |
| §3      | Composants installés (images, binaires, extensions, schéma)             |
| §4      | Prérequis (matériel, logiciel, comptes, secrets)                        |
| §5      | Flux de données (ingestion et interrogation)                            |
| §6      | Configuration réseau (ports, matrice de flux, reverse proxy)            |
| §7      | Configuration sécurité (authentification, RBAC, cloisonnement, secrets) |
| §8      | Procédure d'installation                                                |
| §9      | Recette post-déploiement                                                |

**Hors périmètre** : exploitation courante, sauvegarde, mise à jour et rollback —
traités dans [02-integration-it.md](02-integration-it.md).

---

## 2. Architecture déployée

### 2.1 Vue d'ensemble

EdgeQuake est un système Graph-RAG : il transforme des documents en un **graphe de
connaissances** (entités + relations) doublé d'un **index vectoriel**, puis répond aux
questions en combinant parcours de graphe et recherche sémantique.

Trois composants applicatifs, un composant de données, une dépendance externe :

```
                       Poste utilisateur (navigateur)
                                  │  HTTPS
                                  ▼
                    ┌───────────────────────────┐
                    │  Reverse proxy / WAF       │  TLS, en-têtes, IP allowlist
                    │  (nginx, Traefik, F5…)     │
                    └──────────┬────────┬────────┘
                               │        │
                    :3000      │        │  :8080
                               ▼        ▼
              ┌──────────────────┐   ┌────────────────────────────────┐
              │  Web UI          │   │  API EdgeQuake                 │
              │  Next.js 16      │──▶│  Axum (Rust)                   │
              │  React 19        │   │  REST + SSE + WebSocket        │
              │  container       │   │  + workers d'ingestion in-proc │
              └──────────────────┘   └────┬───────────────────┬───────┘
                                          │ :5432             │ HTTPS
                                          ▼                   ▼
                        ┌──────────────────────────┐   ┌────────────────────┐
                        │  PostgreSQL 16 / 17 / 18 │   │  Fournisseur LLM   │
                        │  + pgvector (vecteurs)   │   │  OpenAI, Azure,    │
                        │  + Apache AGE (graphe)   │   │  Mistral, Vertex,  │
                        │  + tables relationnelles │   │  Ollama (on-prem)  │
                        └──────────────────────────┘   └────────────────────┘
                                     ▲
                                     │  volume persistant
                              /var/lib/postgresql
```

### 2.2 Rôle de chaque composant déployé

| Composant            | Technologie                 | Rôle                                                                                                            | Étatful ?          |
| -------------------- | --------------------------- | --------------------------------------------------------------------------------------------------------------- | ------------------ |
| **Web UI**           | Next.js 16 / React 19       | Interface : dépôt de documents, interrogation, visualisation du graphe (Sigma.js), administration               | Non — sans état    |
| **API**              | Rust / Axum, binaire unique | REST OpenAPI 3.0, streaming SSE, WebSocket de progression, orchestration RAG, **workers d'ingestion embarqués** | Non — état en base |
| **PostgreSQL**       | PG 16, 17 ou 18             | Unique magasin persistant : documents, chunks, vecteurs, graphe, file de tâches, identités, audit               | **Oui**            |
| **LLM / Embeddings** | Externe ou on-premise       | Extraction d'entités, génération de réponses, vectorisation, vision PDF                                         | N/A                |

> **Point d'architecture majeur** : il n'y a **pas** de Redis, pas de broker de
> messages, pas de base vectorielle dédiée, pas de base graphe séparée. PostgreSQL
> est le point unique de vérité (_single source of truth_). Un déploiement complet
> = 3 conteneurs + 1 volume.

### 2.3 Topologies supportées

| Topologie            | Description                                                               | Usage                          |
| -------------------- | ------------------------------------------------------------------------- | ------------------------------ |
| **Mono-nœud**        | 3 conteneurs sur un hôte, volume local                                    | Pilote, POC, équipe unique     |
| **API externalisée** | API + UI conteneurisées, PostgreSQL sur infrastructure d'entreprise gérée | **Recommandé en production**   |
| **Multi-réplique**   | N réplicas d'API derrière un répartiteur de charge, PostgreSQL partagé    | Charge d'ingestion élevée / HA |

**Multi-réplique — condition impérative** : positionner
`EDGEQUAKE_TASK_DELIVERY=notify_only` (les workers s'hydratent depuis PostgreSQL par
_claim_ ; valeur `bridged` réservée aux phases de migration) et
`EDGEQUAKE_REPLICAS=<N>`. **Attention** : toute valeur non reconnue de
`EDGEQUAKE_TASK_DELIVERY` retombe silencieusement sur le mode `local`
(mono-processus). La distribution des
tâches s'appuie alors sur un mécanisme _claim/lease_ PostgreSQL
(`SELECT … FOR UPDATE SKIP LOCKED`) ; le canal mémoire interne ne sert qu'à réveiller
le worker. Le démarrage **refuse** une configuration multi-réplique incohérente
(`validate_delivery_for_replicas`).

### 2.4 Modèle d'exécution de l'API

Le binaire `edgequake` héberge dans le même processus :

1. le serveur HTTP Axum (routes REST, SSE, WebSocket) ;
2. le pool de **workers d'ingestion** (défaut : 4, `EDGEQUAKE_TASK_MAX_WORKERS`) ;
3. les sondes d'observabilité (métriques Prometheus, traces OTLP).

Conséquence d'exploitation : une charge d'ingestion massive et une charge
d'interrogation partagent le même processus. Sur un profil mixte exigeant, séparer
les rôles en déployant deux jeux de réplicas (l'un avec `EDGEQUAKE_TASK_MAX_WORKERS=1`
dédié aux requêtes, l'autre dimensionné pour l'ingestion).

---

## 3. Composants installés

### 3.1 Images conteneur

| Service    | Image                                      | Tag de référence                                            | Architectures                |
| ---------- | ------------------------------------------ | ----------------------------------------------------------- | ---------------------------- |
| API        | `ghcr.io/raphaelmansuy/edgequake`          | `0.26.9`                                                    | `linux/amd64`, `linux/arm64` |
| Web UI     | `ghcr.io/raphaelmansuy/edgequake-frontend` | `0.26.9`                                                    | `linux/amd64`, `linux/arm64` |
| PostgreSQL | `ghcr.io/raphaelmansuy/edgequake-postgres` | `0.21.0-pg18` (défaut PG18)<br>`0.21.0-pg17`, `0.21.0-pg16` | `linux/amd64`, `linux/arm64` |

> En environnement fermé, ces trois images doivent être **répliquées dans le registre
> interne** et les tags **figés** (jamais `latest`). Voir §8.1.

### 3.2 Contenu de l'image API

Le binaire est statiquement autonome, à l'exception de :

- **pdfium** — extraction PDF texte, **embarqué dans le binaire à la compilation**
  (mécanisme _pdfium-auto_, SPEC-095) ; à l'exécution, la bibliothèque est extraite
  localement — pas téléchargée — vers `PDFIUM_AUTO_CACHE_DIR` (défaut
  `/tmp/edgequake-pdfium-cache`). Déploiements durcis : `PDFIUM_LIB_PATH` vers une
  bibliothèque pré-placée en lecture seule ;
- **certificats TLS système** — pour joindre les fournisseurs LLM.

Aucun interpréteur Python, aucun runtime Node côté API, aucune dépendance système
installée à chaud.

### 3.3 Extensions PostgreSQL — versions épinglées

Les versions sont **contractuelles** : le démarrage vérifie la présence des deux
extensions, contrôle une version minimale de pgvector (constante _CVE-safe_ dans le
code), et la matrice complète des pins est vérifiée par l'outillage du projet
(OPS-17, `verify-postgres-extensions.sh`).

| Majeure PG           | Image de base          | pgvector  | Apache AGE    |
| -------------------- | ---------------------- | --------- | ------------- |
| **PG 18** _(défaut)_ | `postgres:18-bookworm` | **0.8.5** | **1.8.0-rc0** |
| PG 17                | `postgres:17-bookworm` | **0.8.5** | **1.7.0-rc0** |
| PG 16                | `postgres:16-bookworm` | **0.8.5** | **1.6.0-rc0** |

Une variante PG18 avec `pgvectorscale 0.9.0` existe pour les très grands volumes
vectoriels (`Dockerfile.postgres.pg18-vectorscale`).

**Si vous fournissez votre propre PostgreSQL** (topologie recommandée), ces deux
extensions doivent être installées aux versions ci-dessus, et l'utilisateur applicatif
doit pouvoir exécuter `CREATE EXTENSION`. Vérification :

```bash
psql "$DATABASE_URL" -c "SELECT extname, extversion FROM pg_extension
                         WHERE extname IN ('vector','age');"
```

### 3.4 Schéma de base

- **147 fichiers de migration SQL**, numérotés **001 → 149** (numérotation non
  contiguë), tous appliqués depuis v0.26.1 et **inchangés en v0.26.9** (aucune
  migration ajoutée entre 0.26.1 et 0.26.9).
- Verrouillage par empreintes : `edgequake/migrations/checksums.lock` — toute
  modification d'une migration déjà publiée est détectée et rejetée en CI.
- Familles d'objets : documents et chunks, embeddings (pgvector, index HNSW), graphe
  AGE, file de tâches, identités et clés d'API, journal d'audit, conversations,
  lignage (_lineage_), assets multimodaux, layout de pages PDF.

> **Règle structurante : l'API ne migre jamais la base.** L'application du schéma est
> un acte d'exploitation explicite (`edgequake migrate`). Si le schéma est en retard
> ou en avance sur le binaire, le processus s'arrête avec le **code de sortie 78**
> (`EX_CONFIG`), ce qui permet à un orchestrateur de distinguer « migration requise »
> d'un « plantage ». Détail en [02-integration-it.md §5](02-integration-it.md#5-mise-à-jour).

---

## 4. Prérequis

### 4.1 Matériel

| Profil                   | vCPU    | RAM       | Disque     | Commentaire                                           |
| ------------------------ | ------- | --------- | ---------- | ----------------------------------------------------- |
| Minimum (démonstration)  | 2       | 4 Go      | 10 Go      | Ingestion lente                                       |
| **Production — nominal** | **4–8** | **16 Go** | **50 Go+** | Corpus jusqu'à quelques dizaines de milliers de pages |
| Corpus volumineux        | 8–16    | 32 Go     | 200 Go+    | Prévoir un stockage rapide (SSD/NVMe) pour PostgreSQL |

Dimensionnement disque : compter le volume brut des documents **× 3 à 4**
(original conservé + markdown + chunks + embeddings + graphe) — ordre de grandeur
indicatif, à confirmer sur un corpus pilote représentatif.

L'API elle-même reste sobre (200–400 Mo résident typique) ; la mémoire est consommée
majoritairement par PostgreSQL (`shared_buffers`, construction des index HNSW).

### 4.2 Logiciel

| Élément        | Version                      | Nécessaire pour                                         |
| -------------- | ---------------------------- | ------------------------------------------------------- |
| Docker Engine  | ≥ 24                         | Déploiement conteneurisé                                |
| Docker Compose | v2                           | Orchestration mono-nœud                                 |
| PostgreSQL     | 16, 17 ou **18**             | Base de données                                         |
| pgvector       | 0.8.5                        | Index vectoriels                                        |
| Apache AGE     | 1.6/1.7/1.8 selon la majeure | Graphe de connaissances                                 |
| Rust toolchain | 1.95                         | **Uniquement** en cas de compilation depuis les sources |

`shm_size` du conteneur PostgreSQL : **256 Mo minimum** (déjà positionné dans les
fichiers compose fournis) — la valeur Docker par défaut (64 Mo) expose aux erreurs
`could not resize shared memory segment` lors des opérations parallèles et des
constructions d'index HNSW.

### 4.3 Fournisseur LLM

EdgeQuake est agnostique du fournisseur. Trois postures :

| Posture            | Fournisseurs                                            | Implication données                                  |
| ------------------ | ------------------------------------------------------- | ---------------------------------------------------- |
| **Cloud public**   | OpenAI, Anthropic, Gemini, Mistral, xAI                 | Le contenu des chunks **sort** du SI                 |
| **Cloud maîtrisé** | Azure OpenAI, Google Vertex AI                          | Sortie vers un tenant contractualisé, région choisie |
| **On-premise**     | Ollama, LM Studio, tout point d'accès compatible OpenAI | **Aucune sortie de données**                         |

> **Point d'attention sécurité** : l'extraction d'entités envoie le **texte intégral
> de chaque chunk** au LLM, et la vision PDF y envoie les **images de page**. Le choix
> du fournisseur est donc une décision de classification de données, pas un réglage
> technique. Pour des documents sensibles, la seule posture sans exfiltration est
> l'on-premise (Ollama/vLLM/TEI derrière `OPENAI_BASE_URL`).

Rôles de modèles distincts, configurables séparément :

| Rôle          | Variable                                  | Usage                                        |
| ------------- | ----------------------------------------- | -------------------------------------------- |
| LLM principal | `EDGEQUAKE_LLM_PROVIDER` / `_MODEL`       | Extraction d'entités, génération de réponses |
| Embeddings    | `EDGEQUAKE_EMBEDDING_PROVIDER` / `_MODEL` | Vectorisation chunks et entités              |
| Vision        | `EDGEQUAKE_VISION_PROVIDER` / `_MODEL`    | Conversion PDF → markdown                    |

Depuis la **v0.26.0**, deux coupe-circuits pilotent le remplissage des chunks PDF
au budget de tokens (activés par défaut) : `EDGEQUAKE_PDF_PACK` et
`EDGEQUAKE_PDF_CROSS_PAGE_PACK`. Les positionner à `0` restaure le découpage
page-à-page antérieur — voir
[03 §3.2](03-deep-dive-architecture-algorithme.md#32-étape-1--découpage-chunking).

Panachage possible : LLM on-premise + embeddings sur un nœud GPU dédié
(`OLLAMA_EMBEDDING_HOST`).

### 4.4 Secrets à provisionner

| Secret                               | Obligatoire                                    | Contrainte                                            |
| ------------------------------------ | ---------------------------------------------- | ----------------------------------------------------- |
| `POSTGRES_PASSWORD` / `DATABASE_URL` | Oui                                            | —                                                     |
| `JWT_SECRET`                         | Oui en production                              | **≥ 32 octets**, aléatoire — sinon refus de démarrage |
| `EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD` | Oui au premier démarrage avec authentification | ≥ 8 caractères, complexité mixte                      |
| `EDGEQUAKE_MASTER_API_KEY`           | Optionnel                                      | Amorçage de la création d'utilisateurs sans JWT       |
| Clé du fournisseur LLM               | Selon fournisseur                              | `OPENAI_API_KEY`, `MISTRAL_API_KEY`, …                |

Ces valeurs doivent provenir du coffre d'entreprise (Vault, Secrets Manager, secrets
Kubernetes) et **jamais** d'un fichier `.env` versionné.

---

## 5. Flux de données

### 5.1 Flux d'ingestion

```
[1] DÉPÔT
    POST /api/v1/documents/upload  (ou /documents/pdf)
    → validation type MIME + taille (EDGEQUAKE_MAX_UPLOAD_BYTES)
    → original persisté en base
    → création d'une tâche Pending + retour immédiat d'un track_id
                        │
                        ▼
[2] PRISE EN CHARGE (worker)
    claim_next : SELECT … FOR UPDATE SKIP LOCKED
    → pose d'un bail (lease) renouvelé périodiquement
    → équité inter-tenant (fairness hold)
                        │
        ┌───────────────┴───────────────┐
        ▼ PDF                            ▼ texte / markdown
[3a] CONVERSION (TaskType::PdfProcessing)
     pdfium (texte) ou LLM vision (image de page)
     → markdown + assets (figures, tableaux, layout)
     → tâche Completed, barrière markdown franchie
        └───────────────┬───────────────┘
                        ▼
[4] INGESTION KG (TaskType::Insert — nouveau bail)
    chunking          → 1200 tokens, recouvrement 100, adaptatif
    extraction LLM    → entités + relations (format tuple)
    gleaning          → 2ᵉ passe : +15–25 % d'entités
    normalisation     → "John Doe"/"john doe" → JOHN_DOE
    fusion (merge)    → dédoublonnage, fusion des descriptions et des poids
    embeddings        → vecteurs chunks + entités
    persistance       → AGE (nœuds/arêtes) + pgvector + tables relationnelles
                        │
                        ▼
[5] RESTITUTION
    display_status = completed
    progression temps réel : WS /ws/progress/{track_id} ou SSE
```

**Ce qui traverse le réseau vers le LLM** : à l'étape [3a] les images de page PDF ;
à l'étape [4] le texte de chaque chunk (extraction) puis les textes à vectoriser.
Rien d'autre ne sort.

### 5.2 Flux d'interrogation

```
POST /api/v1/query   {"query": "...", "mode": "hybrid"}
        │
        ▼
[1] Extraction de mots-clés (bas niveau = entités, haut niveau = thèmes)
        │
        ├──────────────┬──────────────┬──────────────┐
        ▼              ▼              ▼              ▼
   Recherche      Ancrage        Expansion      Communautés
   vectorielle    d'entités      de graphe      (Louvain)
   (pgvector      (pgvector      (PPR sur AGE,
    HNSW)          entités)       BFS en repli)
        └──────────────┴──────────────┴──────────────┘
                       │
                       ▼
[2] Fusion de contexte : dédoublonnage, scoring, élagage de pertinence,
    troncature au budget de tokens
                       │
                       ▼
[3] Appel LLM de génération  →  réponse + sources citées
                       │
                       ▼
[4] Restitution : JSON, ou SSE via /api/v1/query/stream
    Traçabilité : /api/v1/query/context/{retrieval_id}
```

Six modes de récupération (`naive`, `local`, `global`, `hybrid` — défaut, `mix`,
`bypass`). Algorithmique détaillée en
[03-deep-dive-architecture-algorithme.md §5](03-deep-dive-architecture-algorithme.md#5-algorithme-dinterrogation).

### 5.3 Persistance — où atterrit la donnée

| Donnée                | Emplacement PostgreSQL                        | Remarque                                                                       |
| --------------------- | --------------------------------------------- | ------------------------------------------------------------------------------ |
| Fichier original      | table `document_originals`                    | Restitution `/download/original`                                               |
| Markdown converti     | table documents/pages                         | Restitution `/download/markdown`                                               |
| Chunks + texte        | tables relationnelles + index FTS             | `chunk_content`, `chunk_fts` ; intervalle `page_start`/`page_end` depuis v0.26 |
| Embeddings            | colonnes `vector` / `halfvec`, index **HNSW** | pgvector                                                                       |
| Entités et relations  | graphe **Apache AGE**                         | Requêtes Cypher                                                                |
| Assets multimodaux    | table `document_mm_assets`                    | Figures, recadrages de graphiques                                              |
| File de tâches        | tables `tasks`                                | Claim/lease/cancel                                                             |
| Identités, clés d'API | tables d'authentification                     | Mots de passe hachés                                                           |
| Journal d'audit       | tables d'audit                                | Voir §7.6                                                                      |
| Lignage               | tables lineage                                | Chunk → entité → document                                                      |

**Aucune donnée métier hors PostgreSQL.** Sauvegarder la base, c'est sauvegarder le
système (cf. [02-integration-it.md §4](02-integration-it.md#4-sauvegarde-et-restauration)).

---

## 6. Configuration réseau

### 6.1 Ports exposés

| Service    | Port conteneur | Port hôte par défaut    | Protocole       | Exposition recommandée     |
| ---------- | -------------- | ----------------------- | --------------- | -------------------------- |
| Web UI     | 3000           | 3000 (`FRONTEND_PORT`)  | HTTP            | Derrière reverse proxy TLS |
| API        | 8080           | 8080 (`EDGEQUAKE_PORT`) | HTTP + WS + SSE | Derrière reverse proxy TLS |
| PostgreSQL | 5432           | **non publié**          | TCP             | Réseau interne uniquement  |

En développement local (`make dev`) les ports par défaut sont 3010 (UI) et 8090 (API)
pour éviter les collisions.

### 6.2 Matrice de flux

| #   | Source            | Destination       | Port       | Protocole            | Objet                                      |
| --- | ----------------- | ----------------- | ---------- | -------------------- | ------------------------------------------ |
| F1  | Poste utilisateur | Reverse proxy     | 443        | HTTPS                | Interface et API                           |
| F2  | Reverse proxy     | Web UI            | 3000       | HTTP                 | Rendu Next.js                              |
| F3  | Reverse proxy     | API               | 8080       | HTTP / WS            | REST, SSE, WebSocket                       |
| F4  | Web UI (SSR)      | API               | 8080       | HTTP                 | Rendu côté serveur                         |
| F5  | Navigateur        | API               | 443 → 8080 | HTTPS / WSS          | Appels directs + progression temps réel    |
| F6  | API               | PostgreSQL        | 5432       | TCP (TLS recommandé) | Toutes les opérations de données           |
| F7  | API               | Fournisseur LLM   | 443        | HTTPS                | Extraction, embeddings, génération, vision |
| F8  | API               | Ollama on-premise | 11434      | HTTP                 | Alternative on-premise à F7                |
| F9  | Supervision       | API `/metrics`    | 8080       | HTTP                 | Collecte Prometheus                        |
| F10 | API               | Collecteur OTLP   | 4318       | HTTP                 | Traces (optionnel — Langfuse, Jaeger)      |

**Flux F5 — attention** : la Web UI n'est pas un proxy inverse universel. Le
navigateur appelle l'API **directement** pour le streaming et les WebSockets. L'URL
publiée est portée par `EDGEQUAKE_API_URL`, lue à l'exécution (pas au build), et doit
être **résolvable depuis le poste client**, pas seulement depuis le conteneur.

### 6.3 Reverse proxy — exigences

Le proxy en amont doit :

1. **terminer TLS** (l'API sert en clair) ;
2. **relayer les WebSockets** — en-têtes `Upgrade` / `Connection` sur `/ws/*` ;
3. **désactiver la mise en tampon** sur `/api/v1/query/stream` et
   `/documents/pdf/progress/stream/*` (SSE) — sinon la réponse n'arrive qu'à la fin ;
4. **relever les délais d'attente** : une ingestion ou une requête RAG longue peut
   dépasser 60 s (`proxy_read_timeout 600s`) ;
5. **autoriser la taille des dépôts** — aligner `client_max_body_size` sur
   `EDGEQUAKE_MAX_UPLOAD_BYTES` ;
6. **restreindre par IP** l'accès à `/metrics`, `/health`, `/ready`, `/live` et à
   `/api/v1/admin/*` (cf. §7.5) ;
7. **ne pas journaliser les paramètres de requête** sur `/ws/*` — voir §6.5.

### 6.4 Progression temps réel — contrat WebSocket et SSE (SPEC-149)

Depuis la **v0.26.9**, la progression temps réel (ingestion, conversion PDF,
suppression) repose sur un contrat explicite. Deux routes WebSocket coexistent :

| Route | Usage | Portée |
|---|---|---|
| `/ws/pipeline/progress` | Canal **multiplexé** : le client s'abonne explicitement aux traitements qu'il suit | Plusieurs `track_id` sur une seule connexion |
| `/ws/progress/{track_id}` | Canal dédié à un seul traitement | Un `track_id` |

Sur le canal multiplexé, le client envoie des commandes typées — `subscribe`,
`unsubscribe`, `cancel`, `ping` — par exemple :

```json
{ "type": "subscribe", "track_ids": ["insert-92828e49-f82f-414b-a8f7-94f593b5745c"] }
```

Le serveur **ne diffuse que les traitements explicitement demandés**, et seulement
après avoir vérifié que le `track_id` appartient bien au contexte tenant/espace de
travail de la session (`get_task_for_context`). L'acquittement `subscribed`
n'énumère que les identifiants **acceptés** : un `track_id` appartenant à un autre
tenant est simplement absent de la réponse, sans fuite d'information. Le
cloisonnement *fail-closed* de §7.4 s'applique donc aussi au temps réel.

**Conséquences pour l'exploitation :**

- le proxy doit relayer les WebSockets **dans les deux sens** (le client émet, il
  n'est plus seulement récepteur) ;
- une session perd ses abonnements à la reconnexion ; c'est l'interface qui les
  rétablit — un « Connection Lost » transitoire après un rafraîchissement de jeton
  est le comportement attendu, pas une panne ;
- les flux **SSE** (`/api/v1/query/stream`, `/documents/pdf/progress/stream/*`) sont
  désormais consommés avec un en-tête `Authorization` (lecture de flux `fetch`, plus
  d'`EventSource`). Un WAF qui dépouille les en-têtes d'autorisation sur les réponses
  `text/event-stream` provoque un **401** au lieu d'un flux.

### 6.5 Authentification du WebSocket — jeton en query string

Quand `EDGEQUAKE_AUTH_ENABLED=true`, la connexion WebSocket est authentifiée par un
jeton lu **soit** dans l'en-tête `Authorization`, **soit** dans le paramètre de
requête `?token=…` (l'API navigateur `WebSocket` n'autorise pas d'en-tête
personnalisé — c'est une contrainte du standard, pas un choix d'EdgeQuake).

> **Action requise** : configurer le reverse proxy pour **ne pas écrire les query
> strings des routes `/ws/*` dans les journaux d'accès**, et vérifier la même chose
> côté WAF et côté collecteur de logs. À défaut, des jetons de session valides sont
> archivés en clair. Exemple nginx :
>
> ```nginx
> location /ws/ {
>     access_log off;              # ou un format sans $query_string
>     proxy_pass http://edgequake_api;
>     proxy_http_version 1.1;
>     proxy_set_header Upgrade $http_upgrade;
>     proxy_set_header Connection "upgrade";
>     proxy_read_timeout 600s;
> }
> ```

### 6.6 Accès sortant

En environnement filtré, ouvrir explicitement :

- le point d'accès du fournisseur LLM (ex. `api.openai.com:443`, ou l'URL du tenant
  Azure/Vertex) ;
- le registre d'images au moment du déploiement uniquement (ou pré-charger les images).

Aucun autre accès sortant n'est requis en fonctionnement nominal.

---

## 7. Configuration sécurité

### 7.1 Contrôles bloquants au démarrage

L'API valide sa configuration **avant** de servir le moindre trafic
(`startup_security.rs`). Comportements :

| Condition                                                        | Verdict       | Effet                                                          |
| ---------------------------------------------------------------- | ------------- | -------------------------------------------------------------- |
| `JWT_SECRET` = valeur par défaut, ou < 32 octets, hors mode dev  | **FATAL**     | Arrêt du processus (code 1)                                    |
| `EDGEQUAKE_CORS_ORIGINS` vide avec une `DATABASE_URL` non locale | **FATAL**     | Arrêt — le CORS ouvert est refusé en production                |
| Authentification désactivée avec une base non locale             | Avertissement | Journalisé ; **devient fatal** si `EDGEQUAKE_STRICT_STARTUP=1` |
| Authentification activée sans aucune clé d'API ni clé maître     | Avertissement | Idem                                                           |
| Schéma de base en décalage avec le binaire                       | **FATAL**     | Arrêt avec **code 78**                                         |

> **Recommandation {client}** : positionner `EDGEQUAKE_STRICT_STARTUP=1` en production.
> Tout avertissement devient alors bloquant, ce qui interdit une mise en service
> partiellement durcie.

Le mode `EDGEQUAKE_DEV_MODE=true` désactive ces garde-fous et ouvre l'API **sans
authentification**. Il est utilisé par le quickstart de démonstration et doit être
**explicitement à `false`** en production.

### 7.2 Authentification — mécanismes et activation

#### 7.2.1 Mécanismes disponibles

Trois mécanismes, cumulables sur une même instance :

| Mécanisme                  | Usage                                                | Présentation                                        |
| -------------------------- | ---------------------------------------------------- | --------------------------------------------------- |
| **JWT** (access + refresh) | Utilisateurs interactifs (Web UI)                    | `Authorization: Bearer <jwt>`                       |
| **Clé d'API**              | Intégrations serveur à serveur, SDK, automatisations | `X-API-Key: <clé>` ou `Authorization: Bearer <clé>` |
| **OIDC**                   | Fédération avec l'annuaire d'entreprise (SSO)        | `/api/v1/auth/oidc/login` → `/auth/oidc/callback`   |

Les identités résident **en PostgreSQL**. Les mots de passe sont hachés en
**Argon2id** (mémoire 64 Mio, 3 itérations, parallélisme 4 — coûts ajustables). Les
clés d'API sont comparées en **temps constant**. Un verrouillage de compte s'applique
après échecs répétés (§7.2.7).

#### 7.2.2 Matrice des modes de fonctionnement

Le comportement effectif résulte de deux variables. Une seule combinaison est
admissible en production :

| `EDGEQUAKE_DEV_MODE` | `EDGEQUAKE_AUTH_ENABLED` | Comportement                                                                    | Usage admis                     |
| -------------------- | ------------------------ | ------------------------------------------------------------------------------- | ------------------------------- |
| `true`               | _(indifférent)_          | **API entièrement ouverte**, garde-fous de démarrage désactivés                 | Démonstration locale uniquement |
| `false`              | `false`                  | API ouverte, avertissement au démarrage (fatal si `EDGEQUAKE_STRICT_STARTUP=1`) | Aucun                           |
| `false`              | `true`                   | **Authentification exigée** sur toutes les routes protégées                     | **Production**                  |

Restent servies sans authentification, par conception : `/health`, `/ready`, `/live`,
`/metrics`, `/auth/login`, `/auth/refresh`, `/auth/oidc/*`, `/setup/*` — à filtrer au
réseau (§7.5).

#### 7.2.3 Procédure d'activation — installation neuve

**Étape 1 — Générer les secrets** (à stocker dans le coffre d'entreprise) :

```bash
openssl rand -base64 48        # JWT_SECRET  (≥ 32 octets requis, sinon refus de démarrage)
openssl rand -hex 32           # EDGEQUAKE_MASTER_API_KEY (optionnel, cf. 7.2.5)
```

**Étape 2 — Positionner l'environnement de l'API** (avant le premier démarrage) :

```bash
EDGEQUAKE_DEV_MODE=false
EDGEQUAKE_AUTH_ENABLED=true
EDGEQUAKE_STRICT_STARTUP=1                          # tout avertissement devient bloquant
JWT_SECRET=<secret ≥ 32 octets, depuis le coffre>
EDGEQUAKE_CORS_ORIGINS=https://edgequake.intra.{client}   # obligatoire hors base locale

# Amorçage du premier administrateur — lu au premier démarrage uniquement
EDGEQUAKE_BOOTSTRAP_ADMIN_USERNAME=admin
EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD=<≥ 8 caractères, complexité mixte, depuis le coffre>
EDGEQUAKE_BOOTSTRAP_ADMIN_EMAIL=admin@{client}.example    # optionnel
```

**Étape 3 — Positionner l'environnement de la Web UI** :

```bash
NEXT_PUBLIC_AUTH_ENABLED=true          # active l'écran de connexion et la gestion de session
NEXT_PUBLIC_DISABLE_DEMO_LOGIN=true    # masque « Continuer sans connexion »
```

**Étape 4 — Démarrer, puis vérifier la création de l'administrateur** :

```bash
docker compose up -d
docker logs edgequake-api 2>&1 | grep -i bootstrap    # trace de création du compte
```

Au démarrage, si aucun utilisateur capable de se connecter n'existe en base, l'API
crée le compte d'amorçage. S'il existe déjà des utilisateurs, les variables
`BOOTSTRAP_*` sont ignorées — elles ne peuvent ni écraser ni dupliquer un compte.

**Étape 5 — Première connexion et vérification** :

```bash
# Connexion : doit renvoyer access_token + refresh_token
curl -s -X POST https://edgequake.intra.{client}/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"<mot de passe>"}' | jq

# Contre-épreuve : sans jeton, une route protégée doit répondre 401
curl -s -o /dev/null -w '%{http_code}\n' https://edgequake.intra.{client}/api/v1/documents
```

Connexion via l'interface : `https://edgequake.intra.{client}/login`.
**Changer le mot de passe d'amorçage dès la première session**, puis retirer
`EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD` de l'environnement.

#### 7.2.4 Activation sur une instance existante en mode ouvert

Pour une instance déployée en mode démonstration (`EDGEQUAKE_DEV_MODE=true`) à
basculer en mode authentifié :

1. Annoncer l'interruption : la bascule ferme l'API aux appels non authentifiés.
2. Positionner les variables des étapes 2 et 3 ci-dessus (API **et** Web UI).
3. Redémarrer les deux services : `docker compose up -d api frontend`.
4. Dérouler les vérifications de l'étape 5.
5. Mettre à jour les intégrations consommatrices : chaque client d'API doit
   désormais présenter un jeton ou une clé (7.2.5).

Aucune donnée n'est affectée : la bascule ne modifie que le contrôle d'accès.
Les instances antérieures à v0.15 voient leurs comptes hérités du magasin KV
importés automatiquement en PostgreSQL au premier démarrage.

#### 7.2.5 Clés d'API pour les intégrations

Deux familles, à usages distincts :

**Clés gérées en base** (recommandé) — créées par un administrateur authentifié,
révocables individuellement, préfixe `sk_` :

```bash
# Création (JWT administrateur requis)
curl -s -X POST https://…/api/v1/api-keys \
  -H "Authorization: Bearer <jwt-admin>" \
  -H "Content-Type: application/json" \
  -d '{"name":"integration-ged","expires_in_days":365}' | jq
# Champs (tous optionnels) : name, scopes, expires_in_days
# → la clé est hachée en base (Argon2) et n'est affichée qu'à la création ;
#   la stocker immédiatement dans le coffre

# Inventaire et révocation
curl -s https://…/api/v1/api-keys -H "Authorization: Bearer <jwt-admin>" | jq
curl -s -X DELETE https://…/api/v1/api-keys/{key_id} -H "Authorization: Bearer <jwt-admin>"
```

**Clé maître** (`EDGEQUAKE_MASTER_API_KEY`) — clé d'amorçage définie dans
l'environnement, permettant notamment `POST /api/v1/users` sans JWT. À réserver à
la phase d'installation et aux procédures de secours ; ne pas l'utiliser comme clé
d'intégration courante. Des clés statiques supplémentaires peuvent être déclarées
via `EDGEQUAKE_API_KEYS` (liste séparée par des virgules) — préférer les clés en
base, révocables unitairement.

#### 7.2.6 Fédération OIDC (SSO d'entreprise)

```bash
EDGEQUAKE_OIDC_ISSUER_URL=https://idp.intra.{client}/realms/edgequake
EDGEQUAKE_OIDC_CLIENT_ID=edgequake
EDGEQUAKE_OIDC_CLIENT_SECRET=<depuis le coffre>
EDGEQUAKE_OIDC_REDIRECT_URI=https://edgequake.intra.{client}/api/v1/auth/oidc/callback
EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL=https://edgequake.intra.{client}/
```

Parcours : l'utilisateur est dirigé vers `/api/v1/auth/oidc/login`, s'authentifie
auprès du fournisseur d'identité, revient sur le `callback`, et reçoit une session
JWT EdgeQuake. Déclarer l'URI de redirection à l'identique côté fournisseur
d'identité. L'OIDC s'ajoute à l'authentification locale ; il ne la remplace pas.

#### 7.2.7 Paramètres de session et de verrouillage

| Variable                      | Défaut          | Rôle                                                                | Recommandation                                                                                                                                                                                                         |
| ----------------------------- | --------------- | ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `JWT_EXPIRY_HOURS`            | `24`            | Durée de vie du jeton d'accès                                       | **Abaisser à 1 h ou moins** — la révocation des jetons étant locale au processus (denylist `jti` en mémoire — non partagée entre réplicas, vidée au redémarrage), l'expiration est la borne réelle d'une session volée |
| `REFRESH_TOKEN_EXPIRY_DAYS`   | `30`            | Durée de vie du jeton de rafraîchissement                           | 7 jours en environnement sensible                                                                                                                                                                                      |
| `MAX_LOGIN_ATTEMPTS`          | `5`             | Échecs avant verrouillage du compte                                 | Conserver                                                                                                                                                                                                              |
| `LOCKOUT_DURATION_MINUTES`    | `15`            | Durée du verrouillage                                               | Conserver                                                                                                                                                                                                              |
| `JWT_ISSUER` / `JWT_AUDIENCE` | _(non définis)_ | Validation `iss`/`aud` des jetons — **fail-closed dès que définis** | Définir en production                                                                                                                                                                                                  |
| `API_KEY_PREFIX`              | `sk_`           | Préfixe des clés générées                                           | Conserver                                                                                                                                                                                                              |

#### 7.2.8 Dépannage de l'activation

| Symptôme                                          | Cause                                                                                               | Action                                                                    |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| L'API s'arrête immédiatement, code 1              | `JWT_SECRET` absent, < 32 octets, ou égal à la valeur de démonstration                              | Fournir un secret conforme (étape 1)                                      |
| Arrêt avec « CORS is required in production »     | `EDGEQUAKE_CORS_ORIGINS` vide avec base non locale                                                  | Renseigner les origines exactes de l'UI                                   |
| `401` sur `/auth/login` avec le compte d'amorçage | Variables `BOOTSTRAP_*` posées **après** le premier démarrage (elles ne sont lues qu'à la création) | Créer le compte via la clé maître : `POST /api/v1/users` avec `X-API-Key` |
| Compte verrouillé après essais                    | Verrouillage anti-force-brute (5 échecs / 15 min)                                                   | Attendre l'expiration ou ajuster §7.2.7                                   |
| L'UI n'affiche pas d'écran de connexion           | `NEXT_PUBLIC_AUTH_ENABLED` non positionné côté frontend                                             | Étape 3, puis redémarrage du conteneur UI                                 |
| Sessions invalidées en masse                      | Rotation de `JWT_SECRET`                                                                            | Comportement attendu — planifier les rotations hors heures ouvrées        |

Référence complémentaire :
[Durcissement de l'authentification](../operations/runtime-auth-hardening.md)
(clé maître, OIDC, cas particuliers).

### 7.3 Autorisation (RBAC)

Hiérarchie stricte `Admin > User > Readonly` :

| Rôle         | Portée                                                                                     |
| ------------ | ------------------------------------------------------------------------------------------ |
| **Admin**    | Toutes les permissions, y compris `system:admin` (endpoints `/admin/*`, gestion des rôles) |
| **User**     | Lecture/écriture documentaire et interrogation dans ses espaces de travail                 |
| **Readonly** | Consultation et interrogation seulement                                                    |

Un rôle ne peut promouvoir que des rôles de niveau inférieur ou égal
(`can_manage_role`).

### 7.4 Cloisonnement multi-tenant

Deux niveaux imbriqués : **tenant** → **workspace**. L'isolation est appliquée au
niveau de la couche de stockage (filtrage par `tenant_id` / `workspace_id`), pas
uniquement au niveau applicatif, avec une posture _fail-closed_ : en cas de contexte
d'isolation absent ou ambigu, la requête est **refusée** plutôt que servie
globalement. Cela couvre l'interrogation, la suppression et les opérations de
récupération.

Une **RLS PostgreSQL** peut compléter le dispositif pour une défense en profondeur —
voir [../security/best-practices.md](../security/best-practices.md#postgresql-row-level-security-rls).

### 7.5 Endpoints non authentifiés — à filtrer au réseau

Les routes suivantes sont servies **hors chaîne d'authentification**, par conception
(sondes d'orchestrateur et de supervision) :

| Route      | Contenu                                                                    | Risque si exposée                     |
| ---------- | -------------------------------------------------------------------------- | ------------------------------------- |
| `/health`  | État détaillé : composants de stockage, file de tâches, état de migration  | Divulgation d'architecture interne    |
| `/ready`   | Aptitude à servir (200 / 503)                                              | Faible                                |
| `/live`    | Vivacité du processus                                                      | Faible                                |
| `/metrics` | **Toutes les métriques Prometheus** : volumétrie, taux d'erreur, coûts LLM | Divulgation de télémétrie exploitable |

> **Action requise** : restreindre ces quatre routes au sous-réseau de supervision et
> à l'orchestrateur au niveau du reverse proxy. Elles ne doivent jamais être
> joignables depuis un poste utilisateur.

### 7.6 Journal d'audit

Le crate `edgequake-audit` enregistre les événements de conformité :

| Type d'événement                                    | Résultats possibles                        | Sévérités                           |
| --------------------------------------------------- | ------------------------------------------ | ----------------------------------- |
| `Authentication`, `Authorization`                   | `Success`, `Failure`, `Blocked`, `Warning` | `Low`, `Medium`, `High`, `Critical` |
| `DocumentUpload`, `DocumentQuery`, `GraphTraversal` | idem                                       | idem                                |
| `TenantAccess`, `WorkspaceAccess`                   | idem                                       | idem                                |
| `RateLimitExceeded`, `SecurityViolation`            | idem                                       | idem                                |
| `DataExport`, `ConfigChange`                        | idem                                       | idem                                |

Ces événements sont persistés en base et émis dans les journaux structurés — à
collecter vers le SIEM (§ [02-integration-it.md §3.4](02-integration-it.md#34-journaux)).

### 7.7 Limitation de débit

Le crate `edgequake-rate-limiter` applique des quotas par tenant sur quatre paliers
(`free`, `basic`, `premium`, `enterprise`), paramétrés par fenêtre glissante
(`requests_per_window`, `window_seconds`, `burst_size`, `refill_rate`). Les
dépassements sont comptés (`edgequake_rate_limit_exceeded_total`) et audités
(`RateLimitExceeded`).

### 7.8 Chiffrement

| Couche                        | Dispositif                                                                     |
| ----------------------------- | ------------------------------------------------------------------------------ |
| En transit — client → proxy   | TLS terminé au reverse proxy (obligatoire)                                     |
| En transit — API → PostgreSQL | `sslmode=require` (`verify-full` recommandé) dans `DATABASE_URL`               |
| En transit — API → LLM        | HTTPS                                                                          |
| Au repos                      | Chiffrement du volume / du stockage PostgreSQL (responsabilité infrastructure) |

### 7.9 Traitement des fichiers déposés

- Validation du type MIME et de l'extension (`file_validation.rs`) ;
- Plafond de taille (`EDGEQUAKE_MAX_UPLOAD_BYTES`) et de lot
  (`EDGEQUAKE_MAX_BATCH_UPLOAD_FILES`) ;
- Normalisation des chemins contre la traversée de répertoire (`path_validation.rs`) ;
- Aucune exécution du contenu déposé ; l'analyse PDF est confinée à pdfium ou à une
  conversion en images.

> **Complément recommandé** : EdgeQuake n'embarque pas d'antivirus. Interposer une
> analyse antimalware en amont (au niveau du proxy ou d'un ICAP) si le dépôt est
> ouvert à des utilisateurs non maîtrisés.

---

## 8. Procédure d'installation

### 8.1 Préparation (environnement fermé)

```bash
# 1. Réplication des images vers le registre interne
for img in edgequake:0.26.9 edgequake-frontend:0.26.9 edgequake-postgres:0.21.0-pg18; do
  docker pull  ghcr.io/raphaelmansuy/$img
  docker tag   ghcr.io/raphaelmansuy/$img registry.intra.{client}/edgequake/$img
  docker push  registry.intra.{client}/edgequake/$img
done
```

Figer ensuite les références d'images dans le fichier compose interne (jamais
`latest`).

### 8.2 Base de données

```bash
# Extensions (si PostgreSQL fourni par l'entreprise)
psql "$ADMIN_DATABASE_URL" -c "CREATE EXTENSION IF NOT EXISTS vector;"
psql "$ADMIN_DATABASE_URL" -c "CREATE EXTENSION IF NOT EXISTS age;"

# Vérification des versions attendues
psql "$DATABASE_URL" -c "SELECT extname, extversion FROM pg_extension
                         WHERE extname IN ('vector','age');"
```

### 8.3 Application du schéma — **avant** tout démarrage de l'API

```bash
# Simulation : liste les migrations en attente, signale les suppressions irréversibles
docker run --rm -e DATABASE_URL="$DATABASE_URL" \
  registry.intra.{client}/edgequake/edgequake:0.26.9 migrate dry-run

# Application
docker run --rm -e DATABASE_URL="$DATABASE_URL" \
  registry.intra.{client}/edgequake/edgequake:0.26.9 migrate
```

Sur une installation neuve, une seule exécution de `migrate` suffit.

### 8.4 Démarrage des services

```bash
docker compose -f docker-compose.prod.yml up -d
docker compose -f docker-compose.prod.yml ps
```

> `docker-compose.prod.yml` désigne le fichier de production de l'exploitant,
> **dérivé** du modèle `docker-compose.quickstart.yml` fourni par le dépôt, avec
> les durcissements du §7 appliqués (le dépôt ne livre pas ce fichier tel quel).

L'ordre de démarrage est porté par les `depends_on` avec conditions de santé :
PostgreSQL sain → API saine → Web UI.

### 8.5 Sondes de santé configurées

| Service    | Sonde                                  | Intervalle | Délai de grâce |
| ---------- | -------------------------------------- | ---------- | -------------- |
| PostgreSQL | `pg_isready -U edgequake -d edgequake` | 10 s       | 10 s           |
| API        | `curl -f http://localhost:8080/health` | 20 s       | 15 s           |
| Web UI     | `wget --spider http://localhost:3000`  | 20 s       | 20 s           |

Sous Kubernetes, câbler `/live` en _liveness_ et `/ready` en _readiness_ — voir
[../operations/deployment.md](../operations/deployment.md).

---

## 9. Recette post-déploiement

À exécuter et à consigner après chaque mise en service.

| #   | Vérification                             | Commande                                                             | Attendu                                              |
| --- | ---------------------------------------- | -------------------------------------------------------------------- | ---------------------------------------------------- |
| R1  | Version déployée                         | `curl -s http://API/version`                                         | `0.26.9`                                             |
| R2  | Vivacité                                 | `curl -s http://API/live`                                            | 200                                                  |
| R3  | Aptitude au trafic                       | `curl -sf http://API/ready`                                          | **200** (503 = migration, stockage ou file en cause) |
| R4  | Santé détaillée                          | `curl -s http://API/health \| jq .status`                            | `healthy` (non `degraded`)                           |
| R5  | Extensions PG                            | requête `pg_extension` (§8.2)                                        | `vector 0.8.5`, `age 1.x` attendu                    |
| R6  | Schéma à jour                            | `edgequake migrate status`                                           | Aucune migration en attente                          |
| R7  | Refus sans authentification              | `curl -s -o /dev/null -w '%{http_code}' http://API/api/v1/documents` | **401** (pas 200)                                    |
| R8  | CORS restreint                           | requête `OPTIONS` avec `Origin` étranger                             | Origine refusée                                      |
| R9  | Métriques exposées                       | `curl -s http://API/metrics \| head`                                 | Format Prometheus                                    |
| R10 | Métriques non joignables du poste client | depuis un poste utilisateur                                          | **Connexion refusée**                                |
| R11 | Fournisseur LLM joignable                | `curl -s http://API/api/v1/models/health`                            | Fournisseur `healthy`                                |
| R12 | Ingestion de bout en bout                | dépôt d'un PDF de test, suivi du `track_id`                          | `display_status = completed`                         |
| R13 | Interrogation de bout en bout            | `POST /api/v1/query` mode `hybrid`                                   | Réponse avec sources citées                          |
| R14 | Isolation multi-tenant                   | requête du tenant A sur un document du tenant B                      | **Refus**                                            |
| R15 | Journal d'audit alimenté                 | consultation des événements d'authentification                       | Événements présents                                  |

---

## 10. Références

| Sujet                                               | Document                                                                           |
| --------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Exploitation, monitoring, sauvegarde, MAJ, rollback | [02-integration-it.md](02-integration-it.md)                                       |
| Architecture interne et algorithmique               | [03-deep-dive-architecture-algorithme.md](03-deep-dive-architecture-algorithme.md) |
| Catalogue exhaustif des variables                   | [../operations/configuration.md](../operations/configuration.md)                   |
| Durcissement sécurité détaillé                      | [../security/best-practices.md](../security/best-practices.md)                     |
| Déploiement Kubernetes                              | [../operations/deployment.md](../operations/deployment.md)                         |
| Référence API REST                                  | [../api-reference/rest-api.md](../api-reference/rest-api.md)                       |
| Couche de données PostgreSQL                        | [../data-layer/postgres.md](../data-layer/postgres.md)                             |
| Compatibilité Langfuse et déploiement Kubernetes    | [04-langfuse-kubernetes.md](04-langfuse-kubernetes.md)                             |
