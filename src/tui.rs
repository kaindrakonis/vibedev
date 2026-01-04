// TUI module - Real-time monitoring dashboard (like btop)
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
        Block, Borders, Gauge, Paragraph, Row, Sparkline, Table, Tabs, Wrap,
    },
    Frame, Terminal,
};
use std::collections::{HashMap, VecDeque};
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const UPDATE_INTERVAL_MS: u64 = 1000; // 1 second updates
const HISTORY_SIZE: usize = 60; // Keep 60 seconds of history

/// Real-time metrics tracker
#[derive(Debug, Clone)]
pub struct MetricsHistory {
    pub timestamps: VecDeque<u64>,
    pub storage: VecDeque<u64>,
    pub conversations: VecDeque<usize>,
    pub messages: VecDeque<usize>,
    pub tokens: VecDeque<u64>,
    pub cost: VecDeque<f64>,
    pub files: VecDeque<usize>,
    pub active_sessions: VecDeque<usize>,
}

impl MetricsHistory {
    fn new() -> Self {
        Self {
            timestamps: VecDeque::with_capacity(HISTORY_SIZE),
            storage: VecDeque::with_capacity(HISTORY_SIZE),
            conversations: VecDeque::with_capacity(HISTORY_SIZE),
            messages: VecDeque::with_capacity(HISTORY_SIZE),
            tokens: VecDeque::with_capacity(HISTORY_SIZE),
            cost: VecDeque::with_capacity(HISTORY_SIZE),
            files: VecDeque::with_capacity(HISTORY_SIZE),
            active_sessions: VecDeque::with_capacity(HISTORY_SIZE),
        }
    }

    fn push(&mut self, snapshot: MetricsSnapshot) {
        if self.timestamps.len() >= HISTORY_SIZE {
            self.timestamps.pop_front();
            self.storage.pop_front();
            self.conversations.pop_front();
            self.messages.pop_front();
            self.tokens.pop_front();
            self.cost.pop_front();
            self.files.pop_front();
            self.active_sessions.pop_front();
        }

        self.timestamps.push_back(snapshot.timestamp);
        self.storage.push_back(snapshot.storage);
        self.conversations.push_back(snapshot.conversations);
        self.messages.push_back(snapshot.messages);
        self.tokens.push_back(snapshot.tokens);
        self.cost.push_back(snapshot.cost);
        self.files.push_back(snapshot.files);
        self.active_sessions.push_back(snapshot.active_sessions);
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub timestamp: u64,
    pub storage: u64,
    pub conversations: usize,
    pub messages: usize,
    pub tokens: u64,
    pub cost: f64,
    pub files: usize,
    pub active_sessions: usize,
}

/// Real-time monitoring app
pub struct App {
    pub findings: Option<DiscoveryFindings>,
    pub insights: Option<ViralInsights>,
    pub base_dir: PathBuf,
    pub status_message: String,
    pub tool_sizes: HashMap<String, u64>,
    pub estimated_tokens: u64,
    pub estimated_cost: f64,
    pub total_conversations: usize,
    pub total_messages: usize,
    pub total_files: usize,
    pub active_sessions: usize,
    pub history: MetricsHistory,
    pub last_update: Instant,
    pub update_count: u64,
    pub paused: bool,
    pub selected_tab: usize,
    pub achievements_unlocked: usize,
    pub hourly_activity: [u64; 24],
    pub start_time: Instant,
    pub uptime: Duration,
}

impl App {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            findings: None,
            insights: None,
            base_dir,
            status_message: "Starting real-time monitoring...".to_string(),
            tool_sizes: HashMap::new(),
            estimated_tokens: 0,
            estimated_cost: 0.0,
            total_conversations: 0,
            total_messages: 0,
            total_files: 0,
            active_sessions: 0,
            history: MetricsHistory::new(),
            last_update: Instant::now(),
            update_count: 0,
            paused: false,
            selected_tab: 0,
            achievements_unlocked: 0,
            hourly_activity: [0; 24],
            start_time: Instant::now(),
            uptime: Duration::ZERO,
        }
    }

    pub fn update(&mut self) -> Result<()> {
        if self.paused {
            return Ok(());
        }

        self.uptime = self.start_time.elapsed();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Scan for changes
        let discovery = LogDiscovery::new(self.base_dir.clone(), true);
        let findings = discovery.scan()?;

        // Calculate metrics
        self.tool_sizes.clear();
        for loc in &findings.locations {
            *self
                .tool_sizes
                .entry(loc.tool.name().to_string())
                .or_insert(0) += loc.size_bytes;
        }

        self.total_files = findings.total_files;
        self.estimated_tokens = findings.total_size_bytes / 4;
        self.estimated_cost = (self.estimated_tokens as f64 / 1_000_000.0) * 12.0;

        // Load conversation stats
        let analyzer = ConversationAnalyzer::new(self.base_dir.clone());
        if let Ok(stats) = analyzer.analyze() {
            self.total_conversations = stats.total_conversations;
            self.total_messages = stats.total_messages;
            self.estimated_tokens = stats.total_tokens_estimate;
            self.estimated_cost = (self.estimated_tokens as f64 / 1_000_000.0) * 12.0;
        }

        // Detect active sessions (files modified in last 5 minutes)
        self.active_sessions = findings
            .locations
            .iter()
            .filter(|loc| {
                if let Some(newest) = loc.newest_entry {
                    let age = chrono::Utc::now().signed_duration_since(newest);
                    age.num_minutes() < 5
                } else {
                    false
                }
            })
            .count();

        // Save snapshot to history
        let snapshot = MetricsSnapshot {
            timestamp: now,
            storage: findings.total_size_bytes,
            conversations: self.total_conversations,
            messages: self.total_messages,
            tokens: self.estimated_tokens,
            cost: self.estimated_cost,
            files: self.total_files,
            active_sessions: self.active_sessions,
        };
        self.history.push(snapshot);

        self.findings = Some(findings);
        self.update_count += 1;
        self.last_update = Instant::now();

        // Load insights on first update
        if self.update_count == 1 {
            self.load_insights();
        }

        self.status_message = format!(
            "Updates: {} | Active: {} | Storage: {} | Cost: ${:.2}",
            self.update_count,
            self.active_sessions,
            format_bytes(self.findings.as_ref().map(|f| f.total_size_bytes).unwrap_or(0)),
            self.estimated_cost
        );

        Ok(())
    }

    fn load_insights(&mut self) {
        // Load Claude Code stats
        let parser = ClaudeCodeParser::new(self.base_dir.clone());
        if let Ok(stats) = parser.parse() {
            self.total_conversations = stats.total_conversations;
            self.total_messages = stats.total_messages;
            self.estimated_tokens = stats.estimated_tokens.max(self.estimated_tokens);
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

            self.achievements_unlocked = insights.achievements.iter().filter(|a| a.unlocked).count();
            self.insights = Some(insights);
        }
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn next_tab(&mut self) {
        self.selected_tab = (self.selected_tab + 1) % 3;
    }

    pub fn prev_tab(&mut self) {
        self.selected_tab = if self.selected_tab == 0 {
            2
        } else {
            self.selected_tab - 1
        };
    }
}

/// Run the real-time TUI
pub fn run_tui(base_dir: PathBuf) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(base_dir);

    // Initial update
    app.update()?;

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
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, app))?;

        // Update every second
        let timeout = UPDATE_INTERVAL_MS
            .saturating_sub(last_tick.elapsed().as_millis() as u64);

        if event::poll(Duration::from_millis(timeout))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Char('p') | KeyCode::Char(' ') => app.toggle_pause(),
                        KeyCode::Tab => app.next_tab(),
                        KeyCode::BackTab => app.prev_tab(),
                        KeyCode::Char('1') => app.selected_tab = 0,
                        KeyCode::Char('2') => app.selected_tab = 1,
                        KeyCode::Char('3') => app.selected_tab = 2,
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= Duration::from_millis(UPDATE_INTERVAL_MS) && !app.paused {
            app.update()?;
            last_tick = Instant::now();
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(5),  // Gauges
            Constraint::Min(10),    // Main content
            Constraint::Length(1),  // Footer
        ])
        .split(f.area());

    // Header
    render_header(f, app, chunks[0]);

    // Gauges
    render_gauges(f, app, chunks[1]);

    // Main content
    match app.selected_tab {
        0 => render_realtime(f, app, chunks[2]),
        1 => render_activity(f, app, chunks[2]),
        2 => render_tools(f, app, chunks[2]),
        _ => {}
    }

    // Footer
    let footer = if app.paused {
        " [PAUSED] p:Resume  q:Quit  Tab:Switch  1-3:Jump "
    } else {
        " p:Pause  q:Quit  Tab:Switch  1-3:Jump "
    };
    let footer_widget = Paragraph::new(footer)
        .style(Style::default().fg(if app.paused { Color::Yellow } else { Color::DarkGray }));
    f.render_widget(footer_widget, chunks[3]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let titles = vec![
        "[1] Real-Time",
        "[2] Activity",
        "[3] Tools",
    ];
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " vibedev Monitor {} | Uptime: {}s ",
                    if app.paused { "[PAUSED]" } else { "" },
                    app.uptime.as_secs()
                ))
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
    f.render_widget(tabs, area);
}

fn render_gauges(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(area);

    // Active sessions gauge
    let active_pct = ((app.active_sessions as f64 / 10.0) * 100.0).min(100.0) as u16;
    let active_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" Active "))
        .gauge_style(Style::default().fg(if app.active_sessions > 0 { Color::Green } else { Color::DarkGray }))
        .percent(active_pct)
        .label(format!("{}", app.active_sessions));
    f.render_widget(active_gauge, chunks[0]);

    // Conversations gauge
    let conv_pct = ((app.total_conversations as f64 / 2000.0) * 100.0).min(100.0) as u16;
    let conv_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" Convos "))
        .gauge_style(Style::default().fg(Color::Cyan))
        .percent(conv_pct)
        .label(format!("{}", app.total_conversations));
    f.render_widget(conv_gauge, chunks[1]);

    // Messages gauge
    let msg_pct = ((app.total_messages as f64 / 500000.0) * 100.0).min(100.0) as u16;
    let msg_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" Messages "))
        .gauge_style(Style::default().fg(Color::Blue))
        .percent(msg_pct)
        .label(format_large(app.total_messages));
    f.render_widget(msg_gauge, chunks[2]);

    // Tokens gauge
    let token_pct = ((app.estimated_tokens as f64 / 500_000_000.0) * 100.0).min(100.0) as u16;
    let token_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" Tokens "))
        .gauge_style(Style::default().fg(Color::Yellow))
        .percent(token_pct)
        .label(format_tokens(app.estimated_tokens));
    f.render_widget(token_gauge, chunks[3]);

    // Cost gauge
    let cost_pct = ((app.estimated_cost / 2000.0) * 100.0).min(100.0) as u16;
    let cost_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" Cost "))
        .gauge_style(Style::default().fg(Color::Magenta))
        .percent(cost_pct)
        .label(format!("${:.0}", app.estimated_cost));
    f.render_widget(cost_gauge, chunks[4]);
}

fn render_realtime(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Storage sparkline
            Constraint::Length(8),  // Conversations sparkline
            Constraint::Length(8),  // Tokens sparkline
            Constraint::Min(4),     // Live stats
        ])
        .split(area);

    // Storage over time
    let storage_data: Vec<u64> = app.history.storage.iter().cloned().collect();
    let max_storage = *storage_data.iter().max().unwrap_or(&1);
    let storage_sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Storage ({}s) - Current: {} ",
                    storage_data.len(),
                    format_bytes(storage_data.last().cloned().unwrap_or(0))
                )),
        )
        .data(&storage_data)
        .max(max_storage.max(1))
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(storage_sparkline, chunks[0]);

    // Conversations over time
    let conv_data: Vec<u64> = app.history.conversations.iter().map(|&x| x as u64).collect();
    let max_conv = *conv_data.iter().max().unwrap_or(&1);
    let conv_sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Conversations ({}s) - Current: {} ",
                    conv_data.len(),
                    conv_data.last().cloned().unwrap_or(0)
                )),
        )
        .data(&conv_data)
        .max(max_conv.max(1))
        .style(Style::default().fg(Color::Green));
    f.render_widget(conv_sparkline, chunks[1]);

    // Tokens over time
    let token_data: Vec<u64> = app.history.tokens.iter().cloned().collect();
    let max_tokens = *token_data.iter().max().unwrap_or(&1);
    let tokens_sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Tokens ({}s) - Current: {} ",
                    token_data.len(),
                    format_tokens(token_data.last().cloned().unwrap_or(0))
                )),
        )
        .data(&token_data)
        .max(max_tokens.max(1))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(tokens_sparkline, chunks[2]);

    // Live stats
    let stats_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[3]);

    // Calculate rates (per second)
    let storage_rate = if app.history.storage.len() >= 2 {
        let first = app.history.storage.front().cloned().unwrap_or(0);
        let last = app.history.storage.back().cloned().unwrap_or(0);
        let time_diff = app.history.timestamps.len() as f64;
        ((last as f64 - first as f64) / time_diff) as i64
    } else {
        0
    };

    let conv_rate = if app.history.conversations.len() >= 2 {
        let first = app.history.conversations.front().cloned().unwrap_or(0);
        let last = app.history.conversations.back().cloned().unwrap_or(0);
        let time_diff = app.history.timestamps.len() as f64;
        ((last as f64 - first as f64) / time_diff * 60.0) as i64 // per minute
    } else {
        0
    };

    let stats_text = format!(
        "Storage Rate: {}/s\n\
         Conversation Rate: {}/min\n\
         Files Tracked: {}\n\
         Tools Found: {}\n\
         Active Sessions: {}\n\
         Updates: {}",
        format_bytes_signed(storage_rate),
        conv_rate,
        app.total_files,
        app.tool_sizes.len(),
        app.active_sessions,
        app.update_count
    );

    let stats_para = Paragraph::new(stats_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Live Stats "),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(stats_para, stats_chunks[0]);

    // Recent changes
    let changes_text = if app.history.storage.len() >= 2 {
        let storage_delta = app.history.storage.back().cloned().unwrap_or(0) as i64
            - app.history.storage.front().cloned().unwrap_or(0) as i64;
        let conv_delta = app.history.conversations.back().cloned().unwrap_or(0) as i64
            - app.history.conversations.front().cloned().unwrap_or(0) as i64;
        let msg_delta = app.history.messages.back().cloned().unwrap_or(0) as i64
            - app.history.messages.front().cloned().unwrap_or(0) as i64;
        let cost_delta = app.history.cost.back().cloned().unwrap_or(0.0)
            - app.history.cost.front().cloned().unwrap_or(0.0);

        format!(
            "Storage: {}\n\
             Conversations: {:+}\n\
             Messages: {:+}\n\
             Cost: ${:+.2}\n\n\
             Since: {}s ago",
            format_bytes_signed(storage_delta),
            conv_delta,
            msg_delta,
            cost_delta,
            app.history.timestamps.len()
        )
    } else {
        "Collecting data...".to_string()
    };

    let changes_para = Paragraph::new(changes_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Changes (Last 60s) "),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(changes_para, stats_chunks[1]);
}

fn render_activity(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Active sessions sparkline
            Constraint::Min(8),     // Hourly heatmap
        ])
        .split(area);

    // Active sessions over time
    let active_data: Vec<u64> = app.history.active_sessions.iter().map(|&x| x as u64).collect();
    let max_active = *active_data.iter().max().unwrap_or(&1).max(&1);
    let active_sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Active Sessions ({}s) - Current: {} ",
                    active_data.len(),
                    active_data.last().cloned().unwrap_or(0)
                )),
        )
        .data(&active_data)
        .max(max_active)
        .style(Style::default().fg(Color::Green));
    f.render_widget(active_sparkline, chunks[0]);

    // Hourly activity heatmap
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

    let peak_hour = app
        .hourly_activity
        .iter()
        .enumerate()
        .max_by_key(|(_, &v)| v)
        .map(|(h, _)| h)
        .unwrap_or(0);

    hourly_lines.push(Line::from(""));
    hourly_lines.push(Line::from(vec![
        Span::raw("Peak: "),
        Span::styled(
            format!("{:02}:00 - {:02}:00", peak_hour, (peak_hour + 1) % 24),
            Style::default().fg(Color::Green).bold(),
        ),
    ]));

    let heatmap = Paragraph::new(hourly_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 24-Hour Activity Heatmap "),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(heatmap, chunks[1]);
}

fn render_tools(f: &mut Frame, app: &App, area: Rect) {
    let Some(ref findings) = app.findings else {
        let placeholder = Paragraph::new("Scanning...")
            .block(Block::default().borders(Borders::ALL).title(" Tools "));
        f.render_widget(placeholder, area);
        return;
    };

    let mut tool_data: Vec<_> = app.tool_sizes.iter().collect();
    tool_data.sort_by(|a, b| b.1.cmp(a.1));

    let rows: Vec<Row> = tool_data
        .iter()
        .map(|(name, size)| {
            let pct = (**size as f64 / findings.total_size_bytes as f64) * 100.0;
            let bar_width = ((pct / 100.0) * 30.0) as usize;
            let bar = "█".repeat(bar_width) + &"░".repeat(30 - bar_width);

            Row::new(vec![
                Span::styled(name.to_string(), Style::default().fg(Color::Cyan)),
                Span::styled(format_bytes(**size), Style::default().fg(Color::Yellow)),
                Span::raw(format!("{:>5.1}%", pct)),
                Span::styled(bar, Style::default().fg(Color::Green)),
            ])
        })
        .collect();

    let header = Row::new(vec!["Tool", "Size", "%", "Distribution"])
        .style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::White),
        )
        .bottom_margin(1);

    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Min(32),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Tools ({}) ", tool_data.len())),
    );

    f.render_widget(table, area);
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

fn format_bytes_signed(bytes: i64) -> String {
    let abs_bytes = bytes.abs() as u64;
    let sign = if bytes >= 0 { "+" } else { "-" };
    format!("{}{}", sign, format_bytes(abs_bytes))
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

fn format_large(num: usize) -> String {
    if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}K", num as f64 / 1_000.0)
    } else {
        format!("{}", num)
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
        ColoredColorize::bright_black("Run 'vibedev tui' for real-time monitoring dashboard")
    );

    Ok(())
}
