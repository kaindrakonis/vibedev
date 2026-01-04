// TUI module - Interactive terminal UI with rich insights
use crate::analyzer::ConversationAnalyzer;
use crate::claude_code_parser::ClaudeCodeParser;
use crate::discovery::LogDiscovery;
use crate::models::DiscoveryFindings;
use crate::viral_insights::{ViralAnalyzer, ViralInsights};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{
        Bar, BarChart, BarGroup, Block, Borders, List, ListItem, Paragraph, Row, Sparkline, Table,
        Tabs, Wrap,
    },
    Frame, Terminal,
};
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Rich app state with insights data
pub struct App {
    pub findings: Option<DiscoveryFindings>,
    pub insights: Option<ViralInsights>,
    pub conversation_stats: Option<ConversationStats>,
    pub scan_progress: f64,
    pub scanning: bool,
    pub selected_tab: usize,
    pub selected_row: usize,
    pub scroll_offset: usize,
    pub base_dir: PathBuf,
    pub status_message: String,
    pub start_time: Instant,
    pub tool_sizes: HashMap<String, u64>,
    pub estimated_tokens: u64,
    pub estimated_cost: f64,
    pub total_conversations: usize,
    pub total_messages: usize,
    pub projects: HashMap<String, usize>,
    pub hourly_activity: [u64; 24],
    pub daily_activity: [u64; 7],
    pub achievements_unlocked: usize,
    pub recommendations: Vec<Recommendation>,
}

#[derive(Debug, Clone)]
pub struct ConversationStats {
    pub total_conversations: usize,
    pub total_messages: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub avg_messages_per_conv: f64,
    pub projects: HashMap<String, usize>,
}

#[derive(Debug, Clone)]
pub struct Recommendation {
    pub priority: Priority,
    pub title: String,
    pub description: String,
    pub savings: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Priority {
    High,
    Medium,
    Low,
}

impl App {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            findings: None,
            insights: None,
            conversation_stats: None,
            scan_progress: 0.0,
            scanning: false,
            selected_tab: 0,
            selected_row: 0,
            scroll_offset: 0,
            base_dir,
            status_message: "Loading... Press 'q' to quit".to_string(),
            start_time: Instant::now(),
            tool_sizes: HashMap::new(),
            estimated_tokens: 0,
            estimated_cost: 0.0,
            total_conversations: 0,
            total_messages: 0,
            projects: HashMap::new(),
            hourly_activity: [0; 24],
            daily_activity: [0; 7],
            achievements_unlocked: 0,
            recommendations: Vec::new(),
        }
    }

    pub fn start_scan(&mut self) {
        self.scanning = true;
        self.scan_progress = 0.0;
        self.status_message = "Scanning AI tool logs...".to_string();
        self.start_time = Instant::now();
    }

    pub fn finish_scan(&mut self, findings: DiscoveryFindings) {
        self.scanning = false;
        self.scan_progress = 100.0;

        // Calculate per-tool sizes
        self.tool_sizes.clear();
        for loc in &findings.locations {
            *self
                .tool_sizes
                .entry(loc.tool.name().to_string())
                .or_insert(0) += loc.size_bytes;
        }

        // Estimate tokens (rough: 1 byte ≈ 0.25 tokens for compressed text)
        self.estimated_tokens = findings.total_size_bytes / 4;

        // Estimate cost (Claude Sonnet pricing: ~$12/M tokens blended)
        self.estimated_cost = (self.estimated_tokens as f64 / 1_000_000.0) * 12.0;

        // Generate recommendations
        self.generate_recommendations(&findings);

        let elapsed = self.start_time.elapsed();
        self.status_message = format!(
            "Found {} files ({}) across {} tools in {:.1}s | Est. ${:.2} spent",
            findings.total_files,
            format_bytes(findings.total_size_bytes),
            findings.tools_found.len(),
            elapsed.as_secs_f64(),
            self.estimated_cost
        );
        self.findings = Some(findings);
    }

    pub fn load_insights(&mut self) {
        // Load Claude Code stats
        let parser = ClaudeCodeParser::new(self.base_dir.clone());
        if let Ok(stats) = parser.parse() {
            self.total_conversations = stats.total_conversations;
            self.total_messages = stats.total_messages;
            self.projects = stats.projects;
            self.estimated_tokens = stats.estimated_tokens.max(self.estimated_tokens);
        }

        // Load conversation stats from analyzer
        let analyzer = ConversationAnalyzer::new(self.base_dir.clone());
        if let Ok(stats) = analyzer.analyze() {
            // Calculate user/assistant messages from by_tool
            let mut user_msgs = 0;
            let mut asst_msgs = 0;
            for tool_stats in stats.by_tool.values() {
                user_msgs += tool_stats.user_messages;
                asst_msgs += tool_stats.assistant_messages;
            }

            self.conversation_stats = Some(ConversationStats {
                total_conversations: stats.total_conversations,
                total_messages: stats.total_messages,
                user_messages: user_msgs,
                assistant_messages: asst_msgs,
                avg_messages_per_conv: if stats.total_conversations > 0 {
                    stats.total_messages as f64 / stats.total_conversations as f64
                } else {
                    0.0
                },
                projects: HashMap::new(), // Not available in this analyzer
            });
            self.total_conversations = stats.total_conversations;
            self.total_messages = stats.total_messages;

            // Update cost estimate with actual tokens
            self.estimated_tokens = stats.total_tokens_estimate;
            self.estimated_cost = (self.estimated_tokens as f64 / 1_000_000.0) * 12.0;
        }

        // Load viral insights
        let viral = ViralAnalyzer::new(
            self.base_dir.clone(),
            self.estimated_tokens,
            self.estimated_cost,
        );
        if let Ok(insights) = viral.analyze() {
            // Copy hourly heatmap
            for (hour, count) in &insights.time_analytics.hourly_heatmap {
                if *hour < 24 {
                    self.hourly_activity[*hour] = *count as u64;
                }
            }

            // Copy daily heatmap
            let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
            for (i, day) in days.iter().enumerate() {
                if let Some(count) = insights.time_analytics.daily_heatmap.get(*day) {
                    self.daily_activity[i] = *count as u64;
                }
            }

            self.achievements_unlocked =
                insights.achievements.iter().filter(|a| a.unlocked).count();
            self.insights = Some(insights);
        }
    }

    fn generate_recommendations(&mut self, findings: &DiscoveryFindings) {
        self.recommendations.clear();

        // Check for large debug logs
        let debug_size: u64 = findings
            .locations
            .iter()
            .filter(|l| matches!(l.log_type, crate::models::LogType::Debug))
            .map(|l| l.size_bytes)
            .sum();

        if debug_size > 100 * 1024 * 1024 {
            self.recommendations.push(Recommendation {
                priority: Priority::High,
                title: "Clean debug logs".to_string(),
                description: format!(
                    "Debug logs are using {}. These can be safely deleted.",
                    format_bytes(debug_size)
                ),
                savings: Some(format!("Save {}", format_bytes(debug_size))),
            });
        }

        // Check for old file-history
        let file_history_size: u64 = findings
            .locations
            .iter()
            .filter(|l| matches!(l.log_type, crate::models::LogType::FileHistory))
            .map(|l| l.size_bytes)
            .sum();

        if file_history_size > 500 * 1024 * 1024 {
            self.recommendations.push(Recommendation {
                priority: Priority::Medium,
                title: "Prune file history".to_string(),
                description: format!(
                    "File history is using {}. Consider keeping only recent backups.",
                    format_bytes(file_history_size)
                ),
                savings: Some(format!("Save ~{}", format_bytes(file_history_size / 2))),
            });
        }

        // Check for multiple similar tools
        if findings.tools_found.len() > 3 {
            self.recommendations.push(Recommendation {
                priority: Priority::Low,
                title: "Consolidate AI tools".to_string(),
                description: format!(
                    "You have {} AI tools installed. Consider standardizing on fewer tools.",
                    findings.tools_found.len()
                ),
                savings: None,
            });
        }

        // Cost optimization
        if self.estimated_cost > 50.0 {
            self.recommendations.push(Recommendation {
                priority: Priority::Medium,
                title: "Review usage patterns".to_string(),
                description: format!(
                    "Estimated spend: ${:.2}. Use caching or batch similar queries.",
                    self.estimated_cost
                ),
                savings: Some("Save ~30% with caching".to_string()),
            });
        }

        // Backup recommendation
        if findings.total_size_bytes > 1024 * 1024 * 1024 {
            self.recommendations.push(Recommendation {
                priority: Priority::High,
                title: "Create backup".to_string(),
                description: "Your AI logs exceed 1GB. Create a backup archive.".to_string(),
                savings: Some("Run: vibedev backup".to_string()),
            });
        }
    }

    pub fn next_tab(&mut self) {
        self.selected_tab = (self.selected_tab + 1) % 5;
        self.selected_row = 0;
        self.scroll_offset = 0;
    }

    pub fn prev_tab(&mut self) {
        self.selected_tab = if self.selected_tab == 0 {
            4
        } else {
            self.selected_tab - 1
        };
        self.selected_row = 0;
        self.scroll_offset = 0;
    }

    pub fn next_row(&mut self) {
        if let Some(ref findings) = self.findings {
            let max_rows = match self.selected_tab {
                1 => findings.locations.len(),
                4 => self.recommendations.len(),
                _ => 10,
            };
            if max_rows > 0 {
                self.selected_row = (self.selected_row + 1) % max_rows;
            }
        }
    }

    pub fn prev_row(&mut self) {
        if let Some(ref findings) = self.findings {
            let max_rows = match self.selected_tab {
                1 => findings.locations.len(),
                4 => self.recommendations.len(),
                _ => 10,
            };
            if max_rows > 0 {
                self.selected_row = if self.selected_row == 0 {
                    max_rows - 1
                } else {
                    self.selected_row - 1
                };
            }
        }
    }
}

/// Run the TUI application
pub fn run_tui(base_dir: PathBuf) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(base_dir.clone());

    // Auto-start scan
    app.start_scan();
    let discovery = LogDiscovery::new(base_dir, true);

    // Run scan
    let findings = discovery.scan()?;
    app.finish_scan(findings);

    // Load rich insights
    app.load_insights();

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Tab => app.next_tab(),
                        KeyCode::BackTab => app.prev_tab(),
                        KeyCode::Down | KeyCode::Char('j') => app.next_row(),
                        KeyCode::Up | KeyCode::Char('k') => app.prev_row(),
                        KeyCode::Char('r') if !app.scanning => {
                            app.start_scan();
                            let discovery = LogDiscovery::new(app.base_dir.clone(), true);
                            if let Ok(findings) = discovery.scan() {
                                app.finish_scan(findings);
                                app.load_insights();
                            }
                        }
                        KeyCode::Char('1') => app.selected_tab = 0,
                        KeyCode::Char('2') => app.selected_tab = 1,
                        KeyCode::Char('3') => app.selected_tab = 2,
                        KeyCode::Char('4') => app.selected_tab = 3,
                        KeyCode::Char('5') => app.selected_tab = 4,
                        _ => {}
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Title + tabs
            Constraint::Length(3), // Status bar
            Constraint::Min(10),   // Main content
            Constraint::Length(1), // Footer
        ])
        .split(f.area());

    // Header with tabs
    let titles = vec![
        "[1] Dashboard",
        "[2] Storage",
        "[3] Activity",
        "[4] Insights",
        "[5] Actions",
    ];
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" vibedev - AI Coding Assistant Analyzer ")
                .title_style(Style::default().fg(Color::Cyan).bold()),
        )
        .select(app.selected_tab)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider(symbols::DOT);
    f.render_widget(tabs, chunks[0]);

    // Status bar
    let status = Paragraph::new(app.status_message.clone())
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(status, chunks[1]);

    // Main content area
    match app.selected_tab {
        0 => render_dashboard(f, app, chunks[2]),
        1 => render_storage(f, app, chunks[2]),
        2 => render_activity(f, app, chunks[2]),
        3 => render_insights(f, app, chunks[2]),
        4 => render_actions(f, app, chunks[2]),
        _ => {}
    }

    // Footer
    let footer = Paragraph::new(" q:Quit  Tab:Navigate  r:Rescan  1-5:Jump  j/k:Select ")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[3]);
}

fn render_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Key metrics
            Constraint::Length(8), // Activity sparklines
            Constraint::Min(5),    // Tool breakdown
        ])
        .split(area);

    // Top: Key metrics in boxes
    let metrics_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(chunks[0]);

    // Storage metric
    let storage_text = app
        .findings
        .as_ref()
        .map(|f| format_bytes(f.total_size_bytes))
        .unwrap_or_else(|| "0 B".to_string());
    render_metric_box(f, metrics_chunks[0], "STORAGE", &storage_text, Color::Cyan);

    // Conversations metric
    let conv_text = format!("{}", app.total_conversations);
    render_metric_box(
        f,
        metrics_chunks[1],
        "CONVERSATIONS",
        &conv_text,
        Color::Green,
    );

    // Tokens metric
    let tokens_text = format_tokens(app.estimated_tokens);
    render_metric_box(f, metrics_chunks[2], "TOKENS", &tokens_text, Color::Yellow);

    // Cost metric
    let cost_text = format!("${:.2}", app.estimated_cost);
    render_metric_box(
        f,
        metrics_chunks[3],
        "EST. COST",
        &cost_text,
        Color::Magenta,
    );

    // Middle: Activity sparklines
    let spark_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // Hourly activity sparkline
    let hourly_data: Vec<u64> = app.hourly_activity.to_vec();
    let max_hourly = *hourly_data.iter().max().unwrap_or(&1);
    let hourly_sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Hourly Activity (24h) "),
        )
        .data(&hourly_data)
        .max(max_hourly.max(1))
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(hourly_sparkline, spark_chunks[0]);

    // Daily activity sparkline
    let daily_data: Vec<u64> = app.daily_activity.to_vec();
    let max_daily = *daily_data.iter().max().unwrap_or(&1);
    let daily_sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Weekly Activity (Mon-Sun) "),
        )
        .data(&daily_data)
        .max(max_daily.max(1))
        .style(Style::default().fg(Color::Green));
    f.render_widget(daily_sparkline, spark_chunks[1]);

    // Bottom: Tool breakdown
    render_tool_bars(f, app, chunks[2]);
}

fn render_metric_box(f: &mut Frame, area: Rect, label: &str, value: &str, color: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = vec![
        Line::from(Span::styled(
            label,
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            value,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
    ];

    let paragraph = Paragraph::new(text).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(paragraph, inner);
}

fn render_tool_bars(f: &mut Frame, app: &App, area: Rect) {
    let Some(ref findings) = app.findings else {
        let placeholder = Paragraph::new("No data")
            .block(Block::default().borders(Borders::ALL).title(" Tools "));
        f.render_widget(placeholder, area);
        return;
    };

    let mut tool_data: Vec<_> = app.tool_sizes.iter().collect();
    tool_data.sort_by(|a, b| b.1.cmp(a.1));

    let max_size = tool_data.first().map(|(_, s)| **s).unwrap_or(1);

    let bars: Vec<Bar> = tool_data
        .iter()
        .take(8)
        .map(|(name, size)| {
            let height = ((**size as f64 / max_size as f64) * 100.0) as u64;
            Bar::default()
                .value(height)
                .label(Line::from(truncate(name, 10)))
                .text_value(format_bytes(**size))
                .style(Style::default().fg(Color::Cyan))
        })
        .collect();

    let bar_chart = BarChart::default()
        .block(Block::default().borders(Borders::ALL).title(format!(
            " Storage by Tool ({}) ",
            findings.tools_found.len()
        )))
        .data(BarGroup::default().bars(&bars))
        .bar_width(10)
        .bar_gap(1)
        .max(100);

    f.render_widget(bar_chart, area);
}

fn render_storage(f: &mut Frame, app: &App, area: Rect) {
    let Some(ref findings) = app.findings else {
        let placeholder = Paragraph::new("No data. Press 'r' to scan.")
            .block(Block::default().borders(Borders::ALL).title(" Storage "));
        f.render_widget(placeholder, area);
        return;
    };

    // Sort locations by size (largest first)
    let mut locations: Vec<_> = findings.locations.iter().collect();
    locations.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    let rows: Vec<Row> = locations
        .iter()
        .enumerate()
        .map(|(i, loc)| {
            let style = if i == app.selected_row {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let pct = (loc.size_bytes as f64 / findings.total_size_bytes as f64) * 100.0;
            let bar_width = ((pct / 100.0) * 20.0) as usize;
            let bar = "█".repeat(bar_width) + &"░".repeat(20 - bar_width);

            // Color code by type
            let type_color = match loc.log_type {
                crate::models::LogType::Debug => Color::Red,
                crate::models::LogType::History => Color::Green,
                crate::models::LogType::FileHistory => Color::Yellow,
                crate::models::LogType::Session => Color::Cyan,
                _ => Color::Gray,
            };

            Row::new(vec![
                Span::styled(
                    loc.tool.name().to_string(),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("{:?}", loc.log_type),
                    Style::default().fg(type_color),
                ),
                Span::styled(
                    format_bytes(loc.size_bytes),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(format!("{:>5.1}%", pct)),
                Span::styled(bar, Style::default().fg(Color::Green)),
                Span::raw(loc.file_count.to_string()),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec!["Tool", "Type", "Size", "%", "Usage", "Files"])
        .style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::White),
        )
        .bottom_margin(1);

    let table = Table::new(
        rows,
        [
            Constraint::Length(15),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(7),
            Constraint::Length(22),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(format!(
        " Storage Locations ({}) - Total: {} ",
        findings.locations.len(),
        format_bytes(findings.total_size_bytes)
    )));

    f.render_widget(table, area);
}

fn render_activity(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Hourly heatmap
            Constraint::Min(8),     // Stats
        ])
        .split(area);

    // Hourly heatmap visualization
    let mut hourly_lines: Vec<Line> = vec![Line::from(Span::styled(
        "  00  01  02  03  04  05  06  07  08  09  10  11  12  13  14  15  16  17  18  19  20  21  22  23",
        Style::default().fg(Color::DarkGray),
    ))];

    let max_hourly = *app.hourly_activity.iter().max().unwrap_or(&1).max(&1);
    let mut hour_spans: Vec<Span> = vec![];

    for (i, &count) in app.hourly_activity.iter().enumerate() {
        let intensity = if max_hourly > 0 {
            (count as f64 / max_hourly as f64 * 4.0) as usize
        } else {
            0
        };

        let block = match intensity {
            0 => "  . ",
            1 => " ░░ ",
            2 => " ▒▒ ",
            3 => " ▓▓ ",
            _ => " ██ ",
        };

        let color = match intensity {
            0 => Color::DarkGray,
            1 => Color::Blue,
            2 => Color::Cyan,
            3 => Color::Green,
            _ => Color::Yellow,
        };

        hour_spans.push(Span::styled(block, Style::default().fg(color)));

        if (i + 1) % 24 == 0 {
            hourly_lines.push(Line::from(hour_spans.clone()));
            hour_spans.clear();
        }
    }
    if !hour_spans.is_empty() {
        hourly_lines.push(Line::from(hour_spans));
    }

    // Peak hour indicator
    let peak_hour = app
        .hourly_activity
        .iter()
        .enumerate()
        .max_by_key(|(_, &v)| v)
        .map(|(h, _)| h)
        .unwrap_or(0);

    hourly_lines.push(Line::from(""));
    hourly_lines.push(Line::from(vec![
        Span::raw("Peak productivity: "),
        Span::styled(
            format!("{:02}:00 - {:02}:00", peak_hour, (peak_hour + 1) % 24),
            Style::default().fg(Color::Green).bold(),
        ),
    ]));

    let heatmap = Paragraph::new(hourly_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Hourly Activity Heatmap "),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(heatmap, chunks[0]);

    // Stats section
    let chunks_bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // Daily breakdown
    let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let max_daily = *app.daily_activity.iter().max().unwrap_or(&1).max(&1);

    let daily_bars: Vec<Bar> = days
        .iter()
        .enumerate()
        .map(|(i, day)| {
            let count = app.daily_activity[i];
            let height = if max_daily > 0 {
                ((count as f64 / max_daily as f64) * 100.0) as u64
            } else {
                0
            };
            Bar::default()
                .value(height)
                .label(Line::from(*day))
                .text_value(count.to_string())
                .style(Style::default().fg(if i < 5 { Color::Cyan } else { Color::Magenta }))
        })
        .collect();

    let daily_chart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Daily Activity "),
        )
        .data(BarGroup::default().bars(&daily_bars))
        .bar_width(5)
        .bar_gap(1)
        .max(100);

    f.render_widget(daily_chart, chunks_bottom[0]);

    // Session stats
    let session_info = if let Some(ref insights) = app.insights {
        let ta = &insights.time_analytics;
        format!(
            "Late night sessions: {}\n\
             Binge coding (8h+): {}\n\
             Most productive hour: {:02}:00\n\
             Most productive day: {}\n\
             Avg gap between sessions: {:.1}h",
            ta.late_night_sessions,
            ta.binge_coding_sessions,
            ta.most_productive_hour,
            ta.most_productive_day,
            ta.average_session_gap_hours
        )
    } else {
        "Loading session analytics...".to_string()
    };

    let session_para = Paragraph::new(session_info)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Session Stats "),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(session_para, chunks_bottom[1]);
}

fn render_insights(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: Fun facts & Comparisons
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    let fun_facts_text = if let Some(ref insights) = app.insights {
        let ff = &insights.fun_facts;
        let cmp = &insights.comparisons;
        format!(
            "Your AI usage equals:\n\n\
             {} novels worth of text\n\
             {} pages if printed\n\
             {:.4}% of Wikipedia\n\
             {} hours of reading\n\
             {} kg CO2 footprint\n\
             {} coffees worth ($5 each)\n\n\
             That's {} Harry Potter series!",
            format!("{:.1}", ff.tokens_in_books),
            ff.pages_if_printed,
            ff.tokens_in_wikipedia,
            format!("{:.0}", ff.reading_time_hours),
            format!("{:.2}", ff.carbon_footprint_kg),
            ff.cost_in_coffee,
            format!("{:.1}", cmp.harry_potter_series),
        )
    } else {
        "Loading fun facts...".to_string()
    };

    let fun_facts = Paragraph::new(fun_facts_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Fun Facts ")
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(fun_facts, left_chunks[0]);

    // Behavior patterns
    let behavior_text = if let Some(ref insights) = app.insights {
        let bp = &insights.behavior_patterns;
        format!(
            "Frustration moments: {}\n\
             'Go on' count: {}\n\
             Retry attempts: {}\n\
             Politeness score: {:.0}%\n\
             Command spam events: {}\n\
             Typos detected: {}",
            bp.frustration_count,
            bp.go_on_count,
            bp.retry_count,
            bp.politeness_score * 100.0,
            bp.command_spam_events,
            bp.typo_count,
        )
    } else {
        "Loading behavior analysis...".to_string()
    };

    let behavior = Paragraph::new(behavior_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Behavior Patterns ")
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(behavior, left_chunks[1]);

    // Right: Achievements
    let achievements_list: Vec<ListItem> = if let Some(ref insights) = app.insights {
        insights
            .achievements
            .iter()
            .map(|a| {
                let style = if a.unlocked {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let status = if a.unlocked { "[x]" } else { "[ ]" };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{} {} ", status, a.emoji), style),
                    Span::styled(&a.name, style.add_modifier(Modifier::BOLD)),
                    Span::styled(format!(" - {}", a.description), style),
                ]))
            })
            .collect()
    } else {
        vec![ListItem::new("Loading achievements...")]
    };

    let achievements = List::new(achievements_list).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " Achievements ({}/{}) ",
                app.achievements_unlocked,
                app.insights
                    .as_ref()
                    .map(|i| i.achievements.len())
                    .unwrap_or(0)
            ))
            .border_style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(achievements, chunks[1]);
}

fn render_actions(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(8)])
        .split(area);

    // Recommendations list
    let rec_items: Vec<ListItem> = app
        .recommendations
        .iter()
        .enumerate()
        .map(|(i, rec)| {
            let (icon, color) = match rec.priority {
                Priority::High => ("!!!", Color::Red),
                Priority::Medium => (" ! ", Color::Yellow),
                Priority::Low => ("   ", Color::Green),
            };

            let style = if i == app.selected_row {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            let savings_text = rec
                .savings
                .as_ref()
                .map(|s| format!(" [{}]", s))
                .unwrap_or_default();

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(icon, Style::default().fg(color).bold()),
                    Span::styled(&rec.title, Style::default().bold().fg(Color::White)),
                    Span::styled(savings_text, Style::default().fg(Color::Green)),
                ]),
                Line::from(Span::styled(
                    format!("    {}", rec.description),
                    Style::default().fg(Color::Gray),
                )),
            ])
            .style(style)
        })
        .collect();

    let recommendations = List::new(rec_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Recommendations ({}) ", app.recommendations.len()))
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(recommendations, chunks[0]);

    // Quick actions
    let actions_text = "\
Commands you can run:\n\n\
  vibedev backup              Create backup archive\n\
  vibedev analyze --html      Generate HTML report\n\
  vibedev prepare             Export training dataset\n\
  vibedev insights            Full insights dashboard";

    let actions = Paragraph::new(actions_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Quick Actions ")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(actions, chunks[1]);
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000_000 {
        format!("{:.1}B", tokens as f64 / 1_000_000_000.0)
    } else if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        format!("{}", tokens)
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Print CLI output (non-interactive mode)
pub fn print_cli_output(base_dir: PathBuf) -> Result<()> {
    use colored::Colorize as ColoredColorize;
    use std::time::Instant;

    println!(
        "{}",
        ColoredColorize::bold(ColoredColorize::cyan("vibedev - Scanning..."))
    );
    println!();

    let start = Instant::now();
    let discovery = crate::discovery::LogDiscovery::new(base_dir.clone(), true);
    let findings = discovery.scan()?;
    let elapsed = start.elapsed();

    // Calculate tool sizes
    let mut tool_sizes: HashMap<String, u64> = HashMap::new();
    for loc in &findings.locations {
        *tool_sizes.entry(loc.tool.name().to_string()).or_insert(0) += loc.size_bytes;
    }

    // Sort by size
    let mut tool_items: Vec<_> = tool_sizes.iter().collect();
    tool_items.sort_by(|a, b| b.1.cmp(a.1));

    // Estimate tokens and cost
    let estimated_tokens = findings.total_size_bytes / 4;
    let estimated_cost = (estimated_tokens as f64 / 1_000_000.0) * 12.0;

    // Header
    println!(
        "{}",
        ColoredColorize::bright_black(
            "═══════════════════════════════════════════════════════════════════════════════"
        )
    );
    println!(
        "  {}  {:>12}  {:>7}  {}",
        ColoredColorize::bold("Tool"),
        ColoredColorize::bold("Size"),
        ColoredColorize::bold("%"),
        ColoredColorize::bold("Distribution")
    );
    println!(
        "{}",
        ColoredColorize::bright_black(
            "───────────────────────────────────────────────────────────────────────────────"
        )
    );

    // Tool rows with visual bars
    for (name, size) in &tool_items {
        let pct = (**size as f64 / findings.total_size_bytes as f64) * 100.0;
        let bar_width = ((pct / 100.0) * 40.0) as usize;
        let bar = "█".repeat(bar_width);
        let empty = "░".repeat(40 - bar_width);

        println!(
            "  {:<18} {:>10}  {:>5.1}%  {}{}",
            ColoredColorize::cyan(name.as_str()),
            ColoredColorize::yellow(format_bytes(**size).as_str()),
            pct,
            ColoredColorize::green(bar.as_str()),
            ColoredColorize::bright_black(empty.as_str())
        );
    }

    println!(
        "{}",
        ColoredColorize::bright_black(
            "═══════════════════════════════════════════════════════════════════════════════"
        )
    );

    // Summary
    println!();
    println!(
        "{}",
        ColoredColorize::underline(ColoredColorize::bold("Summary"))
    );
    println!(
        "  Total Storage:  {}",
        ColoredColorize::bold(ColoredColorize::yellow(
            format_bytes(findings.total_size_bytes).as_str()
        ))
    );
    println!(
        "  Total Files:    {}",
        ColoredColorize::cyan(findings.total_files.to_string().as_str())
    );
    println!(
        "  Tools Found:    {}",
        ColoredColorize::cyan(findings.tools_found.len().to_string().as_str())
    );
    println!(
        "  Est. Tokens:    {}",
        ColoredColorize::yellow(format_tokens(estimated_tokens).as_str())
    );
    println!(
        "  Est. Cost:      {}",
        ColoredColorize::bold(ColoredColorize::magenta(
            format!("${:.2}", estimated_cost).as_str()
        ))
    );
    println!("  Scan Time:      {:.2}s", elapsed.as_secs_f64());

    // Load more stats
    let parser = ClaudeCodeParser::new(base_dir.clone());
    if let Ok(stats) = parser.parse() {
        println!();
        println!(
            "{}",
            ColoredColorize::underline(ColoredColorize::bold("Conversations"))
        );
        println!(
            "  Total:          {}",
            ColoredColorize::cyan(stats.total_conversations.to_string().as_str())
        );
        println!(
            "  Messages:       {}",
            ColoredColorize::cyan(stats.total_messages.to_string().as_str())
        );
        println!(
            "  User:           {}",
            ColoredColorize::green(stats.user_messages.to_string().as_str())
        );
        println!(
            "  Assistant:      {}",
            ColoredColorize::blue(stats.assistant_messages.to_string().as_str())
        );
    }

    // Top locations
    println!();
    println!(
        "{}",
        ColoredColorize::underline(ColoredColorize::bold("Top Locations by Size"))
    );

    let mut locations: Vec<_> = findings.locations.iter().collect();
    locations.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    for loc in locations.iter().take(10) {
        let pct = (loc.size_bytes as f64 / findings.total_size_bytes as f64) * 100.0;
        println!(
            "  {:>10} {:>5.1}%  {} {} {}",
            ColoredColorize::yellow(format_bytes(loc.size_bytes).as_str()),
            pct,
            ColoredColorize::cyan(loc.tool.name()),
            ColoredColorize::bright_black(format!("{:?}", loc.log_type).as_str()),
            ColoredColorize::bright_black(truncate(&loc.path.to_string_lossy(), 40).as_str())
        );
    }

    println!();
    println!(
        "{}",
        ColoredColorize::bright_black("Run 'vibedev tui' for interactive dashboard")
    );

    Ok(())
}
