#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::info;
use walkdir::WalkDir;

#[derive(Debug, Serialize)]
pub struct ConversationStats {
    pub total_conversations: usize,
    pub total_messages: usize,
    pub total_tokens_estimate: u64,
    pub by_tool: HashMap<String, ToolStats>,
}

#[derive(Debug, Serialize)]
pub struct ToolStats {
    pub conversations: usize,
    pub messages: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub tokens: u64,
}

#[derive(Debug, Deserialize)]
struct ClineMessage {
    role: String,
    content: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ClineMetadata {
    model_usage: Option<ModelUsage>,
}

#[derive(Debug, Deserialize)]
struct ModelUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

pub struct ConversationAnalyzer {
    base_dir: PathBuf,
}

impl ConversationAnalyzer {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn analyze(&self) -> Result<ConversationStats> {
        info!("Analyzing conversations...");
        
        let mut stats = ConversationStats {
            total_conversations: 0,
            total_messages: 0,
            total_tokens_estimate: 0,
            by_tool: HashMap::new(),
        };

        // Analyze Cline tasks
        self.analyze_cline_tasks(&mut stats)?;
        
        // Analyze Claude Code history
        self.analyze_claude_history(&mut stats)?;

        Ok(stats)
    }

    fn analyze_cline_tasks(&self, stats: &mut ConversationStats) -> Result<()> {
        let patterns = vec![
            ".config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks",
            ".config/Code/User/globalStorage/kilocode.kilo-code/tasks",
            ".config/Code/User/globalStorage/rooveterinaryinc.roo-cline/tasks",
            ".var/app/com.visualstudio.code/config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks",
            ".var/app/com.visualstudio.code/config/Code/User/globalStorage/kilocode.kilo-code/tasks",
        ];

        for pattern in patterns {
            let path = self.base_dir.join(pattern);
            if !path.exists() {
                continue;
            }

            let tool_name = if pattern.contains("claude-dev") {
                "Cline"
            } else if pattern.contains("kilo-code") {
                "Kilo"
            } else {
                "Roo-Cline"
            };

            for entry in WalkDir::new(&path).min_depth(1).max_depth(1) {
                let entry = entry?;
                if !entry.file_type().is_dir() {
                    continue;
                }

                let api_history = entry.path().join("api_conversation_history.json");
                let metadata_file = entry.path().join("task_metadata.json");

                if api_history.exists() {
                    self.process_cline_conversation(
                        &api_history,
                        &metadata_file,
                        tool_name,
                        stats,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn process_cline_conversation(
        &self,
        api_history: &PathBuf,
        metadata_file: &PathBuf,
        tool_name: &str,
        stats: &mut ConversationStats,
    ) -> Result<()> {
        let content = fs::read_to_string(api_history)?;
        let messages: Vec<ClineMessage> = serde_json::from_str(&content)?;

        let mut user_count = 0;
        let mut assistant_count = 0;

        for msg in &messages {
            if msg.role == "user" {
                user_count += 1;
            } else if msg.role == "assistant" {
                assistant_count += 1;
            }
        }

        let mut tokens = 0u64;
        if metadata_file.exists() {
            if let Ok(meta_content) = fs::read_to_string(metadata_file) {
                if let Ok(metadata) = serde_json::from_str::<ClineMetadata>(&meta_content) {
                    if let Some(usage) = metadata.model_usage {
                        tokens = usage.input_tokens.unwrap_or(0) + usage.output_tokens.unwrap_or(0);
                    }
                }
            }
        }

        // If no token count, estimate (rough: 4 chars per token)
        if tokens == 0 {
            let char_count: usize = messages
                .iter()
                .map(|m| m.content.to_string().len())
                .sum();
            tokens = (char_count / 4) as u64;
        }

        stats.total_conversations += 1;
        stats.total_messages += messages.len();
        stats.total_tokens_estimate += tokens;

        let tool_stats = stats.by_tool.entry(tool_name.to_string()).or_insert(ToolStats {
            conversations: 0,
            messages: 0,
            user_messages: 0,
            assistant_messages: 0,
            tokens: 0,
        });

        tool_stats.conversations += 1;
        tool_stats.messages += messages.len();
        tool_stats.user_messages += user_count;
        tool_stats.assistant_messages += assistant_count;
        tool_stats.tokens += tokens;

        Ok(())
    }

    fn analyze_claude_history(&self, stats: &mut ConversationStats) -> Result<()> {
        let history_path = self.base_dir.join(".claude/history.jsonl");
        if !history_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&history_path)?;
        let lines: Vec<&str> = content.lines().collect();

        // Claude Code history is event-based, estimate conversations
        let conversations_est = lines.len() / 10; // rough estimate
        
        stats.total_conversations += conversations_est;

        let tool_stats = stats.by_tool.entry("Claude Code".to_string()).or_insert(ToolStats {
            conversations: 0,
            messages: 0,
            user_messages: 0,
            assistant_messages: 0,
            tokens: 0,
        });

        tool_stats.conversations += conversations_est;

        Ok(())
    }
}
