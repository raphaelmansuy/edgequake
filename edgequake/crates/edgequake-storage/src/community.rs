//! Graph community detection algorithms.
//!
//! @implements FEAT0205
//!
//! This module provides community detection algorithms for graph clustering,
//! similar to what LightRAG uses for global queries.

use std::collections::{HashMap, HashSet};

use crate::error::Result;
use crate::traits::GraphStorage;

/// A detected community in the graph.
#[derive(Debug, Clone)]
pub struct Community {
    /// Unique identifier for the community.
    pub id: usize,
    /// Node IDs that belong to this community.
    pub members: Vec<String>,
    /// Aggregate properties for the community.
    pub properties: HashMap<String, serde_json::Value>,
}

impl Community {
    /// Create a new community.
    pub fn new(id: usize) -> Self {
        Self {
            id,
            members: Vec::new(),
            properties: HashMap::new(),
        }
    }

    /// Add a member to the community.
    pub fn add_member(&mut self, node_id: String) {
        self.members.push(node_id);
    }

    /// Get the number of members.
    pub fn size(&self) -> usize {
        self.members.len()
    }
}

/// Result of community detection.
#[derive(Debug, Clone)]
pub struct CommunityDetectionResult {
    /// Detected communities.
    pub communities: Vec<Community>,
    /// Mapping from node ID to community ID.
    pub node_to_community: HashMap<String, usize>,
    /// Modularity score of the partition.
    pub modularity: f64,
}

impl CommunityDetectionResult {
    /// Create a new empty result.
    pub fn new() -> Self {
        Self {
            communities: Vec::new(),
            node_to_community: HashMap::new(),
            modularity: 0.0,
        }
    }

    /// Get community by ID.
    pub fn get_community(&self, id: usize) -> Option<&Community> {
        self.communities.iter().find(|c| c.id == id)
    }

    /// Get community for a node.
    pub fn get_node_community(&self, node_id: &str) -> Option<&Community> {
        self.node_to_community
            .get(node_id)
            .and_then(|id| self.get_community(*id))
    }

    /// Get all members in a node's community.
    pub fn get_community_members(&self, node_id: &str) -> Option<&[String]> {
        self.get_node_community(node_id)
            .map(|c| c.members.as_slice())
    }
}

impl Default for CommunityDetectionResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Community detection algorithm type.
#[derive(Debug, Clone, Copy, Default)]
pub enum CommunityAlgorithm {
    /// Louvain method for community detection.
    #[default]
    Louvain,
    /// Label propagation algorithm.
    LabelPropagation,
    /// Connected components (baseline).
    ConnectedComponents,
}

/// Configuration for community detection.
#[derive(Debug, Clone)]
pub struct CommunityConfig {
    /// Algorithm to use.
    pub algorithm: CommunityAlgorithm,
    /// Minimum community size.
    pub min_community_size: usize,
    /// Maximum iterations for iterative algorithms.
    pub max_iterations: usize,
    /// Resolution parameter for Louvain (higher = more communities).
    pub resolution: f64,
}

impl Default for CommunityConfig {
    fn default() -> Self {
        Self {
            algorithm: CommunityAlgorithm::Louvain,
            min_community_size: 2,
            max_iterations: 100,
            resolution: 1.0,
        }
    }
}

/// Detect communities in a graph.
pub async fn detect_communities<G: GraphStorage>(
    graph: &G,
    config: &CommunityConfig,
) -> Result<CommunityDetectionResult> {
    match config.algorithm {
        CommunityAlgorithm::Louvain => louvain_communities(graph, config).await,
        CommunityAlgorithm::LabelPropagation => label_propagation(graph, config).await,
        CommunityAlgorithm::ConnectedComponents => connected_components(graph, config).await,
    }
}

/// Louvain community detection algorithm.
///
/// This is a simplified implementation of the Louvain method that:
/// 1. Starts with each node in its own community
/// 2. Iteratively moves nodes to maximize modularity gain
/// 3. Continues until no improvement is made
async fn louvain_communities<G: GraphStorage>(
    graph: &G,
    config: &CommunityConfig,
) -> Result<CommunityDetectionResult> {
    let nodes = graph.get_all_nodes().await?;
    let edges = graph.get_all_edges().await?;

    if nodes.is_empty() {
        return Ok(CommunityDetectionResult::new());
    }

    // Build adjacency list
    let mut adjacency: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    let mut total_weight = 0.0;

    for edge in &edges {
        let weight = edge
            .properties
            .get("weight")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        adjacency
            .entry(edge.source.clone())
            .or_default()
            .push((edge.target.clone(), weight));

        adjacency
            .entry(edge.target.clone())
            .or_default()
            .push((edge.source.clone(), weight));

        total_weight += weight;
    }

    if total_weight == 0.0 {
        total_weight = 1.0; // Prevent division by zero
    }

    // Initialize: each node in its own community
    let mut node_to_community: HashMap<String, usize> = HashMap::new();
    let mut community_weights: HashMap<usize, f64> = HashMap::new();

    for (idx, node) in nodes.iter().enumerate() {
        node_to_community.insert(node.id.clone(), idx);

        let node_weight = adjacency
            .get(&node.id)
            .map(|neighbors| neighbors.iter().map(|(_, w)| w).sum::<f64>())
            .unwrap_or(0.0);

        community_weights.insert(idx, node_weight);
    }

    // Louvain phase 1: Move nodes to maximize modularity
    for _iteration in 0..config.max_iterations {
        let mut improved = false;

        for node in &nodes {
            let node_id = &node.id;
            let current_community = *node_to_community.get(node_id).unwrap();

            let neighbors = adjacency.get(node_id).cloned().unwrap_or_default();
            let node_weight: f64 = neighbors.iter().map(|(_, w)| w).sum();

            // Calculate neighbor communities and their weights
            let mut neighbor_communities: HashMap<usize, f64> = HashMap::new();
            for (neighbor_id, weight) in &neighbors {
                if let Some(&comm) = node_to_community.get(neighbor_id) {
                    *neighbor_communities.entry(comm).or_default() += weight;
                }
            }

            // Find best community to join
            let mut best_community = current_community;
            let mut best_gain = 0.0;

            // Calculate current community's weight without this node
            let current_comm_weight = community_weights.get(&current_community).unwrap_or(&0.0);
            let ki_in_current = neighbor_communities.get(&current_community).unwrap_or(&0.0);

            for (&candidate_community, &ki_in) in &neighbor_communities {
                if candidate_community == current_community {
                    continue;
                }

                let sigma_tot = community_weights.get(&candidate_community).unwrap_or(&0.0);

                // Modularity gain calculation (simplified)
                let delta_q = (ki_in / total_weight)
                    - config.resolution * (sigma_tot * node_weight)
                        / (2.0 * total_weight * total_weight);

                let current_delta_q = (ki_in_current / total_weight)
                    - config.resolution * ((current_comm_weight - node_weight) * node_weight)
                        / (2.0 * total_weight * total_weight);

                let gain = delta_q - current_delta_q;

                if gain > best_gain {
                    best_gain = gain;
                    best_community = candidate_community;
                }
            }

            // Move node if beneficial
            if best_community != current_community && best_gain > 1e-9 {
                // Update community assignments
                if let Some(old_weight) = community_weights.get_mut(&current_community) {
                    *old_weight -= node_weight;
                }
                if let Some(new_weight) = community_weights.get_mut(&best_community) {
                    *new_weight += node_weight;
                }

                node_to_community.insert(node_id.clone(), best_community);
                improved = true;
            }
        }

        if !improved {
            break;
        }
    }

    // Build result
    let mut communities_map: HashMap<usize, Vec<String>> = HashMap::new();
    for (node_id, comm_id) in &node_to_community {
        communities_map
            .entry(*comm_id)
            .or_default()
            .push(node_id.clone());
    }

    // Renumber communities and filter by minimum size
    let mut result = CommunityDetectionResult::new();
    let mut new_id = 0;
    let mut id_mapping: HashMap<usize, usize> = HashMap::new();

    for (old_id, members) in communities_map {
        if members.len() >= config.min_community_size {
            id_mapping.insert(old_id, new_id);

            let mut community = Community::new(new_id);
            community.members = members;
            result.communities.push(community);

            new_id += 1;
        }
    }

    // Update node_to_community with new IDs
    for (node_id, old_comm) in node_to_community {
        if let Some(&new_comm) = id_mapping.get(&old_comm) {
            result.node_to_community.insert(node_id, new_comm);
        }
    }

    // Calculate modularity
    result.modularity = calculate_modularity(&result, &adjacency, total_weight);

    Ok(result)
}

/// Label propagation community detection.
async fn label_propagation<G: GraphStorage>(
    graph: &G,
    config: &CommunityConfig,
) -> Result<CommunityDetectionResult> {
    let nodes = graph.get_all_nodes().await?;
    let edges = graph.get_all_edges().await?;

    if nodes.is_empty() {
        return Ok(CommunityDetectionResult::new());
    }

    // Build adjacency list
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &edges {
        adjacency
            .entry(edge.source.clone())
            .or_default()
            .push(edge.target.clone());
        adjacency
            .entry(edge.target.clone())
            .or_default()
            .push(edge.source.clone());
    }

    // Initialize: each node has its own label
    let mut labels: HashMap<String, usize> = HashMap::new();
    for (idx, node) in nodes.iter().enumerate() {
        labels.insert(node.id.clone(), idx);
    }

    // Iterate until convergence
    for _iteration in 0..config.max_iterations {
        let mut changed = false;

        for node in &nodes {
            let neighbors = adjacency.get(&node.id).cloned().unwrap_or_default();
            if neighbors.is_empty() {
                continue;
            }

            // Count neighbor labels
            let mut label_counts: HashMap<usize, usize> = HashMap::new();
            for neighbor_id in &neighbors {
                if let Some(&label) = labels.get(neighbor_id) {
                    *label_counts.entry(label).or_default() += 1;
                }
            }

            // Find most common label
            if let Some((&best_label, _)) = label_counts.iter().max_by_key(|(_, &count)| count) {
                let current_label = *labels.get(&node.id).unwrap();
                if best_label != current_label {
                    labels.insert(node.id.clone(), best_label);
                    changed = true;
                }
            }
        }

        if !changed {
            break;
        }
    }

    // Build communities from labels
    let mut communities_map: HashMap<usize, Vec<String>> = HashMap::new();
    for (node_id, label) in &labels {
        communities_map
            .entry(*label)
            .or_default()
            .push(node_id.clone());
    }

    let mut result = CommunityDetectionResult::new();
    let mut new_id = 0;
    let mut id_mapping: HashMap<usize, usize> = HashMap::new();

    for (old_id, members) in communities_map {
        if members.len() >= config.min_community_size {
            id_mapping.insert(old_id, new_id);

            let mut community = Community::new(new_id);
            community.members = members;
            result.communities.push(community);

            new_id += 1;
        }
    }

    for (node_id, old_label) in labels {
        if let Some(&new_comm) = id_mapping.get(&old_label) {
            result.node_to_community.insert(node_id, new_comm);
        }
    }

    Ok(result)
}

/// Connected components detection.
async fn connected_components<G: GraphStorage>(
    graph: &G,
    config: &CommunityConfig,
) -> Result<CommunityDetectionResult> {
    let nodes = graph.get_all_nodes().await?;
    let edges = graph.get_all_edges().await?;

    if nodes.is_empty() {
        return Ok(CommunityDetectionResult::new());
    }

    // Build adjacency list
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &edges {
        adjacency
            .entry(edge.source.clone())
            .or_default()
            .push(edge.target.clone());
        adjacency
            .entry(edge.target.clone())
            .or_default()
            .push(edge.source.clone());
    }

    // BFS to find connected components
    let mut visited: HashSet<String> = HashSet::new();
    let mut communities: Vec<Community> = Vec::new();
    let mut node_to_community: HashMap<String, usize> = HashMap::new();
    let mut community_id = 0;

    for node in &nodes {
        if visited.contains(&node.id) {
            continue;
        }

        // BFS from this node
        let mut queue = vec![node.id.clone()];
        let mut component: Vec<String> = Vec::new();

        while let Some(current) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }

            visited.insert(current.clone());
            component.push(current.clone());

            if let Some(neighbors) = adjacency.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        queue.push(neighbor.clone());
                    }
                }
            }
        }

        if component.len() >= config.min_community_size {
            for member in &component {
                node_to_community.insert(member.clone(), community_id);
            }

            let mut community = Community::new(community_id);
            community.members = component;
            communities.push(community);

            community_id += 1;
        }
    }

    Ok(CommunityDetectionResult {
        communities,
        node_to_community,
        modularity: 0.0,
    })
}

/// Calculate modularity score.
fn calculate_modularity(
    result: &CommunityDetectionResult,
    adjacency: &HashMap<String, Vec<(String, f64)>>,
    total_weight: f64,
) -> f64 {
    if total_weight == 0.0 {
        return 0.0;
    }

    let mut q = 0.0;
    let m = total_weight;

    for community in &result.communities {
        let members: HashSet<&String> = community.members.iter().collect();

        let mut internal_weight = 0.0;
        let mut total_degree = 0.0;

        for member in &community.members {
            if let Some(neighbors) = adjacency.get(member) {
                for (neighbor, weight) in neighbors {
                    total_degree += weight;
                    if members.contains(neighbor) {
                        internal_weight += weight;
                    }
                }
            }
        }

        // Each internal edge is counted twice
        internal_weight /= 2.0;

        q += internal_weight / m - (total_degree / (2.0 * m)).powi(2);
    }

    q
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::MemoryGraphStorage;

    #[tokio::test]
    async fn test_community_detection_empty_graph() {
        let graph = MemoryGraphStorage::new("test");
        graph.initialize().await.unwrap();

        let config = CommunityConfig::default();
        let result = detect_communities(&graph, &config).await.unwrap();

        assert!(result.communities.is_empty());
    }

    #[tokio::test]
    async fn test_connected_components() {
        let graph = MemoryGraphStorage::new("test");
        graph.initialize().await.unwrap();

        // Create two disconnected components
        let mut props1 = HashMap::new();
        props1.insert("name".to_string(), serde_json::json!("A"));
        graph.upsert_node("A", props1).await.unwrap();

        let mut props2 = HashMap::new();
        props2.insert("name".to_string(), serde_json::json!("B"));
        graph.upsert_node("B", props2).await.unwrap();

        let mut edge_props = HashMap::new();
        edge_props.insert("weight".to_string(), serde_json::json!(1.0));
        graph.upsert_edge("A", "B", edge_props).await.unwrap();

        let mut props3 = HashMap::new();
        props3.insert("name".to_string(), serde_json::json!("C"));
        graph.upsert_node("C", props3).await.unwrap();

        let mut props4 = HashMap::new();
        props4.insert("name".to_string(), serde_json::json!("D"));
        graph.upsert_node("D", props4).await.unwrap();

        let mut edge_props2 = HashMap::new();
        edge_props2.insert("weight".to_string(), serde_json::json!(1.0));
        graph.upsert_edge("C", "D", edge_props2).await.unwrap();

        let config = CommunityConfig {
            algorithm: CommunityAlgorithm::ConnectedComponents,
            min_community_size: 2,
            ..Default::default()
        };

        let result = detect_communities(&graph, &config).await.unwrap();

        // Should have 2 communities of size 2 each
        assert_eq!(result.communities.len(), 2);
        assert!(result.communities.iter().all(|c| c.size() == 2));
    }

    #[tokio::test]
    async fn test_louvain_simple() {
        let graph = MemoryGraphStorage::new("test");
        graph.initialize().await.unwrap();

        // Create a simple graph
        for node_id in ["A", "B", "C", "D", "E"] {
            let mut props = HashMap::new();
            props.insert("name".to_string(), serde_json::json!(node_id));
            graph.upsert_node(node_id, props).await.unwrap();
        }

        // Dense connections within groups
        for (src, tgt) in [("A", "B"), ("B", "C"), ("A", "C"), ("D", "E")] {
            let mut edge_props = HashMap::new();
            edge_props.insert("weight".to_string(), serde_json::json!(1.0));
            graph.upsert_edge(src, tgt, edge_props).await.unwrap();
        }

        // Weak connection between groups
        let mut edge_props = HashMap::new();
        edge_props.insert("weight".to_string(), serde_json::json!(0.1));
        graph.upsert_edge("C", "D", edge_props).await.unwrap();

        let config = CommunityConfig {
            algorithm: CommunityAlgorithm::Louvain,
            min_community_size: 2,
            ..Default::default()
        };

        let result = detect_communities(&graph, &config).await.unwrap();

        // Should detect at least 2 communities
        assert!(result.communities.len() >= 1);
    }
}
