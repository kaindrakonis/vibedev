#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
pub struct HistoryEntry {
    pub display: String,
    #[serde(default)]
    pub timestamp: i64,
    #[serde(default)]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConversationMessage {
    #[serde(default)]
    pub r#type: String,
    pub message: Option<MessageContent>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MessageContent {
    pub role: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ClaudeCodeStats {
    pub total_prompts: usize,
    pub total_conversations: usize,
    pub total_messages: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub projects: HashMap<String, usize>,
    pub estimated_tokens: u64,
    pub frustration_prompts: Vec<String>,
    pub go_on_count: usize,
}

pub struct ClaudeCodeParser {
    base_dir: PathBuf,
}

impl ClaudeCodeParser {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn parse(&self) -> Result<ClaudeCodeStats> {
        let mut stats = ClaudeCodeStats {
            total_prompts: 0,
            total_conversations: 0,
            total_messages: 0,
            user_messages: 0,
            assistant_messages: 0,
            projects: HashMap::new(),
            estimated_tokens: 0,
            frustration_prompts: Vec::new(),
            go_on_count: 0,
        };

        // Parse history.jsonl
        let history_path = self.base_dir.join(".claude/history.jsonl");
        if history_path.exists() {
            self.parse_history(&history_path, &mut stats)?;
        }

        // Parse conversation files in projects/
        let projects_dir = self.base_dir.join(".claude/projects");
        if projects_dir.exists() {
            self.parse_conversations(&projects_dir, &mut stats)?;
        }

        Ok(stats)
    }

    fn parse_history(&self, path: &PathBuf, stats: &mut ClaudeCodeStats) -> Result<()> {
        let content = fs::read_to_string(path)?;

        let frustration_keywords = vec!["wtf", "fuck", "no,", "stop", "please", "beg", "come on"];

        for line in content.lines() {
            if let Ok(entry) = serde_json::from_str::<HistoryEntry>(line) {
                stats.total_prompts += 1;

                // Track project usage
                if let Some(project) = entry.project {
                    *stats.projects.entry(project).or_insert(0) += 1;
                }

                // Detect "go on"
                let display_lower = entry.display.to_lowercase();
                if display_lower == "go on" || display_lower == "go on?" {
                    stats.go_on_count += 1;
                }

                // Detect frustration
                for keyword in &frustration_keywords {
                    if display_lower.contains(keyword) {
                        if stats.frustration_prompts.len() < 10 {
                            stats.frustration_prompts.push(entry.display.clone());
                        }
                        break;
                    }
                }

                // Estimate tokens from prompt length
                stats.estimated_tokens += (entry.display.len() / 4) as u64;
            }
        }

        Ok(())
    }

    fn parse_conversations(
        &self,
        projects_dir: &PathBuf,
        stats: &mut ClaudeCodeStats,
    ) -> Result<()> {
        for entry in WalkDir::new(projects_dir)
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

            // Parse conversation file
            if let Ok(content) = fs::read_to_string(path) {
                let mut has_messages = false;

                for line in content.lines() {
                    if let Ok(msg) = serde_json::from_str::<ConversationMessage>(line) {
                        if let Some(message_content) = msg.message {
                            has_messages = true;
                            stats.total_messages += 1;

                            if message_content.role == "user" {
                                stats.user_messages += 1;
                            } else if message_content.role == "assistant" {
                                stats.assistant_messages += 1;
                            }

                            // Estimate tokens
                            let content_str = message_content.content.to_string();
                            stats.estimated_tokens += (content_str.len() / 4) as u64;
                        }
                    }
                }

                if has_messages {
                    stats.total_conversations += 1;
                }
            }
        }

        Ok(())
    }
}
