# Nexus

A Rust-powered Obsidian vault knowledge graph analyzer. No Zig required.

## What It Does

Nexus parses your Obsidian vault's wikilinks and tags, builds a directed
knowledge graph, then computes PageRank, betweenness centrality, and community
clusters to surface the hidden structure of your notes. It also scans local git
repositories for recent activity. Results are presented through an interactive
TUI dashboard or exported as JSON.

## Features

- Parses `[[wikilinks]]`, `[[link|alias]]`, and `#tags` from Markdown files
- Builds a directed knowledge graph with phantom node detection
- Computes PageRank to find your most important notes
- Computes betweenness centrality to find bridge concepts
- Detects note clusters via label propagation
- Calculates a vault health score (connectivity, completeness, structure)
- Identifies orphan notes, phantom notes, and suggested connections
- Scans git repositories for commit activity and language stats
- Six-tab TUI dashboard with scrollable tables and color-coded metrics

## Build

```
cargo build --release
```

## Usage

Launch the TUI dashboard:

```
cargo run -- --vault /path/to/vault
```

Export analysis as JSON:

```
cargo run -- --vault /path/to/vault --json
```

### Options

| Flag | Description |
|------|-------------|
| `--vault <PATH>` | Path to Obsidian vault (required) |
| `--repos <PATH>` | Path to scan for git repos (default: vault parent) |
| `--json` | Output JSON to stdout instead of launching TUI |
| `--top <N>` | Number of top results to show (default: 10) |

## TUI Navigation

- **Tab / Shift-Tab** -- switch tabs
- **1-6** -- jump to tab directly
- **j/k or Up/Down** -- scroll
- **q** -- quit
