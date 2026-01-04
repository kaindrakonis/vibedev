// Claude Code log parser
use super::{EntryCategory, LogEntry, LogLevel, LogMetadata, LogParser, ParsedLog};
use crate::models::AiTool;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub struct ClaudeParser;

impl LogParser for ClaudeParser {
    fn can_parse(&self, path: &Path) -> bool {
        path.to_string_lossy().contains(".claude")
    }

    fn parse(&self, path: &Path) -> Result<ParsedLog> {
        let mut entries = Vec::new();
        let mut oldest: Option<DateTime<Utc>> = None;
        let mut newest: Option<DateTime<Utc>> = None;

        // Check if this is history.jsonl
        if path.file_name().and_then(|n| n.to_str()) == Some("history.jsonl") {
            let file = fs::File::open(path)?;
            let reader = BufReader::new(file);

            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }

                if let Ok(json) = serde_json::from_str::<Value>(&line) {
                    if let Some(entry) = parse_history_entry(&json) {
                        // Update date range
                        if let Some(ts) = entry.timestamp {
                            oldest = Some(oldest.map_or(ts, |o| o.min(ts)));
                            newest = Some(newest.map_or(ts, |n| n.max(ts)));
                        }
                        entries.push(entry);
                    }
                }
            }
        } else if path.is_dir() && path.join("history.jsonl").exists() {
            // Recursively parse if it's the .claude directory
            return self.parse(&path.join("history.jsonl"));
        }

        let file_size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let entry_count = entries.len();

        Ok(ParsedLog {
            tool: AiTool::ClaudeCode,
            entries,
            metadata: LogMetadata {
                file_size,
                entry_count,
                date_range: (oldest, newest),
            },
        })
    }
}

fn parse_history_entry(json: &Value) -> Option<LogEntry> {
    // Extract timestamp
    let timestamp = json
        .get("timestamp")
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    // Detect category based on content
    let category = if json.get("userMessage").is_some() || json.get("prompt").is_some() {
        EntryCategory::UserPrompt
    } else if json.get("assistantMessage").is_some() || json.get("response").is_some() {
        EntryCategory::AssistantResponse
    } else if json.get("tool_use").is_some() || json.get("toolUse").is_some() {
        EntryCategory::ToolUse
    } else if json.get("error").is_some() {
        EntryCategory::Error
    } else {
        EntryCategory::SystemEvent
    };

    // Extract message content
    let message = json
        .get("userMessage")
        .or_else(|| json.get("assistantMessage"))
        .or_else(|| json.get("prompt"))
        .or_else(|| json.get("response"))
        .or_else(|| json.get("message"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Determine log level
    let level = if json.get("error").is_some() {
        LogLevel::Error
    } else if category == EntryCategory::UserPrompt {
        LogLevel::Info
    } else {
        LogLevel::Debug
    };

    Some(LogEntry {
        timestamp,
        level,
        message,
        category,
    })
}

pub fn analyze_claude_logs(claude_dir: &Path) -> Result<ClaudeAnalysis> {
    let mut analysis = ClaudeAnalysis::default();

    // Parse history.jsonl
    let history_file = claude_dir.join("history.jsonl");
    if history_file.exists() {
        let parser = ClaudeParser;
        let parsed = parser.parse(&history_file)?;

        // Count sessions and prompts
        let mut session_count = 0;
        let mut prompt_count = 0;
        let mut response_count = 0;

        for entry in &parsed.entries {
            match entry.category {
                EntryCategory::UserPrompt => {
                    prompt_count += 1;
                    session_count += 1; // Each prompt starts a potential session
                }
                EntryCategory::AssistantResponse => {
                    response_count += 1;
                }
                _ => {}
            }
        }

        analysis.session_count = session_count;
        analysis.prompt_count = prompt_count;
        analysis.response_count = response_count;

        // Estimate tokens (rough estimate: 1 token ≈ 4 characters)
        let total_chars: usize = parsed.entries.iter().map(|e| e.message.len()).sum();
        analysis.estimated_tokens = (total_chars / 4) as u64;
    }

    // Count debug files
    let debug_dir = claude_dir.join("debug");
    if debug_dir.exists() {
        analysis.debug_file_count = fs::read_dir(&debug_dir)?.count();
    }

    // Count file history
    let file_history_dir = claude_dir.join("file-history");
    if file_history_dir.exists() {
        analysis.file_history_count = fs::read_dir(&file_history_dir)?.count();
    }

    Ok(analysis)
}

#[derive(Debug, Clone, Default)]
pub struct ClaudeAnalysis {
    pub session_count: usize,
    pub prompt_count: usize,
    pub response_count: usize,
    pub estimated_tokens: u64,
    pub debug_file_count: usize,
    pub file_history_count: usize,
}
