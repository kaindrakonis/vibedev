// Cline log parser
use super::{EntryCategory, LogEntry, LogLevel, LogMetadata, LogParser, ParsedLog};
use crate::models::AiTool;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub struct ClineParser;

impl LogParser for ClineParser {
    fn can_parse(&self, path: &Path) -> bool {
        path.to_string_lossy().contains("cline")
    }

    fn parse(&self, path: &Path) -> Result<ParsedLog> {
        let mut entries = Vec::new();
        let mut oldest: Option<DateTime<Utc>> = None;
        let mut newest: Option<DateTime<Utc>> = None;

        // Cline typically stores logs in a logs directory
        let log_dir = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent().unwrap_or(path).to_path_buf()
        };

        // Look for common Cline log files
        let possible_files = vec![
            log_dir.join("cline.log"),
            log_dir.join("main.log"),
            log_dir.join("history.jsonl"),
        ];

        for log_file in possible_files {
            if log_file.exists() {
                if log_file.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                    // Parse JSONL format
                    if let Ok(file) = fs::File::open(&log_file) {
                        let reader = BufReader::new(file);
                        for line in reader.lines().map_while(Result::ok) {
                            if line.trim().is_empty() {
                                continue;
                            }
                            if let Ok(json) = serde_json::from_str::<Value>(&line) {
                                if let Some(entry) = parse_cline_json_entry(&json) {
                                    if let Some(ts) = entry.timestamp {
                                        oldest = Some(oldest.map_or(ts, |o| o.min(ts)));
                                        newest = Some(newest.map_or(ts, |n| n.max(ts)));
                                    }
                                    entries.push(entry);
                                }
                            }
                        }
                    }
                } else {
                    // Parse plain text log format
                    if let Ok(file) = fs::File::open(&log_file) {
                        let reader = BufReader::new(file);
                        for line in reader.lines().map_while(Result::ok) {
                            if let Some(entry) = parse_cline_text_entry(&line) {
                                if let Some(ts) = entry.timestamp {
                                    oldest = Some(oldest.map_or(ts, |o| o.min(ts)));
                                    newest = Some(newest.map_or(ts, |n| n.max(ts)));
                                }
                                entries.push(entry);
                            }
                        }
                    }
                }
            }
        }

        let file_size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let entry_count = entries.len();

        Ok(ParsedLog {
            tool: AiTool::Cline,
            entries,
            metadata: LogMetadata {
                file_size,
                entry_count,
                date_range: (oldest, newest),
            },
        })
    }
}

fn parse_cline_json_entry(json: &Value) -> Option<LogEntry> {
    let timestamp = json
        .get("timestamp")
        .or_else(|| json.get("ts"))
        .or_else(|| json.get("time"))
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let category = if json.get("type").and_then(|v| v.as_str()) == Some("user_message")
        || json.get("role").and_then(|v| v.as_str()) == Some("user")
    {
        EntryCategory::UserPrompt
    } else if json.get("type").and_then(|v| v.as_str()) == Some("assistant_message")
        || json.get("role").and_then(|v| v.as_str()) == Some("assistant")
    {
        EntryCategory::AssistantResponse
    } else if json.get("tool").is_some() || json.get("tool_call").is_some() {
        EntryCategory::ToolUse
    } else {
        EntryCategory::SystemEvent
    };

    let message = json
        .get("message")
        .or_else(|| json.get("content"))
        .or_else(|| json.get("text"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let level = if json.get("level").and_then(|v| v.as_str()) == Some("error") {
        LogLevel::Error
    } else if json.get("level").and_then(|v| v.as_str()) == Some("warn") {
        LogLevel::Warn
    } else {
        LogLevel::Info
    };

    Some(LogEntry {
        timestamp,
        level,
        message,
        category,
    })
}

fn parse_cline_text_entry(line: &str) -> Option<LogEntry> {
    // Parse common log formats: "[TIMESTAMP] LEVEL: Message"
    let parts: Vec<&str> = line.splitn(3, ['[', ']']).collect();

    // Try to extract timestamp from parts[1] if it exists
    let timestamp = if parts.len() >= 2 {
        chrono::DateTime::parse_from_rfc3339(parts[1].trim())
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .or_else(|| {
                // Try other common formats
                chrono::NaiveDateTime::parse_from_str(parts[1].trim(), "%Y-%m-%d %H:%M:%S")
                    .ok()
                    .map(|ndt| ndt.and_utc())
            })
    } else {
        None
    };

    let level = if line.contains("ERROR") {
        LogLevel::Error
    } else if line.contains("WARN") {
        LogLevel::Warn
    } else if line.contains("DEBUG") {
        LogLevel::Debug
    } else {
        LogLevel::Info
    };

    let category = if line.contains("user:") || line.contains("User:") {
        EntryCategory::UserPrompt
    } else if line.contains("assistant:") || line.contains("Assistant:") {
        EntryCategory::AssistantResponse
    } else if line.contains("tool:") || line.contains("Tool:") {
        EntryCategory::ToolUse
    } else {
        EntryCategory::Unknown
    };

    Some(LogEntry {
        timestamp,
        level,
        message: line.to_string(),
        category,
    })
}
