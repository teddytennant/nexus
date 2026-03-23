use crate::graph::KnowledgeGraph;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub id: usize,
    pub members: Vec<String>,
    pub label: String,
    pub density: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMetrics {
    pub pagerank: HashMap<String, f64>,
    pub betweenness_centrality: HashMap<String, f64>,
    pub in_degree: HashMap<String, usize>,
    pub out_degree: HashMap<String, usize>,
    pub clusters: Vec<Cluster>,
}

/// Compute PageRank for all nodes in the graph.
///
/// Uses damping factor 0.85, converges when max delta < 1e-6 or after 100 iterations.
/// Handles dangling nodes by distributing their rank uniformly.
/// Uses index-based vectors instead of HashMaps for better performance on large graphs.
pub fn pagerank(graph: &KnowledgeGraph, damping: f64, max_iter: usize, tolerance: f64) -> HashMap<String, f64> {
    let nodes: Vec<String> = graph.all_node_ids().into_iter().collect();
    let n = nodes.len();
    if n == 0 {
        return HashMap::new();
    }

    // Map node IDs to indices
    let node_index: HashMap<&String, usize> = nodes.iter().enumerate().map(|(i, id)| (id, i)).collect();

    // Precompute out-degrees and reverse adjacency as index-based vectors
    let out_deg: Vec<usize> = nodes.iter().map(|id| graph.out_degree(id)).collect();
    let rev_adj: Vec<Vec<usize>> = nodes
        .iter()
        .map(|id| {
            graph
                .reverse_adjacency
                .get(id)
                .map(|sources| {
                    sources
                        .iter()
                        .filter_map(|s| node_index.get(s).copied())
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();

    // Precompute dangling node indices
    let dangling: Vec<usize> = (0..n).filter(|&i| out_deg[i] == 0).collect();

    let initial = 1.0 / n as f64;
    let mut rank = vec![initial; n];
    let mut new_rank = vec![0.0_f64; n];

    for _ in 0..max_iter {
        let dangling_sum: f64 = dangling.iter().map(|&i| rank[i]).sum();
        let base = (1.0 - damping) / n as f64 + damping * dangling_sum / n as f64;

        for i in 0..n {
            let mut sum = 0.0;
            for &src in &rev_adj[i] {
                if out_deg[src] > 0 {
                    sum += rank[src] / out_deg[src] as f64;
                }
            }
            new_rank[i] = base + damping * sum;
        }

        // Check convergence
        let max_delta: f64 = (0..n)
            .map(|i| (new_rank[i] - rank[i]).abs())
            .fold(0.0_f64, f64::max);

        std::mem::swap(&mut rank, &mut new_rank);

        if max_delta < tolerance {
            break;
        }
    }

    nodes
        .into_iter()
        .enumerate()
        .map(|(i, id)| (id, rank[i]))
        .collect()
}

/// Compute betweenness centrality using Brandes' algorithm.
///
/// Normalized to [0.0, 1.0] range. Uses index-based vectors instead of
/// per-iteration HashMaps for better performance on large graphs.
pub fn betweenness_centrality(graph: &KnowledgeGraph) -> HashMap<String, f64> {
    let nodes: Vec<String> = graph.all_node_ids().into_iter().collect();
    let n = nodes.len();

    if n <= 2 {
        return nodes.iter().map(|id| (id.clone(), 0.0)).collect();
    }

    // Map node IDs to indices for fast lookup
    let node_index: HashMap<&String, usize> = nodes.iter().enumerate().map(|(i, id)| (id, i)).collect();

    // Build index-based adjacency list
    let adj: Vec<Vec<usize>> = nodes
        .iter()
        .map(|id| {
            graph
                .adjacency
                .get(id)
                .map(|targets| {
                    targets
                        .iter()
                        .filter_map(|t| node_index.get(t).copied())
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();

    let mut centrality = vec![0.0_f64; n];

    // Reusable buffers (allocated once, cleared each iteration)
    let mut stack: Vec<usize> = Vec::with_capacity(n);
    let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut sigma = vec![0.0_f64; n];
    let mut dist = vec![-1_i64; n];
    let mut delta = vec![0.0_f64; n];
    let mut queue: VecDeque<usize> = VecDeque::with_capacity(n);

    for s in 0..n {
        // Reset buffers
        stack.clear();
        for pred in &mut predecessors {
            pred.clear();
        }
        sigma.fill(0.0);
        dist.fill(-1);
        delta.fill(0.0);
        queue.clear();

        sigma[s] = 1.0;
        dist[s] = 0;
        queue.push_back(s);

        while let Some(v) = queue.pop_front() {
            stack.push(v);
            let v_dist = dist[v];

            for &w in &adj[v] {
                // First visit?
                if dist[w] < 0 {
                    dist[w] = v_dist + 1;
                    queue.push_back(w);
                }
                // Shortest path via v?
                if dist[w] == v_dist + 1 {
                    sigma[w] += sigma[v];
                    predecessors[w].push(v);
                }
            }
        }

        // Accumulate
        while let Some(w) = stack.pop() {
            for &v in &predecessors[w] {
                let d = (sigma[v] / sigma[w]) * (1.0 + delta[w]);
                delta[v] += d;
            }
            if w != s {
                centrality[w] += delta[w];
            }
        }
    }

    // Normalize: divide by (n-1)(n-2) for directed graphs
    let norm = ((n - 1) * (n - 2)) as f64;
    if norm > 0.0 {
        for val in &mut centrality {
            *val /= norm;
        }
    }

    nodes
        .into_iter()
        .enumerate()
        .map(|(i, id)| (id, centrality[i]))
        .collect()
}

/// Detect communities using label propagation algorithm.
///
/// Treats the graph as undirected for community detection.
/// Uses index-based vectors instead of HashMaps for better performance on large graphs.
pub fn label_propagation(graph: &KnowledgeGraph, pagerank_scores: &HashMap<String, f64>) -> Vec<Cluster> {
    let mut nodes: Vec<String> = graph.all_node_ids().into_iter().collect();
    nodes.sort();
    let n = nodes.len();
    if n == 0 {
        return vec![];
    }

    // Map node IDs to indices
    let node_index: HashMap<&String, usize> = nodes.iter().enumerate().map(|(i, id)| (id, i)).collect();

    // Build undirected adjacency as index-based vectors
    let mut undirected: Vec<Vec<usize>> = vec![vec![]; n];
    for (src, targets) in &graph.adjacency {
        if let Some(&si) = node_index.get(src) {
            for tgt in targets {
                if let Some(&ti) = node_index.get(tgt) {
                    undirected[si].push(ti);
                    undirected[ti].push(si);
                }
            }
        }
    }
    // Deduplicate neighbor lists (edges may appear in both directions)
    for neighbors in &mut undirected {
        neighbors.sort_unstable();
        neighbors.dedup();
    }

    // Initialize: each node is its own label
    let mut labels: Vec<usize> = (0..n).collect();

    // Iterate
    let max_iter = 50;
    for _ in 0..max_iter {
        let mut changed = false;

        for i in 0..n {
            let neighbors = &undirected[i];
            if neighbors.is_empty() {
                continue;
            }

            // Count neighbor labels
            let mut label_counts: HashMap<usize, usize> = HashMap::new();
            for &ni in neighbors {
                *label_counts.entry(labels[ni]).or_insert(0) += 1;
            }

            // Find max count
            let max_count = *label_counts.values().max().unwrap_or(&0);
            let candidates: Vec<usize> = label_counts
                .into_iter()
                .filter(|(_, count)| *count == max_count)
                .map(|(label, _)| label)
                .collect();

            // Pick smallest label among ties (deterministic)
            let best = *candidates.iter().min().unwrap();
            if labels[i] != best {
                labels[i] = best;
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    // Group by label
    let mut groups: HashMap<usize, Vec<String>> = HashMap::new();
    for (i, &label) in labels.iter().enumerate() {
        groups.entry(label).or_default().push(nodes[i].clone());
    }

    // Build clusters
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut cluster_id = 0;

    // Collect singletons
    let mut singletons = Vec::new();

    for (_, mut members) in groups {
        members.sort();
        if members.len() == 1 {
            singletons.push(members.into_iter().next().unwrap());
            continue;
        }

        // Label = highest PageRank member
        let label = members
            .iter()
            .max_by(|a, b| {
                let ra = pagerank_scores.get(*a).unwrap_or(&0.0);
                let rb = pagerank_scores.get(*b).unwrap_or(&0.0);
                ra.partial_cmp(rb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap()
            .clone();

        // Density = actual edges / possible edges (within cluster)
        let member_set: HashSet<&String> = members.iter().collect();
        let mut internal_edges = 0;
        for m in &members {
            if let Some(targets) = graph.adjacency.get(m) {
                for t in targets {
                    if member_set.contains(t) {
                        internal_edges += 1;
                    }
                }
            }
        }
        let possible = members.len() * (members.len() - 1);
        let density = if possible > 0 {
            internal_edges as f64 / possible as f64
        } else {
            0.0
        };

        clusters.push(Cluster {
            id: cluster_id,
            members,
            label,
            density,
        });
        cluster_id += 1;
    }

    // Group singletons into "Uncategorized" (AC5.3)
    if !singletons.is_empty() {
        singletons.sort();
        clusters.push(Cluster {
            id: cluster_id,
            members: singletons,
            label: "Uncategorized".to_string(),
            density: 0.0,
        });
    }

    // Sort clusters by size descending
    clusters.sort_by(|a, b| b.members.len().cmp(&a.members.len()));

    // Re-number IDs
    for (i, c) in clusters.iter_mut().enumerate() {
        c.id = i;
    }

    clusters
}

/// Compute all graph metrics.
pub fn compute_metrics(graph: &KnowledgeGraph) -> GraphMetrics {
    let pr = pagerank(graph, 0.85, 100, 1e-6);
    let bc = betweenness_centrality(graph);
    let clusters = label_propagation(graph, &pr);

    let nodes = graph.all_node_ids();
    let in_degree: HashMap<String, usize> = nodes.iter().map(|id| (id.clone(), graph.in_degree(id))).collect();
    let out_degree: HashMap<String, usize> = nodes.iter().map(|id| (id.clone(), graph.out_degree(id))).collect();

    GraphMetrics {
        pagerank: pr,
        betweenness_centrality: bc,
        in_degree,
        out_degree,
        clusters,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Note;
    use std::path::PathBuf;

    fn make_note(id: &str, links: &[&str]) -> Note {
        Note {
            id: id.to_string(),
            title: id.to_string(),
            path: PathBuf::from(format!("{}.md", id)),
            outgoing_links: links.iter().map(|s| s.to_string()).collect(),
            tags: vec![],
            word_count: 100,
            directory: "test".to_string(),
        }
    }

    fn build_graph(notes: Vec<Note>) -> KnowledgeGraph {
        KnowledgeGraph::from_notes(notes)
    }

    // === AC3.1: Damping factor 0.85 ===
    // === AC3.3: Scores sum to approximately 1.0 ===
    #[test]
    fn test_pagerank_sums_to_one() {
        let graph = build_graph(vec![
            make_note("a", &["b", "c"]),
            make_note("b", &["c"]),
            make_note("c", &["a"]),
        ]);
        let pr = pagerank(&graph, 0.85, 100, 1e-6);
        let sum: f64 = pr.values().sum();
        assert!((sum - 1.0).abs() < 0.01, "PageRank sum was {}", sum);
    }

    // === AC3.4: Nodes with more incoming links rank higher ===
    #[test]
    fn test_pagerank_more_links_higher_rank() {
        let graph = build_graph(vec![
            make_note("a", &["hub"]),
            make_note("b", &["hub"]),
            make_note("c", &["hub"]),
            make_note("hub", &[]),
        ]);
        let pr = pagerank(&graph, 0.85, 100, 1e-6);
        assert!(pr["hub"] > pr["a"]);
        assert!(pr["hub"] > pr["b"]);
        assert!(pr["hub"] > pr["c"]);
    }

    // === AC3.5: Handles dangling nodes ===
    #[test]
    fn test_pagerank_dangling_nodes() {
        let graph = build_graph(vec![
            make_note("a", &["b"]),
            make_note("b", &[]), // dangling
        ]);
        let pr = pagerank(&graph, 0.85, 100, 1e-6);
        let sum: f64 = pr.values().sum();
        assert!((sum - 1.0).abs() < 0.01);
        assert!(pr["b"] > pr["a"]); // b receives link from a + dangling contribution
    }

    // === AC3.2: Convergence ===
    #[test]
    fn test_pagerank_empty_graph() {
        let graph = build_graph(vec![]);
        let pr = pagerank(&graph, 0.85, 100, 1e-6);
        assert!(pr.is_empty());
    }

    // === AC4.1: Bridge concepts have high centrality ===
    #[test]
    fn test_betweenness_bridge_node() {
        // a -> bridge -> b, c -> bridge -> d
        // "bridge" connects two otherwise separate groups
        let graph = build_graph(vec![
            make_note("a", &["bridge"]),
            make_note("bridge", &["b", "d"]),
            make_note("b", &[]),
            make_note("c", &["bridge"]),
            make_note("d", &[]),
        ]);
        let bc = betweenness_centrality(&graph);
        // bridge should have higher centrality than leaf nodes
        assert!(bc["bridge"] > bc["a"]);
        assert!(bc["bridge"] > bc["b"]);
    }

    // === AC4.2: Isolated nodes have centrality 0.0 ===
    #[test]
    fn test_betweenness_isolated() {
        let graph = build_graph(vec![
            make_note("a", &["b"]),
            make_note("b", &[]),
            make_note("isolated", &[]),
        ]);
        let bc = betweenness_centrality(&graph);
        assert_eq!(bc["isolated"], 0.0);
    }

    // === AC4.3: Normalized to [0.0, 1.0] ===
    #[test]
    fn test_betweenness_normalized() {
        let graph = build_graph(vec![
            make_note("a", &["b"]),
            make_note("b", &["c"]),
            make_note("c", &["d"]),
            make_note("d", &[]),
        ]);
        let bc = betweenness_centrality(&graph);
        for val in bc.values() {
            assert!(*val >= 0.0 && *val <= 1.0, "Centrality {} out of range", val);
        }
    }

    // === AC5.1: Dense internal links form clusters ===
    #[test]
    fn test_label_propagation_forms_clusters() {
        // Two dense groups connected by a single bridge
        let graph = build_graph(vec![
            make_note("a1", &["a2", "a3"]),
            make_note("a2", &["a1", "a3"]),
            make_note("a3", &["a1", "a2", "bridge"]),
            make_note("bridge", &["b1"]),
            make_note("b1", &["b2", "b3"]),
            make_note("b2", &["b1", "b3"]),
            make_note("b3", &["b1", "b2"]),
        ]);
        let pr = pagerank(&graph, 0.85, 100, 1e-6);
        let clusters = label_propagation(&graph, &pr);

        // Should have at least 2 non-trivial clusters
        let non_trivial: Vec<_> = clusters.iter().filter(|c| c.members.len() > 1).collect();
        assert!(non_trivial.len() >= 2, "Expected at least 2 clusters, got {}", non_trivial.len());
    }

    // === AC5.3: Singletons grouped as "Uncategorized" ===
    #[test]
    fn test_singletons_uncategorized() {
        let graph = build_graph(vec![
            make_note("a", &["b"]),
            make_note("b", &["a"]),
            make_note("orphan1", &[]),
            make_note("orphan2", &[]),
        ]);
        let pr = pagerank(&graph, 0.85, 100, 1e-6);
        let clusters = label_propagation(&graph, &pr);

        let uncategorized = clusters.iter().find(|c| c.label == "Uncategorized");
        assert!(uncategorized.is_some(), "Expected an Uncategorized cluster");
        assert!(uncategorized.unwrap().members.len() >= 2);
    }

    // === AC5.4: Cluster density computed correctly ===
    #[test]
    fn test_cluster_density() {
        // Fully connected triangle: 3 nodes, 6 directed edges possible, all present
        let graph = build_graph(vec![
            make_note("a", &["b", "c"]),
            make_note("b", &["a", "c"]),
            make_note("c", &["a", "b"]),
        ]);
        let pr = pagerank(&graph, 0.85, 100, 1e-6);
        let clusters = label_propagation(&graph, &pr);

        // Should be one cluster with density 1.0
        let main_cluster = clusters.iter().find(|c| c.members.len() == 3);
        assert!(main_cluster.is_some());
        assert!((main_cluster.unwrap().density - 1.0).abs() < 0.01);
    }

    // === Full metrics computation ===
    #[test]
    fn test_compute_metrics() {
        let graph = build_graph(vec![
            make_note("a", &["b"]),
            make_note("b", &["c"]),
            make_note("c", &[]),
        ]);
        let metrics = compute_metrics(&graph);

        assert_eq!(metrics.pagerank.len(), 3);
        assert_eq!(metrics.betweenness_centrality.len(), 3);
        assert_eq!(metrics.in_degree["a"], 0);
        assert_eq!(metrics.out_degree["a"], 1);
        assert_eq!(metrics.in_degree["c"], 1);
        assert!(!metrics.clusters.is_empty());
    }
}
