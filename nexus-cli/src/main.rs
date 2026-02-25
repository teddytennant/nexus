mod app;
mod ui;

use app::App;
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use nexus_core::algorithms::compute_metrics;
use nexus_core::analysis::analyze;
use nexus_core::git_scanner::scan_repos;
use nexus_core::graph::KnowledgeGraph;
use nexus_core::parser::parse_vault;
use std::io::stdout;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "nexus", about = "Knowledge Graph Intelligence Engine")]
struct Cli {
    /// Path to Obsidian vault
    #[arg(long)]
    vault: PathBuf,

    /// Path to scan for git repos (defaults to vault parent)
    #[arg(long)]
    repos: Option<PathBuf>,

    /// Output analysis as JSON instead of launching TUI
    #[arg(long)]
    json: bool,

    /// Number of top results to show
    #[arg(long, default_value = "10")]
    top: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Validate vault path
    if !cli.vault.exists() || !cli.vault.is_dir() {
        eprintln!("Error: vault path '{}' does not exist or is not a directory", cli.vault.display());
        std::process::exit(1);
    }

    // Parse vault
    eprintln!("Parsing vault at {}...", cli.vault.display());
    let notes = parse_vault(&cli.vault)?;
    eprintln!("Found {} notes", notes.len());

    // Build graph
    let graph = KnowledgeGraph::from_notes(notes);
    eprintln!(
        "Built graph: {} nodes, {} edges, {} phantom nodes",
        graph.node_count(),
        graph.edge_count(),
        graph.phantom_nodes.len()
    );

    // Compute metrics
    eprintln!("Computing graph metrics...");
    let metrics = compute_metrics(&graph);

    // Analyze
    let analysis = analyze(&graph, &metrics, cli.top);

    // Scan repos
    let repos_path = cli.repos.unwrap_or_else(|| {
        cli.vault
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    });
    eprintln!("Scanning repos at {}...", repos_path.display());
    let repos = scan_repos(&repos_path);
    eprintln!("Found {} repos", repos.len());

    if cli.json {
        // JSON output mode
        let output = serde_json::json!({
            "analysis": analysis,
            "repos": repos,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // TUI mode
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;

    let mut terminal = ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(stdout()))?;
    let mut app = App::new(analysis, repos);

    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Tab => app.next_tab(),
                    KeyCode::BackTab => app.prev_tab(),
                    KeyCode::Char('j') | KeyCode::Down => {
                        if app.scroll_offset() < app.max_scroll() {
                            app.scroll_down();
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => app.scroll_up(),
                    KeyCode::Char('1') => app.active_tab = app::Tab::Overview,
                    KeyCode::Char('2') => app.active_tab = app::Tab::Graph,
                    KeyCode::Char('3') => app.active_tab = app::Tab::Clusters,
                    KeyCode::Char('4') => app.active_tab = app::Tab::Bridges,
                    KeyCode::Char('5') => app.active_tab = app::Tab::Pulse,
                    KeyCode::Char('6') => app.active_tab = app::Tab::Suggestions,
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
