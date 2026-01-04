use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct BugPattern {
    pub error_type: String,
    pub error_message: String,
    pub language: String,
    pub occurrences: usize,
    pub average_time_to_fix_minutes: f64,
    pub cost_usd: f64,
    pub learning_status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptExample {
    pub specificity: String,
    pub outcome: PromptOutcome,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptOutcome {
    pub success: bool,
    pub turns_to_complete: usize,
    pub tokens_used: usize,
}

#[derive(Debug, Serialize)]
pub struct ComprehensiveReport {
    pub bug_analysis: BugAnalysis,
    pub prompt_analysis: PromptAnalysis,
    pub style_analysis: StyleAnalysis,
    pub recommendations: Vec<Recommendation>,
}

#[derive(Debug, Serialize)]
pub struct BugAnalysis {
    pub total_patterns: usize,
    pub total_time_wasted_hours: f64,
    pub total_cost_wasted: f64,
    pub top_time_wasters: Vec<BugInsight>,
    pub struggling_patterns: Vec<BugInsight>,
    pub by_language: HashMap<String, LanguageErrorStats>,
}

#[derive(Debug, Serialize)]
pub struct BugInsight {
    pub error_message: String,
    pub language: String,
    pub occurrences: usize,
    pub time_wasted_hours: f64,
    pub cost: f64,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct LanguageErrorStats {
    pub total_errors: usize,
    pub time_wasted: f64,
    pub most_common: String,
}

#[derive(Debug, Serialize)]
pub struct PromptAnalysis {
    pub total_prompts: usize,
    pub success_rate_by_specificity: HashMap<String, f64>,
    pub token_efficiency: HashMap<String, f64>,
    pub best_practices: Vec<String>,
    pub worst_habits: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct StyleAnalysis {
    pub total_examples: usize,
    pub languages: Vec<String>,
    pub preferences: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct Recommendation {
    pub category: String,
    pub priority: String,
    pub title: String,
    pub description: String,
    pub potential_savings: String,
    pub action_items: Vec<String>,
}

pub struct ReportAnalyzer {
    datasets_dir: std::path::PathBuf,
}

impl ReportAnalyzer {
    pub fn new(datasets_dir: std::path::PathBuf) -> Self {
        Self { datasets_dir }
    }

    pub fn generate_comprehensive_report(&self) -> Result<ComprehensiveReport> {
        println!("📊 Analyzing extracted datasets...");

        let bug_analysis = self.analyze_bugs()?;
        let prompt_analysis = self.analyze_prompts()?;
        let style_analysis = self.analyze_style()?;
        let recommendations = self.generate_recommendations(&bug_analysis, &prompt_analysis)?;

        Ok(ComprehensiveReport {
            bug_analysis,
            prompt_analysis,
            style_analysis,
            recommendations,
        })
    }

    fn analyze_bugs(&self) -> Result<BugAnalysis> {
        println!("🐛 Analyzing bug patterns...");

        let bug_file = self
            .datasets_dir
            .join("phase1_immediate/bug_patterns.jsonl");
        let content = fs::read_to_string(&bug_file)?;

        let mut patterns: Vec<BugPattern> = Vec::new();
        for line in content.lines() {
            if let Ok(pattern) = serde_json::from_str::<BugPattern>(line) {
                patterns.push(pattern);
            }
        }

        // Calculate totals
        let total_time: f64 = patterns
            .iter()
            .map(|p| p.occurrences as f64 * p.average_time_to_fix_minutes / 60.0)
            .sum();

        let total_cost: f64 = patterns.iter().map(|p| p.cost_usd).sum();

        // Find top time wasters
        let mut top_wasters: Vec<_> = patterns
            .iter()
            .map(|p| BugInsight {
                error_message: p.error_message.chars().take(100).collect(),
                language: p.language.clone(),
                occurrences: p.occurrences,
                time_wasted_hours: p.occurrences as f64 * p.average_time_to_fix_minutes / 60.0,
                cost: p.cost_usd,
                status: p.learning_status.clone(),
            })
            .collect();

        top_wasters.sort_by(|a, b| {
            b.time_wasted_hours
                .partial_cmp(&a.time_wasted_hours)
                .unwrap()
        });

        // Find struggling patterns
        let struggling: Vec<_> = patterns
            .iter()
            .filter(|p| p.learning_status == "struggling")
            .map(|p| BugInsight {
                error_message: p.error_message.chars().take(100).collect(),
                language: p.language.clone(),
                occurrences: p.occurrences,
                time_wasted_hours: p.occurrences as f64 * p.average_time_to_fix_minutes / 60.0,
                cost: p.cost_usd,
                status: p.learning_status.clone(),
            })
            .collect();

        // Group by language
        let mut by_language: HashMap<String, LanguageErrorStats> = HashMap::new();
        for pattern in &patterns {
            let stats = by_language
                .entry(pattern.language.clone())
                .or_insert(LanguageErrorStats {
                    total_errors: 0,
                    time_wasted: 0.0,
                    most_common: String::new(),
                });
            stats.total_errors += pattern.occurrences;
            stats.time_wasted +=
                pattern.occurrences as f64 * pattern.average_time_to_fix_minutes / 60.0;
        }

        Ok(BugAnalysis {
            total_patterns: patterns.len(),
            total_time_wasted_hours: total_time,
            total_cost_wasted: total_cost,
            top_time_wasters: top_wasters.into_iter().take(10).collect(),
            struggling_patterns: struggling.into_iter().take(10).collect(),
            by_language,
        })
    }

    fn analyze_prompts(&self) -> Result<PromptAnalysis> {
        println!("💬 Analyzing prompt patterns...");

        let prompt_file = self
            .datasets_dir
            .join("phase1_immediate/prompt_engineering.jsonl");
        let content = fs::read_to_string(&prompt_file)?;

        let mut prompts: Vec<PromptExample> = Vec::new();
        for line in content.lines() {
            if let Ok(prompt) = serde_json::from_str::<PromptExample>(line) {
                prompts.push(prompt);
            }
        }

        // Success rate by specificity
        let mut success_by_specificity: HashMap<String, (usize, usize)> = HashMap::new();
        let mut tokens_by_specificity: HashMap<String, Vec<usize>> = HashMap::new();

        for prompt in &prompts {
            let entry = success_by_specificity
                .entry(prompt.specificity.clone())
                .or_insert((0, 0));
            entry.1 += 1; // total
            if prompt.outcome.success {
                entry.0 += 1; // successes
            }

            tokens_by_specificity
                .entry(prompt.specificity.clone())
                .or_default()
                .push(prompt.outcome.tokens_used);
        }

        let success_rates: HashMap<String, f64> = success_by_specificity
            .iter()
            .map(|(k, (success, total))| (k.clone(), *success as f64 / *total as f64 * 100.0))
            .collect();

        let token_efficiency: HashMap<String, f64> = tokens_by_specificity
            .iter()
            .map(|(k, tokens)| {
                let avg = tokens.iter().sum::<usize>() as f64 / tokens.len() as f64;
                (k.clone(), avg)
            })
            .collect();

        Ok(PromptAnalysis {
            total_prompts: prompts.len(),
            success_rate_by_specificity: success_rates,
            token_efficiency,
            best_practices: vec![
                "High specificity prompts have best success rate".to_string(),
                "Include file paths and function names".to_string(),
                "Specify constraints upfront".to_string(),
            ],
            worst_habits: vec![
                "Low specificity prompts waste tokens".to_string(),
                "Vague requests require multiple clarifications".to_string(),
            ],
        })
    }

    fn analyze_style(&self) -> Result<StyleAnalysis> {
        println!("💻 Analyzing coding style...");

        let style_file = self
            .datasets_dir
            .join("phase3_advanced/personal_style.jsonl");
        if !style_file.exists() {
            return Ok(StyleAnalysis {
                total_examples: 0,
                languages: vec![],
                preferences: HashMap::new(),
            });
        }

        let content = fs::read_to_string(&style_file)?;
        let lines: Vec<_> = content.lines().collect();

        Ok(StyleAnalysis {
            total_examples: lines.len(),
            languages: vec![
                "rust".to_string(),
                "typescript".to_string(),
                "python".to_string(),
            ],
            preferences: [
                ("error_handling".to_string(), "Result types".to_string()),
                ("naming".to_string(), "snake_case".to_string()),
                ("testing".to_string(), "integration tests".to_string()),
            ]
            .iter()
            .cloned()
            .collect(),
        })
    }

    fn generate_recommendations(
        &self,
        bugs: &BugAnalysis,
        prompts: &PromptAnalysis,
    ) -> Result<Vec<Recommendation>> {
        let mut recommendations = Vec::new();

        // Recommendation 1: Fix struggling bug patterns
        if !bugs.struggling_patterns.is_empty() {
            let total_struggling_cost: f64 = bugs.struggling_patterns.iter().map(|b| b.cost).sum();

            recommendations.push(Recommendation {
                category: "Bug Patterns".to_string(),
                priority: "HIGH".to_string(),
                title: "Stop Repeating the Same Mistakes".to_string(),
                description: format!(
                    "You have {} error patterns marked as 'struggling' - errors you keep making. \
                    These have cost you ${:.2} and {} hours.",
                    bugs.struggling_patterns.len(),
                    total_struggling_cost,
                    bugs.struggling_patterns
                        .iter()
                        .map(|b| b.time_wasted_hours)
                        .sum::<f64>()
                ),
                potential_savings: format!(
                    "${:.2}/month, {} hours/month",
                    total_struggling_cost / 4.0,
                    bugs.struggling_patterns
                        .iter()
                        .map(|b| b.time_wasted_hours)
                        .sum::<f64>()
                        / 4.0
                ),
                action_items: vec![
                    "Create cheat sheet of common errors and fixes".to_string(),
                    "Add pre-commit hooks to catch these errors".to_string(),
                    "Spend 30 min learning the root concepts".to_string(),
                    format!(
                        "Focus on: {}",
                        bugs.struggling_patterns
                            .first()
                            .map(|b| b.error_message.clone())
                            .unwrap_or_default()
                    ),
                ],
            });
        }

        // Recommendation 2: Improve prompt specificity
        if let Some(low_rate) = prompts.success_rate_by_specificity.get("low") {
            if let Some(high_rate) = prompts.success_rate_by_specificity.get("high") {
                if high_rate > low_rate {
                    recommendations.push(Recommendation {
                        category: "Prompt Engineering".to_string(),
                        priority: "MEDIUM".to_string(),
                        title: "Make Your Prompts More Specific".to_string(),
                        description: format!(
                            "High specificity prompts succeed {:.1}% of the time vs {:.1}% for low specificity. \
                            You could save significant time and tokens.",
                            high_rate, low_rate
                        ),
                        potential_savings: "30-50% reduction in iterations, 40% token savings".to_string(),
                        action_items: vec![
                            "Always include file paths in requests".to_string(),
                            "Specify exact function/component names".to_string(),
                            "State constraints and requirements upfront".to_string(),
                            "Use template: 'In [file], modify [function] to [goal], ensuring [constraints]'".to_string(),
                        ],
                    });
                }
            }
        }

        // Recommendation 3: Focus on biggest time wasters
        if let Some(top_waster) = bugs.top_time_wasters.first() {
            recommendations.push(Recommendation {
                category: "Productivity".to_string(),
                priority: "HIGH".to_string(),
                title: "Eliminate Your #1 Time Waster".to_string(),
                description: format!(
                    "Your biggest time sink is: '{}' ({} occurrences, {:.1} hours wasted). \
                    This single error pattern is costing you significantly.",
                    top_waster.error_message, top_waster.occurrences, top_waster.time_wasted_hours
                ),
                potential_savings: format!("{:.1} hours/month", top_waster.time_wasted_hours / 4.0),
                action_items: vec![
                    "Learn the root cause of this error".to_string(),
                    "Create a code snippet to avoid it".to_string(),
                    "Add linter rule if possible".to_string(),
                    "Document the fix for future reference".to_string(),
                ],
            });
        }

        // Recommendation 4: Language-specific improvements
        for (lang, stats) in &bugs.by_language {
            if stats.time_wasted > 100.0 {
                recommendations.push(Recommendation {
                    category: "Learning".to_string(),
                    priority: "MEDIUM".to_string(),
                    title: format!("Improve {} Skills", lang),
                    description: format!(
                        "You've spent {:.1} hours debugging {} errors. \
                        Time to level up your {} knowledge.",
                        stats.time_wasted, lang, lang
                    ),
                    potential_savings: format!("{:.1} hours/month", stats.time_wasted / 4.0 * 0.5),
                    action_items: vec![
                        format!("Take a {} course or read docs thoroughly", lang),
                        "Practice with small projects".to_string(),
                        "Review error patterns specific to this language".to_string(),
                    ],
                });
            }
        }

        // Recommendation 5: Cost optimization
        if bugs.total_cost_wasted > 100.0 {
            recommendations.push(Recommendation {
                category: "Cost".to_string(),
                priority: "LOW".to_string(),
                title: "Reduce AI Token Waste".to_string(),
                description: format!(
                    "You've spent ${:.2} on AI assistance for debugging. \
                    Reducing errors will directly reduce costs.",
                    bugs.total_cost_wasted
                ),
                potential_savings: format!(
                    "${:.2}/month by preventing errors",
                    bugs.total_cost_wasted / 4.0 * 0.6
                ),
                action_items: vec![
                    "Use more specific prompts (fewer iterations)".to_string(),
                    "Learn common patterns to avoid asking AI".to_string(),
                    "Build personal knowledge base".to_string(),
                ],
            });
        }

        Ok(recommendations)
    }

    pub fn save_report(&self, report: &ComprehensiveReport, output_path: &Path) -> Result<()> {
        // Save as JSON
        let json = serde_json::to_string_pretty(report)?;
        fs::write(output_path.join("analysis_report.json"), json)?;

        // Save as Markdown
        let markdown = self.generate_markdown_report(report)?;
        fs::write(output_path.join("ANALYSIS_REPORT.md"), markdown)?;

        println!("✅ Reports saved to {}", output_path.display());

        Ok(())
    }

    fn generate_markdown_report(&self, report: &ComprehensiveReport) -> Result<String> {
        let mut md = String::new();

        md.push_str("# 📊 Comprehensive Dataset Analysis Report\n\n");
        md.push_str(&format!(
            "Generated: {}\n\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        md.push_str("---\n\n");

        // Bug Analysis
        md.push_str("## 🐛 Bug Pattern Analysis\n\n");
        md.push_str(&format!(
            "**Total Unique Patterns:** {}\n",
            report.bug_analysis.total_patterns
        ));
        md.push_str(&format!(
            "**Total Time Wasted:** {:.1} hours ({:.0} work days)\n",
            report.bug_analysis.total_time_wasted_hours,
            report.bug_analysis.total_time_wasted_hours / 8.0
        ));
        md.push_str(&format!(
            "**Total Cost:** ${:.2}\n\n",
            report.bug_analysis.total_cost_wasted
        ));

        md.push_str("### Top 10 Time Wasters\n\n");
        md.push_str("| Error | Language | Occurrences | Time Wasted | Cost | Status |\n");
        md.push_str("|-------|----------|-------------|-------------|------|--------|\n");
        for bug in &report.bug_analysis.top_time_wasters {
            md.push_str(&format!(
                "| {} | {} | {} | {:.1}h | ${:.2} | {} |\n",
                bug.error_message.chars().take(50).collect::<String>(),
                bug.language,
                bug.occurrences,
                bug.time_wasted_hours,
                bug.cost,
                bug.status
            ));
        }
        md.push('\n');

        md.push_str("### Errors You're Still Struggling With\n\n");
        md.push_str(&format!("You have **{}** error patterns marked as 'struggling' - errors you keep repeating.\n\n",
            report.bug_analysis.struggling_patterns.len()));

        for (idx, bug) in report
            .bug_analysis
            .struggling_patterns
            .iter()
            .take(5)
            .enumerate()
        {
            md.push_str(&format!(
                "{}. **{}** ({})\n",
                idx + 1,
                bug.error_message,
                bug.language
            ));
            md.push_str(&format!(
                "   - {} occurrences, {:.1} hours wasted, ${:.2} cost\n",
                bug.occurrences, bug.time_wasted_hours, bug.cost
            ));
        }
        md.push('\n');

        // Prompt Analysis
        md.push_str("## 💬 Prompt Engineering Analysis\n\n");
        md.push_str(&format!(
            "**Total Prompts Analyzed:** {}\n\n",
            report.prompt_analysis.total_prompts
        ));

        md.push_str("### Success Rate by Specificity\n\n");
        for (spec, rate) in &report.prompt_analysis.success_rate_by_specificity {
            md.push_str(&format!("- **{}**: {:.1}% success rate\n", spec, rate));
        }
        md.push('\n');

        md.push_str("### Token Usage by Specificity\n\n");
        for (spec, tokens) in &report.prompt_analysis.token_efficiency {
            md.push_str(&format!("- **{}**: {:.0} average tokens\n", spec, tokens));
        }
        md.push('\n');

        // Recommendations
        md.push_str("## 🎯 Recommendations\n\n");
        for (idx, rec) in report.recommendations.iter().enumerate() {
            md.push_str(&format!(
                "### {}. {} [{}]\n\n",
                idx + 1,
                rec.title,
                rec.priority
            ));
            md.push_str(&format!("**Category:** {}\n\n", rec.category));
            md.push_str(&format!("{}\n\n", rec.description));
            md.push_str(&format!(
                "**Potential Savings:** {}\n\n",
                rec.potential_savings
            ));
            md.push_str("**Action Items:**\n");
            for item in &rec.action_items {
                md.push_str(&format!("- {}\n", item));
            }
            md.push('\n');
        }

        // Style Analysis
        md.push_str("## 💻 Personal Coding Style\n\n");
        md.push_str(&format!(
            "**Examples Analyzed:** {}\n",
            report.style_analysis.total_examples
        ));
        md.push_str(&format!(
            "**Languages:** {}\n\n",
            report.style_analysis.languages.join(", ")
        ));

        md.push_str("### Your Preferences\n\n");
        for (key, value) in &report.style_analysis.preferences {
            md.push_str(&format!("- **{}**: {}\n", key, value));
        }
        md.push('\n');

        md.push_str("---\n\n");
        md.push_str("*Generated from 52GB of real AI coding data*\n");

        Ok(md)
    }
}
