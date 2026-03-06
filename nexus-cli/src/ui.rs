use crate::app::{App, Tab};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table, Tabs, Wrap};
use ratatui::Frame;

const GREEN: Color = Color::Rgb(120, 200, 120);
const YELLOW: Color = Color::Rgb(230, 200, 80);
const RED: Color = Color::Rgb(220, 80, 80);
const CYAN: Color = Color::Rgb(100, 200, 220);
const DIM: Color = Color::Rgb(120, 120, 140);
const WHITE: Color = Color::Rgb(220, 220, 230);
const ACCENT: Color = Color::Rgb(160, 130, 255);

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tabs
            Constraint::Min(0),   // content
            Constraint::Length(1), // status bar
        ])
        .split(frame.area());

    draw_tabs(frame, app, chunks[0]);

    match app.active_tab {
        Tab::Overview => draw_overview(frame, app, chunks[1]),
        Tab::Graph => draw_graph(frame, app, chunks[1]),
        Tab::Clusters => draw_clusters(frame, app, chunks[1]),
        Tab::Bridges => draw_bridges(frame, app, chunks[1]),
        Tab::Pulse => draw_pulse(frame, app, chunks[1]),
        Tab::Suggestions => draw_suggestions(frame, app, chunks[1]),
    }

    draw_status_bar(frame, app, chunks[2]);
}

fn draw_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|t| {
            let style = if *t == app.active_tab {
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(DIM)
            };
            Line::from(Span::styled(t.label(), style))
        })
        .collect();

    let idx = Tab::ALL
        .iter()
        .position(|t| *t == app.active_tab)
        .unwrap_or(0);

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .title(" Nexus ")
                .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        )
        .select(idx)
        .highlight_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));

    frame.render_widget(tabs, area);
}

fn draw_status_bar(frame: &mut Frame, _app: &App, area: Rect) {
    let text = Line::from(vec![
        Span::styled(" Tab", Style::default().fg(ACCENT)),
        Span::styled("/", Style::default().fg(DIM)),
        Span::styled("Shift-Tab", Style::default().fg(ACCENT)),
        Span::styled(": switch tabs  ", Style::default().fg(DIM)),
        Span::styled("j/k", Style::default().fg(ACCENT)),
        Span::styled(": scroll  ", Style::default().fg(DIM)),
        Span::styled("q", Style::default().fg(ACCENT)),
        Span::styled(": quit", Style::default().fg(DIM)),
    ]);
    let bar = Paragraph::new(text);
    frame.render_widget(bar, area);
}

fn draw_overview(frame: &mut Frame, app: &App, area: Rect) {
    let health = &app.analysis.vault_health;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // health card
            Constraint::Length(12), // top hubs
            Constraint::Min(0),    // top bridges
        ])
        .split(area);

    // Health card
    let health_color = match health.score as u32 {
        90..=100 => GREEN,
        70..=89 => YELLOW,
        _ => RED,
    };

    let health_text = vec![
        Line::from(vec![
            Span::styled("  Vault Health: ", Style::default().fg(WHITE)),
            Span::styled(
                format!("{:.0}/100 ({})", health.score, health.rating),
                Style::default().fg(health_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Notes: ", Style::default().fg(DIM)),
            Span::styled(format!("{}", health.total_notes), Style::default().fg(WHITE)),
            Span::styled("    Links: ", Style::default().fg(DIM)),
            Span::styled(format!("{}", health.total_links), Style::default().fg(WHITE)),
            Span::styled("    Clusters: ", Style::default().fg(DIM)),
            Span::styled(format!("{}", health.total_clusters), Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  Connectivity: ", Style::default().fg(DIM)),
            Span::styled(format!("{:.0}%", health.connectivity * 100.0), Style::default().fg(score_color(health.connectivity))),
            Span::styled("  Completeness: ", Style::default().fg(DIM)),
            Span::styled(format!("{:.0}%", health.completeness * 100.0), Style::default().fg(score_color(health.completeness))),
            Span::styled("  Structure: ", Style::default().fg(DIM)),
            Span::styled(format!("{:.0}%", health.structure * 100.0), Style::default().fg(score_color(health.structure))),
        ]),
        Line::from(vec![
            Span::styled("  Orphans: ", Style::default().fg(DIM)),
            Span::styled(format!("{}", app.analysis.orphan_notes.len()), Style::default().fg(if app.analysis.orphan_notes.is_empty() { GREEN } else { YELLOW })),
            Span::styled("    Phantoms: ", Style::default().fg(DIM)),
            Span::styled(format!("{}", app.analysis.phantom_notes.len()), Style::default().fg(if app.analysis.phantom_notes.is_empty() { GREEN } else { RED })),
        ]),
    ];
    let health_block = Paragraph::new(health_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Health ")
            .title_style(Style::default().fg(ACCENT)),
    );
    frame.render_widget(health_block, chunks[0]);

    // Top hubs
    let hub_rows: Vec<Line> = app
        .analysis
        .hub_notes
        .iter()
        .take(5)
        .enumerate()
        .map(|(i, h)| {
            Line::from(vec![
                Span::styled(format!("  {}. ", i + 1), Style::default().fg(DIM)),
                Span::styled(&h.id, Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("  PR: {:.4}  in: {}  out: {}", h.score, h.in_degree, h.out_degree),
                    Style::default().fg(DIM),
                ),
            ])
        })
        .collect();
    let hubs_block = Paragraph::new(hub_rows).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Top Hub Notes (PageRank) ")
            .title_style(Style::default().fg(ACCENT)),
    );
    frame.render_widget(hubs_block, chunks[1]);

    // Top bridges
    let bridge_rows: Vec<Line> = app
        .analysis
        .bridge_concepts
        .iter()
        .take(5)
        .enumerate()
        .map(|(i, b)| {
            Line::from(vec![
                Span::styled(format!("  {}. ", i + 1), Style::default().fg(DIM)),
                Span::styled(&b.id, Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("  BC: {:.4}  in: {}  out: {}", b.score, b.in_degree, b.out_degree),
                    Style::default().fg(DIM),
                ),
            ])
        })
        .collect();
    let bridges_block = Paragraph::new(bridge_rows).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Top Bridge Concepts (Betweenness) ")
            .title_style(Style::default().fg(ACCENT)),
    );
    frame.render_widget(bridges_block, chunks[2]);
}

fn draw_graph(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("Note").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Cell::from("PageRank").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Cell::from("In").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Cell::from("Out").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Cell::from("Cluster").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
    ]);

    let rows: Vec<Row> = app.analysis.all_notes
        .iter()
        .skip(app.scroll_offset())
        .map(|n| {
            Row::new(vec![
                Cell::from(n.id.clone()).style(Style::default().fg(CYAN)),
                Cell::from(format!("{:.6}", n.score)).style(Style::default().fg(WHITE)),
                Cell::from(format!("{}", n.in_degree)).style(Style::default().fg(GREEN)),
                Cell::from(format!("{}", n.out_degree)).style(Style::default().fg(YELLOW)),
                Cell::from(n.cluster_label.clone()).style(Style::default().fg(DIM)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(15),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(35),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Knowledge Graph — All Notes ")
            .title_style(Style::default().fg(ACCENT))
            .padding(Padding::horizontal(1)),
    );

    frame.render_widget(table, area);
}

fn draw_clusters(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("Cluster").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Cell::from("Members").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Cell::from("Density").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Cell::from("Top Members").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
    ]);

    let rows: Vec<Row> = app
        .analysis
        .cluster_summary
        .iter()
        .skip(app.scroll_offset())
        .map(|c| {
            let density_color = if c.density > 0.5 {
                GREEN
            } else if c.density > 0.2 {
                YELLOW
            } else {
                RED
            };
            Row::new(vec![
                Cell::from(c.label.clone()).style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
                Cell::from(format!("{}", c.member_count)).style(Style::default().fg(WHITE)),
                Cell::from(format!("{:.2}", c.density)).style(Style::default().fg(density_color)),
                Cell::from(c.top_members.join(", ")).style(Style::default().fg(DIM)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(20),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(60),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Clusters ")
            .title_style(Style::default().fg(ACCENT))
            .padding(Padding::horizontal(1)),
    );

    frame.render_widget(table, area);
}

fn draw_bridges(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("Bridge").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Cell::from("Betweenness").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Cell::from("In/Out").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Cell::from("Cluster").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
    ]);

    let rows: Vec<Row> = app
        .analysis
        .bridge_concepts
        .iter()
        .skip(app.scroll_offset())
        .map(|b| {
            Row::new(vec![
                Cell::from(b.id.clone()).style(Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
                Cell::from(format!("{:.6}", b.score)).style(Style::default().fg(WHITE)),
                Cell::from(format!("{}/{}", b.in_degree, b.out_degree)).style(Style::default().fg(CYAN)),
                Cell::from(b.cluster_label.clone()).style(Style::default().fg(DIM)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Percentage(15),
            Constraint::Percentage(35),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Bridge Concepts (Betweenness Centrality) ")
            .title_style(Style::default().fg(ACCENT))
            .padding(Padding::horizontal(1)),
    );

    frame.render_widget(table, area);
}

fn draw_pulse(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("Repository").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Cell::from("Language").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Cell::from("Commits (30d)").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Cell::from("Lines (30d)").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Cell::from("Last Commit").style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
    ]);

    let rows: Vec<Row> = app
        .repos
        .iter()
        .skip(app.scroll_offset())
        .map(|r| {
            let activity_color = if r.commit_count_30d > 10 {
                GREEN
            } else if r.commit_count_30d > 0 {
                YELLOW
            } else {
                RED
            };
            Row::new(vec![
                Cell::from(r.name.clone()).style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
                Cell::from(r.primary_language.clone().unwrap_or_else(|| "-".to_string()))
                    .style(Style::default().fg(WHITE)),
                Cell::from(format!("{}", r.commit_count_30d)).style(Style::default().fg(activity_color)),
                Cell::from(format!("{}", r.lines_changed_30d)).style(Style::default().fg(WHITE)),
                Cell::from(
                    r.last_commit_date
                        .as_ref()
                        .map(|d| d.chars().take(10).collect::<String>())
                        .unwrap_or_else(|| "-".to_string()),
                )
                .style(Style::default().fg(DIM)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(25),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(30),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Project Pulse — Git Activity ")
            .title_style(Style::default().fg(ACCENT))
            .padding(Padding::horizontal(1)),
    );

    frame.render_widget(table, area);
}

fn draw_suggestions(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(30),
            Constraint::Percentage(35),
        ])
        .split(area);

    // Orphan notes
    let orphan_lines: Vec<Line> = app
        .analysis
        .orphan_notes
        .iter()
        .take(15)
        .map(|o| {
            Line::from(vec![
                Span::styled("  - ", Style::default().fg(DIM)),
                Span::styled(o, Style::default().fg(YELLOW)),
                Span::styled("  (no incoming links)", Style::default().fg(DIM)),
            ])
        })
        .collect();
    let orphan_count = app.analysis.orphan_notes.len();
    let orphan_block = Paragraph::new(orphan_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Orphan Notes ({}) ", orphan_count))
                .title_style(Style::default().fg(YELLOW)),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(orphan_block, chunks[0]);

    // Phantom notes
    let phantom_lines: Vec<Line> = app
        .analysis
        .phantom_notes
        .iter()
        .take(15)
        .map(|p| {
            Line::from(vec![
                Span::styled("  - ", Style::default().fg(DIM)),
                Span::styled(p, Style::default().fg(RED)),
                Span::styled("  (referenced but no file)", Style::default().fg(DIM)),
            ])
        })
        .collect();
    let phantom_count = app.analysis.phantom_notes.len();
    let phantom_block = Paragraph::new(phantom_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Phantom Notes ({}) ", phantom_count))
                .title_style(Style::default().fg(RED)),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(phantom_block, chunks[1]);

    // Suggested connections
    let suggestion_lines: Vec<Line> = app
        .analysis
        .suggested_connections
        .iter()
        .take(10)
        .map(|s| {
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(&s.cluster_a, Style::default().fg(CYAN)),
                Span::styled(" <-> ", Style::default().fg(ACCENT)),
                Span::styled(&s.cluster_b, Style::default().fg(CYAN)),
                Span::styled(format!("  ({})", s.reason), Style::default().fg(DIM)),
            ])
        })
        .collect();
    let sugg_count = app.analysis.suggested_connections.len();
    let sugg_block = Paragraph::new(suggestion_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Suggested Connections ({}) ", sugg_count))
                .title_style(Style::default().fg(GREEN)),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(sugg_block, chunks[2]);
}

fn score_color(score: f64) -> Color {
    if score >= 0.8 {
        GREEN
    } else if score >= 0.5 {
        YELLOW
    } else {
        RED
    }
}
