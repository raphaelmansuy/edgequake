---
title: "EdgeQuake — Deep dive architecture & algorithme"
version: "0.26.9"
audience: "Architectes, développeurs, data scientists"
---

# EdgeQuake — Deep dive architecture & algorithme

> **Produit** : EdgeQuake v0.26.9 · **Base algorithmique** : LightRAG ([arXiv:2410.05779](https://arxiv.org/abs/2410.05779))
> **Documents liés** : [Déploiement technique](01-deploiement-technique.md) · [Intégration IT](02-integration-it.md)

Ce document explique **comment EdgeQuake fonctionne à l'intérieur** : le découpage du
code, l'algorithme d'ingestion, le modèle de données, l'algorithme d'interrogation, et
les décisions d'architecture qui les sous-tendent.

---

## Sommaire

1. [Le problème résolu](#1-le-problème-résolu)
2. [Architecture du code](#2-architecture-du-code)
3. [Algorithme d'ingestion](#3-algorithme-dingestion)
4. [Modèle de données](#4-modèle-de-données)
5. [Algorithme d'interrogation](#5-algorithme-dinterrogation)
6. [Ordonnancement des tâches](#6-ordonnancement-des-tâches)
7. [Décisions d'architecture](#7-décisions-darchitecture)
8. [Pour aller plus loin](#8-pour-aller-plus-loin)

---

## 1. Le problème résolu

### 1.1 La limite du RAG classique

Le RAG traditionnel découpe les documents en fragments, les vectorise, et récupère les
*k* fragments les plus proches sémantiquement de la question.

```
Documents → Chunks → Embeddings → Base vectorielle
Question  → Embedding → Top-K chunks similaires → LLM → Réponse
```

Ce schéma fonctionne pour une question factuelle localisée. Il échoue dès que la
réponse suppose de **relier** des informations dispersées.

Question type : *« Comment les travaux de Sarah Chen sur les réseaux de neurones
ont-ils influencé ceux de ses collègues du laboratoire Quantum Dynamics ? »*

Le RAG classique remonte trois fragments : un mentionnant Sarah Chen, un sur les
réseaux de neurones, un sur le laboratoire. **Ces fragments sont déconnectés.** Le
système ne sait pas que Sarah Chen *travaille à* Quantum Dynamics, ni qui sont ses
collègues, ni comment l'influence se propage.

> **Le fond du problème** : un vecteur encode la *similarité*, pas la *relation*.
> Deux textes proches dans l'espace d'embedding ne sont pas nécessairement liés
> factuellement, et deux textes factuellement liés peuvent être éloignés dans cet
> espace.

### 1.2 L'apport du graphe

Une entité qui apparaît dans plusieurs documents devient un **nœud unique** qui les
relie :

```
   Document 1              Document 2              Document 3
  « l'article de »        « le Dr Chen a »        « l'équipe du »
  « Sarah sur les »       « publié ses »          « labo… Sarah »
  « réseaux »             « résultats »
        │                       │                       │
        └───────────────────────┼───────────────────────┘
                                ▼
                        ┌───────────────┐
                        │  SARAH_CHEN   │  ← nœud unifié
                        │   (PERSON)    │
                        └───────────────┘
                          │      │      │
                 WORKS_AT │      │      │ COLLABORATES_WITH
                          ▼      ▼      ▼
                  QUANTUM_LAB  NEURAL_NETWORK  BOB_SMITH
```

Les entités sont le **pont entre les documents**. Le parcours de graphe permet le
raisonnement multi-sauts que la similarité vectorielle seule ne peut pas produire.

### 1.3 Pourquoi LightRAG plutôt que GraphRAG

| Critère | LightRAG (EdgeQuake) | GraphRAG (Microsoft) |
|---|---|---|
| Coût de récupération | ~100 tokens | ~610 000 tokens |
| Appels LLM par requête | 1–2 | plusieurs centaines |
| Mise à jour | **incrémentale** | reconstruction complète |
| Détection de communautés | optionnelle | obligatoire |

Le point décisif en exploitation est la **mise à jour incrémentale** : un nouveau
document fusionne dans le graphe existant sans réindexation globale. Avec GraphRAG,
chaque ajout impose de reconstruire les résumés de communautés — coût prohibitif sur
un corpus vivant.

### 1.4 Ce qu'EdgeQuake ajoute à LightRAG

| Apport | Nature |
|---|---|
| Implémentation Rust asynchrone | Débit et empreinte mémoire (voir §7.1) |
| 6 modes de requête au lieu de 3 | `naive`, `mix`, `bypass` en plus de `local`/`global`/`hybrid` |
| Reprise adaptative sur troncature | Gestion du `finish_reason = "length"` avec escalade de budget de tokens |
| Analyseur JSON tolérant (assainissement, récupération de troncature) | Robustesse aux sorties LLM malformées |
| Multi-tenant *fail-closed* | Cloisonnement au niveau du stockage |
| Marche PPR par défaut | Personalized PageRank au lieu d'un simple BFS |
| Pipeline PDF vision | Conversion multimodale avec repli texte |
| Lignage complet | Traçabilité chunk → entité → document |
| Remplissage PDF au budget | Chunks pleins traversant les pages, citations `p.N–M` (SPEC-135, v0.26) |

---

## 2. Architecture du code

### 2.1 Découpage en 11 crates

Le code est un workspace Cargo de 11 crates, sous `edgequake/crates/`.

```
                       edgequake-api
                    (HTTP, WS, OpenAPI)
                            │
            ┌───────────────┼───────────────┐
            ▼               ▼               ▼
      edgequake-core   edgequake-tasks  edgequake-auth
      (orchestration)  (file, workers)  edgequake-audit
            │               │           edgequake-rate-limiter
      ┌─────┴─────┐         │           edgequake-observability
      ▼           ▼         │
 edgequake-   edgequake-    │
 pipeline     query         │
 (ingestion)  (RAG)         │
      │                     │
      ▼                     │
 edgequake-pdf              │
      │                     │
      └──────────┬──────────┘
                 ▼
         edgequake-storage
       (KV | pgvector | AGE)
                 │
                 ▼
          PostgreSQL 16/17/18
```

| Crate | Responsabilité | Modules notables |
|---|---|---|
| `edgequake-api` | REST, SSE, WebSocket, OpenAPI, middlewares | `routes.rs`, `handlers/`, `startup_security.rs`, `workspace_scope.rs` |
| `edgequake-core` | Façade `EdgeQuake`, câblage des fournisseurs LLM, budgets | `orchestrator/`, `resource/`, `token_budget.rs`, `model_resolution.rs` |
| `edgequake-pipeline` | Chaîne d'ingestion | `chunker/`, `extractor/`, `merger/`, `prompts/`, `text_embedder.rs` |
| `edgequake-query` | Moteur RAG, 6 modes | `engine_impl/`, `graph_ppr.rs`, `fusion.rs`, `hybrid_merge.rs` |
| `edgequake-storage` | Persistance et invariants de données | `traits/`, `adapters/`, `entity_id.rs`, `migration_engine/` |
| `edgequake-pdf` | PDF → markdown, vision, assets | `backend/`, `vision_extract.rs`, `page_layout.rs` |
| `edgequake-tasks` | File, workers, claim/lease, annulation, équité | `claim_eligibility.rs`, `lease.rs`, `fairness.rs`, `worker.rs` |
| `edgequake-auth` | JWT, clés d'API, OIDC, RBAC, contexte tenant | `jwt.rs`, `rbac.rs`, `oidc_config.rs` |
| `edgequake-audit` | Événements de conformité | `event.rs`, `logger.rs` |
| `edgequake-rate-limiter` | Quotas par tenant | `limiter.rs`, `middleware.rs` |
| `edgequake-observability` | Traces, métriques, corrélation | `metrics.rs`, `rag_span.rs`, `langfuse.rs` |

> Il n'existe **pas** de crate `edgequake-graph`. La logique de graphe est répartie
> entre `storage` (persistance AGE), `pipeline` (construction) et `query` (parcours).
> Les fournisseurs LLM proviennent du crate externe `edgequake-llm` publié sur
> crates.io.

### 2.2 Motifs d'architecture

**Façade** — `EdgeQuake` (dans `core`) masque le pipeline et le moteur de requête
derrière deux opérations : `insert()` et `query()`.

**Adaptateur** — trois traits (`KVStorage`, `VectorStorage`, `GraphStorage`)
abstraient le stockage. Les tests utilisent des implémentations mémoire, la production
PostgreSQL, sans changement de code appelant.

**Stratégie** — les six modes de requête sont des stratégies interchangeables
sélectionnées à l'exécution (`engine_impl/modes/`).

**Pipeline en deux phases** — l'ingestion PDF est scindée en une tâche de conversion
puis une tâche d'insertion, séparées par une barrière (§3.1).

### 2.3 Le choix de Rust

| Facteur | Python (LightRAG de référence) | Rust (EdgeQuake) |
|---|---|---|
| Débit d'ingestion | ~100 docs/min | ~1000 docs/min |
| Empreinte mémoire | 2–4 Go | 200–400 Mo |
| Concurrence | limitée par le GIL | asynchrone réelle (Tokio) |
| Erreurs de typage | à l'exécution | à la compilation |
| Déploiement | environnement + dépendances | **binaire unique** |

Le dernier point est le plus structurant en exploitation : pas de gestion
d'environnement virtuel, pas de résolution de dépendances au déploiement, image
conteneur minimale.

---

## 3. Algorithme d'ingestion

### 3.1 Vue d'ensemble — deux phases

```
POST /documents/pdf  ──▶  admission : tâche Pending + track_id retourné
                                    │
                                    ▼
        ┌───────────────────────────────────────────────┐
        │ PHASE 1 — PdfProcessing (conversion seule)    │
        │   pdfium (texte) ou LLM vision (image page)   │
        │   → markdown + assets + layout                │
        │   → tâche Completed                           │
        └───────────────────────┬───────────────────────┘
                                │  barrière markdown
                                ▼
        ┌───────────────────────────────────────────────┐
        │ PHASE 2 — Insert (nouveau bail)               │
        │   chunk → extract → glean → normalize         │
        │        → merge → embed → store                │
        │   → display_status = completed                │
        └───────────────────────────────────────────────┘
```

**Pourquoi deux phases ?** La conversion PDF est coûteuse, longue et parfois soumise à
un fournisseur vision distinct. La scinder permet de : conserver le markdown même si
l'ingestion KG échoue, reprendre l'insertion sans reconvertir, et poser un bail
indépendant sur chaque phase — une conversion de 40 minutes ne bloque pas un worker
sur la totalité de la chaîne.

### 3.2 Étape 1 — Découpage (chunking)

Modules : `chunker/`, `adaptive_chunking.rs`, `contextual_chunk.rs`,
`structure_induce.rs`, `token_estimator.rs`.

```rust
pub struct ChunkerConfig {
    pub chunk_size: usize,        // défaut : 1200 tokens
    pub chunk_overlap: usize,     // défaut : 100 tokens
    pub strategy: ChunkStrategy,  // Token | Sentence | Semantic
}
```

**Découpage adaptatif** — la taille varie selon le document :

| Taille du document | Taille de chunk | Raison |
|---|---|---|
| > 100 Ko | ~600 tokens | Densité d'entités élevée, éviter la saturation d'attention |
| Nominal | 1200 tokens | Compromis rappel / coût |
| Petit | pas de découpage | Un seul appel LLM suffit |

Le **recouvrement** de 100 tokens évite qu'une relation exprimée à cheval sur deux
chunks soit perdue par les deux.

EdgeQuake ajoute deux raffinements : un **préambule de contexte** attaché à chaque
chunk (`chunk_context_preamble`, migration 135) qui rappelle la section d'origine, et
une **induction de structure** (`structure_induce.rs`) exploitant les titres du
markdown pour ne pas couper au milieu d'une unité logique.

#### Remplissage au budget pour les PDF (SPEC-135, v0.26.0)

Depuis la v0.26.0, l'ingestion PDF ne découpe plus page par page : le markdown
converti est **rempli jusqu'au budget de tokens** de l'espace de travail, y compris
**à cheval sur plusieurs pages** (*cross-page packing*).

**Le problème traité** : une page de PDF fait rarement 1200 tokens. Un découpage
page-à-page produit des chunks très inégaux — beaucoup trop courts — ce qui dilue le
signal, multiplie les appels LLM d'extraction et dégrade le rappel.

| Réglage | Défaut | Effet |
|---|---|---|
| `EDGEQUAKE_PDF_PACK` | activé | Remplissage au budget de l'espace de travail |
| `EDGEQUAKE_PDF_CROSS_PAGE_PACK` | activé | Autorise un chunk à enjamber deux pages |

Les deux variables sont des **coupe-circuits** : les positionner à `0` restaure le
découpage page-à-page antérieur.

**Conséquence sur la traçabilité** — chaque chunk porte désormais un intervalle de
pages `page_start` / `page_end`, restitué dans les citations sous la forme `p.N–M`
(et exposé dans `ChunkDetail` du SDK depuis la version **0.4.0**). Un chunk pouvant
couvrir plusieurs pages, une citation n'est plus systématiquement mono-page.

Télémétrie associée dans le span `ingest.chunking` : `fill_p50` (médiane de
remplissage du budget) et `mm_sidecar_appended`.

### 3.3 Étape 2 — Extraction d'entités par LLM

Modules : `extractor/llm.rs`, `prompts/json_prompts.rs`, `prompts/parser/json_parser.rs`,
`prompts/entity_type_policy.rs`.

Chaque chunk est soumis au LLM avec un prompt système **stable** (identique pour tous
les chunks d'un workspace, donc mis en cache par les fournisseurs qui le supportent) et
un prompt utilisateur contenant le texte du chunk. La sortie attendue est un objet
**JSON** :

```json
{
  "entities": [
    {"name": "Sarah Chen", "type": "PERSON", "description": "Chercheuse principale au Quantum Lab"}
  ],
  "relationships": [
    {"source": "Sarah Chen", "target": "Neural Network", "type": "RESEARCHES", "description": "Sarah travaille sur les réseaux de neurones"}
  ]
}
```

Le prompt système injecte le **vocabulaire du workspace** — types d'entités, types de
relations, arêtes typées — en mode strict ou guidé, et l'analyseur **impose** ce
vocabulaire sur la sortie (remappage des types inconnus vers `OTHER` / `RELATED_TO` en
strict). Le détail de ce mécanisme, qui constitue l'application de l'ontologie, fait
l'objet du [document 06](06-algorithme-extraction-ontologie.md) ; la construction du
vocabulaire lui-même, du [document 05](05-ontologie-guide-construction.md).

**Robustesse aux sorties malformées** — l'analyseur (`JsonExtractionParser`) enchaîne :

| Étape | Traitement |
|---|---|
| Extraction | JSON isolé d'une réponse enveloppée (fences ```` ```json ````, préambule) |
| Assainissement | Caractères de contrôle, commentaires, virgules terminales, guillemets simples |
| Récupération de troncature | Suffixes `}`, `]}`, `}]}`… essayés jusqu'à obtenir un JSON valide ; les entités complètes sont conservées, la ligne coupée est perdue |
| Échec fermé | Une réponse sans JSON est une **erreur d'extraction** (chunk marqué en échec, rejouable), jamais un chunk « vide » |

> **Note de révision** — les éditions précédentes de ce document décrivaient un format
> de sortie tuple `entity<|#|>…`. Ce format existe dans le code (`SotaExtractor`,
> `HybridExtractionParser`) mais n'est instancié par **aucun** chemin de production en
> v0.26.5 ; l'ingestion utilise exclusivement `LLMExtractor` (JSON). Le §7.3 est
> corrigé en conséquence.

### 3.4 Budget de sortie, délais et reprises

Modules : `extractor/completion_options.rs`, `pipeline/extraction.rs`, `pipeline/config.rs`.

Le budget de sortie de l'appel d'extraction est **fixe** : 16 384 tokens de complétion
(`extraction_completion_options_with_effort`), avec un effort de raisonnement résolu
selon le fournisseur (les modèles « pensants » locaux sont forcés en mode sans
raisonnement pour l'extraction). La protection contre les réponses trop longues est
assurée en amont par les **plafonds de quantité** du prompt (40 entités / 100 lignes
par chunk, SPEC-117) et en aval par la récupération de troncature de l'analyseur (§3.3).

Chaque chunk est extrait sous **délai** (`EDGEQUAKE_CHUNK_TIMEOUT_SECS`, défaut 180 s ;
600 s recommandés pour un LLM auto-hébergé) et avec **reprise classée** :

| Classe d'erreur | Détection (sous-chaînes du message) | Budget de tentatives |
|---|---|---|
| Transitoire | `timeout`, `provider_unavailable`, `circuit`, surcharge locale | `EDGEQUAKE_CHUNK_MAX_RETRIES` (défaut **3**, max 20) |
| Analyse | `invalid json`, `parse`, `schema`, `empty response` | plafonné à **2** |
| Permanente | `cancelled`, `enforce` | **1** (pas de reprise) |

Le délai entre tentatives est exponentiel (`EDGEQUAKE_CHUNK_RETRY_DELAY_MS` × 2ⁿ,
plafonné à 60 s ; base relevée à 5 s en cas de surcharge d'un fournisseur local).
Chaque reprise incrémente `edgequake_extract_retry_total{reason}` avec `reason ∈
{timeout, network, parse, permanent}` — une hausse durable de `parse` signale un
modèle qui ne tient pas le format JSON ; de `timeout`, un délai sous-dimensionné.

Un chunk qui épuise son budget est enregistré dans `failed_chunks` et devient
rejouable individuellement (`POST /documents/{id}/retry-chunks`, [document 07 §3.4](07-maintenance-graphe-correction-parsing.md)).

> **Note de révision** — l'escalade adaptative de budget (×2, ×4 jusqu'à 32 768
> tokens) décrite dans les éditions précédentes appartient à `SotaExtractor`, qui n'est
> pas le chemin de production (voir §3.3).

### 3.5 Gleaning — extraction en plusieurs passes

Module : `extractor/`, configuration `GleaningConfig { max_gleaning: 1 }`.

Un LLM manque systématiquement des entités en une seule passe :

- limites d'attention sur les textes longs ;
- **références implicites** — « l'entreprise » désignant une organisation nommée plus
  haut ;
- saturation quand le chunk contient beaucoup d'entités.

Le gleaning relance l'extraction en fournissant au modèle la liste de ce qu'il a déjà
trouvé, et en lui demandant explicitement de chercher les mentions implicites.

```
Passe 1  →  SARAH_CHEN, QUANTUM_LAB
            (manqué : « l'entreprise » = QUANTUM_LAB)

Passe 2  →  consigne : « De NOMBREUSES entités ont été manquées.
             Déjà trouvées : SARAH_CHEN, QUANTUM_LAB.
             Cherche les mentions implicites. »
         →  TEAM, EXPANSION_EVENT
```

**Rendement mesuré** : +15 à 25 % de rappel pour 1 à 2 itérations, rendements
décroissants au-delà. Chaque itération coûte un appel LLM supplémentaire — d'où le
défaut à **1**.

### 3.6 Normalisation des entités

Module : `prompts/normalizer.rs`.

```
normalize_entity_name("John Doe")    → "JOHN_DOE"
normalize_entity_name("the company") → "COMPANY"
normalize_entity_name("John's team") → "JOHN_TEAM"
```

Sans normalisation, `"John Doe"`, `"john doe"` et `"JOHN DOE"` produisent **trois
nœuds distincts**, et le graphe perd exactement la propriété qui justifie son
existence : l'unification des mentions à travers les documents.

Voir [../deep-dives/entity-normalization.md](../deep-dives/entity-normalization.md).

### 3.7 Fusion (merge)

Modules : `merger/`, `storage/entity_id.rs`, `entity_fuzzy.rs`,
`graph_batch_dedupe.rs`, `entity_reconcile.rs`.

C'est l'étape qui rend la mise à jour **incrémentale**. Pour chaque entité extraite :

1. **Résolution d'identité** — l'entité existe-t-elle déjà dans cet espace de travail ?
   (identifiant déterministe, plus rapprochement approximatif pour les variantes)
2. **Fusion des descriptions** — les descriptions issues de sources différentes sont
   agrégées, résumées si elles dépassent un budget (`summarizer.rs`)
3. **Fusion des poids de relation** — politique `max` par défaut (associative),
   `mean` disponible via `EDGEQUAKE_WEIGHT_POLICY`
4. **Dédoublonnage par lot** — les arêtes en double d'un même lot sont fusionnées
   avant écriture

> **Pourquoi `max` par défaut ?** La fusion doit être **associative** : le résultat ne
> doit pas dépendre de l'ordre d'ingestion des documents. `max` l'est, une moyenne
> incrémentale ne l'est pas sans conserver le compte. Deux ingestions des mêmes
> documents dans un ordre différent doivent produire le même graphe.

### 3.8 Vectorisation

Modules : `text_embedder.rs`, `storage/dimension_policy.rs`, `embedding_family.rs`.

Sont vectorisés : les **chunks** (recherche de passages) et les **entités**
(ancrage d'entités au moment de la requête).

Deux invariants critiques :

- **Cohérence de dimension** — un espace de travail est lié à une famille de modèle
  d'embedding. Un vecteur de dimension incompatible est **rejeté** et compté
  (`edgequake_vector_dim_mismatch_rejected_total`). Changer de modèle impose une
  reconstruction (`POST /workspaces/{ws}/rebuild-embeddings`).
- **Taille de lot** — `EDGEQUAKE_EMBEDDING_BATCH_SIZE`, minoré avec la capacité du
  fournisseur. Certains backends (TEI, certains déploiements Mistral) plafonnent très
  bas.

### 3.9 Persistance et cohérence

Modules : `persistence/`, `outbox.rs`, `outbox_drain.rs`, `compensation.rs`,
`serving_fence.rs`.

Une ingestion écrit dans trois familles de stockage (relationnel, vectoriel, graphe).
Une panne en cours d'écriture laisserait un état incohérent — un chunk sans son
vecteur, une entité sans son arête.

Le dispositif retenu :

- **outbox** — les effets à propager sont journalisés puis drainés, ce qui rend la
  propagation rejouable ;
- **compensation** — les écritures partielles détectées sont compensées ou mises en
  quarantaine (`edgequake_compensation_quarantine_total`) ;
- **serving fence** — une barrière empêche de servir des données dans un état
  intermédiaire.

### 3.10 Détection de communautés

Modules : `storage/community.rs`, `community_persist.rs`, `community_reports.rs`,
`community_index_service.rs`.

Clustering **Louvain** sur le graphe pour regrouper les entités densément connectées.
Chaque communauté reçoit un résumé thématique, exploité par le mode `global`.

À la différence de GraphRAG, ce calcul est **optionnel et échantillonnable**
(`edgequake_community_detection_sampled_total`) : il n'est pas sur le chemin critique
de l'ingestion, et son absence ne bloque ni l'ingestion ni les autres modes.

Voir [../deep-dives/community-detection.md](../deep-dives/community-detection.md).

### 3.11 Pipeline PDF

Crate `edgequake-pdf`. Deux backends :

| Mode | Moteur | Vitesse | Qualité | Coût |
|---|---|---|---|---|
| **Texte** *(défaut)* | pdfium embarqué | Rapide | Bonne sur PDF textuels | Nul |
| **Vision** | LLM multimodal, page → image | Lente | Récupère tableaux et colonnes | Par page |

Le mode vision existe parce que les analyseurs textuels **détruisent** les tableaux
complexes et se trompent sur l'ordre de lecture en multi-colonnes. Un LLM voyant la
page reconstruit la structure.

Traitements complémentaires : extraction des images intégrées, recadrage de graphiques
(`chart_crop.rs`), filtre de figures en deux passes (`figure_filter.rs`), persistance
du layout de page (`page_layout.rs`, migration 148).

Depuis la v0.26.0, un chemin de conversion **page-as-unit** dédié aux manuscrits
(SPEC-134) traite chaque page comme une unité autonome, et le markdown produit est
ensuite remis au remplissage au budget décrit en §3.2.

**Repli automatique** : un échec vision retombe sur l'extraction texte plutôt que de
faire échouer le document.

Voir [../deep-dives/pdf-processing.md](../deep-dives/pdf-processing.md).

---

## 4. Modèle de données

### 4.1 Trois moteurs, une seule base

```
┌─────────────────────── PostgreSQL 16 / 17 / 18 ────────────────────────┐
│                                                                        │
│  RELATIONNEL              pgvector                 Apache AGE          │
│  ───────────              ────────                 ──────────          │
│  documents                embeddings de chunks     nœuds Entity        │
│  document_pages           embeddings d'entités     arêtes de relation  │
│  chunks + FTS             index HNSW               requêtes Cypher     │
│  tasks (file)             halfvec                  index de citation   │
│  users, api_keys                                                       │
│  audit                                                                 │
│  conversations                                                         │
│  lineage                                                               │
│  mm_assets                                                             │
└────────────────────────────────────────────────────────────────────────┘
```

**Pourquoi tout dans PostgreSQL ?** Une architecture à trois magasins distincts
(relationnel + base vectorielle + base graphe) impose des écritures distribuées sans
transaction commune : la moindre panne laisse les trois désynchronisés, et la
réconciliation devient un projet à elle seule. Dans une base unique, l'ingestion d'un
chunk et de son vecteur est **une transaction**. Le coût d'exploitation d'un seul
moteur à sauvegarder, superviser et mettre à jour est également sans commune mesure.

### 4.2 Index vectoriels

- Type **HNSW** (pgvector 0.8.5), avec convergence du paramètre `ef` (migration 129).
- Support `halfvec` pour réduire de moitié l'empreinte des embeddings (migration 132).
- **Posture *fail-closed*** : si l'index ANN est absent sur des tables vectorielles
  existantes, `/ready` renvoie **503**. Servir sans index reviendrait à effectuer un
  balayage séquentiel et à dégrader silencieusement la qualité des réponses —
  inacceptable pour un système dont la sortie est difficile à auditer visuellement.
  Une base vide, elle, est considérée prête.

### 4.3 Graphe AGE

Nœuds `Entity` (nom normalisé, type, description agrégée) et arêtes typées
(description, mots-clés, poids). Interrogation en Cypher via AGE, avec des index de
citation dédiés (migrations 137, 145) pour remonter rapidement les chunks sources
d'une entité ou d'une relation.

### 4.4 Recherche plein texte

Colonne `tsvector` sur le contenu des chunks (migration 136), utilisée par la
récupération éparse BM25 (`sparse_retrieval.rs`, `l2_bm25_union.rs`) en complément du
vectoriel — la recherche lexicale reste supérieure sur les identifiants, références et
codes exacts, là où l'embedding est faible.

### 4.5 Lignage

Modules : `pipeline/lineage.rs`, tables de lignage, endpoints `/lineage/*`.

Chaîne traçable dans les deux sens : **document → pages → chunks → entités →
relations**. Permet de répondre à « d'où vient cette affirmation ? » et d'évaluer
l'impact d'une suppression (`/documents/{id}/deletion-impact`).

### 4.6 Gouvernance du schéma

- 147 fichiers de migration SQL, numérotés 001 → **149** (numérotation non contiguë).
- Empreintes verrouillées (`checksums.lock`) : une migration publiée est immuable.
- Le moteur de migration (`storage/migration_engine/`) distingue les migrations
  **extensibles** (rétrocompatibles) des **suppressions irréversibles**, ces dernières
  exigeant `--confirm-drop`.
- Les migrations **144–149** sont classées **SAFE SCHEMA** (extensibles) ; les
  seules suppressions irréversibles restent 125 / 126 / 131 / 142.
- L'API ne migre jamais : décalage → sortie **78**.

---

## 5. Algorithme d'interrogation

### 5.1 Chaîne de traitement

```
Question
   │
   ▼
[1] EXTRACTION DE MOTS-CLÉS          keywords/
    ├─ bas niveau  → entités concrètes
    └─ haut niveau → thèmes
   │
   ▼
[2] BRANCHES DE RÉCUPÉRATION (activées selon le mode et l'intention)
    ├─ vectoriel chunks     pgvector HNSW        vector_filter.rs
    ├─ épars BM25           tsvector             sparse_retrieval.rs
    ├─ ancrage d'entités    pgvector entités     entity_rank.rs
    ├─ expansion de graphe  PPR sur AGE          graph_ppr.rs, graph_expand.rs
    └─ communautés          résumés Louvain      community_global.rs
   │
   ▼
[3] SÉLECTION DE CHUNKS               kg_chunk_pick.rs
    adjacence bipartite entité ∪ chunk
   │
   ▼
[4] FUSION ET ÉLAGAGE
    fusion.rs · hybrid_merge.rs · score_scale.rs
    intent_rerank.rs · relevancy_prune.rs · path_prune.rs
    truncation.rs (budget de tokens)
   │
   ▼
[5] GÉNÉRATION                        context_format.rs
    appel LLM avec contexte cité
   │
   ▼
[6] RESTITUTION
    réponse + sources · grounding.rs · retrieval_telemetry.rs
```

### 5.2 Récupération à deux niveaux

C'est le cœur de LightRAG. Une même question est traitée à deux granularités :

| Niveau | Cible | Question type | Ce qui est remonté |
|---|---|---|---|
| **Bas** | Entités et voisinage direct | « Qui est Sarah Chen ? » | Description d'entité + voisins à 1 saut |
| **Haut** | Thèmes, clusters de relations | « Quelles sont les grandes tendances ? » | Résumés agrégés + mots-clés thématiques |

Le mode `hybrid` exécute les deux et fusionne — d'où sa couverture supérieure et son
coût supérieur.

### 5.3 Les six modes

| Mode | Vectoriel | Graphe | Usage | Latence indicative |
|---|---|---|---|---|
| **`naive`** | ✅ | ❌ | Recherche factuelle simple | 100–300 ms |
| **`local`** | ✅ | ✅ entités + voisins | « Qui / qu'est-ce que X ? » | 200–500 ms |
| **`global`** | ❌ | ✅ communautés | « Quels sont les thèmes ? » | 300–800 ms |
| **`hybrid`** *(défaut)* | ✅ | ✅ les deux | Questions complexes multi-facettes | 400–1000 ms |
| **`mix`** | ✅ pondéré | ✅ pondéré | Dosage explicite (`mix_weights.rs`) | variable |
| **`bypass`** | ❌ | ❌ | LLM direct, sans RAG — test et comparaison | dépend du LLM |

Arbre de décision :

```
Test / comparaison sans RAG ?              → bypass
Question sur une entité précise ?          → local
Question thématique, vue d'ensemble ?      → global
Besoin des deux ?                          → hybrid   (défaut)
Recherche factuelle sans relation ?        → naive
Dosage explicite vectoriel / graphe ?      → mix
```

Voir [../deep-dives/query-modes.md](../deep-dives/query-modes.md).

### 5.4 Marche de graphe — PPR par défaut

Module : `graph_ppr.rs` (repli BFS via `EDGEQUAKE_GRAPH_WALK=bfs`).

L'expansion du voisinage utilise un **Personalized PageRank** ancré sur les entités
identifiées dans la question, plutôt qu'un parcours en largeur uniforme.

> **Pourquoi ?** Un BFS à *n* sauts remonte le voisinage **par distance**, sans
> distinction : un nœud hub très connecté inonde le contexte de voisins non
> pertinents. Le PPR pondère par la probabilité de retour vers les nœuds d'ancrage :
> les entités structurellement proches *de la question* sont favorisées, pas
> simplement celles qui sont proches dans le graphe. Sur un graphe réel — dont la
> distribution des degrés suit une loi de puissance — l'écart de pertinence est
> considérable.

Compression et élagage : `graph_walk_compress.rs`, `path_prune.rs`,
`graph_hops.rs`.

### 5.5 Sélection bipartite de chunks

Module : `kg_chunk_pick.rs`.

La sélection des chunks n'est pas faite sur la seule similarité vectorielle : elle
s'appuie sur une **adjacence bipartite entité ∪ chunk**. Un chunk devient candidat
soit par proximité sémantique, soit parce qu'il est la source d'une entité retenue par
la marche de graphe. Cela remonte des passages factuellement pertinents que
l'embedding seul aurait manqués.

### 5.6 Fusion, réordonnancement, élagage

| Étape | Module | Rôle |
|---|---|---|
| Normalisation des scores | `score_scale.rs` | Rendre comparables des scores d'origines hétérogènes |
| Fusion | `fusion.rs`, `hybrid_merge.rs` | Combiner les branches, dédoublonner |
| Branches conditionnées par l'intention | `intent_rerank.rs` | Désactiver les branches non pertinentes pour la question |
| Élagage de pertinence | `relevancy_prune.rs` | Écarter le contexte faiblement lié |
| Protection au réordonnancement | `rerank_protect.rs` | Empêcher qu'un réordonnancement évince les sources fortes |
| Troncature | `truncation.rs` | Respecter le budget de tokens du modèle |
| Hydratation | `chunk_hydration.rs` | Récupérer le texte complet des chunks retenus |

**Branches conditionnées par l'intention** : exécuter systématiquement les cinq
branches gaspille latence et tokens. Une question purement factuelle n'a aucun besoin
des résumés de communautés. Le classement d'intention désactive les branches inutiles
— d'où l'intérêt de la métrique `edgequake_query_arm_duration_seconds`, ventilée par
branche.

### 5.7 Fidélité et ancrage

Modules : `grounding.rs`, `eval/`.

Échantillonnage de fidélité (*faithfulness*) mesurant si la réponse est effectivement
soutenue par le contexte récupéré : heuristique par défaut, juge LLM optionnel
(`EDGEQUAKE_FAITHFULNESS_JUDGE`). Métriques
`edgequake_faithfulness_score` et `edgequake_faithfulness_samples_total`.

### 5.8 Traçabilité de la récupération

Chaque requête produit un `retrieval_id` permettant de consulter *a posteriori* le
contexte exact ayant servi à la génération :

```
GET /api/v1/query/context/{retrieval_id}
GET /api/v1/query/context/artifacts/{artifact_type}/{artifact_id}
```

Indispensable pour auditer une réponse contestée : on reconstitue exactement ce que le
LLM avait sous les yeux.

### 5.9 Caches

Module : `query/cache/`, cache KV fournisseur et cache de prompt (SPEC-126). Réduit le
coût et la latence sur les motifs de requêtes répétés. Portée documentée dans
[../data-layer/llm-cache-scope.md](../data-layer/llm-cache-scope.md).

---

## 6. Ordonnancement des tâches

Crate `edgequake-tasks`.

### 6.1 Distribution par claim/lease

```sql
-- Principe de claim_next
SELECT … FROM tasks
 WHERE status = 'pending' AND (hold_until IS NULL OR hold_until < now())
 ORDER BY …
 FOR UPDATE SKIP LOCKED
 LIMIT 1;
```

`FOR UPDATE SKIP LOCKED` garantit que deux workers — y compris sur deux réplicas
distincts — ne prennent jamais la même tâche, sans verrou global ni broker externe.

**Bail (*lease*)** — le worker qui prend une tâche pose un bail à durée limitée, qu'il
renouvelle pendant le traitement. Si le processus meurt, le bail expire et la tâche
redevient éligible. C'est ce qui rend l'ingestion **durable au redémarrage** : une
tâche admise n'est jamais perdue, au pire reprise.

### 6.2 Annulation durable

Modules : `cancel_decision.rs`, `cancellation.rs`.

L'annulation est un état persistant, pas un signal en mémoire. L'interface affiche
**Stopping…** jusqu'à l'état terminal `Cancelled` — explicitement distinct de
`Failed`, car une annulation demandée par un utilisateur n'est pas un incident et ne
doit pas polluer les indicateurs d'échec.

Une rétractation des effets partiels est déclenchée (`edgequake_retract_on_cancel_total`).

### 6.3 Équité inter-tenant

Modules : `fairness.rs`, `fairness_hold.rs` (migration 138), `tenant_limiter.rs`.

Sans mécanisme d'équité, un tenant déposant 10 000 documents monopolise la totalité
des workers et fait attendre indéfiniment tous les autres. Le mécanisme de mise en
attente (`hold_until`) force la rotation entre tenants, garantissant une progression à
chacun.

### 6.4 Capacité fournisseur

Modules : `provider_budget.rs`, `provider_capacity.rs`, `provider_class.rs`,
`capacity_block.rs`.

Les créneaux d'appel LLM sont comptabilisés par classe de fournisseur
(`edgequake_provider_slots_inflight`, `edgequake_provider_slot_hold_duration_seconds`),
afin de ne pas dépasser les quotas et de ne pas déclencher de limitation côté
fournisseur.

### 6.5 Machine à états

```
Pending ──claim──▶ Processing ──▶ Completed
   │                   │
   │                   ├──▶ Failed     (reprise possible)
   │                   └──▶ Cancelled  (état terminal, sur demande)
   │
   └──hold_until──▶ Pending (équité : remis en file)
```

Détail opérationnel : [../ingestion-cancel-and-fairness.md](../ingestion-cancel-and-fairness.md).

---

## 7. Décisions d'architecture

Chaque décision est présentée avec son alternative écartée et le motif.

### 7.1 Rust plutôt que Python

**Alternative** : réimplémenter LightRAG en Python (écosystème IA natif).
**Motif** : ×10 sur le débit et l'empreinte mémoire, concurrence réelle sans GIL,
erreurs de typage capturées à la compilation, **binaire unique** à déployer.
**Concession** : écosystème IA moins fourni — traité en déléguant les appels LLM à un
crate dédié plutôt qu'en réimplémentant des SDK.

### 7.2 PostgreSQL comme magasin unique

**Alternative** : Qdrant/Weaviate pour les vecteurs + Neo4j pour le graphe.
**Motif** : transactions communes entre les trois familles de données, un seul système
à sauvegarder, superviser et mettre à jour. pgvector et AGE sont matures.
**Concession** : moins de fonctionnalités spécialisées qu'une base vectorielle ou
graphe dédiée — acceptable au vu du gain d'exploitation et de cohérence.

### 7.3 Sortie JSON avec analyseur tolérant, plutôt que format tuple

**Alternative** : format tuple délimité (`entity<|#|>nom<|#|>type<|#|>description`,
hérité de LightRAG), naturellement robuste à la troncature puisque chaque ligne est
autonome. Cette voie est implémentée (`SotaExtractor`) mais **non utilisée** en
production.
**Motif** : le JSON porte nativement les champs typés (`type` de relation, arêtes
typées) et se prête au **contrôle de vocabulaire** au moment de l'analyse ; sa
faiblesse à la troncature est compensée par la récupération de suffixes et l'escalade
de budget (§3.3–3.4).
**Concession** : une réponse tronquée au milieu d'un objet perd cet objet ; une réponse
sans JSON est un échec explicite du chunk, à rejouer.

### 7.4 L'API ne migre jamais la base

**Alternative** : migration automatique au démarrage.
**Motif** : une migration automatique déclenchée par un redémarrage arbitraire est
ingérable en production — a fortiori en multi-réplique, où plusieurs instances
tenteraient de migrer simultanément. Le décalage produit un **code 78** distinct d'un
plantage, sur lequel un orchestrateur peut brancher.
**Concession** : une étape d'exploitation supplémentaire à chaque mise à jour.

### 7.5 Posture *fail-closed*

**Alternative** : dégrader silencieusement (servir sans index ANN, ignorer un contexte
tenant manquant).
**Motif** : la sortie d'un système RAG est du texte en langue naturelle, dont
l'utilisateur ne peut pas déduire visuellement qu'il a été produit à partir d'un
contexte incomplet. Une dégradation silencieuse produit des réponses fausses
d'apparence normale. Refuser le trafic est le seul comportement honnête.
S'applique à : index ANN manquant (`/ready` → 503), isolation tenant ambiguë (refus),
CORS non configuré en production (refus de démarrage).

### 7.6 Deux phases pour l'ingestion PDF

**Alternative** : une seule tâche monolithique.
**Motif** : la conversion est longue et coûteuse ; la scinder préserve le markdown en
cas d'échec de la phase KG, permet la reprise sans reconversion, et évite qu'un bail
unique couvre une opération de plusieurs dizaines de minutes.

### 7.7 Claim/lease PostgreSQL plutôt qu'un broker

**Alternative** : Redis, RabbitMQ, Kafka.
**Motif** : `FOR UPDATE SKIP LOCKED` fournit une distribution correcte sans composant
supplémentaire à déployer, sauvegarder et superviser. La durabilité vient
gratuitement de la base. Le canal en mémoire ne sert qu'au réveil des workers.
**Concession** : débit inférieur à un broker dédié — sans objet à l'échelle visée, où
le facteur limitant est le LLM.

### 7.8 Louvain optionnel et échantillonné

**Alternative** : détection de communautés obligatoire à chaque ingestion (approche
GraphRAG).
**Motif** : c'est précisément ce qui rend GraphRAG inutilisable sur un corpus vivant.
Rendre le calcul optionnel et hors chemin critique préserve l'ingestion incrémentale ;
seul le mode `global` en dépend.

---

## 8. Pour aller plus loin

### 8.1 Deep dives du dépôt

| Sujet | Document |
|---|---|
| Algorithme LightRAG (référence complète) | [../deep-dives/lightrag-algorithm.md](../deep-dives/lightrag-algorithm.md) |
| Stratégies de découpage | [../deep-dives/chunking-strategies.md](../deep-dives/chunking-strategies.md) |
| Extraction d'entités | [../deep-dives/entity-extraction.md](../deep-dives/entity-extraction.md) |
| Normalisation d'entités | [../deep-dives/entity-normalization.md](../deep-dives/entity-normalization.md) |
| Gleaning | [../deep-dives/gleaning.md](../deep-dives/gleaning.md) |
| Détection de communautés | [../deep-dives/community-detection.md](../deep-dives/community-detection.md) |
| Modes de requête | [../deep-dives/query-modes.md](../deep-dives/query-modes.md) |
| Stockage de graphe | [../deep-dives/graph-storage.md](../deep-dives/graph-storage.md) |
| Stockage vectoriel | [../deep-dives/vector-storage.md](../deep-dives/vector-storage.md) |
| Modèles d'embedding | [../deep-dives/embedding-models.md](../deep-dives/embedding-models.md) |
| Traitement PDF | [../deep-dives/pdf-processing.md](../deep-dives/pdf-processing.md) |
| Couche de données | [../deep-dives/data-layer.md](../deep-dives/data-layer.md) · [../data-layer/postgres.md](../data-layer/postgres.md) |
| Suivi du lignage | [../architecture/lineage-tracking.md](../architecture/lineage-tracking.md) |
| Flux de données | [../architecture/data-flow.md](../architecture/data-flow.md) |
| Suivi des coûts | [../deep-dives/cost-tracking.md](../deep-dives/cost-tracking.md) |

### 8.2 Publications de référence

1. Guo, Xia, Yu, Ao, Huang — *LightRAG: Simple and Fast Retrieval-Augmented
   Generation*, [arXiv:2410.05779](https://arxiv.org/abs/2410.05779), 2024.
2. Edge et al. — *From Local to Global: A Graph RAG Approach to Query-Focused
   Summarization*, [arXiv:2404.16130](https://arxiv.org/abs/2404.16130), 2024.

### 8.3 Points d'entrée dans le code

| Élément | Chemin |
|---|---|
| Façade d'orchestration | `edgequake/crates/edgequake-core/src/orchestrator/` |
| Pipeline d'ingestion | `edgequake/crates/edgequake-pipeline/src/ingestion_pipeline.rs` |
| Extraction et prompts | `edgequake/crates/edgequake-pipeline/src/extractor/`, `prompts/` |
| Moteur de requête et modes | `edgequake/crates/edgequake-query/src/engine_impl/` |
| Marche de graphe PPR | `edgequake/crates/edgequake-query/src/graph_ppr.rs` |
| Traits de stockage | `edgequake/crates/edgequake-storage/src/traits/` |
| File et workers | `edgequake/crates/edgequake-tasks/src/worker.rs`, `claim_eligibility.rs` |
| Routes API | `edgequake/crates/edgequake-api/src/routes.rs` |
| Contrôles de sécurité au démarrage | `edgequake/crates/edgequake-api/src/startup_security.rs` |
| Migrations | `edgequake/migrations/` |
