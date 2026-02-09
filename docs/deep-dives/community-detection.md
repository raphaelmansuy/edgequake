# Deep Dive: Community Detection

> **How EdgeQuake Discovers Structure in Knowledge Graphs**

Community detection identifies clusters of densely connected entities in the knowledge graph. This enables the Global Query strategy to reason about themes and topics rather than individual entities.

---

## Overview

EdgeQuake implements graph clustering algorithms to discover communities:

```
┌─────────────────────────────────────────────────────────────────┐
│                    COMMUNITY DETECTION PIPELINE                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Input Graph:                                                   │
│                                                                 │
│     A──────B              E──────F                              │
│     │ ╲    │              │ ╲    │                              │
│     │  ╲   │              │  ╲   │                              │
│     C──────D  . . . . . . G──────H                              │
│                weak                                             │
│              connection                                         │
│                                                                 │
│  ══════════════════════════════════════════════════════════════ │
│                                                                 │
│  After Detection (Louvain):                                     │
│                                                                 │
│  ┌─────────────────┐      ┌─────────────────┐                   │
│  │  Community 0    │      │  Community 1    │                   │
│  │                 │      │                 │                   │
│  │   A──────B      │      │   E──────F      │                   │
│  │   │ ╲    │      │      │   │ ╲    │      │                   │
│  │   │  ╲   │      │      │   │  ╲   │      │                   │
│  │   C──────D      │      │   G──────H      │                   │
│  │                 │      │                 │                   │
│  └─────────────────┘      └─────────────────┘                   │
│                                                                 │
│  Modularity Score: 0.42 (good partition quality)                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Why Community Detection?

| Purpose                     | Benefit                                  |
| --------------------------- | ---------------------------------------- |
| **Global Queries**          | Summarize themes across related entities |
| **Hierarchical Navigation** | Browse knowledge by topic clusters       |
| **Entity Disambiguation**   | Context from community members           |
| **Relationship Discovery**  | Find implicit connections                |

---

## Core Data Structures

### Community

A detected cluster of related entities:

```rust
/// A detected community in the graph.
#[derive(Debug, Clone)]
pub struct Community {
    /// Unique identifier for the community
    pub id: usize,

    /// Node IDs that belong to this community
    pub members: Vec<String>,

    /// Aggregate properties (e.g., summary, keywords)
    pub properties: HashMap<String, serde_json::Value>,
}
```

**Properties:**

| Property     | Type        | Description                         |
| ------------ | ----------- | ----------------------------------- |
| `summary`    | String      | LLM-generated community description |
| `keywords`   | Vec<String> | Top keywords from members           |
| `importance` | f32         | Average member importance           |

### CommunityDetectionResult

Complete output from detection:

```rust
/// Result of community detection.
#[derive(Debug, Clone)]
pub struct CommunityDetectionResult {
    /// Detected communities
    pub communities: Vec<Community>,

    /// Mapping from node ID to community ID
    pub node_to_community: HashMap<String, usize>,

    /// Modularity score of the partition (0.0 to 1.0)
    pub modularity: f64,
}
```

**Modularity Interpretation:**

| Score     | Quality            |
| --------- | ------------------ |
| < 0.3     | Poor (random-like) |
| 0.3 - 0.5 | Moderate           |
| 0.5 - 0.7 | Good               |
| > 0.7     | Excellent          |

---

## Available Algorithms

### Louvain Method (Default)

Greedy modularity optimization in two phases:

```
┌─────────────────────────────────────────────────────────────────┐
│                    LOUVAIN ALGORITHM                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  PHASE 1: Local Optimization                                    │
│  ───────────────────────────                                    │
│                                                                 │
│  for each node n:                                               │
│    current_community = community(n)                             │
│    for each neighbor_community c:                               │
│      gain = modularity_gain(move n to c)                        │
│      if gain > best_gain:                                       │
│        best_gain = gain                                         │
│        best_community = c                                       │
│    if best_gain > 0:                                            │
│      move n to best_community                                   │
│                                                                 │
│  Repeat until no improvement                                    │
│                                                                 │
│  ══════════════════════════════════════════════════════════════ │
│                                                                 │
│  PHASE 2: Aggregation (simplified in EdgeQuake)                 │
│  ──────────────────────────────────────────────                 │
│                                                                 │
│  Collapse communities into super-nodes                          │
│  Repeat Phase 1 on aggregated graph                             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Modularity Gain Formula:**

```
ΔQ = [Σ_in + k_i,in] / 2m - [(Σ_tot + k_i) / 2m]²
     - Σ_in / 2m + (Σ_tot / 2m)² + (k_i / 2m)²

Where:
- Σ_in = sum of weights inside community
- Σ_tot = sum of weights to community
- k_i = node degree
- m = total edge weight
```

**Characteristics:**

| Attribute       | Value                       |
| --------------- | --------------------------- |
| Time Complexity | O(n log n) typical          |
| Resolution      | Configurable (default: 1.0) |
| Quality         | Best overall quality        |
| Use Case        | General-purpose clustering  |

---

### Label Propagation

Fast propagation of community labels:

```
┌─────────────────────────────────────────────────────────────────┐
│                    LABEL PROPAGATION                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Iteration 0:   A₀  B₁  C₂  D₃  E₄  (each node unique label)    │
│                 │   │   │   │                                   │
│                 └───┴───┴───┘                                   │
│                                                                 │
│  Iteration 1:   A₀  B₀  C₀  D₀  E₄  (majority voting)           │
│                                                                 │
│  Iteration 2:   A₀  B₀  C₀  D₀  E₀  (converged)                 │
│                                                                 │
│  Result: Community {A, B, C, D, E}                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Characteristics:**

| Attribute       | Value                          |
| --------------- | ------------------------------ |
| Time Complexity | O(n + m) per iteration         |
| Quality         | Good for clear clusters        |
| Speed           | Very fast                      |
| Use Case        | Quick clustering, large graphs |

---

### Connected Components

Baseline algorithm finding disconnected subgraphs:

```
┌─────────────────────────────────────────────────────────────────┐
│                    CONNECTED COMPONENTS                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Graph:        A───B       E───F                                │
│                │   │                                            │
│                C───D       G                                    │
│                                                                 │
│  Components:   ┌─────────┐ ┌─────┐ ┌───┐                        │
│                │ A,B,C,D │ │ E,F │ │ G │                        │
│                └─────────┘ └─────┘ └───┘                        │
│                                                                 │
│  Simple BFS/DFS traversal                                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Characteristics:**

| Attribute       | Value                      |
| --------------- | -------------------------- |
| Time Complexity | O(n + m)                   |
| Quality         | Baseline (no overlapping)  |
| Speed           | Fastest                    |
| Use Case        | Finding isolated subgraphs |

---

## Configuration

```rust
/// Configuration for community detection.
pub struct CommunityConfig {
    /// Algorithm to use
    pub algorithm: CommunityAlgorithm,

    /// Minimum community size (filter small clusters)
    pub min_community_size: usize,

    /// Maximum iterations for iterative algorithms
    pub max_iterations: usize,

    /// Resolution parameter for Louvain
    /// Higher = more smaller communities
    pub resolution: f64,
}

// Defaults
CommunityConfig {
    algorithm: CommunityAlgorithm::Louvain,
    min_community_size: 2,
    max_iterations: 100,
    resolution: 1.0,
}
```

**Resolution Parameter Effect:**

| Resolution | Result                    |
| ---------- | ------------------------- |
| 0.5        | Fewer, larger communities |
| 1.0        | Balanced (default)        |
| 2.0        | More, smaller communities |

---

## Usage

### Basic Detection

```rust
use edgequake_storage::community::{
    detect_communities, CommunityConfig, CommunityAlgorithm
};

// Run with default settings (Louvain)
let config = CommunityConfig::default();
let result = detect_communities(&graph, &config).await?;

println!("Found {} communities", result.communities.len());
println!("Modularity: {:.4}", result.modularity);

for community in &result.communities {
    println!("Community {}: {} members",
             community.id, community.size());
}
```

### Query Community Members

```rust
// Find which community an entity belongs to
if let Some(community) = result.get_node_community("SARAH_CHEN") {
    println!("SARAH_CHEN is in community {} with {} members",
             community.id, community.size());

    for member in &community.members {
        println!("  - {}", member);
    }
}
```

### Different Algorithms

```rust
// Use Label Propagation for faster results
let config = CommunityConfig {
    algorithm: CommunityAlgorithm::LabelPropagation,
    min_community_size: 3,
    max_iterations: 50,
    ..Default::default()
};

let result = detect_communities(&graph, &config).await?;
```

---

## Integration with Global Queries

The Global Query strategy uses communities to generate high-level summaries:

```
┌─────────────────────────────────────────────────────────────────┐
│                    GLOBAL QUERY PIPELINE                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. Detect Communities                                          │
│     ┌──────────────┐                                            │
│     │ Louvain      │──▶ Communities with members                │
│     └──────────────┘                                            │
│                                                                 │
│  2. Generate Community Summaries (LLM)                          │
│     ┌──────────────┐                                            │
│     │ For each     │                                            │
│     │ community:   │──▶ "This cluster contains AI researchers..." 
│     │ summarize    │                                            │
│     └──────────────┘                                            │
│                                                                 │
│  3. Query Against Summaries                                     │
│     ┌──────────────┐                                            │
│     │ Vector       │──▶ Top-k relevant communities              │
│     │ similarity   │                                            │
│     └──────────────┘                                            │
│                                                                 │
│  4. Synthesize Answer                                           │
│     ┌──────────────┐                                            │
│     │ LLM answer   │──▶ "The major themes in the corpus are..." │
│     │ from themes  │                                            │
│     └──────────────┘                                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Performance Characteristics

### Algorithm Comparison

| Algorithm            | Time        | Memory   | Quality     |
| -------------------- | ----------- | -------- | ----------- |
| Louvain              | O(n log n)  | O(n + m) | ⭐⭐⭐ Best |
| Label Propagation    | O(k(n + m)) | O(n)     | ⭐⭐ Good   |
| Connected Components | O(n + m)    | O(n)     | ⭐ Baseline |

### Benchmarks

Performance on knowledge graphs (100K nodes, 300K edges):

| Algorithm            | Time  | Communities | Modularity |
| -------------------- | ----- | ----------- | ---------- |
| Louvain              | ~2s   | 847         | 0.62       |
| Label Propagation    | ~0.5s | 1203        | 0.48       |
| Connected Components | ~0.1s | 15          | N/A        |

---

## Best Practices

1. **Start with Louvain** - Best quality for most use cases
2. **Tune Resolution** - Increase for finer-grained topics
3. **Filter Small Communities** - Set `min_community_size ≥ 3`
4. **Cache Results** - Community detection can be expensive
5. **Monitor Modularity** - Low scores (<0.3) suggest poor partitioning

---

## Troubleshooting

### Too Few Communities

**Symptoms:** Only 1-2 large communities

**Solutions:**

- Increase `resolution` parameter (try 1.5, 2.0)
- Check edge weights (uniform weights = less structure)
- Verify graph connectivity

### Too Many Communities

**Symptoms:** Mostly singleton communities

**Solutions:**

- Decrease `resolution` parameter (try 0.5)
- Increase `min_community_size`
- Check if graph is too sparse

### Low Modularity

**Symptoms:** Modularity < 0.3

**Possible Causes:**

- Graph has weak community structure
- Random or adversarial connections
- Single large connected component

**Solutions:**

- Use domain knowledge to prune noisy edges
- Consider weighted edges for semantic similarity

---

## See Also

- [Query Modes](./query-modes.md) - How Global uses communities
- [Graph Storage](./graph-storage.md) - How graphs are stored
- [Entity Extraction](./entity-extraction.md) - How entities are created
- [Architecture: Crates](../architecture/crates/README.md) - Storage crate details
