use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::extraction_utils::{estimate_tokens, Conversation};

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptExample {
    pub id: String,
    pub prompt: String,
    pub prompt_type: String, // "request" | "question" | "command" | "clarification"
    pub specificity: String, // "low" | "medium" | "high"
    pub context_provided: bool,
    pub file_paths_included: bool,
    pub constraints_specified: bool,

    pub outcome: PromptOutcome,
    pub better_version: Option<String>,
    pub improvement_suggestions: Vec<String>,

    pub tokens: usize,
    pub conversation_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptOutcome {
    pub success: bool,
    pub turns_to_complete: usize,
    pub tokens_used: usize,
    pub needed_clarification: bool,
    pub ai_response_quality: String, // "excellent" | "good" | "poor"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPattern {
    pub pattern: String,
    pub success_rate: f64,
    pub avg_tokens: usize,
    pub examples: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptEngineeringDataset {
    pub total_prompts: usize,
    pub examples: Vec<PromptExample>,
    pub best_practices: Vec<String>,
    pub anti_patterns: Vec<String>,
    pub effective_patterns: Vec<PromptPattern>,
    pub token_efficiency_score: f64,
}

pub struct PromptEngineeringExtractor;

impl PromptEngineeringExtractor {
    pub fn extract(conversations: &[Conversation]) -> Result<PromptEngineeringDataset> {
        println!(
            "💬 Extracting prompt patterns from {} conversations...",
            conversations.len()
        );

        let mut examples = Vec::new();

        for conv in conversations {
            for (idx, msg) in conv.messages.iter().enumerate() {
                if msg.role == "user" || msg.role == "human" {
                    let example = Self::analyze_prompt(msg, conv, idx);
                    examples.push(example);
                }
            }
        }

        println!("   Analyzed {} user prompts", examples.len());

        // Identify patterns
        let effective_patterns = Self::identify_patterns(&examples);
        let best_practices = Self::extract_best_practices(&examples);
        let anti_patterns = Self::extract_anti_patterns(&examples);

        let token_efficiency = Self::calculate_token_efficiency(&examples);

        Ok(PromptEngineeringDataset {
            total_prompts: examples.len(),
            examples,
            best_practices,
            anti_patterns,
            effective_patterns,
            token_efficiency_score: token_efficiency,
        })
    }

    fn analyze_prompt(
        msg: &crate::extraction_utils::Message,
        conv: &Conversation,
        idx: usize,
    ) -> PromptExample {
        let prompt = &msg.content;
        let tokens = estimate_tokens(prompt);

        // Analyze prompt characteristics
        let has_file_paths = prompt.contains(".rs")
            || prompt.contains(".ts")
            || prompt.contains(".js")
            || prompt.contains("src/");
        let has_function_names = prompt.contains("fn ") || prompt.contains("function ");
        let has_constraints = prompt.contains("make sure")
            || prompt.contains("ensure")
            || prompt.contains("must")
            || prompt.contains("should");

        let specificity = if has_file_paths && has_function_names && has_constraints {
            "high"
        } else if has_file_paths || has_function_names {
            "medium"
        } else {
            "low"
        }
        .to_string();

        let prompt_type = if prompt.contains("?") {
            "question"
        } else if prompt.starts_with("refactor")
            || prompt.starts_with("implement")
            || prompt.starts_with("add")
            || prompt.starts_with("create")
        {
            "command"
        } else {
            "request"
        }
        .to_string();

        // Estimate outcome based on following messages
        let turns_to_complete = conv.messages.len().saturating_sub(idx).min(10);
        let success = turns_to_complete < 5; // If completed quickly, likely successful

        PromptExample {
            id: format!("prompt_{}_{}", conv.id, idx),
            prompt: prompt.chars().take(500).collect(), // Truncate for size
            prompt_type,
            specificity: specificity.clone(),
            context_provided: has_file_paths,
            file_paths_included: has_file_paths,
            constraints_specified: has_constraints,
            outcome: PromptOutcome {
                success,
                turns_to_complete,
                tokens_used: tokens * turns_to_complete,
                needed_clarification: turns_to_complete > 3,
                ai_response_quality: if success { "good" } else { "poor" }.to_string(),
            },
            better_version: if specificity == "low" {
                Some(Self::suggest_improvement(prompt))
            } else {
                None
            },
            improvement_suggestions: Self::generate_suggestions(prompt, &specificity),
            tokens,
            conversation_id: conv.id.clone(),
        }
    }

    fn suggest_improvement(prompt: &str) -> String {
        format!(
            "Improved: {} (add specific file paths, function names, and clear constraints)",
            prompt.chars().take(100).collect::<String>()
        )
    }

    fn generate_suggestions(prompt: &str, specificity: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        if specificity == "low" {
            suggestions.push("Add specific file paths".to_string());
            suggestions.push("Specify exact function/component names".to_string());
            suggestions.push("Include constraints and requirements".to_string());
        }

        if !prompt.contains("test") {
            suggestions.push("Consider requesting tests".to_string());
        }

        suggestions
    }

    fn identify_patterns(examples: &[PromptExample]) -> Vec<PromptPattern> {
        let mut patterns = Vec::new();

        // Pattern: File path + function + constraint = high success
        let specific_prompts: Vec<_> = examples
            .iter()
            .filter(|e| e.specificity == "high")
            .collect();

        if !specific_prompts.is_empty() {
            let success_rate = specific_prompts
                .iter()
                .filter(|e| e.outcome.success)
                .count() as f64
                / specific_prompts.len() as f64;

            patterns.push(PromptPattern {
                pattern: "Specific: file + function + constraints".to_string(),
                success_rate,
                avg_tokens: specific_prompts.iter().map(|e| e.tokens).sum::<usize>()
                    / specific_prompts.len(),
                examples: specific_prompts
                    .iter()
                    .take(3)
                    .map(|e| e.prompt.clone())
                    .collect(),
            });
        }

        patterns
    }

    fn extract_best_practices(examples: &[PromptExample]) -> Vec<String> {
        let mut practices = Vec::new();

        // Analyze successful examples for patterns
        let successful: Vec<_> = examples.iter().filter(|e| e.outcome.success).collect();

        // Check for file path mentions
        let has_paths = successful
            .iter()
            .filter(|e| {
                e.prompt.contains('/') || e.prompt.contains(".rs") || e.prompt.contains(".ts")
            })
            .count();
        if has_paths > successful.len() / 2 {
            practices.push("Include specific file paths in requests".to_string());
        }

        // Check for specific function/class mentions
        let has_specifics = successful
            .iter()
            .filter(|e| {
                e.prompt.contains("function")
                    || e.prompt.contains("fn ")
                    || e.prompt.contains("class ")
            })
            .count();
        if has_specifics > successful.len() / 3 {
            practices.push("Specify exact function/component names".to_string());
        }

        // Check for test mentions
        let has_tests = successful
            .iter()
            .filter(|e| e.prompt.contains("test") || e.prompt.contains("spec"))
            .count();
        if has_tests > 0 {
            practices.push("Request tests alongside implementation".to_string());
        }

        // Add defaults if we didn't find enough
        if practices.is_empty() {
            practices.push("State constraints and requirements upfront".to_string());
            practices.push("Provide context about the project".to_string());
        }

        practices
    }

    fn extract_anti_patterns(examples: &[PromptExample]) -> Vec<String> {
        let mut anti_patterns = Vec::new();

        // Analyze failed examples for patterns
        let failed: Vec<_> = examples.iter().filter(|e| !e.outcome.success).collect();

        // Check for vague requests
        let vague = failed
            .iter()
            .filter(|e| {
                e.prompt.len() < 50 || e.prompt.contains("make it") || e.prompt.contains("fix this")
            })
            .count();
        if vague > failed.len() / 2 {
            anti_patterns.push("Vague requests like 'make it better' or 'fix this'".to_string());
        }

        // Check for missing context
        let no_context = failed
            .iter()
            .filter(|e| !e.prompt.contains('/') && !e.prompt.contains('.'))
            .count();
        if no_context > failed.len() / 2 {
            anti_patterns.push("No file paths or context provided".to_string());
        }

        // Check for long prompts (might have multiple requests)
        let too_long = failed.iter().filter(|e| e.prompt.len() > 1000).count();
        if too_long > 0 {
            anti_patterns.push("Multiple unrelated requests in one prompt".to_string());
        }

        if anti_patterns.is_empty() {
            anti_patterns.push("Unclear success criteria".to_string());
        }

        anti_patterns
    }

    fn calculate_token_efficiency(examples: &[PromptExample]) -> f64 {
        if examples.is_empty() {
            return 0.0;
        }

        let total_tokens: usize = examples.iter().map(|e| e.outcome.tokens_used).sum();
        let successful: usize = examples.iter().filter(|e| e.outcome.success).count();

        // Calculate efficiency based on tokens per successful outcome
        let avg_tokens_per_example = total_tokens as f64 / examples.len() as f64;
        let success_rate = successful as f64 / examples.len() as f64;

        // Efficiency = success rate weighted by inverse of token usage
        // Lower tokens + higher success = higher efficiency
        let token_penalty = (avg_tokens_per_example / 1000.0).min(1.0); // Normalize by 1000 tokens
        success_rate * 100.0 * (1.0 - token_penalty * 0.5)
    }

    pub fn save_to_file(dataset: &PromptEngineeringDataset, output_path: &Path) -> Result<()> {
        // Save examples as JSONL
        let mut lines = Vec::new();
        for example in &dataset.examples {
            lines.push(serde_json::to_string(example)?);
        }
        fs::write(
            output_path.join("prompt_engineering.jsonl"),
            lines.join("\n"),
        )?;

        // Save summary
        #[derive(Serialize)]
        struct Summary {
            total_prompts: usize,
            best_practices: Vec<String>,
            anti_patterns: Vec<String>,
            effective_patterns: Vec<PromptPattern>,
            token_efficiency_score: f64,
        }

        let summary = Summary {
            total_prompts: dataset.total_prompts,
            best_practices: dataset.best_practices.clone(),
            anti_patterns: dataset.anti_patterns.clone(),
            effective_patterns: dataset.effective_patterns.clone(),
            token_efficiency_score: dataset.token_efficiency_score,
        };

        let summary_json = serde_json::to_string_pretty(&summary)?;
        fs::write(
            output_path.join("prompt_engineering_summary.json"),
            summary_json,
        )?;

        println!(
            "✅ Saved {} prompt examples to {}",
            dataset.examples.len(),
            output_path.display()
        );
        println!(
            "   Token efficiency: {:.1}%",
            dataset.token_efficiency_score
        );
        println!(
            "   Best practices identified: {}",
            dataset.best_practices.len()
        );

        Ok(())
    }
}
