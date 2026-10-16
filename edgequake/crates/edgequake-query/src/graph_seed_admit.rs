//! Graph-first entity admission for Ask / exact-name queries.
//!
//! When typed ANN misses (wrong embedding model key, missing vectors), the
//! selected entity and exact label hits still exist in AGE. Admit them before
//! the empty-context apology fires: resolve via [`EntityId::exact_lookup_candidates`],
//! load the node, expand one hop, and pull `source_chunk_ids` from KV.

use std::collections::HashSet;

use edgequake_storage::traits::GraphReadView;
use edgequake_storage::EntityId;

use crate::context::{QueryContext, RetrievedChunk};
use crate::graph_ppr::GraphWalkMode;
use crate::helpers::{build_entity_from_node, build_relationship_from_edge};
use crate::keywords::ExtractedKeywords;

fn bare_entity_norm(name: &str) -> String {
    let bare = name.rsplit("::").next().unwrap_or(name);
    EntityId::new(bare).as_str().to_string()
}

pub const META_GRAPH_SEED_ENTITIES: &str = "graph_seed_entities";
pub const META_GRAPH_SEED_CHUNK_IDS: &str = "graph_seed_chunk_ids";

const MAX_SEED_CHUNKS: usize = 8;
const MAX_LABEL_CANDIDATES: usize = 8;
/// Source pages considered for a question-named entity before lexical rank.
const MAX_NAMED_PAGE_CANDIDATES: usize = 24;
/// How many of those pages move to the front, after abbreviation pages.
const MAX_NAMED_PAGES_FRONT: usize = 4;

/// Admit explicit seed ids and exact label matches into `context`.
#[allow(clippy::too_many_arguments)] // storage views + seeds + scope; keep call sites flat
pub async fn admit_into_context(
    graph: GraphReadView<'_>,
    kv: Option<&dyn edgequake_storage::traits::KVStorage>,
    context: &mut QueryContext,
    seed_entity_ids: &[String],
    query_text: &str,
    keywords: &ExtractedKeywords,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
) -> crate::error::Result<usize> {
    let mut seeds: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for raw in seed_entity_ids {
        let t = raw.trim();
        if t.is_empty() || !seen.insert(t.to_string()) {
            continue;
        }
        seeds.push(t.to_string());
    }

    for cand in exact_label_candidates(query_text, keywords) {
        if seen.insert(cand.clone()) {
            seeds.push(cand);
        }
    }

    if seeds.is_empty() {
        return Ok(0);
    }

    let mut resolved_ids: Vec<String> = Vec::new();
    let mut resolved_seen: HashSet<String> = HashSet::new();

    for seed in &seeds {
        if let Some(id) = resolve_seed_node_id(&graph, seed, workspace_id).await? {
            if resolved_seen.insert(id.clone()) {
                resolved_ids.push(id);
            }
        }
    }

    if resolved_ids.is_empty() {
        return Ok(0);
    }

    let existing: HashSet<String> = context
        .entities
        .iter()
        .map(|e| bare_entity_norm(&e.name))
        .collect();
    let (nodes_map, degrees) = tokio::join!(
        graph.get_nodes_batch(&resolved_ids),
        graph.node_degrees_batch(&resolved_ids),
    );
    let nodes_map = nodes_map?;
    let degrees: std::collections::HashMap<String, usize> = degrees?.into_iter().collect();

    let mut admitted: Vec<String> = Vec::new();
    // Pages are kept in stored order, then truncated to `MAX_SEED_CHUNKS`.
    // A shorter admitted name can occupy that whole window, so the longer
    // name's pages — the ones the question spelled — are collected first.
    // The same longer-name order is used when those names' edges are prepended.
    let mut chunk_groups: Vec<(usize, Vec<String>)> = Vec::new();

    for id in &resolved_ids {
        if existing.contains(&bare_entity_norm(id)) {
            continue;
        }
        let Some(node) = nodes_map.get(id) else {
            continue;
        };
        let degree = degrees.get(id).copied().unwrap_or(0);
        let entity = build_entity_from_node(id, &node.properties, degree, 1.0);
        let mut ids = Vec::new();
        for cid in &entity.source_chunk_ids {
            if !ids.iter().any(|x| x == cid) {
                ids.push(cid.clone());
            }
        }
        let words = bare_entity_norm(id)
            .split('_')
            .filter(|w| !w.is_empty())
            .count();
        chunk_groups.push((words, ids));
        admitted.push(id.clone());
        context.add_entity(entity);
    }
    let chunk_ids = seed_chunks_within_window(chunk_groups, usize::MAX);

    if admitted.is_empty() {
        return Ok(0);
    }

    let edges = crate::graph_expand::expand_neighborhood_edges(
        &graph,
        &admitted,
        1,
        64,
        GraphWalkMode::Bfs,
        tenant_id,
        workspace_id,
    )
    .await?;

    let existing_rel: HashSet<(String, String, String)> = context
        .relationships
        .iter()
        .map(|r| (r.source.clone(), r.target.clone(), r.relation_type.clone()))
        .collect();

    for edge in edges {
        let key = (
            edge.source.clone(),
            edge.target.clone(),
            edge.properties
                .get("relation_type")
                .and_then(|v| v.as_str())
                .unwrap_or("RELATED_TO")
                .to_string(),
        );
        if existing_rel.contains(&key) {
            continue;
        }
        let rel = build_relationship_from_edge(&edge.source, &edge.target, &edge.properties);
        context.add_relationship(rel);
    }

    // Neighbor nodes referenced by new edges (1-hop star).
    let mut neighbor_ids: Vec<String> = Vec::new();
    let mut neighbor_seen: HashSet<String> = admitted.iter().cloned().collect();
    for rel in &context.relationships {
        for endpoint in [&rel.source, &rel.target] {
            if neighbor_seen.insert(endpoint.clone()) {
                neighbor_ids.push(endpoint.clone());
            }
        }
    }
    if !neighbor_ids.is_empty() {
        let n_map = graph.get_nodes_batch(&neighbor_ids).await?;
        let n_deg: std::collections::HashMap<String, usize> = graph
            .node_degrees_batch(&neighbor_ids)
            .await?
            .into_iter()
            .collect();
        let have: HashSet<String> = context
            .entities
            .iter()
            .map(|e| bare_entity_norm(&e.name))
            .collect();
        for id in &neighbor_ids {
            if have.contains(&bare_entity_norm(id)) {
                continue;
            }
            if let Some(node) = n_map.get(id) {
                let degree = n_deg.get(id).copied().unwrap_or(0);
                context.add_entity(build_entity_from_node(id, &node.properties, degree, 0.5));
            }
        }
    }

    materialize_seed_chunks(kv, context, &chunk_ids).await;

    context.metadata.insert(
        META_GRAPH_SEED_ENTITIES.to_string(),
        serde_json::json!(admitted),
    );
    if !chunk_ids.is_empty() {
        context.metadata.insert(
            META_GRAPH_SEED_CHUNK_IDS.to_string(),
            serde_json::json!(chunk_ids),
        );
    }

    tracing::info!(
        admitted = admitted.len(),
        seed_ids = seed_entity_ids.len(),
        chunks = chunk_ids.len(),
        "graph_seed_admit: AGE entities admitted before/alongside ANN"
    );

    Ok(admitted.len())
}

/// Words of this entity's stored name, with a trailing `_(HCC)` acronym removed.
pub fn name_phrase_in_question(entity_name: &str) -> Option<String> {
    let bare = entity_name.rsplit("::").next().unwrap_or(entity_name);
    let bare = strip_trailing_acronym(bare);
    let phrase = bare
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if phrase.is_empty() {
        None
    } else {
        Some(phrase)
    }
}

/// Acronym stored on the node, as in `HEPATOCELLULAR_CARCINOMA_(HCC)`.
fn stored_acronym(entity_name: &str) -> Option<String> {
    let bare = entity_name.rsplit("::").next().unwrap_or(entity_name);
    let idx = bare.rfind("_(")?;
    let rest = &bare[idx + 2..];
    if rest.ends_with(')')
        && rest.len() > 1
        && rest[..rest.len() - 1]
            .chars()
            .all(|c| c.is_ascii_alphanumeric())
    {
        Some(rest[..rest.len() - 1].to_ascii_uppercase())
    } else {
        None
    }
}

fn strip_trailing_acronym(name: &str) -> &str {
    let Some(idx) = name.rfind("_(") else {
        return name;
    };
    let rest = &name[idx + 2..];
    if rest.ends_with(')')
        && rest.len() > 1
        && rest[..rest.len() - 1]
            .chars()
            .all(|c| c.is_ascii_alphanumeric())
    {
        &name[..idx]
    } else {
        name
    }
}

fn question_words(question: &str) -> Vec<String> {
    question
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_ascii_lowercase())
        .collect()
}

fn question_has_phrase(question: &str, phrase: &str) -> bool {
    let words = question_words(question);
    let phrase_words: Vec<&str> = phrase.split_whitespace().collect();
    if phrase_words.is_empty() || words.len() < phrase_words.len() {
        return false;
    }
    words.windows(phrase_words.len()).any(|window| {
        window
            .iter()
            .zip(phrase_words.iter())
            .all(|(word, phrase_word)| word == phrase_word)
    })
}

/// Abbreviations the question itself prints in all capitals (`MGZL`, `ALL`).
///
/// Ordinary words keep a lowercase letter, so `all patients` does not match `ALL`.
/// A one-letter token is the word `A` or `I`, not an abbreviation.
fn all_caps_question_tokens(question: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut current = String::new();
    let flush = |current: &mut String, out: &mut HashSet<String>| {
        let abbrev =
            current.chars().count() >= 2 && current.chars().all(|c| !c.is_ascii_lowercase());
        if abbrev {
            out.insert(std::mem::take(current).to_ascii_lowercase());
        } else {
            current.clear();
        }
    };
    for c in question.chars() {
        if c.is_ascii_alphanumeric() {
            current.push(c);
        } else {
            flush(&mut current, &mut out);
        }
    }
    flush(&mut current, &mut out);
    out
}

/// Words of the sentence that ends at this parenthesis, longest suffix first.
fn name_suffixes_before_paren(before: &str) -> Vec<String> {
    let sentence = before
        .rsplit(['.', '?', '!', '\n'])
        .next()
        .unwrap_or(before);
    let words: Vec<String> = sentence
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_ascii_lowercase())
        .collect();
    (0..words.len())
        .map(|start| words[start..].join(" "))
        .collect()
}

/// `(TOKEN)` sites whose token is an all-caps abbreviation in the question.
fn parenthetical_sites(text: &str, tokens: &HashSet<String>) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'(' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j].is_ascii_alphanumeric() {
                j += 1;
            }
            if j > start && j < bytes.len() && bytes[j] == b')' {
                let token = text[start..j].to_ascii_lowercase();
                if tokens.contains(&token) {
                    found.push((text[..i].to_string(), token));
                }
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    found
}

/// `inner` occurs only as part of the longer stored name `outer`.
fn phrase_covers(outer: &str, inner: &str) -> bool {
    let outer_words: Vec<&str> = outer.split_whitespace().collect();
    let inner_words: Vec<&str> = inner.split_whitespace().collect();
    if inner_words.is_empty() || outer_words.len() <= inner_words.len() {
        return false;
    }
    outer_words
        .windows(inner_words.len())
        .any(|window| window == inner_words.as_slice())
}

/// One-hop edges of entities the question already names.
///
/// Vector search often seeds the parent ("liver cancer") and only then
/// discovers the named entity as a neighbor. Depth 1 from the parent does
/// not return that neighbor's other edges, so the subtype edge is missing
/// from the prompt. Those edges are prepended so token truncation keeps them.
pub async fn expand_entities_named_in_query(
    graph: GraphReadView<'_>,
    context: &mut QueryContext,
    query_text: &str,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
    kv: Option<&dyn edgequake_storage::traits::KVStorage>,
) -> crate::error::Result<usize> {
    let mut named: Vec<(String, String, bool)> = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();
    for entity in &context.entities {
        let Some(phrase) = name_phrase_in_question(&entity.name) else {
            continue;
        };
        let by_name = question_has_phrase(query_text, &phrase);
        let by_stored_acronym = stored_acronym(&entity.name).is_some_and(|acronym| {
            question_words(query_text)
                .iter()
                .any(|word| word.eq_ignore_ascii_case(&acronym))
        });
        if !by_name && !by_stored_acronym {
            continue;
        }
        let Some(id) = resolve_seed_node_id(&graph, &entity.name, workspace_id).await? else {
            continue;
        };
        if seen_ids.insert(id.clone()) {
            named.push((id, phrase, by_name));
        }
    }
    // A retrieved page can place the question's abbreviation in parentheses
    // after the entity's stored name: "mediastinal gray zone lymphoma (MGZL)".
    // The longest suffix of that sentence that is a stored entity is the name.
    // No abbreviation is invented from the entity's initials.
    let mut from_abbreviation: HashSet<String> = HashSet::new();
    let acronyms = all_caps_question_tokens(query_text);
    if !acronyms.is_empty() {
        let mut sites = Vec::new();
        for chunk in &context.chunks {
            sites.extend(parenthetical_sites(&chunk.content, &acronyms));
        }
        if let Some(citation_chunks) = &context.citation_chunks {
            for chunk in citation_chunks {
                sites.extend(parenthetical_sites(&chunk.content, &acronyms));
            }
        }
        for (before, token) in sites {
            for phrase in name_suffixes_before_paren(&before) {
                if named.iter().any(|(_, existing, _)| existing == &phrase) {
                    break;
                }
                let Some(id) =
                    resolve_abbreviation_node(&graph, &phrase, &token, workspace_id).await?
                else {
                    continue;
                };
                if seen_ids.insert(id.clone()) {
                    from_abbreviation.insert(id.clone());
                    named.push((id, phrase, true));
                }
                break;
            }
        }
    }
    // A shorter name that occurs only inside a longer name the question spelled
    // is not a separate node the question named.
    let spelled: Vec<String> = named
        .iter()
        .filter(|(_, _, by_name)| *by_name)
        .map(|(_, phrase, _)| phrase.clone())
        .collect();
    named.retain(|(_, phrase, by_name)| {
        !*by_name || !spelled.iter().any(|longer| phrase_covers(longer, phrase))
    });
    if named.is_empty() {
        return Ok(0);
    }
    // Longer spelled names are prepended last, so prefix truncation keeps them.
    named.sort_by(|a, b| {
        a.1.split_whitespace()
            .count()
            .cmp(&b.1.split_whitespace().count())
    });
    // A shared 64-edge walk is filled by shorter names that also occur in the
    // question ("chemotherapy") before the abbreviation's node is reached.
    // That node's incident edges are the ones the parenthesis named, so they
    // are read on their own and prepended after the other new edges.
    let other_seeds: Vec<String> = named
        .iter()
        .filter(|(id, _, _)| !from_abbreviation.contains(id))
        .map(|(id, _, _)| id.clone())
        .collect();
    let abbrev_seeds: Vec<String> = named
        .iter()
        .filter(|(id, _, _)| from_abbreviation.contains(id))
        .map(|(id, _, _)| id.clone())
        .collect();

    let mut edges = crate::graph_expand::expand_neighborhood_edges(
        &graph,
        &other_seeds,
        1,
        64,
        GraphWalkMode::Bfs,
        tenant_id,
        workspace_id,
    )
    .await?;
    let abbrev_edges = crate::graph_expand::expand_neighborhood_edges(
        &graph,
        &abbrev_seeds,
        1,
        64,
        GraphWalkMode::Bfs,
        tenant_id,
        workspace_id,
    )
    .await?;
    edges.extend(abbrev_edges);

    let mut existing_rel: HashSet<(String, String, String)> = context
        .relationships
        .iter()
        .map(|r| (r.source.clone(), r.target.clone(), r.relation_type.clone()))
        .collect();
    let mut added_rels = Vec::new();
    for edge in edges {
        let rel = build_relationship_from_edge(&edge.source, &edge.target, &edge.properties);
        let key = (
            rel.source.clone(),
            rel.target.clone(),
            rel.relation_type.clone(),
        );
        if !existing_rel.insert(key) {
            continue;
        }
        added_rels.push(rel);
    }
    let added = added_rels.len();
    // A node already in the context is not admitted again, so its source
    // pages stay unloaded. The question's own abbreviation (`ALL`, `MGZL`)
    // names that node. Those pages are the ones LightRAG reads from
    // `source_id` and are placed first so prefix truncation keeps them.
    let acronym_pages =
        acronym_source_chunk_ids(&graph, context, &named, &from_abbreviation, query_text).await?;
    // LightRAG reads `source_id` for every selected entity and round-robins
    // those pages with vector hits, so the named disease is in the prefix.
    // Cosine over the whole pool prefers a neighbor that shares a generic
    // word ("risk", "stage"). Rank only the pages of entities the question
    // named, and keep the best of those behind the abbreviation pages.
    let ranked_pages =
        ranked_named_source_chunk_ids(&graph, context, &named, query_text, kv).await?;
    let mut front_pages = acronym_pages;
    for id in ranked_pages {
        if !front_pages.iter().any(|existing| existing == &id) {
            front_pages.push(id);
        }
    }
    prepend_chunks(kv, context, &front_pages).await;
    if added == 0 {
        return Ok(0);
    }

    let mut neighbor_ids: Vec<String> = Vec::new();
    let mut neighbor_seen: HashSet<String> = named.iter().map(|(id, _, _)| id.clone()).collect();
    for rel in &added_rels {
        for endpoint in [&rel.source, &rel.target] {
            if neighbor_seen.insert(endpoint.clone()) {
                neighbor_ids.push(endpoint.clone());
            }
        }
    }
    let mut added_entities = Vec::new();
    if !neighbor_ids.is_empty() {
        let n_map = graph.get_nodes_batch(&neighbor_ids).await?;
        let n_deg: std::collections::HashMap<String, usize> = graph
            .node_degrees_batch(&neighbor_ids)
            .await?
            .into_iter()
            .collect();
        let have: HashSet<String> = context
            .entities
            .iter()
            .map(|e| bare_entity_norm(&e.name))
            .collect();
        for id in &neighbor_ids {
            if have.contains(&bare_entity_norm(id)) {
                continue;
            }
            if let Some(node) = n_map.get(id) {
                let degree = n_deg.get(id).copied().unwrap_or(0);
                added_entities.push(build_entity_from_node(id, &node.properties, degree, 1.0));
            }
        }
    }

    for entity in added_entities.into_iter().rev() {
        context.entities.insert(0, entity);
    }
    for rel in added_rels.into_iter().rev() {
        context.relationships.insert(0, rel);
    }
    Ok(added)
}

/// Exact id only. The parenthetical scan tries many suffixes; label search
/// would treat a non-name as a fuzzy hit.
async fn exact_node_id(
    graph: &GraphReadView<'_>,
    raw: &str,
    workspace_id: Option<&str>,
) -> crate::error::Result<Option<String>> {
    for cand in EntityId::exact_lookup_candidates(raw, workspace_id) {
        if let Some(node) = graph.get_node(&cand).await? {
            return Ok(Some(node.id));
        }
    }
    Ok(None)
}

/// Prefer the node that stores this parenthetical (`NAME_(ALL)`) over the bare name.
async fn resolve_abbreviation_node(
    graph: &GraphReadView<'_>,
    phrase: &str,
    token: &str,
    workspace_id: Option<&str>,
) -> crate::error::Result<Option<String>> {
    let decorated = format!("{phrase} ({})", token.to_ascii_uppercase());
    if let Some(id) = exact_node_id(graph, &decorated, workspace_id).await? {
        return Ok(Some(id));
    }
    exact_node_id(graph, phrase, workspace_id).await
}

async fn resolve_seed_node_id(
    graph: &GraphReadView<'_>,
    raw: &str,
    workspace_id: Option<&str>,
) -> crate::error::Result<Option<String>> {
    for cand in EntityId::exact_lookup_candidates(raw, workspace_id) {
        if let Some(node) = graph.get_node(&cand).await? {
            return Ok(Some(node.id));
        }
    }

    // Lexical label search: first exact (case-insensitive) hit.
    let labels = graph
        .search_labels(raw, 5, None, workspace_id)
        .await
        .unwrap_or_default();
    let want = EntityId::new(EntityId::bare_name_from_graph_node_id(raw));
    for label in labels {
        let bare = EntityId::bare_name_from_graph_node_id(&label);
        if EntityId::new(bare).as_str() == want.as_str() || label.eq_ignore_ascii_case(raw) {
            for cand in EntityId::exact_lookup_candidates(&label, workspace_id) {
                if let Some(node) = graph.get_node(&cand).await? {
                    return Ok(Some(node.id));
                }
            }
        }
    }
    Ok(None)
}

/// Tokens / keywords that may be exact entity names (includes hyphenated unigrams).
pub fn exact_label_candidates(query_text: &str, keywords: &ExtractedKeywords) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let push = |s: &str, out: &mut Vec<String>, seen: &mut HashSet<String>| {
        let t = s.trim();
        if t.len() < 2 || !seen.insert(t.to_ascii_lowercase()) {
            return;
        }
        out.push(t.to_string());
    };

    for kw in keywords.low_level.iter().chain(keywords.high_level.iter()) {
        push(kw, &mut out, &mut seen);
        if out.len() >= MAX_LABEL_CANDIDATES {
            return out;
        }
    }

    // Hyphenated / underscored tokens from the question (e.g. Gemma3-4b).
    for tok in query_text.split(|c: char| c.is_whitespace() || "?,.;:!()[]{}\"'".contains(c)) {
        let t = tok.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_');
        if t.len() >= 3
            && (t.contains('-') || t.contains('_') || t.chars().any(|c| c.is_ascii_digit()))
        {
            push(t, &mut out, &mut seen);
            if out.len() >= MAX_LABEL_CANDIDATES {
                return out;
            }
        }
    }

    out
}

/// Longer stored names occupy the seed window first.
///
/// A shorter admitted name can hold enough source pages to fill the window
/// by itself. The page that belongs to the longer name is the one the
/// question spelled, so it is kept.
fn seed_chunks_within_window(mut groups: Vec<(usize, Vec<String>)>, window: usize) -> Vec<String> {
    groups.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let mut chunk_ids: Vec<String> = Vec::new();
    for (_, ids) in groups {
        for cid in ids {
            if !chunk_ids.iter().any(|existing| existing == &cid) {
                chunk_ids.push(cid);
            }
            if chunk_ids.len() >= window {
                return chunk_ids;
            }
        }
    }
    chunk_ids
}

/// Source pages of a node the question already names by its stored abbreviation.
///
/// Admission skips that node when it is already in the context, and the
/// in-memory entity often has an empty `source_chunk_ids` list. The ids live
/// on the graph node. LightRAG reads them from every selected entity.
async fn acronym_source_chunk_ids(
    graph: &GraphReadView<'_>,
    context: &QueryContext,
    named: &[(String, String, bool)],
    from_abbreviation: &HashSet<String>,
    query_text: &str,
) -> crate::error::Result<Vec<String>> {
    let tokens = all_caps_question_tokens(query_text);
    let mut wanted: Vec<(usize, String)> = Vec::new();
    for (id, phrase, _) in named {
        let stored = stored_acronym(id);
        let token_match = stored.as_ref().is_some_and(|acronym| {
            tokens
                .iter()
                .any(|token| token.eq_ignore_ascii_case(acronym))
        });
        if !token_match && !from_abbreviation.contains(id) {
            continue;
        }
        wanted.push((phrase.split_whitespace().count(), id.clone()));
    }
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<String> = wanted.iter().map(|(_, id)| id.clone()).collect();
    let nodes = graph.get_nodes_batch(&ids).await?;
    let mut groups = Vec::new();
    for (words, id) in wanted {
        let from_entity = context.entities.iter().find(|entity| {
            bare_entity_norm(&entity.name) == bare_entity_norm(&id)
                && !entity.source_chunk_ids.is_empty()
        });
        let chunk_ids = if let Some(entity) = from_entity {
            entity.source_chunk_ids.clone()
        } else if let Some(node) = nodes.get(&id) {
            crate::helpers::extract_entity_source_tracking(&node.properties).source_chunk_ids
        } else {
            Vec::new()
        };
        if !chunk_ids.is_empty() {
            groups.push((words, chunk_ids));
        }
    }
    Ok(seed_chunks_within_window(groups, MAX_SEED_CHUNKS))
}

/// Source pages of every entity the question named, best lexical match first.
///
/// The candidate pool is only those entities. A neighbor retrieved because it
/// shares a generic word is not in the pool, so it cannot occupy the prefix.
async fn ranked_named_source_chunk_ids(
    graph: &GraphReadView<'_>,
    context: &QueryContext,
    named: &[(String, String, bool)],
    query_text: &str,
    kv: Option<&dyn edgequake_storage::traits::KVStorage>,
) -> crate::error::Result<Vec<String>> {
    if named.is_empty() || query_text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut wanted: Vec<(usize, String)> = named
        .iter()
        .map(|(id, phrase, _)| (phrase.split_whitespace().count(), id.clone()))
        .collect();
    wanted.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    wanted.dedup_by(|a, b| a.1 == b.1);
    let ids: Vec<String> = wanted.iter().map(|(_, id)| id.clone()).collect();
    let nodes = graph.get_nodes_batch(&ids).await?;
    let mut groups = Vec::new();
    for (words, id) in wanted {
        let from_entity = context.entities.iter().find(|entity| {
            bare_entity_norm(&entity.name) == bare_entity_norm(&id)
                && !entity.source_chunk_ids.is_empty()
        });
        let chunk_ids = if let Some(entity) = from_entity {
            entity.source_chunk_ids.clone()
        } else if let Some(node) = nodes.get(&id) {
            crate::helpers::extract_entity_source_tracking(&node.properties).source_chunk_ids
        } else {
            Vec::new()
        };
        if !chunk_ids.is_empty() {
            groups.push((words, chunk_ids));
        }
    }
    let candidates = seed_chunks_within_window(groups, MAX_NAMED_PAGE_CANDIDATES);
    if candidates.is_empty() {
        return Ok(Vec::new());
    }
    let mut chunks = Vec::new();
    let mut missing = Vec::new();
    for id in &candidates {
        if let Some(existing) = context.chunks.iter().find(|chunk| chunk.id == *id) {
            chunks.push(existing.clone());
        } else {
            missing.push(id.clone());
        }
    }
    if !missing.is_empty() {
        if let Some(kv) = kv {
            let fetched =
                edgequake_storage::chunk_content::batch_fetch_chunk_contents(kv, &missing)
                    .await
                    .unwrap_or_default();
            for id in missing {
                if let Some(body) = fetched.get(&id) {
                    if !body.is_empty() {
                        chunks.push(RetrievedChunk::new(id, body.clone(), 1.0));
                    }
                }
            }
        }
    }
    if chunks.is_empty() {
        return Ok(Vec::new());
    }
    let ranked =
        crate::l2_bm25_union::bm25_order_chunks(query_text, &chunks, MAX_NAMED_PAGES_FRONT).await;
    Ok(ranked.into_iter().map(|chunk| chunk.id).collect())
}

/// Place `chunk_ids` at the front. Prefix truncation keeps that prefix.
async fn prepend_chunks(
    kv: Option<&dyn edgequake_storage::traits::KVStorage>,
    context: &mut QueryContext,
    chunk_ids: &[String],
) {
    if chunk_ids.is_empty() {
        return;
    }
    let mut already = Vec::new();
    let mut rest = Vec::new();
    for chunk in context.chunks.drain(..) {
        if chunk_ids.iter().any(|id| id == &chunk.id) {
            already.push(chunk);
        } else {
            rest.push(chunk);
        }
    }
    let missing: Vec<String> = chunk_ids
        .iter()
        .filter(|id| already.iter().all(|chunk| chunk.id != **id))
        .cloned()
        .collect();
    let fetched = if let Some(kv) = kv {
        edgequake_storage::chunk_content::batch_fetch_chunk_contents(kv, &missing)
            .await
            .unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };
    let mut prefix = Vec::new();
    for id in chunk_ids {
        if let Some(pos) = already.iter().position(|chunk| chunk.id == *id) {
            prefix.push(already.remove(pos));
        } else if let Some(body) = fetched.get(id) {
            if !body.is_empty() {
                prefix.push(RetrievedChunk::new(id.clone(), body.clone(), 1.0));
            }
        }
    }
    prefix.append(&mut rest);
    context.token_count = prefix.iter().map(|chunk| chunk.token_count).sum();
    context.chunks = prefix;
}

async fn materialize_seed_chunks(
    kv: Option<&dyn edgequake_storage::traits::KVStorage>,
    context: &mut QueryContext,
    chunk_ids: &[String],
) {
    let Some(kv) = kv else {
        return;
    };
    if chunk_ids.is_empty() {
        return;
    }
    let have: HashSet<&str> = context.chunks.iter().map(|c| c.id.as_str()).collect();
    let need: Vec<String> = chunk_ids
        .iter()
        .filter(|id| !have.contains(id.as_str()))
        .take(MAX_SEED_CHUNKS)
        .cloned()
        .collect();
    if need.is_empty() {
        return;
    }
    let Ok(fetched) = edgequake_storage::chunk_content::batch_fetch_chunk_contents(kv, &need).await
    else {
        return;
    };
    for id in need {
        if let Some(body) = fetched.get(&id) {
            if !body.is_empty() {
                context.add_chunk(RetrievedChunk::new(id, body.clone(), 1.0));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keywords::ExtractedKeywords;

    #[test]
    fn exact_label_candidates_include_hyphenated_token() {
        let kw = ExtractedKeywords::new(
            vec![],
            vec!["Gemma3-4b".into()],
            crate::keywords::QueryIntent::Exploratory,
        );
        let c = exact_label_candidates(
            "What is Gemma3-4b (Organization) in this knowledge graph?",
            &kw,
        );
        assert!(
            c.iter().any(|s| s.eq_ignore_ascii_case("Gemma3-4b")),
            "candidates={c:?}"
        );
    }

    #[test]
    fn stored_name_keeps_words_and_drops_only_an_empty_name() {
        assert_eq!(
            name_phrase_in_question("HEPATOCELLULAR_CARCINOMA_(HCC)"),
            Some("hepatocellular carcinoma".into())
        );
        assert_eq!(name_phrase_in_question("CANCER"), Some("cancer".into()));
        assert_eq!(
            name_phrase_in_question("LIVER_CANCER"),
            Some("liver cancer".into())
        );
        assert!(phrase_covers("liver cancer", "cancer"));
        assert!(!phrase_covers("hepatocellular carcinoma", "cancer"));
    }

    #[tokio::test]
    async fn named_entity_already_in_context_still_expands_its_edges() {
        use std::collections::HashMap;

        use crate::context::{QueryContext, RetrievedEntity, RetrievedRelationship};

        use edgequake_storage::adapters::memory::MemoryGraphStorage;
        use edgequake_storage::traits::{GraphReadView, GraphStorage, GraphStorageMutateOps};

        let graph = MemoryGraphStorage::new("named-expand");
        graph.initialize().await.unwrap();
        graph
            .upsert_node("HEPATOCELLULAR_CARCINOMA", HashMap::new())
            .await
            .unwrap();
        graph
            .upsert_edge(
                "FIBROLAMELLAR_HEPATOCELLULAR_CARCINOMA",
                "HEPATOCELLULAR_CARCINOMA",
                HashMap::from([("relation_type".into(), serde_json::json!("IS_A_SUBTYPE_OF"))]),
            )
            .await
            .unwrap();

        let mut context = QueryContext::new();
        context.add_entity(RetrievedEntity::new(
            "HEPATOCELLULAR_CARCINOMA",
            "DISEASE",
            "The most common type of primary liver cancer in adults.",
        ));
        context.add_relationship(RetrievedRelationship::new(
            "HEPATOCELLULAR_CARCINOMA",
            "LIVER_CANCER",
            "IS_A",
        ));

        let view = GraphReadView::new(&graph);
        let added = expand_entities_named_in_query(
            view,
            &mut context,
            "Which liver cancer subtype is listed under hepatocellular carcinoma in the ontology?",
            None,
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(added, 1);
        assert!(context.relationships.iter().any(|r| {
            r.relation_type == "IS_A_SUBTYPE_OF" && r.target == "HEPATOCELLULAR_CARCINOMA"
        }));
    }

    #[tokio::test]
    async fn stored_acronym_expands_and_invented_initials_do_not() {
        use std::collections::HashMap;

        use crate::context::{QueryContext, RetrievedEntity};
        use edgequake_storage::adapters::memory::MemoryGraphStorage;
        use edgequake_storage::traits::{GraphReadView, GraphStorage, GraphStorageMutateOps};

        let graph = MemoryGraphStorage::new("stored-acronym");
        graph.initialize().await.unwrap();
        graph
            .upsert_node("MEDIASTINAL_GRAY_ZONE_LYMPHOMA_(MGZL)", HashMap::new())
            .await
            .unwrap();
        graph
            .upsert_node("MEDIASTINAL_GRAY_ZONE_LYMPHOMA", HashMap::new())
            .await
            .unwrap();
        graph
            .upsert_edge(
                "MEDIASTINAL_GRAY_ZONE_LYMPHOMA_(MGZL)",
                "INVOLVED-SITE_RADIATION_THERAPY",
                HashMap::from([("relation_type".into(), serde_json::json!("TREATED_WITH"))]),
            )
            .await
            .unwrap();

        let mut stored = QueryContext::new();
        stored.add_entity(RetrievedEntity::new(
            "MEDIASTINAL_GRAY_ZONE_LYMPHOMA_(MGZL)",
            "DISEASE",
            "A lymphoma.",
        ));
        let added = expand_entities_named_in_query(
            GraphReadView::new(&graph),
            &mut stored,
            "Which additional therapy may be used for localized MGZL?",
            None,
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(added, 1);

        let mut invented = QueryContext::new();
        invented.add_entity(RetrievedEntity::new(
            "MEDIASTINAL_GRAY_ZONE_LYMPHOMA",
            "DISEASE",
            "A lymphoma.",
        ));
        let added = expand_entities_named_in_query(
            GraphReadView::new(&graph),
            &mut invented,
            "Which additional therapy may be used for localized MGZL?",
            None,
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(added, 0);
    }

    #[tokio::test]
    async fn shorter_name_inside_a_longer_spelled_name_is_not_expanded() {
        use std::collections::HashMap;

        use crate::context::{QueryContext, RetrievedEntity};
        use edgequake_storage::adapters::memory::MemoryGraphStorage;
        use edgequake_storage::traits::{GraphReadView, GraphStorage, GraphStorageMutateOps};

        let graph = MemoryGraphStorage::new("containment");
        graph.initialize().await.unwrap();
        graph
            .upsert_node("LIVER_CANCER", HashMap::new())
            .await
            .unwrap();
        graph.upsert_node("CANCER", HashMap::new()).await.unwrap();
        graph
            .upsert_edge(
                "CANCER",
                "RISK",
                HashMap::from([("relation_type".into(), serde_json::json!("HAS"))]),
            )
            .await
            .unwrap();
        graph
            .upsert_edge(
                "LIVER_CANCER",
                "HEPATOCELLULAR_CARCINOMA",
                HashMap::from([("relation_type".into(), serde_json::json!("INCLUDES"))]),
            )
            .await
            .unwrap();

        let mut context = QueryContext::new();
        context.add_entity(RetrievedEntity::new("LIVER_CANCER", "DISEASE", "Liver."));
        context.add_entity(RetrievedEntity::new("CANCER", "DISEASE", "Cancer."));
        let added = expand_entities_named_in_query(
            GraphReadView::new(&graph),
            &mut context,
            "Which liver cancer subtype is listed?",
            None,
            None,
            None,
        )
        .await
        .unwrap();
        assert!(added >= 1);
        assert!(context
            .relationships
            .iter()
            .any(|r| r.relation_type == "INCLUDES"));
        assert!(context
            .relationships
            .iter()
            .all(|r| r.relation_type != "HAS"));
    }

    #[test]
    fn parenthetical_name_is_the_longest_stored_suffix_not_an_invented_initialism() {
        let suffixes = name_suffixes_before_paren(
            "Earlier sentence. Questions to ask Mediastinal gray zone lymphoma ",
        );
        assert_eq!(
            suffixes.first().map(String::as_str),
            Some("questions to ask mediastinal gray zone lymphoma")
        );
        assert!(suffixes
            .iter()
            .any(|s| s == "mediastinal gray zone lymphoma"));
        assert!(suffixes.iter().any(|s| s == "lymphoma"));
        let tokens = all_caps_question_tokens("localized MGZL and all patients");
        assert!(tokens.contains("mgzl"));
        assert!(!tokens.contains("all"));
        assert!(!all_caps_question_tokens("Why is Surgery not used?").contains("surgery"));
    }

    #[tokio::test]
    async fn chunk_defined_abbreviation_expands_when_the_node_has_no_stored_acronym() {
        use std::collections::HashMap;

        use crate::context::{QueryContext, RetrievedChunk};
        use edgequake_storage::adapters::memory::MemoryGraphStorage;
        use edgequake_storage::traits::{GraphReadView, GraphStorage, GraphStorageMutateOps};

        let graph = MemoryGraphStorage::new("chunk-abbrev");
        graph.initialize().await.unwrap();
        graph
            .upsert_node("MEDIASTINAL_GRAY_ZONE_LYMPHOMA", HashMap::new())
            .await
            .unwrap();
        graph.upsert_node("LYMPHOMA", HashMap::new()).await.unwrap();
        graph
            .upsert_node("INVOLVED-SITE_RADIATION_THERAPY", HashMap::new())
            .await
            .unwrap();
        graph
            .upsert_edge(
                "MEDIASTINAL_GRAY_ZONE_LYMPHOMA",
                "INVOLVED-SITE_RADIATION_THERAPY",
                HashMap::from([("relation_type".into(), serde_json::json!("TREATED_WITH"))]),
            )
            .await
            .unwrap();
        graph
            .upsert_edge(
                "LYMPHOMA",
                "RISK",
                HashMap::from([("relation_type".into(), serde_json::json!("HAS"))]),
            )
            .await
            .unwrap();

        let mut context = QueryContext::new();
        context.add_chunk(RetrievedChunk::new(
            "chunk-mgzl",
            "Questions to ask Mediastinal gray zone lymphoma (MGZL) is usually treated with chemotherapy. Involved-site radiation therapy (ISRT) may be added in those with localized disease.",
            1.0,
        ));
        let added = expand_entities_named_in_query(
            GraphReadView::new(&graph),
            &mut context,
            "Which additional therapy may be used alongside chemotherapy for localized MGZL?",
            None,
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(added, 1);
        assert!(context.relationships.iter().any(|r| {
            r.relation_type == "TREATED_WITH"
                && (r.source == "MEDIASTINAL_GRAY_ZONE_LYMPHOMA"
                    || r.target == "MEDIASTINAL_GRAY_ZONE_LYMPHOMA")
        }));
        assert!(context
            .relationships
            .iter()
            .all(|r| r.relation_type != "HAS"));
    }

    #[tokio::test]
    async fn chunk_abbreviation_prefers_the_node_that_stores_the_same_acronym() {
        use std::collections::HashMap;

        use crate::context::QueryContext;
        use edgequake_storage::adapters::memory::MemoryGraphStorage;
        use edgequake_storage::traits::{GraphReadView, GraphStorage, GraphStorageMutateOps};

        let graph = MemoryGraphStorage::new("chunk-abbrev-prefer");
        graph.initialize().await.unwrap();
        graph
            .upsert_node("ACUTE_LYMPHOBLASTIC_LEUKEMIA", HashMap::new())
            .await
            .unwrap();
        graph
            .upsert_node("ACUTE_LYMPHOBLASTIC_LEUKEMIA_(ALL)", HashMap::new())
            .await
            .unwrap();
        graph
            .upsert_edge(
                "ACUTE_LYMPHOBLASTIC_LEUKEMIA",
                "T_LYMPHOCYTE",
                HashMap::from([("relation_type".into(), serde_json::json!("AFFECTS"))]),
            )
            .await
            .unwrap();
        graph
            .upsert_edge(
                "ACUTE_LYMPHOBLASTIC_LEUKEMIA_(ALL)",
                "BLOOD_TEST",
                HashMap::from([("relation_type".into(), serde_json::json!("REQUIRES"))]),
            )
            .await
            .unwrap();

        let mut context = QueryContext::new();
        context.add_chunk(crate::context::RetrievedChunk::new(
            "chunk-all",
            "Key points Acute lymphoblastic leukemia (ALL) requires several tests.",
            1.0,
        ));
        let added = expand_entities_named_in_query(
            GraphReadView::new(&graph),
            &mut context,
            "What diagnostic procedures are essential for confirming ALL in children?",
            None,
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(added, 1);
        assert!(context
            .relationships
            .iter()
            .any(|r| r.relation_type == "REQUIRES"));
        assert!(context
            .relationships
            .iter()
            .all(|r| r.relation_type != "AFFECTS"));
    }

    #[test]
    fn longer_name_page_survives_the_seed_window() {
        let shorter: Vec<String> = (0..8).map(|i| format!("short-{i}")).collect();
        let ids = seed_chunks_within_window(vec![(2, shorter), (4, vec!["gold-64".into()])], 8);
        assert_eq!(ids.len(), 8);
        assert_eq!(ids[0], "gold-64");
        assert!(!ids.iter().any(|id| id == "short-7"));
    }

    #[tokio::test]
    async fn acronym_node_already_in_context_contributes_its_source_page() {
        use std::collections::HashMap;
        use std::sync::Arc;

        use crate::context::{QueryContext, RetrievedChunk, RetrievedEntity};
        use edgequake_storage::adapters::memory::{MemoryGraphStorage, MemoryKVStorage};
        use edgequake_storage::traits::{
            GraphReadView, GraphStorage, GraphStorageMutateOps, KVStorage,
        };

        let graph = MemoryGraphStorage::new("acronym-pages");
        graph.initialize().await.unwrap();
        graph
            .upsert_node(
                "ACUTE_LYMPHOBLASTIC_LEUKEMIA_(ALL)",
                HashMap::from([("source_chunk_ids".into(), serde_json::json!(["gold-64"]))]),
            )
            .await
            .unwrap();

        let kv = Arc::new(MemoryKVStorage::new("acronym-pages"));
        kv.upsert(&[(
            "gold-64".into(),
            serde_json::json!({"content": "Clinical trials are preferred for BCR::ABL1-negative B-ALL."}),
        )])
        .await
        .unwrap();

        let mut context = QueryContext::new();
        context.add_entity(RetrievedEntity::new(
            "ACUTE_LYMPHOBLASTIC_LEUKEMIA_(ALL)",
            "DISEASE",
            "A leukemia.",
        ));
        for i in 0..8 {
            context.add_chunk(RetrievedChunk::new(
                format!("short-{i}"),
                "A shorter page.",
                0.2,
            ));
        }

        let added = expand_entities_named_in_query(
            GraphReadView::new(&graph),
            &mut context,
            "What is the role of clinical trials in the treatment of specific ALL subtypes?",
            None,
            None,
            Some(kv.as_ref()),
        )
        .await
        .unwrap();
        assert_eq!(added, 0);
        assert_eq!(context.chunks[0].id, "gold-64");
        assert!(context.chunks[0].content.contains("ABL1"));
    }

    #[tokio::test]
    async fn spelled_name_page_that_shares_the_question_leads() {
        use std::collections::HashMap;
        use std::sync::Arc;

        use crate::context::{QueryContext, RetrievedChunk, RetrievedEntity};
        use edgequake_storage::adapters::memory::{MemoryGraphStorage, MemoryKVStorage};
        use edgequake_storage::traits::{
            GraphReadView, GraphStorage, GraphStorageMutateOps, KVStorage,
        };

        let graph = MemoryGraphStorage::new("spelled-pages");
        graph.initialize().await.unwrap();
        graph
            .upsert_node(
                "BONE_CANCER",
                HashMap::from([(
                    "source_chunk_ids".into(),
                    serde_json::json!(["thyroid-page", "bone-page"]),
                )]),
            )
            .await
            .unwrap();
        graph
            .upsert_node(
                "THYROID_CANCER",
                HashMap::from([(
                    "source_chunk_ids".into(),
                    serde_json::json!(["thyroid-only"]),
                )]),
            )
            .await
            .unwrap();

        let kv = Arc::new(MemoryKVStorage::new("spelled-pages"));
        kv.upsert(&[
            (
                "thyroid-page".into(),
                serde_json::json!({"content": "Who is at risk? There are different types of thyroid cancer. Most are curable with the right treatment."}),
            ),
            (
                "bone-page".into(),
                serde_json::json!({"content": "Family history of cancer and germline mutations are risk factors for bone cancer."}),
            ),
            (
                "thyroid-only".into(),
                serde_json::json!({"content": "About thyroid cancer. Who is at risk?"}),
            ),
        ])
        .await
        .unwrap();

        let mut context = QueryContext::new();
        context.add_entity(RetrievedEntity::new(
            "BONE_CANCER",
            "DISEASE",
            "A cancer of bone.",
        ));
        context.add_entity(RetrievedEntity::new(
            "THYROID_CANCER",
            "DISEASE",
            "A cancer of the thyroid.",
        ));
        for i in 0..6 {
            context.add_chunk(RetrievedChunk::new(
                format!("filler-{i}"),
                "About thyroid cancer. Who is at risk?",
                0.9,
            ));
        }

        let added = expand_entities_named_in_query(
            GraphReadView::new(&graph),
            &mut context,
            "What are the main risk factors for bone cancer?",
            None,
            None,
            Some(kv.as_ref()),
        )
        .await
        .unwrap();
        assert_eq!(added, 0);
        assert_eq!(context.chunks[0].id, "bone-page");
        assert!(context
            .chunks
            .iter()
            .all(|chunk| chunk.id != "thyroid-only"));
    }
}
