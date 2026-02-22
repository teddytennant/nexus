use crate::parser::Note;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub notes: HashMap<String, Note>,
    pub adjacency: HashMap<String, HashSet<String>>,
    pub reverse_adjacency: HashMap<String, HashSet<String>>,
    pub phantom_nodes: HashSet<String>,
}

impl KnowledgeGraph {
    /// Build a knowledge graph from parsed notes.
    pub fn from_notes(notes: Vec<Note>) -> Self {
        let mut graph = KnowledgeGraph {
            notes: HashMap::new(),
            adjacency: HashMap::new(),
            reverse_adjacency: HashMap::new(),
            phantom_nodes: HashSet::new(),
        };

        // Index all notes by ID
        let note_ids: HashSet<String> = notes.iter().map(|n| n.id.clone()).collect();
        for note in notes {
            graph.adjacency.entry(note.id.clone()).or_default();
            graph.reverse_adjacency.entry(note.id.clone()).or_default();
            graph.notes.insert(note.id.clone(), note);
        }

        // Build edges
        let note_ids_snapshot: Vec<(String, Vec<String>)> = graph
            .notes
            .iter()
            .map(|(id, note)| (id.clone(), note.outgoing_links.clone()))
            .collect();

        for (source_id, links) in note_ids_snapshot {
            for target_id in links {
                // Skip self-links (AC2.4)
                if target_id == source_id {
                    continue;
                }

                // Track phantom nodes (AC2.3)
                if !note_ids.contains(&target_id) {
                    graph.phantom_nodes.insert(target_id.clone());
                    graph.adjacency.entry(target_id.clone()).or_default();
                    graph.reverse_adjacency.entry(target_id.clone()).or_default();
                }

                // Add edge (deduplicated by HashSet) (AC2.5)
                graph
                    .adjacency
                    .entry(source_id.clone())
                    .or_default()
                    .insert(target_id.clone());
                graph
                    .reverse_adjacency
                    .entry(target_id.clone())
                    .or_default()
                    .insert(source_id.clone());
            }
        }

        graph
    }

    /// Get all node IDs (real + phantom).
    pub fn all_node_ids(&self) -> HashSet<String> {
        let mut ids: HashSet<String> = self.adjacency.keys().cloned().collect();
        ids.extend(self.reverse_adjacency.keys().cloned());
        ids
    }

    /// Number of nodes.
    pub fn node_count(&self) -> usize {
        self.all_node_ids().len()
    }

    /// Number of directed edges.
    pub fn edge_count(&self) -> usize {
        self.adjacency.values().map(|v| v.len()).sum()
    }

    /// In-degree of a node.
    pub fn in_degree(&self, id: &str) -> usize {
        self.reverse_adjacency
            .get(id)
            .map_or(0, |s| s.len())
    }

    /// Out-degree of a node.
    pub fn out_degree(&self, id: &str) -> usize {
        self.adjacency.get(id).map_or(0, |s| s.len())
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

    // === AC2.1: Each note becomes a node ===
    #[test]
    fn test_notes_become_nodes() {
        let notes = vec![
            make_note("a", &["b"]),
            make_note("b", &["a"]),
        ];
        let graph = KnowledgeGraph::from_notes(notes);
        assert!(graph.notes.contains_key("a"));
        assert!(graph.notes.contains_key("b"));
        assert_eq!(graph.notes.len(), 2);
    }

    // === AC2.2: Each wikilink becomes a directed edge ===
    #[test]
    fn test_wikilinks_become_edges() {
        let notes = vec![
            make_note("a", &["b"]),
            make_note("b", &[]),
        ];
        let graph = KnowledgeGraph::from_notes(notes);
        assert!(graph.adjacency["a"].contains("b"));
        assert!(!graph.adjacency["b"].contains("a"));
    }

    // === AC2.3: Phantom nodes ===
    #[test]
    fn test_phantom_nodes() {
        let notes = vec![
            make_note("a", &["nonexistent"]),
        ];
        let graph = KnowledgeGraph::from_notes(notes);
        assert!(graph.phantom_nodes.contains("nonexistent"));
        assert!(graph.all_node_ids().contains("nonexistent"));
    }

    // === AC2.4: Self-links ignored ===
    #[test]
    fn test_self_links_ignored() {
        let notes = vec![
            make_note("a", &["a"]),
        ];
        let graph = KnowledgeGraph::from_notes(notes);
        assert!(graph.adjacency["a"].is_empty());
    }

    // === AC2.5: Duplicate links deduplicated ===
    #[test]
    fn test_duplicate_links_deduplicated() {
        let notes = vec![
            make_note("a", &["b", "b", "b"]),
            make_note("b", &[]),
        ];
        let graph = KnowledgeGraph::from_notes(notes);
        assert_eq!(graph.adjacency["a"].len(), 1);
    }

    // === Degree tests ===
    #[test]
    fn test_in_out_degree() {
        let notes = vec![
            make_note("a", &["b", "c"]),
            make_note("b", &["c"]),
            make_note("c", &[]),
        ];
        let graph = KnowledgeGraph::from_notes(notes);
        assert_eq!(graph.out_degree("a"), 2);
        assert_eq!(graph.out_degree("c"), 0);
        assert_eq!(graph.in_degree("c"), 2);
        assert_eq!(graph.in_degree("a"), 0);
    }

    #[test]
    fn test_edge_count() {
        let notes = vec![
            make_note("a", &["b", "c"]),
            make_note("b", &["c"]),
            make_note("c", &[]),
        ];
        let graph = KnowledgeGraph::from_notes(notes);
        assert_eq!(graph.edge_count(), 3);
    }

    #[test]
    fn test_node_count_with_phantoms() {
        let notes = vec![
            make_note("a", &["b", "phantom"]),
            make_note("b", &[]),
        ];
        let graph = KnowledgeGraph::from_notes(notes);
        assert_eq!(graph.node_count(), 3); // a, b, phantom
    }
}
