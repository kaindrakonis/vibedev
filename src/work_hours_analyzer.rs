use anyhow::Result;
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};
use walkdir::WalkDir;

const SESSION_GAP_MINUTES: i64 = 30; // Gap > 30 min = new session
const SESSION_BUFFER_MINUTES: i64 = 5; // Add 5 min before/after each session

#[derive(Debug, Serialize)]
pub struct WorkHoursAnalysis {
    pub total_hours: f64,
    pub total_sessions: usize,
    pub average_session_hours: f64,
    pub longest_session_hours: f64,
    pub shortest_session_hours: f64,
    pub hours_by_day: HashMap<String, f64>,
    pub hours_by_week: HashMap<String, f64>,
    pub hours_by_month: HashMap<String, f64>,
    pub hours_by_weekday: HashMap<String, f64>,
    pub hours_by_hour_of_day: HashMap<usize, f64>,
    pub hours_by_project: HashMap<String, f64>,
    pub hours_by_tool: HashMap<String, f64>,
    pub sessions: Vec<SessionInfo>,
    pub daily_average: f64,
    pub weekly_average: f64,
    pub busiest_day: String,
    pub busiest_hour: usize,
    pub work_life_balance_score: f64, // 0-100, higher = better balance
}

#[derive(Debug, Serialize, Clone)]
pub struct SessionInfo {
    pub start_time: String,
    pub end_time: String,
    pub duration_hours: f64,
    pub message_count: usize,
    pub tool: String,
    pub project: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClineMessage {
    role: String,
    content: serde_json::Value,
    #[serde(default)]
    ts: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ClaudeMessage {
    #[serde(default)]
    r#type: String,
    message: Option<MessageContent>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    role: String,
    content: serde_json::Value,
}

pub struct WorkHoursAnalyzer {
    base_dir: PathBuf,
}

impl WorkHoursAnalyzer {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn analyze(&self) -> Result<WorkHoursAnalysis> {
        info!("⏱️  Analyzing work hours from timestamps...");

        let mut all_sessions = Vec::new();

        // Parse Cline/Roo-Cline/Kilo sessions
        self.parse_cline_sessions(&mut all_sessions)?;

        // Parse Claude Code sessions
        self.parse_claude_sessions(&mut all_sessions)?;

        // Sort sessions by start time
        all_sessions.sort_by(|a, b| a.start_time.cmp(&b.start_time));

        // Calculate aggregate stats
        let stats = self.calculate_stats(&all_sessions)?;

        Ok(stats)
    }

    fn parse_cline_sessions(&self, all_sessions: &mut Vec<SessionInfo>) -> Result<()> {
        let patterns = vec![
            ("Cline", ".config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks"),
            ("Cline (Flatpak)", ".var/app/com.visualstudio.code/config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks"),
            ("Kilo", ".config/Code/User/globalStorage/kilocode.kilo-code/tasks"),
            ("Roo-Cline", ".config/Code/User/globalStorage/rooveterinaryinc.roo-cline/tasks"),
            ("Roo-Cline (Cursor)", ".config/Cursor/User/globalStorage/rooveterinaryinc.roo-cline/tasks"),
        ];

        for (tool_name, pattern) in patterns {
            let path = self.base_dir.join(pattern);
            if !path.exists() {
                continue;
            }

            for entry in WalkDir::new(&path)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_dir() {
                    continue;
                }

                let api_history = entry.path().join("api_conversation_history.json");
                if !api_history.exists() {
                    continue;
                }

                if let Ok(file_content) = fs::read_to_string(&api_history) {
                    if let Ok(messages) = serde_json::from_str::<Vec<ClineMessage>>(&file_content) {
                        // Count messages by role for session analysis
                        let user_msgs = messages.iter().filter(|m| m.role == "user").count();
                        let assistant_msgs =
                            messages.iter().filter(|m| m.role == "assistant").count();
                        let total_chars: usize =
                            messages.iter().map(|m| m.content.to_string().len()).sum();

                        debug!(
                            "Session has {} user msgs, {} assistant msgs, {} chars",
                            user_msgs, assistant_msgs, total_chars
                        );

                        // Extract timestamps and group into sessions
                        let mut timestamps: Vec<i64> =
                            messages.iter().filter_map(|m| m.ts).collect();

                        if timestamps.is_empty() {
                            continue;
                        }

                        timestamps.sort();

                        // Group into sessions based on gaps
                        let sessions = self.group_into_sessions(&timestamps, tool_name, None);
                        all_sessions.extend(sessions);
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_claude_sessions(&self, all_sessions: &mut Vec<SessionInfo>) -> Result<()> {
        let projects_dir = self.base_dir.join(".claude/projects");
        if !projects_dir.exists() {
            return Ok(());
        }

        for entry in WalkDir::new(&projects_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext != "jsonl" {
                    continue;
                }
            } else {
                continue;
            }

            if let Ok(file_content) = fs::read_to_string(path) {
                let mut timestamps: Vec<(i64, Option<String>)> = Vec::new();
                let mut user_msgs = 0;
                let mut assistant_msgs = 0;
                let mut total_content_size = 0usize;

                for line in file_content.lines() {
                    if let Ok(msg) = serde_json::from_str::<ClaudeMessage>(line) {
                        // Track message types for analysis
                        if msg.r#type == "user" || msg.r#type == "human" {
                            user_msgs += 1;
                        } else if msg.r#type == "assistant" {
                            assistant_msgs += 1;
                        }

                        // Also check nested message content for role and size
                        if let Some(ref content) = msg.message {
                            if content.role == "user" {
                                user_msgs += 1;
                            } else if content.role == "assistant" {
                                assistant_msgs += 1;
                            }
                            // Track content size for token estimation
                            total_content_size += content.content.to_string().len();
                        }

                        if let Some(ts_str) = msg.timestamp {
                            // Parse ISO 8601 timestamp
                            if let Ok(dt) = DateTime::parse_from_rfc3339(&ts_str) {
                                let ts_millis = dt.timestamp_millis();
                                timestamps.push((ts_millis, msg.cwd.clone()));
                            }
                        }
                    }
                }

                debug!(
                    "Claude session: {} user msgs, {} assistant msgs, {} chars",
                    user_msgs, assistant_msgs, total_content_size
                );

                if timestamps.is_empty() {
                    continue;
                }

                timestamps.sort_by_key(|t| t.0);

                // Extract just the timestamps for session grouping
                let ts_only: Vec<i64> = timestamps.iter().map(|t| t.0).collect();

                // Get project from first message with cwd
                let project = timestamps.iter().find_map(|t| t.1.clone());

                let sessions = self.group_into_sessions(&ts_only, "Claude Code", project);
                all_sessions.extend(sessions);
            }
        }

        Ok(())
    }

    fn group_into_sessions(
        &self,
        timestamps: &[i64],
        tool: &str,
        project: Option<String>,
    ) -> Vec<SessionInfo> {
        let mut sessions = Vec::new();

        if timestamps.is_empty() {
            return sessions;
        }

        let mut session_start = timestamps[0];
        let mut session_end = timestamps[0];
        let mut message_count = 1;

        for &current in timestamps.iter().skip(1) {
            let gap_minutes = (current - session_end) / (1000 * 60);

            if gap_minutes > SESSION_GAP_MINUTES {
                // End current session, start new one
                let session = self.create_session(
                    session_start,
                    session_end,
                    message_count,
                    tool,
                    project.clone(),
                );
                sessions.push(session);

                session_start = current;
                session_end = current;
                message_count = 1;
            } else {
                // Continue current session
                session_end = current;
                message_count += 1;
            }
        }

        // Add final session
        let session = self.create_session(session_start, session_end, message_count, tool, project);
        sessions.push(session);

        sessions
    }

    fn create_session(
        &self,
        start_ts: i64,
        end_ts: i64,
        message_count: usize,
        tool: &str,
        project: Option<String>,
    ) -> SessionInfo {
        // Add buffer time
        let start_with_buffer = start_ts - (SESSION_BUFFER_MINUTES * 60 * 1000);
        let end_with_buffer = end_ts + (SESSION_BUFFER_MINUTES * 60 * 1000);

        let duration_ms = end_with_buffer - start_with_buffer;
        let duration_hours = duration_ms as f64 / (1000.0 * 60.0 * 60.0);

        let start_time = DateTime::<Utc>::from_timestamp(start_ts / 1000, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| "Unknown".to_string());

        let end_time = DateTime::<Utc>::from_timestamp(end_ts / 1000, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| "Unknown".to_string());

        SessionInfo {
            start_time,
            end_time,
            duration_hours,
            message_count,
            tool: tool.to_string(),
            project,
        }
    }

    fn calculate_stats(&self, sessions: &[SessionInfo]) -> Result<WorkHoursAnalysis> {
        let total_hours: f64 = sessions.iter().map(|s| s.duration_hours).sum();
        let total_sessions = sessions.len();
        let average_session_hours = if total_sessions > 0 {
            total_hours / total_sessions as f64
        } else {
            0.0
        };

        let longest_session_hours = sessions
            .iter()
            .map(|s| s.duration_hours)
            .max_by(|a, b| a.total_cmp(b))
            .unwrap_or(0.0);

        let shortest_session_hours = sessions
            .iter()
            .map(|s| s.duration_hours)
            .min_by(|a, b| a.total_cmp(b))
            .unwrap_or(0.0);

        // Hours by day/week/month
        let mut hours_by_day: HashMap<String, f64> = HashMap::new();
        let mut hours_by_week: HashMap<String, f64> = HashMap::new();
        let mut hours_by_month: HashMap<String, f64> = HashMap::new();
        let mut hours_by_weekday: HashMap<String, f64> = HashMap::new();
        let mut hours_by_hour_of_day: HashMap<usize, f64> = HashMap::new();
        let mut hours_by_project: HashMap<String, f64> = HashMap::new();
        let mut hours_by_tool: HashMap<String, f64> = HashMap::new();

        for session in sessions {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&session.start_time) {
                let dt_utc = dt.with_timezone(&Utc);

                // By day
                let day_key = format!("{}", dt_utc.format("%Y-%m-%d"));
                *hours_by_day.entry(day_key).or_insert(0.0) += session.duration_hours;

                // By week
                let week_key = format!("{}-W{:02}", dt_utc.year(), dt_utc.iso_week().week());
                *hours_by_week.entry(week_key).or_insert(0.0) += session.duration_hours;

                // By month
                let month_key = format!("{}", dt_utc.format("%Y-%m"));
                *hours_by_month.entry(month_key).or_insert(0.0) += session.duration_hours;

                // By weekday
                let weekday = dt_utc.weekday().to_string();
                *hours_by_weekday.entry(weekday).or_insert(0.0) += session.duration_hours;

                // By hour of day
                let hour = dt_utc.hour() as usize;
                *hours_by_hour_of_day.entry(hour).or_insert(0.0) += session.duration_hours;
            }

            // By project
            if let Some(ref project) = session.project {
                let project_name = project.rsplit('/').next().unwrap_or(project);
                *hours_by_project
                    .entry(project_name.to_string())
                    .or_insert(0.0) += session.duration_hours;
            }

            // By tool
            *hours_by_tool.entry(session.tool.clone()).or_insert(0.0) += session.duration_hours;
        }

        // Calculate averages
        let num_days = hours_by_day.len() as f64;
        let num_weeks = hours_by_week.len() as f64;
        let daily_average = if num_days > 0.0 {
            total_hours / num_days
        } else {
            0.0
        };
        let weekly_average = if num_weeks > 0.0 {
            total_hours / num_weeks
        } else {
            0.0
        };

        // Find busiest day and hour
        let busiest_day = hours_by_weekday
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        let busiest_hour = hours_by_hour_of_day
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(k, _)| *k)
            .unwrap_or(12);

        // Work-life balance score (0-100)
        // Based on: reasonable daily hours, weekend work, late night work
        let work_life_balance_score = self.calculate_work_life_balance(
            daily_average,
            &hours_by_weekday,
            &hours_by_hour_of_day,
        );

        Ok(WorkHoursAnalysis {
            total_hours,
            total_sessions,
            average_session_hours,
            longest_session_hours,
            shortest_session_hours,
            hours_by_day,
            hours_by_week,
            hours_by_month,
            hours_by_weekday,
            hours_by_hour_of_day,
            hours_by_project,
            hours_by_tool,
            sessions: sessions.to_vec(),
            daily_average,
            weekly_average,
            busiest_day,
            busiest_hour,
            work_life_balance_score,
        })
    }

    fn calculate_work_life_balance(
        &self,
        daily_avg: f64,
        by_weekday: &HashMap<String, f64>,
        by_hour: &HashMap<usize, f64>,
    ) -> f64 {
        let mut score = 100.0;

        // Penalty for working too many hours per day
        if daily_avg > 8.0 {
            score -= (daily_avg - 8.0) * 5.0; // -5 points per hour over 8
        }

        // Penalty for weekend work
        let weekend_hours: f64 =
            by_weekday.get("Saturday").unwrap_or(&0.0) + by_weekday.get("Sunday").unwrap_or(&0.0);
        let total_hours: f64 = by_weekday.values().sum();
        if total_hours > 0.0 {
            let weekend_pct = (weekend_hours / total_hours) * 100.0;
            if weekend_pct > 10.0 {
                score -= weekend_pct - 10.0; // Penalty for >10% weekend work
            }
        }

        // Penalty for late night work (22:00-05:00)
        let late_night_hours: f64 = (22..24)
            .chain(0..5)
            .map(|h| by_hour.get(&h).unwrap_or(&0.0))
            .sum();
        if total_hours > 0.0 {
            let late_night_pct = (late_night_hours / total_hours) * 100.0;
            if late_night_pct > 5.0 {
                score -= (late_night_pct - 5.0) * 2.0; // Penalty for >5% late night
            }
        }

        score.clamp(0.0, 100.0)
    }
}

// ASCII chart generation
pub fn generate_hours_chart(hours_by_hour: &HashMap<usize, f64>) -> String {
    let max_hours = hours_by_hour
        .values()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(&1.0);

    let mut chart = String::from("\n📊 Hours by Time of Day\n\n");

    for hour in 0..24 {
        let hours = hours_by_hour.get(&hour).unwrap_or(&0.0);
        let bar_length = (hours / max_hours * 50.0) as usize;
        let bar = "█".repeat(bar_length);

        chart.push_str(&format!("{:02}:00 │{:<50}│ {:.1}h\n", hour, bar, hours));
    }

    chart
}

pub fn generate_weekday_chart(hours_by_weekday: &HashMap<String, f64>) -> String {
    let weekdays = vec![
        ("Monday", "Mon"),
        ("Tuesday", "Tue"),
        ("Wednesday", "Wed"),
        ("Thursday", "Thu"),
        ("Friday", "Fri"),
        ("Saturday", "Sat"),
        ("Sunday", "Sun"),
    ];

    let max_hours = hours_by_weekday
        .values()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(&1.0);

    let mut chart = String::from("\n📅 Hours by Day of Week\n\n");

    for (full_name, short_name) in &weekdays {
        // Try both full and abbreviated names
        let hours = hours_by_weekday
            .get(&full_name.to_string())
            .or_else(|| hours_by_weekday.get(&short_name.to_string()))
            .unwrap_or(&0.0);
        let bar_length = (hours / max_hours * 50.0) as usize;
        let bar = "█".repeat(bar_length);

        chart.push_str(&format!("{:<9} │{:<50}│ {:.1}h\n", full_name, bar, hours));
    }

    chart
}

pub fn generate_tool_chart(hours_by_tool: &HashMap<String, f64>) -> String {
    let mut tools: Vec<_> = hours_by_tool.iter().collect();
    tools.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());

    let max_hours = tools.first().map(|(_, h)| **h).unwrap_or(1.0);

    let mut chart = String::from("\n🛠️  Hours by Tool\n\n");

    for (tool, hours) in tools {
        let bar_length = (hours / max_hours * 50.0) as usize;
        let bar = "█".repeat(bar_length);

        chart.push_str(&format!("{:<15} │{:<50}│ {:.1}h\n", tool, bar, hours));
    }

    chart
}
