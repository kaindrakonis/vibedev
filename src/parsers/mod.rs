#![allow(dead_code)]

pub mod claude;
pub mod cline;
pub mod cursor;
pub mod generic;

use crate::models::*;
use anyhow::Result;
use std::path::Path;

pub trait LogParser: Send + Sync {
    fn can_parse(&self, path: &Path) -> bool;
    fn parse(&self, path: &Path) -> Result<ParsedLog>;
}

#[derive(Debug, Clone)]
pub struct ParsedLog {
    pub tool: AiTool,
    pub entries: Vec<LogEntry>,
    pub metadata: LogMetadata,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub level: LogLevel,
    pub message: String,
    pub category: EntryCategory,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntryCategory {
    UserPrompt,
    AssistantResponse,
    SystemEvent,
    Error,
    Performance,
    ToolUse,
    FileOperation,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct LogMetadata {
    pub file_size: u64,
    pub entry_count: usize,
    pub date_range: (
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
    ),
}
