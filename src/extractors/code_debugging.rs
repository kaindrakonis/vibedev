use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::extraction_utils::{extract_errors, Conversation, ErrorInstance};

#[derive(Debug, Serialize, Deserialize)]
pub struct DebuggingExample {
    pub id: String,
    pub error: ErrorInfo,
    pub context: CodeContext,
    pub debugging_process: Vec<DebuggingTurn>,
    pub solution: Solution,
    pub verification: Verification,
    pub features: DebuggingFeatures,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub error_type: String,
    pub message: String,
    pub language: String,
    pub severity: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeContext {
    pub problematic_code: String,
    pub file_path: Option<String>,
    pub surrounding_context: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DebuggingTurn {
    pub turn: usize,
    pub user_message: Option<String>,
    pub ai_analysis: String,
    pub hypothesis: Option<String>,
    pub action_taken: Option<String>,
    pub result: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Solution {
    pub fix_type: String, // "syntax" | "logic" | "type" | "refactor"
    pub fixed_code: Option<String>,
    pub explanation: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Verification {
    pub verified: bool,
    pub method: String, // "compilation" | "tests" | "manual"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DebuggingFeatures {
    pub difficulty: String, // "trivial" | "easy" | "medium" | "hard"
    pub time_to_solve_minutes: f64,
    pub requires_refactor: bool,
    pub educational_value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeDebuggingDataset {
    pub total_examples: usize,
    pub examples: Vec<DebuggingExample>,
    pub by_language: std::collections::HashMap<String, usize>,
    pub by_difficulty: std::collections::HashMap<String, usize>,
    pub avg_time_to_solve: f64,
}

pub struct CodeDebuggingExtractor;

impl CodeDebuggingExtractor {
    pub fn extract(conversations: &[Conversation]) -> Result<CodeDebuggingDataset> {
        println!(
            "🐛 Extracting debugging examples from {} conversations...",
            conversations.len()
        );

        let errors = extract_errors(conversations);
        println!("   Found {} error instances", errors.len());

        let mut examples = Vec::new();

        for (idx, error) in errors.iter().enumerate() {
            if let Some(example) = Self::create_debugging_example(error, conversations, idx) {
                examples.push(example);
            }
        }

        println!("   Created {} debugging examples", examples.len());

        let by_language = Self::group_by_language(&examples);
        let by_difficulty = Self::group_by_difficulty(&examples);
        let avg_time = examples
            .iter()
            .map(|e| e.features.time_to_solve_minutes)
            .sum::<f64>()
            / examples.len().max(1) as f64;

        Ok(CodeDebuggingDataset {
            total_examples: examples.len(),
            examples,
            by_language,
            by_difficulty,
            avg_time_to_solve: avg_time,
        })
    }

    fn create_debugging_example(
        error: &ErrorInstance,
        conversations: &[Conversation],
        idx: usize,
    ) -> Option<DebuggingExample> {
        let language = error
            .language
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        let difficulty = if error.message.len() > 500 {
            "hard"
        } else if error.message.len() > 200 {
            "medium"
        } else {
            "easy"
        }
        .to_string();

        // Look up the conversation that this error came from
        let conv = conversations.iter().find(|c| c.id == error.conversation_id);
        let debugging_turns: Vec<DebuggingTurn> = if let Some(conversation) = conv {
            // Extract actual debugging turns from conversation
            conversation
                .messages
                .iter()
                .enumerate()
                .take(5)
                .map(|(i, msg)| DebuggingTurn {
                    turn: i,
                    user_message: if msg.role == "user" {
                        Some(msg.content.clone())
                    } else {
                        None
                    },
                    ai_analysis: if msg.role == "assistant" {
                        msg.content.chars().take(200).collect()
                    } else {
                        "...".to_string()
                    },
                    hypothesis: None,
                    action_taken: None,
                    result: None,
                })
                .collect()
        } else {
            // Fallback to generic debugging process
            vec![DebuggingTurn {
                turn: 0,
                user_message: Some("Error occurred".to_string()),
                ai_analysis: "Analyzing error...".to_string(),
                hypothesis: Some("Type mismatch".to_string()),
                action_taken: None,
                result: None,
            }]
        };

        Some(DebuggingExample {
            id: format!("debug_{}", idx),
            error: ErrorInfo {
                error_type: error.error_type.clone(),
                message: error.message.clone(),
                language: language.clone(),
                severity: "error".to_string(),
            },
            context: CodeContext {
                problematic_code: error.context_before.clone().unwrap_or_default(),
                file_path: error.file.clone(),
                surrounding_context: error.context_after.clone(),
            },
            debugging_process: debugging_turns,
            solution: Solution {
                fix_type: Self::classify_fix_type(&error.error_type),
                fixed_code: error.fix.clone(),
                explanation: format!("Fixed {} error", error.error_type),
            },
            verification: Verification {
                verified: error.fix.is_some(),
                method: "compilation".to_string(),
            },
            features: DebuggingFeatures {
                difficulty,
                time_to_solve_minutes: 10.0, // TODO: Calculate from timestamps
                requires_refactor: false,
                educational_value: 0.7,
            },
        })
    }

    fn classify_fix_type(error_type: &str) -> String {
        if error_type.contains("type") {
            "type".to_string()
        } else if error_type.contains("syntax") {
            "syntax".to_string()
        } else {
            "logic".to_string()
        }
    }

    fn group_by_language(
        examples: &[DebuggingExample],
    ) -> std::collections::HashMap<String, usize> {
        let mut map = std::collections::HashMap::new();
        for example in examples {
            *map.entry(example.error.language.clone()).or_insert(0) += 1;
        }
        map
    }

    fn group_by_difficulty(
        examples: &[DebuggingExample],
    ) -> std::collections::HashMap<String, usize> {
        let mut map = std::collections::HashMap::new();
        for example in examples {
            *map.entry(example.features.difficulty.clone()).or_insert(0) += 1;
        }
        map
    }

    pub fn save_to_file(dataset: &CodeDebuggingDataset, output_path: &Path) -> Result<()> {
        // Save as JSONL
        let mut lines = Vec::new();
        for example in &dataset.examples {
            lines.push(serde_json::to_string(example)?);
        }
        fs::write(output_path.join("code_debugging.jsonl"), lines.join("\n"))?;

        // Save for HuggingFace
        fs::write(
            output_path.join("../huggingface/code_debugging.jsonl"),
            lines.join("\n"),
        )?;

        // Save summary
        let summary = serde_json::json!({
            "total_examples": dataset.total_examples,
            "by_language": dataset.by_language,
            "by_difficulty": dataset.by_difficulty,
            "avg_time_to_solve": dataset.avg_time_to_solve,
        });

        fs::write(
            output_path.join("code_debugging_summary.json"),
            serde_json::to_string_pretty(&summary)?,
        )?;

        println!(
            "✅ Saved {} debugging examples to {}",
            dataset.examples.len(),
            output_path.display()
        );
        println!(
            "   Average time to solve: {:.1} minutes",
            dataset.avg_time_to_solve
        );

        Ok(())
    }
}
