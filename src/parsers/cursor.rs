// Cursor log parser
use super::{EntryCategory, LogEntry, LogLevel, LogMetadata, LogParser, ParsedLog};
use crate::models::AiTool;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub struct CursorParser;

impl LogParser for CursorParser {
    fn can_parse(&self, path: &Path) -> bool {
        path.to_string_lossy().contains("cursor")
    }

    fn parse(&self, path: &Path) -> Result<ParsedLog> {
        let mut entries = Vec::new();
        let mut oldest: Option<DateTime<Utc>> = None;
        let mut newest: Option<DateTime<Utc>> = None;

        // Cursor typically stores logs in User/.cursor/logs or similar
        let log_path = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent().unwrap_or(path).to_path_buf()
        };

        // Look for Cursor log files
        let possible_files = vec![
            log_path.join("main.log"),
            log_path.join("renderer.log"),
            log_path.join("extensionHost.log"),
            log_path.join("chat.log"),
        ];

        for log_file in possible_files {
            if log_file.exists() {
                if let Ok(file) = fs::File::open(&log_file) {
                    let reader = BufReader::new(file);
                    for line in reader.lines().map_while(Result::ok) {
                        if line.trim().is_empty() {
                            continue;
                        }

                        // Try parsing as JSON first
                        if let Ok(json) = serde_json::from_str::<Value>(&line) {
                            if let Some(entry) = parse_cursor_json_entry(&json) {
                                if let Some(ts) = entry.timestamp {
                                    oldest = Some(oldest.map_or(ts, |o| o.min(ts)));
                                    newest = Some(newest.map_or(ts, |n| n.max(ts)));
                                }
                                entries.push(entry);
                            }
                        } else {
                            // Parse as plain text
                            if let Some(entry) = parse_cursor_text_entry(&line) {
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
            tool: AiTool::Cursor,
            entries,
            metadata: LogMetadata {
                file_size,
                entry_count,
                date_range: (oldest, newest),
            },
        })
    }
}

fn parse_cursor_json_entry(json: &Value) -> Option<LogEntry> {
    let timestamp = json
        .get("timestamp")
        .or_else(|| json.get("time"))
        .or_else(|| json.get("ts"))
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let level_str = json
        .get("level")
        .and_then(|v| v.as_str())
        .unwrap_or("info")
        .to_lowercase();

    let level = match level_str.as_str() {
        "error" => LogLevel::Error,
        "warn" | "warning" => LogLevel::Warn,
        "debug" => LogLevel::Debug,
        _ => LogLevel::Info,
    };

    let category = if json.get("category").and_then(|v| v.as_str()) == Some("chat")
        || json.get("type").and_then(|v| v.as_str()) == Some("user_input")
    {
        EntryCategory::UserPrompt
    } else if json.get("type").and_then(|v| v.as_str()) == Some("ai_response") {
        EntryCategory::AssistantResponse
    } else if json.get("type").and_then(|v| v.as_str()) == Some("code_edit")
        || json.get("type").and_then(|v| v.as_str()) == Some("file_operation")
    {
        EntryCategory::FileOperation
    } else {
        EntryCategory::SystemEvent
    };

    let message = json
        .get("message")
        .or_else(|| json.get("msg"))
        .or_else(|| json.get("text"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some(LogEntry {
        timestamp,
        level,
        message,
        category,
    })
}

fn parse_cursor_text_entry(line: &str) -> Option<LogEntry> {
    // Cursor uses VSCode-style logging: [timestamp] [level] message
    // Example: [2024-01-03 10:00:00.000] [info] Extension activated

    let timestamp = None; // Parse timestamp if needed

    let level = if line.contains("[error]") {
        LogLevel::Error
    } else if line.contains("[warn]") {
        LogLevel::Warn
    } else if line.contains("[debug]") {
        LogLevel::Debug
    } else {
        LogLevel::Info
    };

    let category = if line.contains("chat") || line.contains("user input") {
        EntryCategory::UserPrompt
    } else if line.contains("AI response") || line.contains("completion") {
        EntryCategory::AssistantResponse
    } else if line.contains("file") || line.contains("edit") {
        EntryCategory::FileOperation
    } else {
        EntryCategory::SystemEvent
    };

    Some(LogEntry {
        timestamp,
        level,
        message: line.to_string(),
        category,
    })
}
