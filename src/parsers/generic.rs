// Generic log parser fallback
use super::{EntryCategory, LogEntry, LogLevel, LogMetadata, LogParser, ParsedLog};
use crate::models::AiTool;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub struct GenericParser;

impl LogParser for GenericParser {
    fn can_parse(&self, _path: &Path) -> bool {
        true // Accepts anything as a last resort
    }

    fn parse(&self, path: &Path) -> Result<ParsedLog> {
        let mut entries = Vec::new();
        let mut oldest: Option<DateTime<Utc>> = None;
        let mut newest: Option<DateTime<Utc>> = None;

        // Try to read as a text file
        if path.is_file() {
            if let Ok(file) = fs::File::open(path) {
                let reader = BufReader::new(file);
                let mut line_count = 0;

                for line in reader.lines().map_while(Result::ok) {
                    line_count += 1;
                    if line.trim().is_empty() {
                        continue;
                    }

                    // Very basic parsing - just extract lines
                    let entry = parse_generic_line(&line);
                    if let Some(ts) = entry.timestamp {
                        oldest = Some(oldest.map_or(ts, |o| o.min(ts)));
                        newest = Some(newest.map_or(ts, |n| n.max(ts)));
                    }
                    entries.push(entry);

                    // Limit entries to avoid memory issues with huge files
                    if line_count > 10000 {
                        break;
                    }
                }
            }
        } else if path.is_dir() {
            // Count files in directory as basic metric
            if let Ok(dir_entries) = fs::read_dir(path) {
                let file_count = dir_entries.count();
                entries.push(LogEntry {
                    timestamp: None,
                    level: LogLevel::Info,
                    message: format!("Directory with {} files", file_count),
                    category: EntryCategory::SystemEvent,
                });
            }
        }

        let file_size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let entry_count = entries.len();

        // Determine tool from path
        let tool = AiTool::from_path(path).unwrap_or_else(|| {
            AiTool::Other(
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string(),
            )
        });

        Ok(ParsedLog {
            tool,
            entries,
            metadata: LogMetadata {
                file_size,
                entry_count,
                date_range: (oldest, newest),
            },
        })
    }
}

fn parse_generic_line(line: &str) -> LogEntry {
    // Try to extract basic info from the line
    let level = if line.to_lowercase().contains("error") {
        LogLevel::Error
    } else if line.to_lowercase().contains("warn") {
        LogLevel::Warn
    } else if line.to_lowercase().contains("debug") {
        LogLevel::Debug
    } else {
        LogLevel::Unknown
    };

    let category = if line.to_lowercase().contains("user")
        || line.to_lowercase().contains("prompt")
        || line.to_lowercase().contains("question")
    {
        EntryCategory::UserPrompt
    } else if line.to_lowercase().contains("assistant")
        || line.to_lowercase().contains("response")
        || line.to_lowercase().contains("answer")
    {
        EntryCategory::AssistantResponse
    } else if line.to_lowercase().contains("tool")
        || line.to_lowercase().contains("function")
        || line.to_lowercase().contains("call")
    {
        EntryCategory::ToolUse
    } else if line.to_lowercase().contains("file")
        || line.to_lowercase().contains("edit")
        || line.to_lowercase().contains("write")
    {
        EntryCategory::FileOperation
    } else {
        EntryCategory::Unknown
    };

    // Try to extract timestamp (very basic ISO 8601 detection)
    let timestamp = line
        .split_whitespace()
        .find(|s| s.contains('T') && s.contains(':'))
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    LogEntry {
        timestamp,
        level,
        message: line.to_string(),
        category,
    }
}
