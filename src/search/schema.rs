use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::Path;
use tantivy::schema::*;
use tantivy::TantivyDocument;

use crate::models::AiTool;
use crate::parsers::{EntryCategory, LogEntry, LogLevel};

// Field names constants
pub const FIELD_DOC_ID: &str = "doc_id";
pub const FIELD_TOOL: &str = "tool";
pub const FIELD_LOG_TYPE: &str = "log_type";
pub const FIELD_TIMESTAMP: &str = "timestamp";
pub const FIELD_LEVEL: &str = "level";
pub const FIELD_CATEGORY: &str = "category";
pub const FIELD_MESSAGE: &str = "message";
pub const FIELD_FILE_PATH: &str = "file_path";
pub const FIELD_PROJECT: &str = "project";

/// Builds the Tantivy schema for indexing log entries
pub fn build_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    // doc_id: u64 (unique identifier)
    schema_builder.add_u64_field(FIELD_DOC_ID, STORED | INDEXED | FAST);

    // tool: TEXT (AI tool name like "Claude Code", "Cursor")
    let text_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("default")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();
    schema_builder.add_text_field(FIELD_TOOL, text_options.clone());

    // log_type: TEXT (Debug, History, Session, etc.)
    schema_builder.add_text_field(FIELD_LOG_TYPE, text_options.clone());

    // timestamp: DATE (for range queries)
    schema_builder.add_date_field(FIELD_TIMESTAMP, STORED | INDEXED | FAST);

    // level: TEXT (Debug, Info, Warn, Error)
    schema_builder.add_text_field(FIELD_LEVEL, text_options.clone());

    // category: TEXT (UserPrompt, AssistantResponse, ToolUse, Error)
    schema_builder.add_text_field(FIELD_CATEGORY, text_options.clone());

    // message: TEXT (full log message content, tokenized)
    let message_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("default")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();
    schema_builder.add_text_field(FIELD_MESSAGE, message_options);

    // file_path: TEXT (source file path)
    schema_builder.add_text_field(FIELD_FILE_PATH, text_options.clone());

    // project: TEXT (project name extracted from path)
    schema_builder.add_text_field(FIELD_PROJECT, text_options);

    schema_builder.build()
}

/// Represents a log entry document in the Tantivy index
pub struct LogEntryDocument {
    pub doc_id: u64,
    pub tool: String,
    pub log_type: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub level: String,
    pub category: String,
    pub message: String,
    pub file_path: String,
    pub project: String,
}

impl LogEntryDocument {
    /// Creates a Tantivy document from a LogEntry
    pub fn from_log_entry(
        log_entry: &LogEntry,
        doc_id: u64,
        tool: &AiTool,
        log_type: &str,
        file_path: &Path,
    ) -> Self {
        let project = extract_project_name(file_path);

        LogEntryDocument {
            doc_id,
            tool: tool.name().to_string(),
            log_type: log_type.to_string(),
            timestamp: log_entry.timestamp,
            level: format!("{:?}", log_entry.level),
            category: format!("{:?}", log_entry.category),
            message: log_entry.message.clone(),
            file_path: file_path.to_string_lossy().to_string(),
            project,
        }
    }

    /// Converts this LogEntryDocument to a Tantivy Document
    pub fn to_tantivy_document(&self, schema: &Schema) -> TantivyDocument {
        let mut doc = TantivyDocument::default();

        // Add fields using the schema
        let doc_id_field = schema.get_field(FIELD_DOC_ID).unwrap();
        doc.add_u64(doc_id_field, self.doc_id);

        let tool_field = schema.get_field(FIELD_TOOL).unwrap();
        doc.add_text(tool_field, &self.tool);

        let log_type_field = schema.get_field(FIELD_LOG_TYPE).unwrap();
        doc.add_text(log_type_field, &self.log_type);

        if let Some(timestamp) = self.timestamp {
            let timestamp_field = schema.get_field(FIELD_TIMESTAMP).unwrap();
            // Convert chrono DateTime to tantivy DateTime (Unix timestamp in microseconds)
            let tantivy_dt = tantivy::DateTime::from_timestamp_micros(timestamp.timestamp_micros());
            doc.add_date(timestamp_field, tantivy_dt);
        }

        let level_field = schema.get_field(FIELD_LEVEL).unwrap();
        doc.add_text(level_field, &self.level);

        let category_field = schema.get_field(FIELD_CATEGORY).unwrap();
        doc.add_text(category_field, &self.category);

        let message_field = schema.get_field(FIELD_MESSAGE).unwrap();
        doc.add_text(message_field, &self.message);

        let file_path_field = schema.get_field(FIELD_FILE_PATH).unwrap();
        doc.add_text(file_path_field, &self.file_path);

        let project_field = schema.get_field(FIELD_PROJECT).unwrap();
        doc.add_text(project_field, &self.project);

        doc
    }
}

/// Extracts project name from a file path
/// Examples:
/// - ~/.config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks/vibedev/task.json -> "vibedev"
/// - ~/.claude/projects/my-app/history.jsonl -> "my-app"
fn extract_project_name(path: &Path) -> String {
    let path_str = path.to_string_lossy();

    // Try to extract from common patterns
    if path_str.contains("/projects/") {
        if let Some(project_start) = path_str.find("/projects/") {
            let after_projects = &path_str[project_start + "/projects/".len()..];
            if let Some(slash_pos) = after_projects.find('/') {
                return after_projects[..slash_pos].to_string();
            }
        }
    }

    if path_str.contains("/tasks/") {
        if let Some(tasks_start) = path_str.find("/tasks/") {
            let after_tasks = &path_str[tasks_start + "/tasks/".len()..];
            if let Some(slash_pos) = after_tasks.find('/') {
                return after_tasks[..slash_pos].to_string();
            }
        }
    }

    // Fallback: use parent directory name
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;

    #[test]
    fn test_schema_fields() {
        let schema = build_schema();

        // Verify all fields exist
        assert!(schema.get_field(FIELD_DOC_ID).is_ok());
        assert!(schema.get_field(FIELD_TOOL).is_ok());
        assert!(schema.get_field(FIELD_LOG_TYPE).is_ok());
        assert!(schema.get_field(FIELD_TIMESTAMP).is_ok());
        assert!(schema.get_field(FIELD_LEVEL).is_ok());
        assert!(schema.get_field(FIELD_CATEGORY).is_ok());
        assert!(schema.get_field(FIELD_MESSAGE).is_ok());
        assert!(schema.get_field(FIELD_FILE_PATH).is_ok());
        assert!(schema.get_field(FIELD_PROJECT).is_ok());
    }

    #[test]
    fn test_log_entry_to_document() {
        let log_entry = LogEntry {
            timestamp: Some(Utc::now()),
            level: LogLevel::Info,
            message: "Test message".to_string(),
            category: EntryCategory::UserPrompt,
        };

        let tool = AiTool::ClaudeCode;
        let log_type = "History";
        let file_path = PathBuf::from("/home/user/.claude/projects/my-app/history.jsonl");

        let doc = LogEntryDocument::from_log_entry(&log_entry, 1, &tool, log_type, &file_path);

        assert_eq!(doc.doc_id, 1);
        assert_eq!(doc.tool, "Claude Code");
        assert_eq!(doc.log_type, "History");
        assert_eq!(doc.level, "Info");
        assert_eq!(doc.category, "UserPrompt");
        assert_eq!(doc.message, "Test message");
        assert_eq!(doc.project, "my-app");
    }

    #[test]
    fn test_extract_project_name() {
        let path1 = PathBuf::from("/home/user/.claude/projects/vibedev/history.jsonl");
        assert_eq!(extract_project_name(&path1), "vibedev");

        let path2 =
            PathBuf::from("/home/user/.config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks/my-app/task.json");
        assert_eq!(extract_project_name(&path2), "my-app");

        let path3 = PathBuf::from("/home/user/.cursor/main.log");
        assert_eq!(extract_project_name(&path3), ".cursor");
    }

    #[test]
    fn test_to_tantivy_document() {
        let schema = build_schema();
        let log_entry = LogEntry {
            timestamp: Some(Utc::now()),
            level: LogLevel::Error,
            message: "Test error".to_string(),
            category: EntryCategory::Error,
        };

        let tool = AiTool::Cursor;
        let log_type = "Debug";
        let file_path = PathBuf::from("/home/user/.cursor/main.log");

        let entry_doc =
            LogEntryDocument::from_log_entry(&log_entry, 42, &tool, log_type, &file_path);
        let tantivy_doc = entry_doc.to_tantivy_document(&schema);

        // Verify document was created (basic check)
        assert!(!tantivy_doc.is_empty());
    }
}
