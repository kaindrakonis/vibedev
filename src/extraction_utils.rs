#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Helper utilities for data extraction

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub tool: String,
    pub timestamp: String,
    pub messages: Vec<Message>,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub tokens: Option<TokenCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub parameters: serde_json::Value,
    pub result: Option<String>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCount {
    pub input: usize,
    pub output: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInstance {
    pub error_type: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub language: Option<String>,
    pub context_before: Option<String>,
    pub context_after: Option<String>,
    pub fix: Option<String>,
    pub conversation_id: String,
}

/// Load all conversations from home directory
pub fn load_all_conversations(base_dir: &Path) -> Result<Vec<Conversation>> {
    let mut conversations = Vec::new();

    // Load Cline conversations
    conversations.extend(load_cline_conversations(base_dir)?);

    // Load Claude Code conversations
    conversations.extend(load_claude_code_conversations(base_dir)?);

    // Load Roo-Cline conversations
    conversations.extend(load_roo_cline_conversations(base_dir)?);

    Ok(conversations)
}

fn load_cline_conversations(base_dir: &Path) -> Result<Vec<Conversation>> {
    let mut conversations = Vec::new();

    // Look for Cline task directories
    let cline_dirs = vec![
        base_dir.join(".config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks"),
        base_dir.join(".var/app/com.visualstudio.code/config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks"),
    ];

    for dir in cline_dirs {
        if !dir.exists() {
            continue;
        }

        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let api_file = path.join("api_conversation_history.json");
                if api_file.exists() {
                    if let Ok(conv) = parse_cline_conversation(&api_file) {
                        conversations.push(conv);
                    }
                }
            }
        }
    }

    Ok(conversations)
}

fn parse_cline_conversation(path: &Path) -> Result<Conversation> {
    let content = fs::read_to_string(path)?;
    let messages: Vec<serde_json::Value> = serde_json::from_str(&content)?;

    let mut parsed_messages = Vec::new();
    for msg in messages {
        if let Some(role) = msg.get("role").and_then(|r| r.as_str()) {
            let content = msg
                .get("content")
                .and_then(|c| {
                    if c.is_array() {
                        c.as_array().and_then(|arr| {
                            arr.first()
                                .and_then(|item| item.get("text"))
                                .and_then(|t| t.as_str())
                        })
                    } else {
                        c.as_str()
                    }
                })
                .unwrap_or("")
                .to_string();

            parsed_messages.push(Message {
                role: role.to_string(),
                content,
                timestamp: msg
                    .get("ts")
                    .and_then(|t| t.as_i64())
                    .map(|t| t.to_string()),
                tool_calls: Vec::new(), // TODO: Parse tool calls
                tokens: None,
            });
        }
    }

    Ok(Conversation {
        id: path
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
        tool: "Cline".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        messages: parsed_messages,
        file_path: path.to_path_buf(),
    })
}

fn load_claude_code_conversations(base_dir: &Path) -> Result<Vec<Conversation>> {
    let mut conversations = Vec::new();

    let claude_dir = base_dir.join(".claude/projects");
    if !claude_dir.exists() {
        return Ok(conversations);
    }

    for entry in walkdir::WalkDir::new(&claude_dir)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            if let Ok(conv) = parse_claude_code_conversation(path) {
                conversations.push(conv);
            }
        }
    }

    Ok(conversations)
}

fn parse_claude_code_conversation(path: &Path) -> Result<Conversation> {
    let content = fs::read_to_string(path)?;
    let mut messages = Vec::new();

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(role) = msg.get("role").and_then(|r| r.as_str()) {
                let content = msg
                    .get("content")
                    .map(|c| c.to_string())
                    .unwrap_or_default();

                messages.push(Message {
                    role: role.to_string(),
                    content,
                    timestamp: msg
                        .get("timestamp")
                        .and_then(|t| t.as_str())
                        .map(|s| s.to_string()),
                    tool_calls: Vec::new(),
                    tokens: None,
                });
            }
        }
    }

    Ok(Conversation {
        id: path.file_stem().unwrap().to_string_lossy().to_string(),
        tool: "Claude Code".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        messages,
        file_path: path.to_path_buf(),
    })
}

fn load_roo_cline_conversations(base_dir: &Path) -> Result<Vec<Conversation>> {
    // Similar to Cline but different path
    let mut conversations = Vec::new();

    let roo_dir = base_dir.join(".config/Code/User/globalStorage/rooveterinaryinc.roo-cline/tasks");
    if !roo_dir.exists() {
        return Ok(conversations);
    }

    for entry in fs::read_dir(&roo_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let api_file = path.join("api_conversation_history.json");
            if api_file.exists() {
                if let Ok(mut conv) = parse_cline_conversation(&api_file) {
                    conv.tool = "Roo-Cline".to_string();
                    conversations.push(conv);
                }
            }
        }
    }

    Ok(conversations)
}

/// Extract errors from conversations
pub fn extract_errors(conversations: &[Conversation]) -> Vec<ErrorInstance> {
    let mut errors = Vec::new();

    for conv in conversations {
        for msg in conv.messages.iter() {
            // Look for error patterns in content
            if msg.content.contains("error")
                || msg.content.contains("Error")
                || msg.content.contains("ERROR")
                || msg.content.contains("failed")
            {
                // Try to extract structured error info
                if let Some(error) = parse_error_from_message(&msg.content, &conv.id) {
                    errors.push(error);
                }
            }
        }
    }

    errors
}

fn parse_error_from_message(content: &str, conv_id: &str) -> Option<ErrorInstance> {
    // Basic error parsing - look for common patterns

    // Check for Rust errors
    if content.contains("error[E") {
        return Some(ErrorInstance {
            error_type: "compile_error".to_string(),
            message: content.lines().take(3).collect::<Vec<_>>().join("\n"),
            file: None,
            line: None,
            language: Some("rust".to_string()),
            context_before: None,
            context_after: None,
            fix: None,
            conversation_id: conv_id.to_string(),
        });
    }

    // Check for TypeScript errors
    if content.contains("TS") && content.contains("error") {
        return Some(ErrorInstance {
            error_type: "type_error".to_string(),
            message: content.lines().take(3).collect::<Vec<_>>().join("\n"),
            file: None,
            line: None,
            language: Some("typescript".to_string()),
            context_before: None,
            context_after: None,
            fix: None,
            conversation_id: conv_id.to_string(),
        });
    }

    // Generic error
    if content.to_lowercase().contains("error") {
        Some(ErrorInstance {
            error_type: "generic_error".to_string(),
            message: content.chars().take(200).collect(),
            file: None,
            line: None,
            language: None,
            context_before: None,
            context_after: None,
            fix: None,
            conversation_id: conv_id.to_string(),
        })
    } else {
        None
    }
}

/// Calculate tokens for text (rough estimate: 4 chars per token)
pub fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Deduplicate items by a key function
pub fn deduplicate<T, F, K>(items: Vec<T>, key_fn: F) -> Vec<T>
where
    F: Fn(&T) -> K,
    K: std::hash::Hash + Eq,
{
    let mut seen = std::collections::HashSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(key_fn(item)))
        .collect()
}
