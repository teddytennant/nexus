# Nexus — Knowledge Graph Intelligence Engine

> Make the invisible structure of your mind visible.

## Overview

Nexus is a Rust-powered CLI tool that parses an Obsidian vault, builds a directed knowledge graph from wikilinks, computes graph-theoretic metrics, scans local git repositories for activity, and presents unified intelligence through a Ratatui TUI dashboard.

It answers the questions: *What are my core ideas? What connects them? Where are the gaps? What should I work on next?*

## Architecture

```
nexus/
├── Cargo.toml              # Workspace root
├── nexus-core/             # Library crate
│   ├── src/
│   │   ├── lib.rs          # Public API
│   │   ├── parser.rs       # Obsidian markdown parser
│   │   ├── graph.rs        # Knowledge graph data structure
│   │   ├── algorithms.rs   # PageRank, centrality, clustering
│   │   ├── git_scanner.rs  # Git repo activity scanning
│   │   └── analysis.rs     # High-level analysis & suggestions
│   └── Cargo.toml
└── nexus-cli/              # Binary crate (TUI)
    ├── src/
    │   ├── main.rs         # Entry point & CLI args
    │   ├── app.rs          # Application state
    │   ├── ui.rs           # Ratatui rendering
    │   └── tabs/           # Tab-specific rendering
    │       ├── overview.rs
    │       ├── graph.rs
    │       ├── clusters.rs
    │       ├── bridges.rs
    │       ├── pulse.rs
    │       └── suggestions.rs
    └── Cargo.toml
```

## Data Model

### Note
```rust
struct Note {
    id: String,           // Filename without extension (e.g. "gradient")
    title: String,        // First H1 or filename
    path: PathBuf,        // Absolute path to file
    outgoing_links: Vec<String>,  // Wikilinks found in content
    tags: Vec<String>,    // #tags found in content
    word_count: usize,
    directory: String,    // Parent directory (e.g. "philosophy", "geopolitics")
}
```

### KnowledgeGraph
```rust
struct KnowledgeGraph {
    notes: HashMap<String, Note>,
    adjacency: HashMap<String, HashSet<String>>,  // outgoing edges
    reverse_adjacency: HashMap<String, HashSet<String>>,  // incoming edges
}
```

### GraphMetrics
```rust
struct GraphMetrics {
    pagerank: HashMap<String, f64>,
    betweenness_centrality: HashMap<String, f64>,
    in_degree: HashMap<String, usize>,
    out_degree: HashMap<String, usize>,
    clusters: Vec<Cluster>,
}

struct Cluster {
    id: usize,
    members: Vec<String>,
    label: String,  // Most central node's name
    density: f64,
}
```

### GitRepoStats
```rust
struct GitRepoStats {
    name: String,
    path: PathBuf,
    last_commit_date: Option<String>,
    commit_count_30d: usize,
    primary_language: Option<String>,
    lines_changed_30d: usize,
}
```

## Features & Acceptance Criteria

### F1: Obsidian Vault Parsing
- **Input**: Path to Obsidian vault directory
- **Behavior**: Recursively scan for `.md` files, extract:
  - Wikilinks: `[[link]]` and `[[link|alias]]` patterns
  - Tags: `#tag` patterns (not inside code blocks)
  - Title: First `# Heading` or filename
  - Word count
  - Parent directory
- **AC1.1**: Parses `[[simple-link]]` correctly
- **AC1.2**: Parses `[[link|display text]]` extracting "link" as the target
- **AC1.3**: Ignores wikilinks inside code blocks (``` and `)
- **AC1.4**: Extracts `#tags` but not `# headings`
- **AC1.5**: Handles files with no links gracefully (orphan notes)
- **AC1.6**: Correctly normalizes link targets to lowercase kebab-case IDs

### F2: Knowledge Graph Construction
- **Behavior**: Build directed graph from parsed notes
- **AC2.1**: Each note becomes a node
- **AC2.2**: Each wikilink becomes a directed edge (source → target)
- **AC2.3**: Links to non-existent notes create "phantom" nodes (referenced but no file)
- **AC2.4**: Self-links are ignored
- **AC2.5**: Duplicate links between same source/target are deduplicated

### F3: PageRank Computation
- **Behavior**: Compute PageRank scores for all nodes
- **AC3.1**: Uses damping factor of 0.85
- **AC3.2**: Converges when max delta < 1e-6 or after 100 iterations
- **AC3.3**: Scores sum to approximately 1.0
- **AC3.4**: Nodes with more high-quality incoming links rank higher
- **AC3.5**: Handles dangling nodes (no outgoing links) by distributing rank uniformly

### F4: Betweenness Centrality
- **Behavior**: Compute betweenness centrality for all nodes using Brandes' algorithm
- **AC4.1**: Bridge concepts (connecting otherwise separate clusters) have high scores
- **AC4.2**: Isolated nodes have centrality of 0.0
- **AC4.3**: Scores are normalized to [0.0, 1.0]

### F5: Community Detection
- **Behavior**: Detect clusters using label propagation algorithm
- **AC5.1**: Connected components with dense internal links form clusters
- **AC5.2**: Each cluster is labeled by its highest-PageRank member
- **AC5.3**: Singleton clusters (orphan notes) are grouped into an "Uncategorized" cluster
- **AC5.4**: Cluster density is computed as actual_edges / possible_edges

### F6: Git Repository Scanning
- **Behavior**: Scan a directory for git repos and extract activity metrics
- **AC6.1**: Finds repos by looking for `.git` directories (non-recursive, depth 1)
- **AC6.2**: Extracts last commit date via `git log`
- **AC6.3**: Counts commits in last 30 days
- **AC6.4**: Counts lines changed in last 30 days via `git diff --stat`
- **AC6.5**: Determines primary language from file extensions in recent commits

### F7: Analysis & Suggestions
- **Behavior**: Generate actionable insights from graph metrics
- **AC7.1**: **Hub Notes**: Top 10 notes by PageRank (your core ideas)
- **AC7.2**: **Bridge Concepts**: Top 10 notes by betweenness centrality (ideas connecting domains)
- **AC7.3**: **Orphan Notes**: Notes with 0 incoming links (potentially forgotten ideas)
- **AC7.4**: **Phantom Notes**: Referenced in links but no file exists (knowledge gaps)
- **AC7.5**: **Cluster Summary**: Each cluster with member count, density, and label
- **AC7.6**: **Cross-Cluster Bridges**: Notes that connect two or more different clusters
- **AC7.7**: **Suggested Connections**: Pairs of clusters with no links between them that share common themes (based on directory co-occurrence)

### F8: TUI Dashboard
- **Behavior**: Ratatui-based terminal UI with tab navigation
- **Tabs**:
  1. **Overview**: Total notes, links, clusters, top 5 hubs, top 5 bridges, vault health score
  2. **Knowledge Graph**: Sorted table of all notes with PageRank, in-degree, out-degree, cluster
  3. **Clusters**: List of clusters with members, density, inter-cluster connections
  4. **Bridges**: Bridge concepts ranked by betweenness centrality with the clusters they connect
  5. **Pulse**: Git repo activity dashboard sorted by recent activity
  6. **Suggestions**: Orphans, phantoms, suggested connections, recommended actions
- **Navigation**: Tab/Shift-Tab for tabs, j/k or Up/Down for scrolling, q to quit
- **AC8.1**: Renders without panic on terminal widths >= 80 columns
- **AC8.2**: Tab switching is instant (no recomputation)
- **AC8.3**: Scrollable lists for all data tables
- **AC8.4**: Color-coded metrics (green = healthy, yellow = attention, red = critical)

### F9: CLI Interface
- **Behavior**: `nexus [OPTIONS]`
- **Flags**:
  - `--vault <PATH>`: Path to Obsidian vault (required)
  - `--repos <PATH>`: Path to scan for git repos (optional, defaults to vault parent)
  - `--json`: Output analysis as JSON instead of launching TUI
  - `--top <N>`: Number of top results to show (default: 10)
- **AC9.1**: `--json` outputs valid JSON to stdout
- **AC9.2**: Missing `--vault` prints usage and exits with code 1
- **AC9.3**: Invalid vault path prints error and exits with code 1

## Non-Goals (v1)
- No LLM integration (pure graph algorithms)
- No file modification (read-only)
- No network access
- No real-time file watching (run once, display results)

## Dependencies

### nexus-core
- `regex` — Wikilink and tag extraction
- `walkdir` — Recursive directory traversal
- `serde` + `serde_json` — JSON serialization

### nexus-cli
- `ratatui` + `crossterm` — Terminal UI
- `clap` — CLI argument parsing
- `nexus-core` — Core library

## Vault Health Score

Computed as a weighted average:
- **Connectivity** (40%): 1.0 - (orphan_count / total_notes)
- **Completeness** (30%): 1.0 - (phantom_count / total_unique_references)
- **Structure** (30%): average cluster density across non-singleton clusters

Score ranges: 90-100 = Excellent, 70-89 = Good, 50-69 = Needs Work, <50 = Fragmented
