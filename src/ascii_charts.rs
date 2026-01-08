//! ASCII Chart rendering for beautiful terminal visualizations
//! Inspired by Claude Code's /model output
//!
//! This module provides 10+ beautiful ASCII chart types:
//! - LineChart: Time-series with multi-series support
//! - BarChart: Horizontal bars with percentages
//! - ActivityHeatmap: GitHub-style contribution graph
//! - StatsCard: Key metrics in card format
//! - FunFact: Whimsical token comparisons
//! - StreakCounter: Streak with flames
//! - Histogram: Distribution visualization
//! - ProgressBar: Goal tracking
//! - Leaderboard: Ranked list with medals
//! - CalendarView: Monthly calendar with activity
//! - TimeDistribution: Hour-of-day breakdown
//! - ComparisonChart: Side-by-side comparison

#![allow(dead_code)]

use chrono::{DateTime, Datelike, Utc};
use colored::Colorize;
use std::collections::HashMap;

/// Characters for line chart drawing
const CHART_CHARS: [char; 9] = ['┼', '│', '─', '╭', '╮', '╰', '╯', '┤', '┬'];

/// Braille-style sparkline characters (8 levels)
const SPARK_CHARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Box drawing characters
const BOX_H: char = '─';
const BOX_V: char = '│';
const BOX_TL: char = '╭';
const BOX_TR: char = '╮';
const BOX_BL: char = '╰';
const BOX_BR: char = '╯';

/// Color palette for multi-series charts
const COLORS: [&str; 6] = ["cyan", "magenta", "yellow", "green", "blue", "red"];

/// A data point with timestamp and value
#[derive(Clone, Debug)]
pub struct DataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

/// A data series for charting
#[derive(Clone, Debug)]
pub struct Series {
    pub name: String,
    pub data: Vec<DataPoint>,
    pub color: String,
}

impl Series {
    pub fn new(name: &str, color: &str) -> Self {
        Self {
            name: name.to_string(),
            data: Vec::new(),
            color: color.to_string(),
        }
    }

    pub fn add(&mut self, timestamp: DateTime<Utc>, value: f64) {
        self.data.push(DataPoint { timestamp, value });
    }

    pub fn max_value(&self) -> f64 {
        self.data.iter().map(|d| d.value).fold(0.0, f64::max)
    }

    pub fn sum(&self) -> f64 {
        self.data.iter().map(|d| d.value).sum()
    }
}

/// Line chart renderer (like the tokens per day chart)
pub struct LineChart {
    pub title: String,
    pub series: Vec<Series>,
    pub width: usize,
    pub height: usize,
    pub y_label_width: usize,
}

impl LineChart {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            series: Vec::new(),
            width: 56,
            height: 8,
            y_label_width: 6,
        }
    }

    pub fn with_size(mut self, width: usize, height: usize) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn add_series(&mut self, series: Series) {
        self.series.push(series);
    }

    /// Render the chart to a string
    pub fn render(&self) -> String {
        let mut output = String::new();

        // Title
        output.push_str(&format!("  {}\n", self.title.bold()));

        if self.series.is_empty() || self.series.iter().all(|s| s.data.is_empty()) {
            output.push_str("    No data available\n");
            return output;
        }

        // Find global max and date range
        let max_val = self
            .series
            .iter()
            .map(|s| s.max_value())
            .fold(0.0, f64::max);

        let all_points: Vec<&DataPoint> = self.series.iter().flat_map(|s| &s.data).collect();

        if all_points.is_empty() {
            output.push_str("    No data points\n");
            return output;
        }

        let min_time = all_points
            .iter()
            .map(|d| d.timestamp)
            .min()
            .unwrap_or(Utc::now());
        let max_time = all_points
            .iter()
            .map(|d| d.timestamp)
            .max()
            .unwrap_or(Utc::now());

        // Create the chart grid
        let chart_width = self.width - self.y_label_width - 2;
        let chart_height = self.height;

        // Initialize grid with spaces
        let mut grid: Vec<Vec<char>> = vec![vec![' '; chart_width]; chart_height];

        // Plot each series
        for (series_idx, series) in self.series.iter().enumerate() {
            let color_idx = series_idx % COLORS.len();
            let _color = COLORS[color_idx];

            for (i, point) in series.data.iter().enumerate() {
                // Calculate x position based on time
                let time_range = (max_time - min_time).num_seconds().max(1) as f64;
                let time_offset = (point.timestamp - min_time).num_seconds() as f64;
                let x = ((time_offset / time_range) * (chart_width - 1) as f64) as usize;
                let x = x.min(chart_width - 1);

                // Calculate y position based on value
                let y = if max_val > 0.0 {
                    ((1.0 - point.value / max_val) * (chart_height - 1) as f64) as usize
                } else {
                    chart_height - 1
                };
                let y = y.min(chart_height - 1);

                // Determine the character based on neighbors
                let prev_y = if i > 0 {
                    let prev = &series.data[i - 1];
                    let prev_val = prev.value / max_val.max(1.0);
                    Some(((1.0 - prev_val) * (chart_height - 1) as f64) as usize)
                } else {
                    None
                };

                let next_y = if i < series.data.len() - 1 {
                    let next = &series.data[i + 1];
                    let next_val = next.value / max_val.max(1.0);
                    Some(((1.0 - next_val) * (chart_height - 1) as f64) as usize)
                } else {
                    None
                };

                // Choose character based on direction
                let ch = match (prev_y, next_y) {
                    (Some(py), Some(ny)) if py > y && ny > y => '╰', // valley going up both sides
                    (Some(py), Some(ny)) if py < y && ny < y => '╭', // peak
                    (Some(py), Some(ny)) if py > y && ny < y => '╯', // going down then up
                    (Some(py), Some(ny)) if py < y && ny > y => '╮', // going up then down
                    (Some(py), None) if py < y => '╯',
                    (Some(py), None) if py > y => '╮',
                    (None, Some(ny)) if ny < y => '╰',
                    (None, Some(ny)) if ny > y => '╭',
                    _ => '│',
                };

                grid[y][x] = ch;

                // Draw connecting lines
                if let Some(py) = prev_y {
                    if i > 0 {
                        let prev_point = &series.data[i - 1];
                        let prev_time_offset = (prev_point.timestamp - min_time).num_seconds() as f64;
                        let prev_x =
                            ((prev_time_offset / time_range) * (chart_width - 1) as f64) as usize;
                        let prev_x = prev_x.min(chart_width - 1);

                        // Draw horizontal line between points
                        for dx in (prev_x + 1)..x {
                            if grid[y][dx] == ' ' {
                                grid[y][dx] = '─';
                            }
                        }

                        // Draw vertical connection if needed
                        let min_y = y.min(py);
                        let max_y = y.max(py);
                        for dy in (min_y + 1)..max_y {
                            if dy < chart_height {
                                let connect_x = if y < py { x } else { prev_x };
                                if connect_x < chart_width && grid[dy][connect_x] == ' ' {
                                    grid[dy][connect_x] = '│';
                                }
                            }
                        }
                    }
                }
            }
        }

        // Render y-axis labels and grid
        let y_labels = calculate_y_labels(max_val, chart_height);

        for (row_idx, row) in grid.iter().enumerate() {
            let y_label = &y_labels[row_idx];
            output.push_str(&format!("{:>width$} ", y_label, width = self.y_label_width));

            if row_idx == 0 {
                output.push_str("┼");
            } else {
                output.push_str("┤");
            }

            // Color the line based on series
            let row_str: String = row.iter().collect();
            if !self.series.is_empty() {
                let colored_row = match self.series[0].color.as_str() {
                    "cyan" => row_str.cyan(),
                    "magenta" => row_str.magenta(),
                    "yellow" => row_str.yellow(),
                    "green" => row_str.green(),
                    "blue" => row_str.blue(),
                    "red" => row_str.red(),
                    _ => row_str.white(),
                };
                output.push_str(&format!("{}", colored_row));
            } else {
                output.push_str(&row_str);
            }
            output.push('\n');
        }

        // X-axis
        output.push_str(&format!(
            "{:>width$} └",
            "",
            width = self.y_label_width
        ));
        output.push_str(&"─".repeat(chart_width));
        output.push('\n');

        // X-axis labels (dates)
        let date_labels = calculate_x_labels(min_time, max_time, chart_width);
        output.push_str(&format!(
            "{:>width$}  {}",
            "",
            date_labels,
            width = self.y_label_width
        ));
        output.push('\n');

        // Legend
        if self.series.len() > 1 {
            output.push_str("  ");
            for series in &self.series {
                let bullet = match series.color.as_str() {
                    "cyan" => "●".cyan(),
                    "magenta" => "●".magenta(),
                    "yellow" => "●".yellow(),
                    "green" => "●".green(),
                    "blue" => "●".blue(),
                    "red" => "●".red(),
                    _ => "●".white(),
                };
                output.push_str(&format!("{} {} · ", bullet, series.name));
            }
            output.push('\n');
        }

        output
    }
}

/// Format large numbers with K/M/B suffixes
fn format_number(n: f64) -> String {
    if n >= 1_000_000_000.0 {
        format!("{:.1}B", n / 1_000_000_000.0)
    } else if n >= 1_000_000.0 {
        format!("{:.1}M", n / 1_000_000.0)
    } else if n >= 1_000.0 {
        format!("{:.0}k", n / 1_000.0)
    } else if n >= 1.0 {
        format!("{:.0}", n)
    } else {
        format!("{:.1}", n)
    }
}

/// Calculate Y-axis labels
fn calculate_y_labels(max_val: f64, height: usize) -> Vec<String> {
    let mut labels = Vec::with_capacity(height);
    for i in 0..height {
        let val = max_val * (1.0 - i as f64 / (height - 1).max(1) as f64);
        labels.push(format_number(val));
    }
    labels
}

/// Calculate X-axis date labels
fn calculate_x_labels(min_time: DateTime<Utc>, max_time: DateTime<Utc>, width: usize) -> String {
    let mut labels = String::new();

    // Show 3-4 date labels spread across the width
    let num_labels = 4;
    let label_spacing = width / num_labels;

    for i in 0..num_labels {
        let t = min_time
            + chrono::Duration::seconds(
                ((max_time - min_time).num_seconds() as f64 * i as f64 / (num_labels - 1) as f64)
                    as i64,
            );
        let label = format!("{} {}", month_abbrev(t.month()), t.day());

        if i == 0 {
            labels.push_str(&label);
        } else {
            let padding = label_spacing.saturating_sub(label.len() / 2);
            labels.push_str(&" ".repeat(padding));
            labels.push_str(&label);
        }
    }

    labels
}

fn month_abbrev(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

/// Horizontal bar chart for breakdown displays
pub struct BarChart {
    pub title: String,
    pub items: Vec<BarItem>,
    pub width: usize,
    pub show_percentages: bool,
}

#[derive(Clone, Debug)]
pub struct BarItem {
    pub label: String,
    pub value: f64,
    pub sub_label: Option<String>,
    pub color: String,
}

impl BarChart {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            items: Vec::new(),
            width: 40,
            show_percentages: true,
        }
    }

    pub fn add(&mut self, label: &str, value: f64, color: &str) {
        self.items.push(BarItem {
            label: label.to_string(),
            value,
            sub_label: None,
            color: color.to_string(),
        });
    }

    pub fn add_with_detail(&mut self, label: &str, value: f64, detail: &str, color: &str) {
        self.items.push(BarItem {
            label: label.to_string(),
            value,
            sub_label: Some(detail.to_string()),
            color: color.to_string(),
        });
    }

    pub fn render(&self) -> String {
        let mut output = String::new();

        if !self.title.is_empty() {
            output.push_str(&format!("  {}\n\n", self.title.bold()));
        }

        if self.items.is_empty() {
            output.push_str("  No data available\n");
            return output;
        }

        let total: f64 = self.items.iter().map(|i| i.value).sum();
        let max_label_len = self.items.iter().map(|i| i.label.len()).max().unwrap_or(10);

        // Two-column layout for many items
        let use_columns = self.items.len() > 4;
        let bar_width = if use_columns { 20 } else { self.width };

        if use_columns {
            // Two-column grid layout
            let items_per_col = (self.items.len() + 1) / 2;

            for row in 0..items_per_col {
                let left_idx = row;
                let right_idx = row + items_per_col;

                // Left column
                if left_idx < self.items.len() {
                    let item = &self.items[left_idx];
                    output.push_str(&format_bar_item(item, total, bar_width, max_label_len));
                }

                // Right column
                if right_idx < self.items.len() {
                    output.push_str("  ");
                    let item = &self.items[right_idx];
                    output.push_str(&format_bar_item(item, total, bar_width, max_label_len));
                }

                output.push('\n');

                // Sub-labels
                if left_idx < self.items.len() {
                    if let Some(ref sub) = self.items[left_idx].sub_label {
                        output.push_str(&format!("    {}\n", sub.dimmed()));
                    }
                }
            }
        } else {
            // Single column layout
            for item in &self.items {
                let pct = if total > 0.0 {
                    item.value / total * 100.0
                } else {
                    0.0
                };

                let bullet = colorize_bullet(&item.color);
                let bar_filled = ((pct / 100.0) * bar_width as f64) as usize;
                let bar = format!(
                    "{}{}",
                    "█".repeat(bar_filled),
                    "░".repeat(bar_width - bar_filled)
                );

                let colored_bar = colorize_text(&bar, &item.color);

                output.push_str(&format!(
                    "  {} {:width$} ({:5.1}%)\n",
                    bullet,
                    item.label,
                    pct,
                    width = max_label_len
                ));
                output.push_str(&format!("    {}\n", colored_bar));

                if let Some(ref sub) = item.sub_label {
                    output.push_str(&format!("    {}\n", sub.dimmed()));
                }
            }
        }

        output
    }
}

fn format_bar_item(item: &BarItem, total: f64, _bar_width: usize, _max_label: usize) -> String {
    let pct = if total > 0.0 {
        item.value / total * 100.0
    } else {
        0.0
    };

    let bullet = colorize_bullet(&item.color);

    format!("  {} {} ({:.1}%)", bullet, item.label, pct)
}

fn colorize_bullet(color: &str) -> colored::ColoredString {
    match color {
        "cyan" => "●".cyan(),
        "magenta" => "●".magenta(),
        "yellow" => "●".yellow(),
        "green" => "●".green(),
        "blue" => "●".blue(),
        "red" => "●".red(),
        "white" => "●".white(),
        _ => "●".white(),
    }
}

fn colorize_text(text: &str, color: &str) -> colored::ColoredString {
    match color {
        "cyan" => text.cyan(),
        "magenta" => text.magenta(),
        "yellow" => text.yellow(),
        "green" => text.green(),
        "blue" => text.blue(),
        "red" => text.red(),
        _ => text.white(),
    }
}

/// Sparkline for compact trend visualization
pub struct Sparkline {
    values: Vec<f64>,
    width: usize,
}

impl Sparkline {
    pub fn new(values: &[f64]) -> Self {
        Self {
            values: values.to_vec(),
            width: values.len(),
        }
    }

    pub fn with_width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    pub fn render(&self) -> String {
        if self.values.is_empty() {
            return "─".repeat(self.width);
        }

        let min = self.values.iter().cloned().fold(f64::MAX, f64::min);
        let max = self.values.iter().cloned().fold(f64::MIN, f64::max);
        let range = (max - min).max(0.001);

        // Resample if needed
        let resampled = if self.values.len() != self.width {
            resample(&self.values, self.width)
        } else {
            self.values.clone()
        };

        resampled
            .iter()
            .map(|&v| {
                let normalized = ((v - min) / range).clamp(0.0, 1.0);
                let idx = (normalized * 7.0) as usize;
                SPARK_CHARS[idx.min(7)]
            })
            .collect()
    }

    pub fn render_colored(&self, color: &str) -> colored::ColoredString {
        let spark = self.render();
        colorize_text(&spark, color)
    }
}

fn resample(values: &[f64], target_len: usize) -> Vec<f64> {
    if values.is_empty() || target_len == 0 {
        return vec![];
    }

    let mut result = Vec::with_capacity(target_len);
    let ratio = values.len() as f64 / target_len as f64;

    for i in 0..target_len {
        let src_idx = (i as f64 * ratio) as usize;
        result.push(values[src_idx.min(values.len() - 1)]);
    }

    result
}

/// Statistics panel with key metrics
pub struct StatsPanel {
    pub title: String,
    pub metrics: Vec<(String, String, Option<String>)>, // (label, value, trend)
}

impl StatsPanel {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            metrics: Vec::new(),
        }
    }

    pub fn add(&mut self, label: &str, value: &str) {
        self.metrics.push((label.to_string(), value.to_string(), None));
    }

    pub fn add_with_trend(&mut self, label: &str, value: &str, trend: &str) {
        self.metrics.push((
            label.to_string(),
            value.to_string(),
            Some(trend.to_string()),
        ));
    }

    pub fn render(&self) -> String {
        let mut output = String::new();

        // Box drawing
        let max_label = self
            .metrics
            .iter()
            .map(|(l, _, _)| l.len())
            .max()
            .unwrap_or(10);
        let max_value = self
            .metrics
            .iter()
            .map(|(_, v, _)| v.len())
            .max()
            .unwrap_or(10);
        let inner_width = max_label + max_value + 5;

        // Title
        output.push_str(&format!("  {}\n", self.title.bold()));

        // Top border
        output.push_str(&format!(
            "  {}{}{}",
            BOX_TL,
            BOX_H.to_string().repeat(inner_width),
            BOX_TR
        ));
        output.push('\n');

        // Metrics
        for (label, value, trend) in &self.metrics {
            let trend_str = trend
                .as_ref()
                .map(|t| {
                    if t.starts_with('+') || t.starts_with('↑') {
                        format!(" {}", t.green())
                    } else if t.starts_with('-') || t.starts_with('↓') {
                        format!(" {}", t.red())
                    } else {
                        format!(" {}", t.dimmed())
                    }
                })
                .unwrap_or_default();

            output.push_str(&format!(
                "  {} {:width$} : {}{}\n",
                BOX_V,
                label,
                value.cyan(),
                trend_str,
                width = max_label
            ));
        }

        // Bottom border
        output.push_str(&format!(
            "  {}{}{}",
            BOX_BL,
            BOX_H.to_string().repeat(inner_width),
            BOX_BR
        ));
        output.push('\n');

        output
    }
}

/// Tool usage breakdown like the /model output
pub struct ToolBreakdown {
    pub tools: Vec<ToolUsage>,
}

#[derive(Clone, Debug)]
pub struct ToolUsage {
    pub name: String,
    pub percentage: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub color: String,
}

impl ToolBreakdown {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn add(
        &mut self,
        name: &str,
        percentage: f64,
        input_tokens: u64,
        output_tokens: u64,
        color: &str,
    ) {
        self.tools.push(ToolUsage {
            name: name.to_string(),
            percentage,
            input_tokens,
            output_tokens,
            color: color.to_string(),
        });
    }

    pub fn render(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("  {}\n\n", "All tools".bold()));

        if self.tools.is_empty() {
            output.push_str("  No tool data available\n");
            return output;
        }

        // Two-column layout
        let items_per_col = (self.tools.len() + 1) / 2;

        for row in 0..items_per_col {
            let left_idx = row;
            let right_idx = row + items_per_col;

            // Left column
            if left_idx < self.tools.len() {
                let tool = &self.tools[left_idx];
                output.push_str(&self.format_tool_entry(tool));
            }

            // Right column
            if right_idx < self.tools.len() {
                output.push_str("    ");
                let tool = &self.tools[right_idx];
                output.push_str(&self.format_tool_entry(tool));
            }

            output.push('\n');

            // Token details for left column
            if left_idx < self.tools.len() {
                let tool = &self.tools[left_idx];
                output.push_str(&format!(
                    "    In: {} · Out: {}",
                    format_tokens(tool.input_tokens).dimmed(),
                    format_tokens(tool.output_tokens).dimmed()
                ));
            }

            // Token details for right column
            if right_idx < self.tools.len() {
                let tool = &self.tools[right_idx];
                output.push_str(&format!(
                    "    In: {} · Out: {}",
                    format_tokens(tool.input_tokens).dimmed(),
                    format_tokens(tool.output_tokens).dimmed()
                ));
            }

            output.push('\n');
        }

        output
    }

    fn format_tool_entry(&self, tool: &ToolUsage) -> String {
        let bullet = colorize_bullet(&tool.color);
        format!("{} {} ({:.1}%)", bullet, tool.name, tool.percentage)
    }
}

impl Default for ToolBreakdown {
    fn default() -> Self {
        Self::new()
    }
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}m", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        format!("{}", tokens)
    }
}

/// Activity Heatmap - GitHub-style yearly contribution graph
/// Shows month x day-of-week activity matrix
pub struct ActivityHeatmap {
    /// Data indexed by (week_number, day_of_week) -> intensity value
    pub data: HashMap<(u32, u32), f64>,
    /// Number of weeks to show (default 52)
    pub weeks: u32,
}

/// Intensity characters for heatmap (5 levels)
const HEAT_CHARS: [char; 5] = ['·', '░', '▒', '▓', '█'];

impl ActivityHeatmap {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            weeks: 52,
        }
    }

    pub fn with_weeks(mut self, weeks: u32) -> Self {
        self.weeks = weeks;
        self
    }

    pub fn set(&mut self, week: u32, day: u32, value: f64) {
        self.data.insert((week, day), value);
    }

    /// Set from a date and value
    pub fn set_date(&mut self, date: DateTime<Utc>, value: f64) {
        let week = date.iso_week().week() as u32;
        let day = date.weekday().num_days_from_monday();
        self.data.insert((week, day), value);
    }

    pub fn render(&self) -> String {
        let mut output = String::new();

        // Month headers
        output.push_str("      ");
        let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec", "Jan"];
        let weeks_per_month = self.weeks / 12;
        for month in &months[..13] {
            output.push_str(month);
            if weeks_per_month > 3 {
                output.push_str(&" ".repeat(weeks_per_month as usize - 3));
            } else {
                output.push(' ');
            }
        }
        output.push('\n');

        let days = ["", "Mon", "", "Wed", "", "Fri", ""];
        let max_val = self.data.values().cloned().fold(0.0f64, f64::max);

        // Each day row
        for day_idx in 0..7u32 {
            output.push_str(&format!("  {:3} ", days[day_idx as usize]));

            for week in 0..self.weeks {
                let val = self.data.get(&(week, day_idx)).unwrap_or(&0.0);
                let intensity = if max_val > 0.0 {
                    (val / max_val * 4.0) as usize
                } else {
                    0
                };
                let ch = HEAT_CHARS[intensity.min(4)];

                // Color by intensity
                let colored_ch = if intensity >= 4 {
                    format!("{}", ch).green().bold()
                } else if intensity >= 3 {
                    format!("{}", ch).green()
                } else if intensity >= 2 {
                    format!("{}", ch).yellow()
                } else if intensity >= 1 {
                    format!("{}", ch).cyan()
                } else {
                    format!("{}", ch).dimmed()
                };

                output.push_str(&format!("{}", colored_ch));
            }

            output.push('\n');
        }

        // Legend
        output.push('\n');
        output.push_str("      Less ");
        for ch in HEAT_CHARS {
            output.push(ch);
            output.push(' ');
        }
        output.push_str("More\n");

        output
    }
}

impl Default for ActivityHeatmap {
    fn default() -> Self {
        Self::new()
    }
}

/// Stats Card - Key metrics in a beautiful card format
pub struct StatsCard {
    pub rows: Vec<Vec<(String, String)>>, // Rows of (label, value) pairs
}

impl StatsCard {
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// Add a row of metric pairs (displayed horizontally)
    pub fn add_row(&mut self, pairs: Vec<(&str, &str)>) {
        self.rows.push(
            pairs
                .into_iter()
                .map(|(l, v)| (l.to_string(), v.to_string()))
                .collect(),
        );
    }

    pub fn render(&self) -> String {
        let mut output = String::new();

        for row in &self.rows {
            output.push_str("  ");
            for (idx, (label, value)) in row.iter().enumerate() {
                if idx > 0 {
                    output.push_str("    ");
                }
                output.push_str(&format!("{}: ", label));
                output.push_str(&format!("{}", value.cyan()));
            }
            output.push('\n');
        }

        output
    }
}

impl Default for StatsCard {
    fn default() -> Self {
        Self::new()
    }
}

/// Fun Fact - Whimsical comparison stats
pub struct FunFact {
    pub fact: String,
    pub source_period: String,
}

impl FunFact {
    pub fn new(fact: &str, period: &str) -> Self {
        Self {
            fact: fact.to_string(),
            source_period: period.to_string(),
        }
    }

    /// Generate fun token comparison
    pub fn token_comparison(tokens: u64) -> Self {
        let comparisons = [
            (77_000, "a short novel"),
            (350_000, "Harry Potter and the Philosopher's Stone"),
            (580_000, "The Great Gatsby"),
            (850_000, "Anna Karenina"),
            (1_200_000, "War and Peace"),
            (4_000_000, "the entire Lord of the Rings trilogy"),
            (10_000_000, "all Harry Potter books combined"),
            (50_000_000, "Wikipedia's featured articles"),
        ];

        let mut best_match = ("a tweet", 280u64);
        let mut multiplier = tokens as f64 / 280.0;

        for (book_tokens, name) in comparisons {
            let m = tokens as f64 / book_tokens as f64;
            if m >= 1.0 && m < multiplier {
                multiplier = m;
                best_match = (name, book_tokens);
            }
        }

        let fact = if multiplier >= 100.0 {
            format!("You've used ~{}x more tokens than {}", multiplier as u64, best_match.0)
        } else if multiplier >= 10.0 {
            format!("You've used ~{:.0}x more tokens than {}", multiplier, best_match.0)
        } else if multiplier >= 1.0 {
            format!("You've used ~{:.1}x the tokens of {}", multiplier, best_match.0)
        } else {
            format!("You've used {} tokens (keep going!)", format_tokens(tokens))
        };

        Self {
            fact,
            source_period: String::new(),
        }
    }

    pub fn render(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("\n  {}\n", self.fact.italic()));
        if !self.source_period.is_empty() {
            output.push_str(&format!("  {}\n", self.source_period.dimmed()));
        }
        output
    }
}

/// Streak Counter - Visual streak display with flames
pub struct StreakCounter {
    pub current: u32,
    pub longest: u32,
    pub active_days: u32,
    pub total_days: u32,
    pub peak_hour: String,
}

impl StreakCounter {
    pub fn new(current: u32, longest: u32) -> Self {
        Self {
            current,
            longest,
            active_days: 0,
            total_days: 0,
            peak_hour: String::new(),
        }
    }

    pub fn with_activity(mut self, active: u32, total: u32) -> Self {
        self.active_days = active;
        self.total_days = total;
        self
    }

    pub fn with_peak(mut self, peak: &str) -> Self {
        self.peak_hour = peak.to_string();
        self
    }

    pub fn render(&self) -> String {
        let mut output = String::new();

        // Streak flames visualization
        let flames = if self.current >= 30 {
            "🔥🔥🔥🔥🔥".to_string()
        } else if self.current >= 14 {
            "🔥🔥🔥🔥".to_string()
        } else if self.current >= 7 {
            "🔥🔥🔥".to_string()
        } else if self.current >= 3 {
            "🔥🔥".to_string()
        } else if self.current >= 1 {
            "🔥".to_string()
        } else {
            "".to_string()
        };

        output.push_str(&format!(
            "  Current streak: {} days {}\n",
            format!("{}", self.current).cyan().bold(),
            flames
        ));
        output.push_str(&format!(
            "  Longest streak: {} days\n",
            format!("{}", self.longest).green()
        ));

        if self.total_days > 0 {
            let pct = (self.active_days as f64 / self.total_days as f64 * 100.0) as u32;
            output.push_str(&format!(
                "  Active days: {}/{} ({}%)\n",
                self.active_days, self.total_days, pct
            ));
        }

        if !self.peak_hour.is_empty() {
            output.push_str(&format!("  Peak hour: {}\n", self.peak_hour.yellow()));
        }

        output
    }
}

/// Histogram - Distribution visualization
pub struct Histogram {
    pub title: String,
    pub buckets: Vec<(String, u64)>,
    pub width: usize,
}

impl Histogram {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            buckets: Vec::new(),
            width: 40,
        }
    }

    pub fn add(&mut self, label: &str, count: u64) {
        self.buckets.push((label.to_string(), count));
    }

    /// Create histogram from values with auto-bucketing
    pub fn from_values(title: &str, values: &[f64], num_buckets: usize) -> Self {
        let mut hist = Self::new(title);

        if values.is_empty() {
            return hist;
        }

        let min = values.iter().cloned().fold(f64::MAX, f64::min);
        let max = values.iter().cloned().fold(f64::MIN, f64::max);
        let range = (max - min).max(0.001);
        let bucket_size = range / num_buckets as f64;

        let mut counts = vec![0u64; num_buckets];

        for &val in values {
            let bucket = ((val - min) / bucket_size) as usize;
            let bucket = bucket.min(num_buckets - 1);
            counts[bucket] += 1;
        }

        for (i, &count) in counts.iter().enumerate() {
            let low = min + i as f64 * bucket_size;
            let high = low + bucket_size;
            let label = if bucket_size >= 1.0 {
                format!("{:.0}-{:.0}", low, high)
            } else {
                format!("{:.1}-{:.1}", low, high)
            };
            hist.add(&label, count);
        }

        hist
    }

    pub fn render(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("  {}\n\n", self.title.bold()));

        if self.buckets.is_empty() {
            output.push_str("  No data\n");
            return output;
        }

        let max_count = self.buckets.iter().map(|(_, c)| *c).max().unwrap_or(1);
        let max_label_len = self.buckets.iter().map(|(l, _)| l.len()).max().unwrap_or(5);

        for (label, count) in &self.buckets {
            let bar_len = if max_count > 0 {
                (*count as f64 / max_count as f64 * self.width as f64) as usize
            } else {
                0
            };

            let bar = "█".repeat(bar_len);
            let colored_bar = if bar_len > self.width * 3 / 4 {
                bar.green()
            } else if bar_len > self.width / 2 {
                bar.cyan()
            } else if bar_len > self.width / 4 {
                bar.yellow()
            } else {
                bar.white()
            };

            output.push_str(&format!(
                "  {:>width$} │{} {}\n",
                label,
                colored_bar,
                count,
                width = max_label_len
            ));
        }

        output
    }
}

/// Progress Bar - Goal tracking visualization
pub struct ProgressBar {
    pub label: String,
    pub current: f64,
    pub target: f64,
    pub width: usize,
    pub show_percentage: bool,
}

impl ProgressBar {
    pub fn new(label: &str, current: f64, target: f64) -> Self {
        Self {
            label: label.to_string(),
            current,
            target,
            width: 30,
            show_percentage: true,
        }
    }

    pub fn render(&self) -> String {
        let pct = (self.current / self.target).min(1.0);
        let filled = (pct * self.width as f64) as usize;
        let empty = self.width - filled;

        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
        let colored_bar = if pct >= 1.0 {
            bar.green().bold()
        } else if pct >= 0.75 {
            bar.green()
        } else if pct >= 0.5 {
            bar.yellow()
        } else if pct >= 0.25 {
            bar.cyan()
        } else {
            bar.red()
        };

        let status = if pct >= 1.0 { "✓" } else { " " };

        if self.show_percentage {
            format!(
                "  {} {} {} ({:.0}%)\n",
                status,
                self.label,
                colored_bar,
                pct * 100.0
            )
        } else {
            format!(
                "  {} {} {} {}/{}\n",
                status,
                self.label,
                colored_bar,
                self.current as u64,
                self.target as u64
            )
        }
    }
}

/// Leaderboard - Ranked list with comparison bars
pub struct Leaderboard {
    pub title: String,
    pub entries: Vec<(String, f64, Option<String>)>, // (name, value, badge)
}

impl Leaderboard {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, name: &str, value: f64) {
        self.entries.push((name.to_string(), value, None));
    }

    pub fn add_with_badge(&mut self, name: &str, value: f64, badge: &str) {
        self.entries
            .push((name.to_string(), value, Some(badge.to_string())));
    }

    pub fn render(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("  {}\n\n", self.title.bold()));

        if self.entries.is_empty() {
            output.push_str("  No entries\n");
            return output;
        }

        // Sort by value descending
        let mut sorted: Vec<_> = self.entries.clone();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let max_val = sorted.first().map(|(_, v, _)| *v).unwrap_or(1.0);
        let max_name_len = sorted.iter().map(|(n, _, _)| n.len()).max().unwrap_or(10);

        for (rank, (name, value, badge)) in sorted.iter().enumerate() {
            let rank_str = match rank {
                0 => "🥇".to_string(),
                1 => "🥈".to_string(),
                2 => "🥉".to_string(),
                _ => format!("{:2}.", rank + 1),
            };

            let bar_width = 20;
            let bar_len = (value / max_val * bar_width as f64) as usize;
            let bar = "▓".repeat(bar_len);
            let colored_bar = match rank {
                0 => bar.yellow(),
                1 => bar.white(),
                2 => bar.red(),
                _ => bar.dimmed(),
            };

            let badge_str = badge
                .as_ref()
                .map(|b| format!(" {}", b))
                .unwrap_or_default();

            output.push_str(&format!(
                "  {} {:width$} {} {}{}\n",
                rank_str,
                name,
                colored_bar,
                format_number(*value),
                badge_str,
                width = max_name_len
            ));
        }

        output
    }
}

/// Calendar View - Monthly calendar with activity markers
pub struct CalendarView {
    pub year: i32,
    pub month: u32,
    pub data: HashMap<u32, f64>, // day -> activity level
}

impl CalendarView {
    pub fn new(year: i32, month: u32) -> Self {
        Self {
            year,
            month,
            data: HashMap::new(),
        }
    }

    pub fn set(&mut self, day: u32, value: f64) {
        self.data.insert(day, value);
    }

    pub fn render(&self) -> String {
        let mut output = String::new();

        let month_names = [
            "", "January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December",
        ];

        output.push_str(&format!(
            "  {} {}\n",
            month_names[self.month as usize].bold(),
            self.year
        ));
        output.push_str("  Su Mo Tu We Th Fr Sa\n");

        // Get first day of month and days in month
        let first_day = chrono::NaiveDate::from_ymd_opt(self.year, self.month, 1);
        if first_day.is_none() {
            return output;
        }
        let first_day = first_day.unwrap();
        let weekday = first_day.weekday().num_days_from_sunday();

        let days_in_month = match self.month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if self.year % 4 == 0 && (self.year % 100 != 0 || self.year % 400 == 0) {
                    29
                } else {
                    28
                }
            }
            _ => 30,
        };

        let max_val = self.data.values().cloned().fold(0.0f64, f64::max);

        // Print leading spaces
        output.push_str("  ");
        for _ in 0..weekday {
            output.push_str("   ");
        }

        let mut current_weekday = weekday;
        for day in 1..=days_in_month {
            let val = self.data.get(&day).unwrap_or(&0.0);
            let intensity = if max_val > 0.0 { val / max_val } else { 0.0 };

            let day_str = format!("{:2}", day);
            let colored_day = if intensity > 0.8 {
                day_str.green().bold()
            } else if intensity > 0.5 {
                day_str.green()
            } else if intensity > 0.2 {
                day_str.yellow()
            } else if intensity > 0.0 {
                day_str.cyan()
            } else {
                day_str.dimmed()
            };

            output.push_str(&format!("{} ", colored_day));

            current_weekday += 1;
            if current_weekday >= 7 {
                output.push('\n');
                output.push_str("  ");
                current_weekday = 0;
            }
        }

        if current_weekday != 0 {
            output.push('\n');
        }

        output
    }
}

/// Time Distribution - Radial-style time breakdown
pub struct TimeDistribution {
    pub title: String,
    pub hours: [f64; 24],
}

impl TimeDistribution {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            hours: [0.0; 24],
        }
    }

    pub fn set(&mut self, hour: usize, value: f64) {
        if hour < 24 {
            self.hours[hour] = value;
        }
    }

    pub fn render(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("  {}\n", self.title.bold()));

        let max_val = self.hours.iter().cloned().fold(0.0f64, f64::max);

        // Morning row (6-12)
        output.push_str("  Morning  ");
        for hour in 6..12 {
            let intensity = if max_val > 0.0 {
                (self.hours[hour] / max_val * 4.0) as usize
            } else {
                0
            };
            let ch = HEAT_CHARS[intensity.min(4)];
            output.push_str(&format!("{}", colorize_heat(ch, intensity)));
        }
        output.push('\n');

        // Afternoon row (12-18)
        output.push_str("  Afternoon");
        for hour in 12..18 {
            let intensity = if max_val > 0.0 {
                (self.hours[hour] / max_val * 4.0) as usize
            } else {
                0
            };
            let ch = HEAT_CHARS[intensity.min(4)];
            output.push_str(&format!("{}", colorize_heat(ch, intensity)));
        }
        output.push('\n');

        // Evening row (18-24)
        output.push_str("  Evening  ");
        for hour in 18..24 {
            let intensity = if max_val > 0.0 {
                (self.hours[hour] / max_val * 4.0) as usize
            } else {
                0
            };
            let ch = HEAT_CHARS[intensity.min(4)];
            output.push_str(&format!("{}", colorize_heat(ch, intensity)));
        }
        output.push('\n');

        // Night row (0-6)
        output.push_str("  Night    ");
        for hour in 0..6 {
            let intensity = if max_val > 0.0 {
                (self.hours[hour] / max_val * 4.0) as usize
            } else {
                0
            };
            let ch = HEAT_CHARS[intensity.min(4)];
            output.push_str(&format!("{}", colorize_heat(ch, intensity)));
        }
        output.push('\n');

        output
    }
}

fn colorize_heat(ch: char, intensity: usize) -> colored::ColoredString {
    let s = format!("{}", ch);
    match intensity {
        4 => s.green().bold(),
        3 => s.green(),
        2 => s.yellow(),
        1 => s.cyan(),
        _ => s.dimmed(),
    }
}

/// Comparison Chart - Side by side comparison of two values
pub struct ComparisonChart {
    pub left_label: String,
    pub left_value: f64,
    pub right_label: String,
    pub right_value: f64,
    pub unit: String,
}

impl ComparisonChart {
    pub fn new(left: &str, left_val: f64, right: &str, right_val: f64) -> Self {
        Self {
            left_label: left.to_string(),
            left_value: left_val,
            right_label: right.to_string(),
            right_value: right_val,
            unit: String::new(),
        }
    }

    pub fn with_unit(mut self, unit: &str) -> Self {
        self.unit = unit.to_string();
        self
    }

    pub fn render(&self) -> String {
        let mut output = String::new();

        let total = self.left_value + self.right_value;
        let left_pct = if total > 0.0 {
            self.left_value / total
        } else {
            0.5
        };
        let right_pct = 1.0 - left_pct;

        let bar_width = 30;
        let left_bar = (left_pct * bar_width as f64) as usize;
        let right_bar = bar_width - left_bar;

        let left_bar_str = "█".repeat(left_bar);
        let right_bar_str = "█".repeat(right_bar);

        output.push_str(&format!(
            "  {} vs {}\n",
            self.left_label.cyan(),
            self.right_label.magenta()
        ));

        output.push_str(&format!(
            "  {}{}\n",
            left_bar_str.cyan(),
            right_bar_str.magenta()
        ));

        output.push_str(&format!(
            "  {}{} ({:.0}%)    {}{} ({:.0}%)\n",
            format_number(self.left_value),
            if self.unit.is_empty() {
                "".to_string()
            } else {
                format!(" {}", self.unit)
            },
            left_pct * 100.0,
            format_number(self.right_value),
            if self.unit.is_empty() {
                "".to_string()
            } else {
                format!(" {}", self.unit)
            },
            right_pct * 100.0,
        ));

        output
    }
}

/// Trend Arrow - Shows trend direction with arrow
pub fn trend_arrow(current: f64, previous: f64) -> colored::ColoredString {
    let pct_change = if previous != 0.0 {
        (current - previous) / previous * 100.0
    } else if current > 0.0 {
        100.0
    } else {
        0.0
    };

    if pct_change > 10.0 {
        format!("↑ +{:.0}%", pct_change).green().bold()
    } else if pct_change > 0.0 {
        format!("↑ +{:.0}%", pct_change).green()
    } else if pct_change < -10.0 {
        format!("↓ {:.0}%", pct_change).red().bold()
    } else if pct_change < 0.0 {
        format!("↓ {:.0}%", pct_change).red()
    } else {
        "→ 0%".dimmed()
    }
}

/// Format duration in human-friendly format
pub fn format_duration_human(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(500.0), "500");
        assert_eq!(format_number(1500.0), "2k");
        assert_eq!(format_number(1_500_000.0), "1.5M");
        assert_eq!(format_number(1_500_000_000.0), "1.5B");
    }

    #[test]
    fn test_sparkline() {
        let spark = Sparkline::new(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let rendered = spark.render();
        assert_eq!(rendered.chars().count(), 5);
    }

    #[test]
    fn test_bar_chart() {
        let mut chart = BarChart::new("Test Chart");
        chart.add("Item 1", 50.0, "cyan");
        chart.add("Item 2", 30.0, "magenta");
        chart.add("Item 3", 20.0, "yellow");
        let rendered = chart.render();
        assert!(rendered.contains("Item 1"));
        assert!(rendered.contains("50.0%"));
    }
}
