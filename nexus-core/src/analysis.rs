use crate::algorithms::{Cluster, GraphMetrics};
use crate::graph::KnowledgeGraph;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analysis {
    pub hub_notes: Vec<RankedNote>,
    pub bridge_concepts: Vec<RankedNote>,
    pub orphan_notes: Vec<String>,
    pub phantom_notes: Vec<String>,
    pub cluster_summary: Vec<ClusterSummary>,
    pub cross_cluster_bridges: Vec<CrossClusterBridge>,
    pub suggested_connections: Vec<SuggestedConnection>,
    pub vault_health: VaultHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedNote {
    pub id: String,
    pub score: f64,
    pub in_degree: usize,
    pub out_degree: usize,
    pub cluster_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSummary {
    pub id: usize,
    pub label: String,
    pub member_count: usize,
    pub density: f64,
    pub top_members: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossClusterBridge {
    pub note_id: String,
    pub clusters_connected: Vec<String>,
    pub betweenness: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedConnection {
    pub cluster_a: String,
    pub cluster_b: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultHealth {
    pub score: f64,
    pub connectivity: f64,
    pub completeness: f64,
    pub structure: f64,
    pub total_notes: usize,
    pub total_links: usize,
    pub total_clusters: usize,
    pub rating: String,
}

/// Build a mapping from note ID to cluster label.
fn build_cluster_map(clusters: &[Cluster]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for cluster in clusters {
        for member in &cluster.members {
            map.insert(member.clone(), cluster.label.clone());
        }
    }
    map
}

/// Analyze the graph and metrics to produce actionable insights.
pub fn analyze(graph: &KnowledgeGraph, metrics: &GraphMetrics, top_n: usize) -> Analysis {
    let cluster_map = build_cluster_map(&metrics.clusters);

    // AC7.1: Hub Notes — top N by PageRank
    let mut hub_notes: Vec<RankedNote> = metrics
        .pagerank
        .iter()
        .map(|(id, score)| RankedNote {
            id: id.clone(),
            score: *score,
            in_degree: *metrics.in_degree.get(id).unwrap_or(&0),
            out_degree: *metrics.out_degree.get(id).unwrap_or(&0),
            cluster_label: cluster_map.get(id).cloned().unwrap_or_default(),
        })
        .collect();
    hub_notes.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    hub_notes.truncate(top_n);

    // AC7.2: Bridge Concepts — top N by betweenness centrality
    let mut bridge_concepts: Vec<RankedNote> = metrics
        .betweenness_centrality
        .iter()
        .map(|(id, score)| RankedNote {
            id: id.clone(),
            score: *score,
            in_degree: *metrics.in_degree.get(id).unwrap_or(&0),
            out_degree: *metrics.out_degree.get(id).unwrap_or(&0),
            cluster_label: cluster_map.get(id).cloned().unwrap_or_default(),
        })
        .collect();
    bridge_concepts.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    bridge_concepts.truncate(top_n);

    // AC7.3: Orphan Notes — 0 incoming links (only real notes, not phantoms)
    let orphan_notes: Vec<String> = graph
        .notes
        .keys()
        .filter(|id| graph.in_degree(id) == 0)
        .cloned()
        .collect();

    // AC7.4: Phantom Notes — referenced but no file
    let mut phantom_notes: Vec<String> = graph.phantom_nodes.iter().cloned().collect();
    phantom_notes.sort();

    // AC7.5: Cluster Summary
    let cluster_summary: Vec<ClusterSummary> = metrics
        .clusters
        .iter()
        .map(|c| {
            let mut top_members: Vec<(String, f64)> = c
                .members
                .iter()
                .map(|m| (m.clone(), *metrics.pagerank.get(m).unwrap_or(&0.0)))
                .collect();
            top_members.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            let top: Vec<String> = top_members.iter().take(5).map(|(id, _)| id.clone()).collect();

            ClusterSummary {
                id: c.id,
                label: c.label.clone(),
                member_count: c.members.len(),
                density: c.density,
                top_members: top,
            }
        })
        .collect();

    // AC7.6: Cross-Cluster Bridges
    let cross_cluster_bridges = find_cross_cluster_bridges(graph, metrics, &cluster_map);

    // AC7.7: Suggested Connections
    let suggested_connections = suggest_connections(graph, metrics, &cluster_map);

    // Vault Health Score
    let vault_health = compute_vault_health(graph, metrics, &orphan_notes, &phantom_notes);

    Analysis {
        hub_notes,
        bridge_concepts,
        orphan_notes,
        phantom_notes,
        cluster_summary,
        cross_cluster_bridges,
        suggested_connections,
        vault_health,
    }
}

fn find_cross_cluster_bridges(
    graph: &KnowledgeGraph,
    metrics: &GraphMetrics,
    cluster_map: &HashMap<String, String>,
) -> Vec<CrossClusterBridge> {
    let mut bridges = Vec::new();

    for (id, bc) in &metrics.betweenness_centrality {
        if *bc <= 0.0 {
            continue;
        }

        let mut connected_clusters: HashSet<String> = HashSet::new();

        // Check outgoing links
        if let Some(targets) = graph.adjacency.get(id) {
            for t in targets {
                if let Some(label) = cluster_map.get(t) {
                    connected_clusters.insert(label.clone());
                }
            }
        }
        // Check incoming links
        if let Some(sources) = graph.reverse_adjacency.get(id) {
            for s in sources {
                if let Some(label) = cluster_map.get(s) {
                    connected_clusters.insert(label.clone());
                }
            }
        }

        // Own cluster
        if let Some(own) = cluster_map.get(id) {
            connected_clusters.insert(own.clone());
        }

        if connected_clusters.len() >= 2 {
            let mut clusters: Vec<String> = connected_clusters.into_iter().collect();
            clusters.sort();
            bridges.push(CrossClusterBridge {
                note_id: id.clone(),
                clusters_connected: clusters,
                betweenness: *bc,
            });
        }
    }

    bridges.sort_by(|a, b| b.betweenness.partial_cmp(&a.betweenness).unwrap());
    bridges
}

fn suggest_connections(
    graph: &KnowledgeGraph,
    metrics: &GraphMetrics,
    _cluster_map: &HashMap<String, String>,
) -> Vec<SuggestedConnection> {
    let mut suggestions = Vec::new();

    // Find pairs of clusters with no links between them
    let non_trivial_clusters: Vec<&Cluster> = metrics
        .clusters
        .iter()
        .filter(|c| c.members.len() > 1 && c.label != "Uncategorized")
        .collect();

    for i in 0..non_trivial_clusters.len() {
        for j in (i + 1)..non_trivial_clusters.len() {
            let a = non_trivial_clusters[i];
            let b = non_trivial_clusters[j];

            let a_set: HashSet<&String> = a.members.iter().collect();
            let b_set: HashSet<&String> = b.members.iter().collect();

            // Check if any link exists between clusters
            let mut has_link = false;
            for member in &a.members {
                if let Some(targets) = graph.adjacency.get(member) {
                    if targets.iter().any(|t| b_set.contains(t)) {
                        has_link = true;
                        break;
                    }
                }
            }
            if !has_link {
                for member in &b.members {
                    if let Some(targets) = graph.adjacency.get(member) {
                        if targets.iter().any(|t| a_set.contains(t)) {
                            has_link = true;
                            break;
                        }
                    }
                }
            }

            if !has_link {
                // Check if they share directory co-occurrence
                let a_dirs: HashSet<String> = a
                    .members
                    .iter()
                    .filter_map(|m| graph.notes.get(m).map(|n| n.directory.clone()))
                    .collect();
                let b_dirs: HashSet<String> = b
                    .members
                    .iter()
                    .filter_map(|m| graph.notes.get(m).map(|n| n.directory.clone()))
                    .collect();

                let shared_dirs: Vec<&String> = a_dirs.intersection(&b_dirs).collect();
                let reason = if !shared_dirs.is_empty() {
                    format!(
                        "Both clusters have notes in: {}",
                        shared_dirs.iter().map(|d| d.as_str()).collect::<Vec<_>>().join(", ")
                    )
                } else {
                    "No links exist between these clusters".to_string()
                };

                suggestions.push(SuggestedConnection {
                    cluster_a: a.label.clone(),
                    cluster_b: b.label.clone(),
                    reason,
                });
            }
        }
    }

    suggestions
}

fn compute_vault_health(
    graph: &KnowledgeGraph,
    metrics: &GraphMetrics,
    orphans: &[String],
    phantoms: &[String],
) -> VaultHealth {
    let total_notes = graph.notes.len();
    let total_links = graph.edge_count();
    let total_clusters = metrics.clusters.len();

    // Connectivity: 1.0 - (orphan_count / total_notes)
    let connectivity = if total_notes > 0 {
        1.0 - (orphans.len() as f64 / total_notes as f64)
    } else {
        0.0
    };

    // Completeness: 1.0 - (phantom_count / total_unique_references)
    let total_refs = graph.all_node_ids().len();
    let completeness = if total_refs > 0 {
        1.0 - (phantoms.len() as f64 / total_refs as f64)
    } else {
        1.0
    };

    // Structure: average cluster density across non-singleton clusters
    let non_singleton: Vec<f64> = metrics
        .clusters
        .iter()
        .filter(|c| c.members.len() > 1)
        .map(|c| c.density)
        .collect();
    let structure = if !non_singleton.is_empty() {
        non_singleton.iter().sum::<f64>() / non_singleton.len() as f64
    } else {
        0.0
    };

    // Weighted score
    let score = (connectivity * 0.4 + completeness * 0.3 + structure * 0.3) * 100.0;

    let rating = match score as u32 {
        90..=100 => "Excellent",
        70..=89 => "Good",
        50..=69 => "Needs Work",
        _ => "Fragmented",
    }
    .to_string();

    VaultHealth {
        score,
        connectivity,
        completeness,
        structure,
        total_notes,
        total_links,
        total_clusters,
        rating,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithms::compute_metrics;
    use crate::parser::Note;
    use std::path::PathBuf;

    fn make_note(id: &str, links: &[&str], dir: &str) -> Note {
        Note {
            id: id.to_string(),
            title: id.to_string(),
            path: PathBuf::from(format!("{}/{}.md", dir, id)),
            outgoing_links: links.iter().map(|s| s.to_string()).collect(),
            tags: vec![],
            word_count: 100,
            directory: dir.to_string(),
        }
    }

    fn build_analysis(notes: Vec<Note>, top_n: usize) -> Analysis {
        let graph = KnowledgeGraph::from_notes(notes);
        let metrics = compute_metrics(&graph);
        analyze(&graph, &metrics, top_n)
    }

    // === AC7.1: Hub Notes ===
    #[test]
    fn test_hub_notes_ranked_by_pagerank() {
        let analysis = build_analysis(
            vec![
                make_note("a", &["hub"], "test"),
                make_note("b", &["hub"], "test"),
                make_note("c", &["hub"], "test"),
                make_note("hub", &[], "test"),
            ],
            10,
        );
        assert!(!analysis.hub_notes.is_empty());
        assert_eq!(analysis.hub_notes[0].id, "hub");
    }

    // === AC7.2: Bridge Concepts ===
    #[test]
    fn test_bridge_concepts_ranked() {
        let analysis = build_analysis(
            vec![
                make_note("a", &["bridge"], "test"),
                make_note("bridge", &["b"], "test"),
                make_note("b", &[], "test"),
            ],
            10,
        );
        assert!(!analysis.bridge_concepts.is_empty());
        // bridge should be ranked high
        let bridge_pos = analysis
            .bridge_concepts
            .iter()
            .position(|n| n.id == "bridge");
        assert!(bridge_pos.is_some());
    }

    // === AC7.3: Orphan Notes ===
    #[test]
    fn test_orphan_notes() {
        let analysis = build_analysis(
            vec![
                make_note("linked", &["target"], "test"),
                make_note("target", &[], "test"),
                make_note("orphan", &[], "test"),
            ],
            10,
        );
        // "orphan" and "linked" both have 0 incoming links
        assert!(analysis.orphan_notes.contains(&"orphan".to_string()));
        assert!(analysis.orphan_notes.contains(&"linked".to_string()));
    }

    // === AC7.4: Phantom Notes ===
    #[test]
    fn test_phantom_notes() {
        let analysis = build_analysis(
            vec![
                make_note("a", &["nonexistent", "also-missing"], "test"),
            ],
            10,
        );
        assert!(analysis.phantom_notes.contains(&"nonexistent".to_string()));
        assert!(analysis.phantom_notes.contains(&"also-missing".to_string()));
    }

    // === AC7.5: Cluster Summary ===
    #[test]
    fn test_cluster_summary() {
        let analysis = build_analysis(
            vec![
                make_note("a", &["b"], "test"),
                make_note("b", &["a"], "test"),
                make_note("c", &[], "test"),
            ],
            10,
        );
        assert!(!analysis.cluster_summary.is_empty());
        // Should have at least one cluster with member_count > 0
        assert!(analysis.cluster_summary.iter().any(|c| c.member_count > 0));
    }

    // === Vault Health ===
    #[test]
    fn test_vault_health_fully_connected() {
        let analysis = build_analysis(
            vec![
                make_note("a", &["b", "c"], "test"),
                make_note("b", &["a", "c"], "test"),
                make_note("c", &["a", "b"], "test"),
            ],
            10,
        );
        // No orphans, no phantoms, dense cluster → high health
        assert!(analysis.vault_health.score > 50.0);
        assert!(analysis.vault_health.connectivity > 0.5);
    }

    #[test]
    fn test_vault_health_fragmented() {
        let analysis = build_analysis(
            vec![
                make_note("a", &["phantom1"], "test"),
                make_note("b", &["phantom2"], "test"),
                make_note("c", &["phantom3"], "test"),
            ],
            10,
        );
        // Many orphans and phantoms → low health
        assert!(analysis.vault_health.score < 80.0);
    }

    #[test]
    fn test_vault_health_rating() {
        let analysis = build_analysis(
            vec![
                make_note("a", &["b", "c"], "test"),
                make_note("b", &["a", "c"], "test"),
                make_note("c", &["a", "b"], "test"),
            ],
            10,
        );
        // Rating should be one of the defined categories
        let valid_ratings = ["Excellent", "Good", "Needs Work", "Fragmented"];
        assert!(valid_ratings.contains(&analysis.vault_health.rating.as_str()));
    }

    // === Top N limiting ===
    #[test]
    fn test_top_n_limiting() {
        let analysis = build_analysis(
            vec![
                make_note("a", &["b"], "test"),
                make_note("b", &["c"], "test"),
                make_note("c", &["d"], "test"),
                make_note("d", &["e"], "test"),
                make_note("e", &[], "test"),
            ],
            2,
        );
        assert!(analysis.hub_notes.len() <= 2);
        assert!(analysis.bridge_concepts.len() <= 2);
    }
}
