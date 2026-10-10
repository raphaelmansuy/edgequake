---
title: EdgeQuake — Dossier technique de déploiement {client}
description: Index du dossier client v0.26.4 (déploiement, intégration IT, deep dive, Langfuse). Historique.
---

> Historical note, 2026-08-30 (produit v0.26.4); may not match current code.
> Prefer: [Déploiement technique](01-deploiement-technique.md) · [Intégration IT](02-integration-it.md) · [Deep dive](03-deep-dive-architecture-algorithme.md) · [Langfuse Kubernetes](04-langfuse-kubernetes.md) · [Deployment](../operations/deployment.md)

## Contrôle documentaire

| Champ           | Valeur                                                                                                               |
| -----------------| ----------------------------------------------------------------------------------------------------------------------|
| Produit couvert | EdgeQuake **v0.26.4** (schéma de base : migrations 001 → 149)                                                        |
| Statut          | Bon pour diffusion client                                                                                            |
| Date d'édition  | 2026-08-30                                                                                                           |
| Méthode         | Rédigé sur la base du code source v0.26.4 ; toute affirmation technique est vérifiable par référence `fichier:ligne` |

## Composition du dossier

| Réf. | Document                                                                       | Objet                                                                                                                                                                         | Public visé                                |
| ---- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ |
| 01   | [Documentation technique de déploiement](01-deploiement-technique.md)          | Architecture déployée, composants installés, prérequis, flux de données, configuration réseau et sécurité, **activation de l'authentification** (§7.2), installation, recette | Architectes, infrastructure, RSSI          |
| 02   | [Guide d'intégration IT](02-integration-it.md)                                 | Procédures d'exploitation, monitoring, sauvegarde/restauration, mise à jour, rollback, runbooks d'incident, checklists                                                        | Exploitation, DBA, supervision             |
| 03   | [Deep dive architecture & algorithme](03-deep-dive-architecture-algorithme.md) | Fonctionnement interne : crates, pipeline d'ingestion, modèle de données, moteur d'interrogation, décisions d'architecture                                                    | Architectes, développeurs, data scientists |
| 04   | [Langfuse — compatibilité & Kubernetes](04-langfuse-kubernetes.md)             | Compatibilité de version Langfuse (**3.1 incompatible**), déploiement en pods séparés, remédiation et validation                                                              | Exploitation, architectes                  |

## Historique des révisions

| Version | Date           | Évolution                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ------- | -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1.0     | 2026-08-17     | Édition initiale des documents 01–03                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 1.1     | 2026-08-19     | Doc 01 : refonte §7.2 en procédure complète d'activation de l'authentification (modes, amorçage, clés d'API, OIDC, sessions, dépannage). Ajout du doc 04 et de la présente page de garde                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 1.2     | 2026-08-19     | Passe de vérification factuelle intégrale contre le code v0.25.0. Corrections : décompte des migrations (146 fichiers SQL, 001→148 non contigu), version Web UI (Next.js 16), mode de livraison multi-réplique (`EDGEQUAKE_TASK_DELIVERY=notify_only`), noms des tables `document_originals` / `document_mm_assets`, schéma de création des clés d'API (`name`/`scopes`/`expires_in_days`), statut pdfium hors-ligne (embarqué à la compilation — doc 04), statuts de tâche en minuscules dans les exemples `jq`/SQL, formulation exacte du contrôle des versions d'extensions au démarrage. Clarification du fichier `docker-compose.prod.yml` (modèle dérivé, non livré tel quel) |
| **1.3** | **2026-08-26** | **Mise à niveau v0.25.0 → v0.26.1** : schéma 148 → **149** (`149_tasks_document_id_column`, SAFE), CLI `migrate` SPEC-137 (alias `--drop-confirm`, drapeaux inconnus fail-closed, sous-commandes `plan` / `guard`), remplissage PDF au budget SPEC-135 (`EDGEQUAKE_PDF_PACK`, citations `p.N–M`), convert page-as-unit SPEC-134, tags d'images 0.26.1. Renumérotation du document Langfuse en **04**                                                                                                                                                                                                                                                                                |
| **1.4** | **2026-08-27** | **Pin produit v0.26.2** : Langfuse 3.1.x ingestion fallback (SPEC-124), Helm/kind Kubernetes (SPEC-138), SSE, `include_stats`. Schéma **149** inchangé. |
| **1.5** | **2026-08-28** | **Pin produit v0.26.3** : SPEC-139 moteur de copie mid-cutover (iw2 21000, W3 coverage-sum, remainder). Schéma **149** inchangé. |
| **1.6** | **2026-08-30** | **Pin produit v0.26.4** : SPEC-144 Next.js 16.3.3 (August Critical RCEs) + proxy SSOT; SPEC-140/141 listes; distroless API. Schéma **149** inchangé. |

## Conventions

- Les commandes sont données pour un déploiement Docker Compose ; les équivalents
  Kubernetes sont référencés lorsqu'ils diffèrent.
- `API` désigne l'hôte du service EdgeQuake ; remplacer par l'URL réelle.
- Les valeurs entre `<…>` proviennent du coffre de secrets d'entreprise et ne
  doivent jamais figurer dans un fichier versionné.
