use nexus_core::analysis::Analysis;
use nexus_core::git_scanner::GitRepoStats;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Overview,
    Graph,
    Clusters,
    Bridges,
    Pulse,
    Suggestions,
}

impl Tab {
    pub const ALL: [Tab; 6] = [
        Tab::Overview,
        Tab::Graph,
        Tab::Clusters,
        Tab::Bridges,
        Tab::Pulse,
        Tab::Suggestions,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Graph => "Graph",
            Tab::Clusters => "Clusters",
            Tab::Bridges => "Bridges",
            Tab::Pulse => "Pulse",
            Tab::Suggestions => "Suggestions",
        }
    }

    pub fn next(&self) -> Tab {
        let idx = Self::ALL.iter().position(|t| t == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(&self) -> Tab {
        let idx = Self::ALL.iter().position(|t| t == self).unwrap_or(0);
        if idx == 0 {
            Self::ALL[Self::ALL.len() - 1]
        } else {
            Self::ALL[idx - 1]
        }
    }
}

pub struct App {
    pub active_tab: Tab,
    pub analysis: Analysis,
    pub repos: Vec<GitRepoStats>,
    pub scroll_offset: usize,
    pub should_quit: bool,
}

impl App {
    pub fn new(analysis: Analysis, repos: Vec<GitRepoStats>) -> Self {
        Self {
            active_tab: Tab::Overview,
            analysis,
            repos,
            scroll_offset: 0,
            should_quit: false,
        }
    }

    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
        self.scroll_offset = 0;
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = self.active_tab.prev();
        self.scroll_offset = 0;
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn max_scroll(&self) -> usize {
        match self.active_tab {
            Tab::Overview => 0,
            Tab::Graph => self.analysis.hub_notes.len().saturating_sub(1),
            Tab::Clusters => self.analysis.cluster_summary.len().saturating_sub(1),
            Tab::Bridges => self.analysis.bridge_concepts.len().saturating_sub(1),
            Tab::Pulse => self.repos.len().saturating_sub(1),
            Tab::Suggestions => {
                (self.analysis.orphan_notes.len()
                    + self.analysis.phantom_notes.len()
                    + self.analysis.suggested_connections.len())
                .saturating_sub(1)
            }
        }
    }
}
