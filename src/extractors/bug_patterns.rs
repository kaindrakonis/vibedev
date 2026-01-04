use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::comprehensive_analyzer::ComprehensiveAnalysis;
use crate::extraction_utils::{extract_errors, Conversation, ErrorInstance};

#[derive(Debug, Serialize, Deserialize)]
pub struct BugPattern {
    pub id: String,
    pub error_type: String,
    pub error_message: String,
    pub language: String,
    pub occurrences: usize,
    pub first_seen: String,
    pub last_seen: String,
    pub average_time_to_fix_minutes: f64,
    pub contexts: Vec<ErrorContext>,
    pub common_fixes: Vec<String>,
    pub cost_usd: f64,
    pub learning_status: String, // "mastered" | "improving" | "struggling"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorContext {
    pub conversation_id: String,
    pub error_full: String,
    pub fix_applied: Option<String>,
    pub turns_to_fix: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BugPatternsDataset {
    pub total_errors: usize,
    pub unique_patterns: usize,
    pub patterns: Vec<BugPattern>,
    pub by_language: HashMap<String, usize>,
    pub by_type: HashMap<String, usize>,
    pub total_time_wasted_hours: f64,
    pub total_cost_wasted_usd: f64,
}

pub struct BugPatternsExtractor;

impl BugPatternsExtractor {
    pub fn extract(
        insights: &ComprehensiveAnalysis,
        conversations: &[Conversation],
    ) -> Result<BugPatternsDataset> {
        println!(
            "🐛 Extracting bug patterns from {} conversations...",
            conversations.len()
        );

        // Extract all errors
        let all_errors = extract_errors(conversations);
        println!("   Found {} error instances", all_errors.len());

        // Group by error message pattern
        let mut pattern_groups: HashMap<String, Vec<ErrorInstance>> = HashMap::new();

        for error in all_errors {
            let pattern_key = Self::normalize_error_message(&error.message);
            pattern_groups.entry(pattern_key).or_default().push(error);
        }

        println!(
            "   Identified {} unique error patterns",
            pattern_groups.len()
        );

        // Create bug patterns
        let mut patterns = Vec::new();

        for (pattern_key, errors) in pattern_groups {
            if errors.is_empty() {
                continue;
            }

            let first_error = &errors[0];
            let language = first_error
                .language
                .clone()
                .unwrap_or_else(|| "unknown".to_string());

            let contexts: Vec<ErrorContext> = errors
                .iter()
                .take(10)
                .map(|e| {
                    ErrorContext {
                        conversation_id: e.conversation_id.clone(),
                        error_full: e.message.clone(),
                        fix_applied: e.fix.clone(),
                        turns_to_fix: 3, // TODO: Calculate actual turns
                    }
                })
                .collect();

            let occurrences = errors.len();
            let avg_time_to_fix = 15.0; // TODO: Calculate from actual data
            let cost = occurrences as f64 * 0.05; // Estimate $0.05 per error

            let learning_status = if occurrences > 20 {
                "struggling"
            } else if occurrences > 5 {
                "improving"
            } else {
                "mastered"
            }
            .to_string();

            patterns.push(BugPattern {
                id: format!("bug_{}", patterns.len()),
                error_type: first_error.error_type.clone(),
                error_message: pattern_key.clone(),
                language: language.clone(),
                occurrences,
                first_seen: "2024-09-01".to_string(), // TODO: Extract from timestamps
                last_seen: "2024-12-31".to_string(),
                average_time_to_fix_minutes: avg_time_to_fix,
                contexts,
                common_fixes: vec![], // TODO: Extract from fixes
                cost_usd: cost,
                learning_status,
            });
        }

        // Sort by occurrences (most frequent first)
        patterns.sort_by(|a, b| b.occurrences.cmp(&a.occurrences));

        // Calculate statistics
        let by_language = Self::group_by_language(&patterns);
        let by_type = Self::group_by_type(&patterns);
        let total_time_wasted = patterns
            .iter()
            .map(|p| p.occurrences as f64 * p.average_time_to_fix_minutes / 60.0)
            .sum();
        let total_cost = patterns.iter().map(|p| p.cost_usd).sum();

        Ok(BugPatternsDataset {
            total_errors: insights.advanced.error_patterns.total_errors,
            unique_patterns: patterns.len(),
            patterns,
            by_language,
            by_type,
            total_time_wasted_hours: total_time_wasted,
            total_cost_wasted_usd: total_cost,
        })
    }

    fn normalize_error_message(msg: &str) -> String {
        // Normalize error message to create pattern key
        msg.lines()
            .take(2)
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(100)
            .collect()
    }

    fn group_by_language(patterns: &[BugPattern]) -> HashMap<String, usize> {
        let mut map = HashMap::new();
        for pattern in patterns {
            *map.entry(pattern.language.clone()).or_insert(0) += pattern.occurrences;
        }
        map
    }

    fn group_by_type(patterns: &[BugPattern]) -> HashMap<String, usize> {
        let mut map = HashMap::new();
        for pattern in patterns {
            *map.entry(pattern.error_type.clone()).or_insert(0) += pattern.occurrences;
        }
        map
    }

    pub fn save_to_file(dataset: &BugPatternsDataset, output_path: &Path) -> Result<()> {
        // Save as JSONL (one pattern per line)
        let mut lines = Vec::new();
        for pattern in &dataset.patterns {
            lines.push(serde_json::to_string(pattern)?);
        }
        fs::write(output_path.join("bug_patterns.jsonl"), lines.join("\n"))?;

        // Save summary as JSON
        let summary = serde_json::to_string_pretty(dataset)?;
        fs::write(output_path.join("bug_patterns_summary.json"), summary)?;

        println!(
            "✅ Saved {} bug patterns to {}",
            dataset.patterns.len(),
            output_path.display()
        );
        println!("   Total errors: {}", dataset.total_errors);
        println!("   Unique patterns: {}", dataset.unique_patterns);
        println!(
            "   Time wasted: {:.1} hours",
            dataset.total_time_wasted_hours
        );
        println!("   Cost wasted: ${:.2}", dataset.total_cost_wasted_usd);

        Ok(())
    }
}
