use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::extraction_utils::{estimate_tokens, Conversation, ToolCall};

#[derive(Debug, Serialize, Deserialize)]
pub struct AgenticSequence {
    pub id: String,
    pub task_description: String,
    pub complexity: String, // "simple" | "moderate" | "complex" | "expert"
    pub trajectory: Vec<AgenticStep>,
    pub outcome: TaskOutcome,
    pub features: SequenceFeatures,
    pub metadata: SequenceMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgenticStep {
    pub step_number: usize,
    pub user_message: Option<String>,
    pub ai_reasoning: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub ai_response: String,
    pub step_outcome: String, // "success" | "partial" | "failure"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskOutcome {
    pub status: String, // "completed" | "failed" | "abandoned"
    pub success: bool,
    pub total_steps: usize,
    pub total_tool_calls: usize,
    pub files_modified: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SequenceFeatures {
    pub planning_quality: f64,
    pub tool_diversity: f64,
    pub error_recovery_count: usize,
    pub context_switches: usize,
    pub parallel_tool_use: bool,
    pub reads_before_writes: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SequenceMetadata {
    pub conversation_id: String,
    pub tool: String,
    pub language: String,
    pub project: String,
    pub total_tokens: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgenticDataset {
    pub total_sequences: usize,
    pub examples: Vec<AgenticSequence>,
    pub by_complexity: std::collections::HashMap<String, usize>,
    pub avg_steps_per_task: f64,
    pub success_rate: f64,
}

pub struct AgenticToolUseExtractor;

impl AgenticToolUseExtractor {
    pub fn extract(conversations: &[Conversation]) -> Result<AgenticDataset> {
        println!(
            "🤖 Extracting agentic tool use sequences from {} conversations...",
            conversations.len()
        );

        let mut sequences = Vec::new();

        for conv in conversations {
            // Only process conversations with multiple turns (multi-step tasks)
            if conv.messages.len() < 5 {
                continue;
            }

            if let Ok(sequence) = Self::convert_to_sequence(conv) {
                sequences.push(sequence);
            }
        }

        println!("   Extracted {} agentic sequences", sequences.len());

        // Calculate statistics
        let by_complexity = Self::group_by_complexity(&sequences);
        let avg_steps: f64 = sequences
            .iter()
            .map(|s| s.trajectory.len() as f64)
            .sum::<f64>()
            / sequences.len().max(1) as f64;

        let success_rate = sequences.iter().filter(|s| s.outcome.success).count() as f64
            / sequences.len().max(1) as f64;

        Ok(AgenticDataset {
            total_sequences: sequences.len(),
            examples: sequences,
            by_complexity,
            avg_steps_per_task: avg_steps,
            success_rate,
        })
    }

    fn convert_to_sequence(conv: &Conversation) -> Result<AgenticSequence> {
        let task_description = conv
            .messages
            .first()
            .map(|m| m.content.chars().take(200).collect())
            .unwrap_or_default();

        let mut trajectory = Vec::new();
        let mut tool_count = 0;
        let mut files_modified = Vec::new();

        for (idx, msg) in conv.messages.iter().enumerate() {
            let step = AgenticStep {
                step_number: idx,
                user_message: if msg.role == "user" {
                    Some(msg.content.clone())
                } else {
                    None
                },
                ai_reasoning: if msg.role == "assistant" && msg.content.contains("let me") {
                    Some(msg.content.chars().take(100).collect())
                } else {
                    None
                },
                tool_calls: msg.tool_calls.clone(),
                ai_response: msg.content.chars().take(300).collect(),
                step_outcome: if !msg.tool_calls.is_empty()
                    && msg.tool_calls.iter().any(|t| t.success)
                {
                    "success"
                } else {
                    "partial"
                }
                .to_string(),
            };

            tool_count += msg.tool_calls.len();

            // Track files that might have been modified
            for tool_call in &msg.tool_calls {
                if tool_call.tool == "write" || tool_call.tool == "edit" {
                    if let Some(file) = tool_call
                        .parameters
                        .get("file_path")
                        .and_then(|f| f.as_str())
                    {
                        if !files_modified.contains(&file.to_string()) {
                            files_modified.push(file.to_string());
                        }
                    }
                }
            }

            trajectory.push(step);
        }

        let complexity = if trajectory.len() > 20 {
            "expert"
        } else if trajectory.len() > 10 {
            "complex"
        } else if trajectory.len() > 5 {
            "moderate"
        } else {
            "simple"
        }
        .to_string();

        let success = trajectory
            .last()
            .map(|s| s.step_outcome == "success")
            .unwrap_or(false);

        let features = Self::extract_features(&trajectory);

        let total_tokens: usize = conv
            .messages
            .iter()
            .map(|m| estimate_tokens(&m.content))
            .sum();

        Ok(AgenticSequence {
            id: conv.id.clone(),
            task_description,
            complexity,
            trajectory,
            outcome: TaskOutcome {
                status: if success { "completed" } else { "partial" }.to_string(),
                success,
                total_steps: conv.messages.len(),
                total_tool_calls: tool_count,
                files_modified,
            },
            features,
            metadata: SequenceMetadata {
                conversation_id: conv.id.clone(),
                tool: conv.tool.clone(),
                language: "unknown".to_string(), // TODO: Detect from files
                project: "unknown".to_string(),
                total_tokens,
            },
        })
    }

    fn extract_features(trajectory: &[AgenticStep]) -> SequenceFeatures {
        let has_planning = trajectory.iter().any(|s| {
            s.ai_reasoning
                .as_ref()
                .is_some_and(|r| r.contains("plan") || r.contains("first"))
        });

        let unique_tools: std::collections::HashSet<String> = trajectory
            .iter()
            .flat_map(|s| s.tool_calls.iter().map(|t| t.tool.clone()))
            .collect();

        let tool_diversity = if !trajectory.is_empty() {
            unique_tools.len() as f64
                / trajectory
                    .iter()
                    .map(|s| s.tool_calls.len())
                    .sum::<usize>()
                    .max(1) as f64
        } else {
            0.0
        };

        let error_recovery = trajectory
            .iter()
            .filter(|s| s.step_outcome == "failure")
            .count();

        SequenceFeatures {
            planning_quality: if has_planning { 0.8 } else { 0.3 },
            tool_diversity,
            error_recovery_count: error_recovery,
            context_switches: 0,       // TODO: Detect context switches
            parallel_tool_use: false,  // TODO: Detect parallel calls
            reads_before_writes: true, // TODO: Analyze read/write order
        }
    }

    fn group_by_complexity(
        sequences: &[AgenticSequence],
    ) -> std::collections::HashMap<String, usize> {
        let mut map = std::collections::HashMap::new();
        for seq in sequences {
            *map.entry(seq.complexity.clone()).or_insert(0) += 1;
        }
        map
    }

    pub fn save_to_file(dataset: &AgenticDataset, output_path: &Path) -> Result<()> {
        // Save as JSONL
        let mut lines = Vec::new();
        for example in &dataset.examples {
            lines.push(serde_json::to_string(example)?);
        }
        fs::write(output_path.join("agentic_tool_use.jsonl"), lines.join("\n"))?;

        // Save for HuggingFace
        fs::write(
            output_path.join("../huggingface/agentic_tool_use.jsonl"),
            lines.join("\n"),
        )?;

        // Save summary
        #[derive(Serialize)]
        struct Summary {
            total_sequences: usize,
            by_complexity: std::collections::HashMap<String, usize>,
            avg_steps_per_task: f64,
            success_rate: f64,
        }

        let summary = Summary {
            total_sequences: dataset.total_sequences,
            by_complexity: dataset.by_complexity.clone(),
            avg_steps_per_task: dataset.avg_steps_per_task,
            success_rate: dataset.success_rate,
        };

        let summary_json = serde_json::to_string_pretty(&summary)?;
        fs::write(
            output_path.join("agentic_tool_use_summary.json"),
            summary_json,
        )?;

        println!(
            "✅ Saved {} agentic sequences to {}",
            dataset.examples.len(),
            output_path.display()
        );
        println!(
            "   Average steps per task: {:.1}",
            dataset.avg_steps_per_task
        );
        println!("   Success rate: {:.1}%", dataset.success_rate * 100.0);

        Ok(())
    }
}
