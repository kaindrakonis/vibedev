//! Traffic Monitor TUI
//!
//! Real-time display of Claude API traffic using ratatui.

use crate::proxy::ProxyEvent;
use crate::traffic::{TrafficLog, TrafficStatus};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs};
use ratatui::Frame;
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

const UPDATE_INTERVAL_MS: u64 = 100;

/// Traffic monitor application state
pub struct TrafficMonitorApp {
    traffic_log: TrafficLog,
    event_rx: mpsc::UnboundedReceiver<ProxyEvent>,
    selected_tab: usize,
    scroll_offset: usize,
    paused: bool,
    should_quit: bool,
    recent_events: Vec<String>,
    max_events: usize,
}

impl TrafficMonitorApp {
    pub fn new(
        traffic_log: TrafficLog,
        event_rx: mpsc::UnboundedReceiver<ProxyEvent>,
    ) -> Self {
        Self {
            traffic_log,
            event_rx,
            selected_tab: 0,
            scroll_offset: 0,
            paused: false,
            should_quit: false,
            recent_events: Vec::new(),
            max_events: 100,
        }
    }

    /// Run the TUI event loop
    pub async fn run(&mut self, terminal: &mut ratatui::Terminal<impl ratatui::backend::Backend>) -> io::Result<()> {
        loop {
            // Draw UI
            terminal.draw(|f| self.draw(f))?;

            // Poll for events with timeout
            if event::poll(Duration::from_millis(UPDATE_INTERVAL_MS))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code);
                    }
                }
            }

            // Check for proxy events
            while let Ok(event) = self.event_rx.try_recv() {
                self.handle_proxy_event(event);
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('p') | KeyCode::Char(' ') => self.paused = !self.paused,
            KeyCode::Tab => self.selected_tab = (self.selected_tab + 1) % 3,
            KeyCode::BackTab => {
                self.selected_tab = if self.selected_tab == 0 { 2 } else { self.selected_tab - 1 };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
            }
            KeyCode::Char('1') => self.selected_tab = 0,
            KeyCode::Char('2') => self.selected_tab = 1,
            KeyCode::Char('3') => self.selected_tab = 2,
            _ => {}
        }
    }

    fn handle_proxy_event(&mut self, event: ProxyEvent) {
        let msg = match event {
            ProxyEvent::RequestStarted { id, model, stream } => {
                format!("#{} {} stream={}", id, model, stream)
            }
            ProxyEvent::RequestCompleted { id, tokens_in, tokens_out, latency_ms } => {
                format!("#{} done: {}in/{}out {}ms", id, tokens_in, tokens_out, latency_ms)
            }
            ProxyEvent::RequestFailed { id, error } => {
                format!("#{} ERROR: {}", id, error)
            }
            ProxyEvent::StreamChunk { id, text } => {
                format!("#{} chunk: {}...", id, text.chars().take(50).collect::<String>())
            }
        };

        self.recent_events.push(msg);
        if self.recent_events.len() > self.max_events {
            self.recent_events.remove(0);
        }
    }

    fn draw(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(10),   // Content
                Constraint::Length(3), // Status bar
            ])
            .split(f.area());

        self.draw_tabs(f, chunks[0]);

        match self.selected_tab {
            0 => self.draw_live_tab(f, chunks[1]),
            1 => self.draw_stats_tab(f, chunks[1]),
            2 => self.draw_history_tab(f, chunks[1]),
            _ => {}
        }

        self.draw_status_bar(f, chunks[2]);
    }

    fn draw_tabs(&self, f: &mut Frame, area: Rect) {
        let titles = vec!["Live", "Stats", "History"];
        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Claude Traffic Monitor"))
            .select(self.selected_tab)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(tabs, area);
    }

    fn draw_live_tab(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        // Recent events
        let events: Vec<Line> = self
            .recent_events
            .iter()
            .rev()
            .take(area.height as usize - 2)
            .map(|e| {
                let color = if e.contains("ERROR") {
                    Color::Red
                } else if e.contains("done") {
                    Color::Green
                } else if e.contains("chunk") {
                    Color::Cyan
                } else {
                    Color::White
                };
                Line::from(Span::styled(e, Style::default().fg(color)))
            })
            .collect();

        let events_widget = Paragraph::new(events)
            .block(Block::default().borders(Borders::ALL).title("Live Events"));
        f.render_widget(events_widget, chunks[0]);

        // Quick stats
        let stats = self.traffic_log.get_stats();
        let stats_text = vec![
            Line::from(format!("Requests: {}", stats.total_requests)),
            Line::from(format!("Success: {}", stats.successful_requests)),
            Line::from(format!("Failed: {}", stats.failed_requests)),
            Line::from(""),
            Line::from(format!("Input tokens: {}", stats.total_input_tokens)),
            Line::from(format!("Output tokens: {}", stats.total_output_tokens)),
            Line::from(format!("Cache read: {}", stats.total_cache_read_tokens)),
            Line::from(""),
            Line::from(Span::styled(
                format!("Cost: ${:.4}", stats.total_cost_usd),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )),
            Line::from(format!("Avg latency: {:.0}ms", stats.avg_latency_ms)),
        ];

        let stats_widget = Paragraph::new(stats_text)
            .block(Block::default().borders(Borders::ALL).title("Quick Stats"));
        f.render_widget(stats_widget, chunks[1]);
    }

    fn draw_stats_tab(&self, f: &mut Frame, area: Rect) {
        let stats = self.traffic_log.get_stats();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Token usage
        let total_tokens = stats.total_input_tokens + stats.total_output_tokens;
        let input_pct = if total_tokens > 0 {
            (stats.total_input_tokens as f64 / total_tokens as f64 * 100.0) as u16
        } else {
            0
        };

        let token_text = vec![
            Line::from(format!("Total tokens: {}", total_tokens)),
            Line::from(format!("  Input:  {} ({:.1}%)", stats.total_input_tokens, input_pct)),
            Line::from(format!("  Output: {} ({:.1}%)", stats.total_output_tokens, 100 - input_pct)),
            Line::from(""),
            Line::from(format!("Cache read tokens: {}", stats.total_cache_read_tokens)),
            Line::from(format!("Cache write tokens: {}", stats.total_cache_write_tokens)),
            Line::from(""),
            Line::from(Span::styled(
                format!("Estimated cost: ${:.6}", stats.total_cost_usd),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            )),
        ];

        let token_widget = Paragraph::new(token_text)
            .block(Block::default().borders(Borders::ALL).title("Token Usage"));
        f.render_widget(token_widget, chunks[0]);

        // Model usage
        let mut model_rows: Vec<Row> = stats
            .models_used
            .iter()
            .map(|(model, count)| {
                Row::new(vec![
                    Cell::from(model.clone()),
                    Cell::from(count.to_string()),
                ])
            })
            .collect();

        if model_rows.is_empty() {
            model_rows.push(Row::new(vec![Cell::from("No requests yet"), Cell::from("-")]));
        }

        let model_table = Table::new(model_rows, [Constraint::Percentage(70), Constraint::Percentage(30)])
            .header(Row::new(vec!["Model", "Requests"]).style(Style::default().add_modifier(Modifier::BOLD)))
            .block(Block::default().borders(Borders::ALL).title("Models Used"));
        f.render_widget(model_table, chunks[1]);
    }

    fn draw_history_tab(&self, f: &mut Frame, area: Rect) {
        let entries = self.traffic_log.get_recent(50);

        let rows: Vec<Row> = entries
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(area.height as usize - 3)
            .map(|(_, entry)| {
                let status_style = match &entry.status {
                    TrafficStatus::Success => Style::default().fg(Color::Green),
                    TrafficStatus::Error(_) => Style::default().fg(Color::Red),
                    TrafficStatus::Pending => Style::default().fg(Color::Yellow),
                    TrafficStatus::Streaming => Style::default().fg(Color::Cyan),
                };

                let status = match &entry.status {
                    TrafficStatus::Success => "OK",
                    TrafficStatus::Error(_) => "ERR",
                    TrafficStatus::Pending => "...",
                    TrafficStatus::Streaming => "STRM",
                };

                let tokens = entry
                    .response
                    .as_ref()
                    .and_then(|r| r.usage.as_ref())
                    .map(|u| format!("{}/{}", u.input_tokens, u.output_tokens))
                    .unwrap_or_else(|| "-".to_string());

                let latency = entry
                    .latency_ms
                    .map(|l| format!("{}ms", l))
                    .unwrap_or_else(|| "-".to_string());

                Row::new(vec![
                    Cell::from(format!("#{}", entry.id)),
                    Cell::from(entry.timestamp.format("%H:%M:%S").to_string()),
                    Cell::from(entry.request.model.clone()).style(Style::default().fg(Color::Cyan)),
                    Cell::from(status).style(status_style),
                    Cell::from(tokens),
                    Cell::from(latency),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(6),  // ID
                Constraint::Length(10), // Time
                Constraint::Min(20),    // Model
                Constraint::Length(5),  // Status
                Constraint::Length(15), // Tokens
                Constraint::Length(10), // Latency
            ],
        )
        .header(
            Row::new(vec!["ID", "Time", "Model", "Status", "Tokens", "Latency"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL).title("Request History"));

        f.render_widget(table, area);
    }

    fn draw_status_bar(&self, f: &mut Frame, area: Rect) {
        let stats = self.traffic_log.get_stats();
        let status = if self.paused { "PAUSED" } else { "RUNNING" };

        let status_line = Line::from(vec![
            Span::styled(
                format!(" {} ", status),
                Style::default()
                    .fg(if self.paused { Color::Yellow } else { Color::Green })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | "),
            Span::raw(format!("Requests: {} ", stats.total_requests)),
            Span::raw(" | "),
            Span::styled(
                format!("Cost: ${:.4} ", stats.total_cost_usd),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(" | "),
            Span::raw("q: quit  p: pause  Tab: switch  ↑↓: scroll"),
        ]);

        let status_widget = Paragraph::new(status_line)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(status_widget, area);
    }
}

/// Initialize and run the traffic monitor TUI
pub async fn run_traffic_monitor(
    traffic_log: TrafficLog,
    event_rx: mpsc::UnboundedReceiver<ProxyEvent>,
) -> io::Result<()> {
    // Setup terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    // Run app
    let mut app = TrafficMonitorApp::new(traffic_log, event_rx);
    let result = app.run(&mut terminal).await;

    // Restore terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}
