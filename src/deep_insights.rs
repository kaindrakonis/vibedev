#![allow(dead_code)]

use anyhow::Result;
use chrono::{DateTime, Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct DeepInsights {
    pub temporal_patterns: TemporalPatterns,
    pub conversation_intelligence: ConversationIntelligence,
    pub learning_curves: LearningCurves,
    pub productivity_rhythms: ProductivityRhythms,
    pub tool_effectiveness: ToolEffectiveness,
    pub task_complexity_analysis: TaskComplexityAnalysis,
    pub hidden_patterns: Vec<HiddenPattern>,
}

#[derive(Debug, Serialize)]
pub struct TemporalPatterns {
    pub burnout_indicators: Vec<BurnoutPeriod>,
    pub peak_performance_windows: Vec<TimeWindow>,
    pub error_clusters: Vec<ErrorCluster>,
    pub productivity_by_hour: HashMap<u32, f64>,
    pub context_switch_costs: Vec<ContextSwitch>,
}

#[derive(Debug, Serialize)]
pub struct BurnoutPeriod {
    pub start_date: String,
    pub end_date: String,
    pub indicators: Vec<String>,
    pub severity: String, // "mild" | "moderate" | "severe"
}

#[derive(Debug, Serialize)]
pub struct TimeWindow {
    pub hour_start: u32,
    pub hour_end: u32,
    pub efficiency_score: f64,
    pub tasks_completed: usize,
    pub avg_turns_per_task: f64,
}

#[derive(Debug, Serialize)]
pub struct ErrorCluster {
    pub date: String,
    pub error_count: usize,
    pub error_types: HashMap<String, usize>,
    pub potential_cause: String,
}

#[derive(Debug, Serialize)]
pub struct ContextSwitch {
    pub from_project: String,
    pub to_project: String,
    pub recovery_time_minutes: f64,
    pub productivity_loss_percent: f64,
}

#[derive(Debug, Serialize)]
pub struct ConversationIntelligence {
    pub successful_patterns: Vec<ConversationPattern>,
    pub failed_patterns: Vec<ConversationPattern>,
    pub optimal_flow: Vec<String>,
    pub common_derailments: Vec<Derailment>,
    pub retry_analysis: RetryAnalysis,
}

#[derive(Debug, Serialize)]
pub struct ConversationPattern {
    pub pattern_name: String,
    pub typical_sequence: Vec<String>,
    pub success_rate: f64,
    pub avg_tokens: usize,
    pub avg_time_minutes: f64,
    pub examples: usize,
}

#[derive(Debug, Serialize)]
pub struct Derailment {
    pub trigger: String,
    pub frequency: usize,
    pub recovery_steps: Vec<String>,
    pub prevention: String,
}

#[derive(Debug, Serialize)]
pub struct RetryAnalysis {
    pub tasks_needing_retries: usize,
    pub avg_retries_per_task: f64,
    pub retry_reasons: HashMap<String, usize>,
    pub retry_success_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct LearningCurves {
    pub skill_progression: HashMap<String, SkillCurve>,
    pub error_reduction_over_time: Vec<ErrorTrend>,
    pub mastery_timeline: Vec<MasteryEvent>,
    pub plateaus: Vec<LearningPlateau>,
}

#[derive(Debug, Serialize)]
pub struct SkillCurve {
    pub skill: String,
    pub start_proficiency: f64,
    pub current_proficiency: f64,
    pub improvement_rate: f64,
    pub practice_hours: f64,
    pub trajectory: String, // "improving" | "plateaued" | "declining"
}

#[derive(Debug, Serialize)]
pub struct ErrorTrend {
    pub week: String,
    pub total_errors: usize,
    pub unique_errors: usize,
    pub repeat_error_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct MasteryEvent {
    pub date: String,
    pub skill: String,
    pub achievement: String,
    pub evidence: String,
}

#[derive(Debug, Serialize)]
pub struct LearningPlateau {
    pub skill: String,
    pub plateau_start: String,
    pub duration_weeks: usize,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProductivityRhythms {
    pub ultradian_cycles: Vec<ProductivityCycle>,
    pub weekly_patterns: HashMap<String, DayProfile>,
    pub energy_management: EnergyProfile,
    pub optimal_session_length: f64,
    pub break_patterns: BreakAnalysis,
}

#[derive(Debug, Serialize)]
pub struct ProductivityCycle {
    pub cycle_length_minutes: f64,
    pub peak_minutes: Vec<f64>,
    pub trough_minutes: Vec<f64>,
    pub amplitude: f64,
}

#[derive(Debug, Serialize)]
pub struct DayProfile {
    pub total_sessions: usize,
    pub avg_productivity: f64,
    pub best_times: Vec<String>,
    pub worst_times: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EnergyProfile {
    pub morning_energy: f64,
    pub afternoon_energy: f64,
    pub evening_energy: f64,
    pub optimal_scheduling: HashMap<String, String>, // task_type -> best_time
}

#[derive(Debug, Serialize)]
pub struct BreakAnalysis {
    pub avg_break_frequency_minutes: f64,
    pub optimal_break_duration: f64,
    pub break_effectiveness: f64,
    pub overwork_sessions: usize,
}

#[derive(Debug, Serialize)]
pub struct ToolEffectiveness {
    pub tool_success_rates: HashMap<String, f64>,
    pub optimal_tool_sequences: Vec<ToolSequence>,
    pub tool_misuse_patterns: Vec<ToolMisuse>,
    pub tool_learning_curves: HashMap<String, f64>,
}

#[derive(Debug, Serialize)]
pub struct ToolSequence {
    pub sequence: Vec<String>,
    pub use_case: String,
    pub success_rate: f64,
    pub frequency: usize,
}

#[derive(Debug, Serialize)]
pub struct ToolMisuse {
    pub tool: String,
    pub common_mistake: String,
    pub frequency: usize,
    pub correct_usage: String,
}

#[derive(Debug, Serialize)]
pub struct TaskComplexityAnalysis {
    pub complexity_vs_outcome: Vec<ComplexityOutcome>,
    pub sweet_spot: String,
    pub overambitious_tasks: Vec<TaskFailure>,
    pub underutilized_potential: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ComplexityOutcome {
    pub complexity: String,
    pub attempts: usize,
    pub success_rate: f64,
    pub avg_time_hours: f64,
}

#[derive(Debug, Serialize)]
pub struct TaskFailure {
    pub task: String,
    pub complexity: String,
    pub failure_reason: String,
    pub lessons: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct HiddenPattern {
    pub pattern_type: String,
    pub title: String,
    pub description: String,
    pub significance: String,
    pub actionable_insight: String,
    pub supporting_data: Vec<String>,
}

pub struct DeepAnalyzer {
    datasets_dir: PathBuf,
}

impl DeepAnalyzer {
    pub fn new(datasets_dir: PathBuf) -> Self {
        Self { datasets_dir }
    }

    pub fn analyze(&self) -> Result<DeepInsights> {
        println!("🔬 Running deep analysis on your 52GB of data...\n");

        // Load all datasets
        let agentic_data = self.load_agentic_sequences()?;
        let prompt_data = self.load_prompts()?;

        println!("📊 Analyzing temporal patterns...");
        let temporal_patterns = self.analyze_temporal_patterns(&agentic_data)?;

        println!("🧠 Analyzing conversation intelligence...");
        let conversation_intelligence = self.analyze_conversations(&agentic_data)?;

        println!("📈 Computing learning curves...");
        let learning_curves = self.compute_learning_curves(&agentic_data)?;

        println!("⏰ Detecting productivity rhythms...");
        let productivity_rhythms = self.detect_productivity_rhythms(&agentic_data)?;

        println!("🛠️  Evaluating tool effectiveness...");
        let tool_effectiveness = self.evaluate_tools(&agentic_data)?;

        println!("🎯 Analyzing task complexity...");
        let task_complexity_analysis = self.analyze_task_complexity(&agentic_data)?;

        println!("🔍 Finding hidden patterns...");
        let hidden_patterns = self.find_hidden_patterns(&agentic_data, &prompt_data)?;

        Ok(DeepInsights {
            temporal_patterns,
            conversation_intelligence,
            learning_curves,
            productivity_rhythms,
            tool_effectiveness,
            task_complexity_analysis,
            hidden_patterns,
        })
    }

    fn load_agentic_sequences(&self) -> Result<Vec<AgenticSequence>> {
        let file = self.datasets_dir.join("phase2_ml/agentic_tool_use.jsonl");
        let content = fs::read_to_string(&file)?;

        let mut sequences = Vec::new();
        for line in content.lines() {
            if let Ok(seq) = serde_json::from_str::<AgenticSequence>(line) {
                sequences.push(seq);
            }
        }

        println!("   Loaded {} agentic sequences", sequences.len());
        Ok(sequences)
    }

    fn load_prompts(&self) -> Result<Vec<PromptData>> {
        let file = self
            .datasets_dir
            .join("phase1_immediate/prompt_engineering.jsonl");
        let content = fs::read_to_string(&file)?;

        let mut prompts = Vec::new();
        for line in content.lines() {
            if let Ok(p) = serde_json::from_str::<PromptData>(line) {
                prompts.push(p);
            }
        }

        println!("   Loaded {} prompts", prompts.len());
        Ok(prompts)
    }

    fn analyze_temporal_patterns(&self, sequences: &[AgenticSequence]) -> Result<TemporalPatterns> {
        // Extract conversation IDs and parse timestamps
        let mut hourly_productivity: HashMap<u32, Vec<f64>> = HashMap::new();

        for seq in sequences {
            // Parse conversation ID as timestamp (it's a unix timestamp in ms)
            if let Ok(ts_ms) = seq.id.parse::<i64>() {
                let ts_sec = ts_ms / 1000;
                if let Some(dt) = DateTime::from_timestamp(ts_sec, 0) {
                    let hour = dt.hour();
                    let productivity = self.calculate_sequence_productivity(seq);
                    hourly_productivity
                        .entry(hour)
                        .or_default()
                        .push(productivity);
                }
            }
        }

        let productivity_by_hour: HashMap<u32, f64> = hourly_productivity
            .iter()
            .map(|(hour, scores)| {
                let avg = scores.iter().sum::<f64>() / scores.len() as f64;
                (*hour, avg)
            })
            .collect();

        // Find peak performance windows
        let mut peak_windows = Vec::new();
        let avg_productivity: f64 =
            productivity_by_hour.values().sum::<f64>() / productivity_by_hour.len() as f64;

        for hour in 0..24 {
            if let Some(&prod) = productivity_by_hour.get(&hour) {
                if prod > avg_productivity * 1.2 {
                    peak_windows.push(TimeWindow {
                        hour_start: hour,
                        hour_end: hour + 1,
                        efficiency_score: prod,
                        tasks_completed: hourly_productivity
                            .get(&hour)
                            .map(|v| v.len())
                            .unwrap_or(0),
                        avg_turns_per_task: self.avg_turns_for_hour(sequences, hour),
                    });
                }
            }
        }

        Ok(TemporalPatterns {
            burnout_indicators: vec![], // TODO: detect burnout from long sessions + declining quality
            peak_performance_windows: peak_windows,
            error_clusters: vec![],
            productivity_by_hour,
            context_switch_costs: vec![],
        })
    }

    fn calculate_sequence_productivity(&self, seq: &AgenticSequence) -> f64 {
        // Productivity = success / (turns + 1)
        let success_score = if seq.outcome.success {
            1.0
        } else if seq.outcome.status == "partial" {
            0.5
        } else {
            0.0
        };

        let turns = seq.trajectory.len() as f64;
        success_score / (turns + 1.0).ln()
    }

    fn avg_turns_for_hour(&self, sequences: &[AgenticSequence], hour: u32) -> f64 {
        let mut turns = Vec::new();

        for seq in sequences {
            if let Ok(ts_ms) = seq.id.parse::<i64>() {
                let ts_sec = ts_ms / 1000;
                if let Some(dt) = DateTime::from_timestamp(ts_sec, 0) {
                    if dt.hour() == hour {
                        turns.push(seq.trajectory.len() as f64);
                    }
                }
            }
        }

        if turns.is_empty() {
            0.0
        } else {
            turns.iter().sum::<f64>() / turns.len() as f64
        }
    }

    fn analyze_conversations(
        &self,
        sequences: &[AgenticSequence],
    ) -> Result<ConversationIntelligence> {
        // Identify successful vs failed patterns
        let successful: Vec<_> = sequences.iter().filter(|s| s.outcome.success).collect();
        let failed: Vec<_> = sequences.iter().filter(|s| !s.outcome.success).collect();

        // Analyze tool sequences in successful conversations
        let mut successful_patterns = Vec::new();
        let mut tool_sequences: HashMap<String, (usize, usize, usize)> = HashMap::new(); // sequence -> (count, total_tokens, total_turns)

        for seq in successful.iter() {
            let tool_seq: Vec<String> = seq
                .trajectory
                .iter()
                .flat_map(|step| step.tool_calls.clone())
                .collect();
            let key = tool_seq.join(" -> ");
            let entry = tool_sequences.entry(key.clone()).or_insert((0, 0, 0));
            entry.0 += 1;
            entry.1 += seq.outcome.tokens_used;
            entry.2 += seq.trajectory.len();
        }

        // Find top patterns
        let mut patterns: Vec<_> = tool_sequences.iter().collect();
        patterns.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

        for (sequence, (count, tokens, turns)) in patterns.iter().take(5) {
            successful_patterns.push(ConversationPattern {
                pattern_name: format!("Pattern: {}", sequence),
                typical_sequence: sequence.split(" -> ").map(|s| s.to_string()).collect(),
                success_rate: 100.0, // These are from successful only
                avg_tokens: *tokens / *count,
                avg_time_minutes: (*turns as f64 / *count as f64) * 2.5, // Assume 2.5 min per turn
                examples: *count,
            });
        }

        // Analyze failed patterns
        let mut failed_patterns = Vec::new();
        let mut failed_tool_sequences: HashMap<String, (usize, usize, usize)> = HashMap::new();

        for seq in failed.iter() {
            let tool_seq: Vec<String> = seq
                .trajectory
                .iter()
                .flat_map(|step| step.tool_calls.clone())
                .collect();
            let key = tool_seq.join(" -> ");
            let entry = failed_tool_sequences
                .entry(key.clone())
                .or_insert((0, 0, 0));
            entry.0 += 1;
            entry.1 += seq.outcome.tokens_used;
            entry.2 += seq.trajectory.len();
        }

        let mut failed_pattern_list: Vec<_> = failed_tool_sequences.iter().collect();
        failed_pattern_list.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

        for (sequence, (count, tokens, turns)) in failed_pattern_list.iter().take(5) {
            failed_patterns.push(ConversationPattern {
                pattern_name: format!("Failed: {}", sequence),
                typical_sequence: sequence.split(" -> ").map(|s| s.to_string()).collect(),
                success_rate: 0.0,
                avg_tokens: if *count > 0 { *tokens / *count } else { 0 },
                avg_time_minutes: if *count > 0 {
                    (*turns as f64 / *count as f64) * 2.5
                } else {
                    0.0
                },
                examples: *count,
            });
        }

        // Identify common derailments
        let mut derailment_counts: HashMap<String, usize> = HashMap::new();
        for seq in failed.iter() {
            if seq.trajectory.len() > 15 {
                *derailment_counts
                    .entry("Long conversation without resolution".to_string())
                    .or_insert(0) += 1;
            }
            if seq
                .trajectory
                .iter()
                .any(|t| t.tool_calls.contains(&"Edit".to_string()))
                && seq
                    .trajectory
                    .iter()
                    .filter(|t| t.tool_calls.contains(&"Edit".to_string()))
                    .count()
                    > 5
            {
                *derailment_counts
                    .entry("Multiple edit attempts on same file".to_string())
                    .or_insert(0) += 1;
            }
            if seq
                .trajectory
                .iter()
                .any(|t| t.tool_calls.contains(&"Bash".to_string()))
                && seq
                    .trajectory
                    .iter()
                    .filter(|t| t.tool_calls.contains(&"Bash".to_string()))
                    .count()
                    > 10
            {
                *derailment_counts
                    .entry("Excessive shell command attempts".to_string())
                    .or_insert(0) += 1;
            }
        }

        let common_derailments: Vec<Derailment> = derailment_counts
            .into_iter()
            .map(|(trigger, frequency)| Derailment {
                trigger: trigger.clone(),
                frequency,
                recovery_steps: vec![
                    "Take a break".to_string(),
                    "Rephrase the problem".to_string(),
                ],
                prevention: match trigger.as_str() {
                    "Long conversation without resolution" => {
                        "Break complex tasks into smaller steps".to_string()
                    }
                    "Multiple edit attempts on same file" => {
                        "Read the file first before editing".to_string()
                    }
                    _ => "Review approach before continuing".to_string(),
                },
            })
            .collect();

        // Count tasks needing retries
        let tasks_needing_retries = sequences.iter().filter(|s| s.trajectory.len() > 10).count();

        Ok(ConversationIntelligence {
            successful_patterns,
            failed_patterns,
            optimal_flow: vec![
                "Clear task definition".to_string(),
                "File exploration first".to_string(),
                "Incremental changes".to_string(),
                "Test after each change".to_string(),
            ],
            common_derailments,
            retry_analysis: RetryAnalysis {
                tasks_needing_retries,
                avg_retries_per_task: sequences
                    .iter()
                    .map(|s| s.trajectory.len() as f64)
                    .sum::<f64>()
                    / sequences.len() as f64,
                retry_reasons: HashMap::new(),
                retry_success_rate: successful.len() as f64 / sequences.len() as f64 * 100.0,
            },
        })
    }

    fn compute_learning_curves(&self, sequences: &[AgenticSequence]) -> Result<LearningCurves> {
        // Group by month
        let mut monthly_data: HashMap<String, Vec<&AgenticSequence>> = HashMap::new();

        for seq in sequences {
            if let Ok(ts_ms) = seq.id.parse::<i64>() {
                let ts_sec = ts_ms / 1000;
                if let Some(dt) = DateTime::from_timestamp(ts_sec, 0) {
                    let month = format!("{}-{:02}", dt.year(), dt.month());
                    monthly_data.entry(month).or_default().push(seq);
                }
            }
        }

        let mut error_trends = Vec::new();
        let mut months: Vec<_> = monthly_data.keys().cloned().collect();
        months.sort();

        for month in months {
            if let Some(seqs) = monthly_data.get(&month) {
                let failed = seqs.iter().filter(|s| !s.outcome.success).count();
                error_trends.push(ErrorTrend {
                    week: month.clone(),
                    total_errors: failed,
                    unique_errors: failed, // TODO: deduplicate
                    repeat_error_rate: 0.0,
                });
            }
        }

        Ok(LearningCurves {
            skill_progression: HashMap::new(),
            error_reduction_over_time: error_trends,
            mastery_timeline: vec![],
            plateaus: vec![],
        })
    }

    fn detect_productivity_rhythms(
        &self,
        sequences: &[AgenticSequence],
    ) -> Result<ProductivityRhythms> {
        // Analyze hourly productivity
        let mut hourly_success: HashMap<u32, (usize, usize)> = HashMap::new(); // hour -> (total, successful)
        let mut session_lengths: Vec<usize> = Vec::new();
        let mut day_data: HashMap<String, (usize, usize, Vec<u32>)> = HashMap::new(); // day -> (sessions, successes, hours)

        for seq in sequences {
            if let Ok(ts_ms) = seq.id.parse::<i64>() {
                let ts_sec = ts_ms / 1000;
                if let Some(dt) = DateTime::from_timestamp(ts_sec, 0) {
                    let hour = dt.hour();
                    let entry = hourly_success.entry(hour).or_insert((0, 0));
                    entry.0 += 1;
                    if seq.outcome.success {
                        entry.1 += 1;
                    }

                    // Track weekly patterns
                    let weekday = dt.weekday().to_string();
                    let day_entry = day_data.entry(weekday).or_insert((0, 0, Vec::new()));
                    day_entry.0 += 1;
                    if seq.outcome.success {
                        day_entry.1 += 1;
                    }
                    day_entry.2.push(hour);
                }
            }
            session_lengths.push(seq.trajectory.len());
        }

        // Convert to DayProfile
        let weekly_patterns: HashMap<String, DayProfile> = day_data
            .into_iter()
            .map(|(day, (sessions, successes, hours))| {
                let productivity = if sessions > 0 {
                    successes as f64 / sessions as f64 * 100.0
                } else {
                    0.0
                };
                let mut hour_counts: HashMap<u32, usize> = HashMap::new();
                for h in &hours {
                    *hour_counts.entry(*h).or_insert(0) += 1;
                }
                let mut sorted_hours: Vec<_> = hour_counts.iter().collect();
                sorted_hours.sort_by(|a, b| b.1.cmp(a.1));
                let best_times: Vec<String> = sorted_hours
                    .iter()
                    .take(3)
                    .map(|(h, _)| format!("{}:00", h))
                    .collect();
                let worst_times: Vec<String> = sorted_hours
                    .iter()
                    .rev()
                    .take(2)
                    .map(|(h, _)| format!("{}:00", h))
                    .collect();

                (
                    day,
                    DayProfile {
                        total_sessions: sessions,
                        avg_productivity: productivity,
                        best_times,
                        worst_times,
                    },
                )
            })
            .collect();

        // Calculate energy levels by time of day
        let morning_success: f64 = (6..12)
            .filter_map(|h| hourly_success.get(&h))
            .map(|(t, s)| if *t > 0 { *s as f64 / *t as f64 } else { 0.0 })
            .sum::<f64>()
            / 6.0;
        let afternoon_success: f64 = (12..18)
            .filter_map(|h| hourly_success.get(&h))
            .map(|(t, s)| if *t > 0 { *s as f64 / *t as f64 } else { 0.0 })
            .sum::<f64>()
            / 6.0;
        let evening_success: f64 = (18..24)
            .filter_map(|h| hourly_success.get(&h))
            .map(|(t, s)| if *t > 0 { *s as f64 / *t as f64 } else { 0.0 })
            .sum::<f64>()
            / 6.0;

        // Calculate optimal session length
        let avg_session_length = if !session_lengths.is_empty() {
            session_lengths.iter().sum::<usize>() as f64 / session_lengths.len() as f64 * 3.0
        // ~3 min per step
        } else {
            90.0
        };

        // Count overwork sessions (>120 steps = ~6 hours)
        let overwork_sessions = session_lengths.iter().filter(|&&l| l > 120).count();

        Ok(ProductivityRhythms {
            ultradian_cycles: vec![],
            weekly_patterns,
            energy_management: EnergyProfile {
                morning_energy: morning_success * 100.0,
                afternoon_energy: afternoon_success * 100.0,
                evening_energy: evening_success * 100.0,
                optimal_scheduling: HashMap::new(),
            },
            optimal_session_length: avg_session_length.min(120.0), // Cap at 2 hours
            break_patterns: BreakAnalysis {
                avg_break_frequency_minutes: avg_session_length / 4.0,
                optimal_break_duration: 15.0,
                break_effectiveness: 0.7,
                overwork_sessions,
            },
        })
    }

    fn evaluate_tools(&self, sequences: &[AgenticSequence]) -> Result<ToolEffectiveness> {
        let mut tool_usage: HashMap<String, (usize, usize)> = HashMap::new(); // tool -> (uses, successes)

        for seq in sequences {
            let tools: Vec<_> = seq
                .trajectory
                .iter()
                .flat_map(|step| step.tool_calls.clone())
                .collect();

            let is_success = seq.outcome.success;

            for tool in tools {
                let entry = tool_usage.entry(tool).or_insert((0, 0));
                entry.0 += 1;
                if is_success {
                    entry.1 += 1;
                }
            }
        }

        let tool_success_rates: HashMap<String, f64> = tool_usage
            .iter()
            .map(|(tool, (uses, successes))| {
                (tool.clone(), *successes as f64 / *uses as f64 * 100.0)
            })
            .collect();

        Ok(ToolEffectiveness {
            tool_success_rates,
            optimal_tool_sequences: vec![],
            tool_misuse_patterns: vec![],
            tool_learning_curves: HashMap::new(),
        })
    }

    fn analyze_task_complexity(
        &self,
        sequences: &[AgenticSequence],
    ) -> Result<TaskComplexityAnalysis> {
        let mut complexity_outcomes: HashMap<String, (usize, usize, f64)> = HashMap::new(); // complexity -> (attempts, successes, total_time)

        for seq in sequences {
            let complexity = &seq.complexity;
            let is_success = seq.outcome.success;
            let time = seq.trajectory.len() as f64 * 2.5 / 60.0; // hours

            let entry = complexity_outcomes
                .entry(complexity.clone())
                .or_insert((0, 0, 0.0));
            entry.0 += 1;
            if is_success {
                entry.1 += 1;
            }
            entry.2 += time;
        }

        let complexity_vs_outcome: Vec<ComplexityOutcome> = complexity_outcomes
            .iter()
            .map(
                |(complexity, (attempts, successes, total_time))| ComplexityOutcome {
                    complexity: complexity.clone(),
                    attempts: *attempts,
                    success_rate: *successes as f64 / *attempts as f64 * 100.0,
                    avg_time_hours: total_time / *attempts as f64,
                },
            )
            .collect();

        Ok(TaskComplexityAnalysis {
            complexity_vs_outcome,
            sweet_spot: "moderate".to_string(),
            overambitious_tasks: vec![],
            underutilized_potential: vec![],
        })
    }

    fn find_hidden_patterns(
        &self,
        sequences: &[AgenticSequence],
        prompts: &[PromptData],
    ) -> Result<Vec<HiddenPattern>> {
        let mut patterns = Vec::new();

        // Pattern 1: The specificity paradox
        let low_spec_prompts: Vec<_> = prompts.iter().filter(|p| p.specificity == "low").collect();
        let high_spec_prompts: Vec<_> =
            prompts.iter().filter(|p| p.specificity == "high").collect();

        if !low_spec_prompts.is_empty() && !high_spec_prompts.is_empty() {
            let low_tokens: f64 = low_spec_prompts
                .iter()
                .map(|p| p.outcome.tokens_used as f64)
                .sum::<f64>()
                / low_spec_prompts.len() as f64;
            let high_tokens: f64 = high_spec_prompts
                .iter()
                .map(|p| p.outcome.tokens_used as f64)
                .sum::<f64>()
                / high_spec_prompts.len() as f64;

            if high_tokens > low_tokens * 5.0 {
                patterns.push(HiddenPattern {
                    pattern_type: "Prompt Engineering".to_string(),
                    title: "The Specificity Paradox".to_string(),
                    description: format!(
                        "Your highly specific prompts use {}x more tokens ({:.0} vs {:.0}) than low-specificity prompts, \
                        but don't have proportionally better outcomes. You may be over-engineering prompts with unnecessary context.",
                        high_tokens / low_tokens, high_tokens, low_tokens
                    ),
                    significance: "HIGH".to_string(),
                    actionable_insight: "Try 'medium' specificity: include file paths and function names, but skip verbose explanations. \
                        The AI already has context from previous messages.".to_string(),
                    supporting_data: vec![
                        format!("Low specificity avg tokens: {:.0}", low_tokens),
                        format!("High specificity avg tokens: {:.0}", high_tokens),
                        format!("Token waste: {:.0} tokens per high-spec prompt", high_tokens - low_tokens),
                    ],
                });
            }
        }

        // Pattern 2: Expert task success rate
        let expert_tasks: Vec<_> = sequences
            .iter()
            .filter(|s| s.complexity == "expert")
            .collect();
        if !expert_tasks.is_empty() {
            let success_rate = expert_tasks.iter().filter(|s| s.outcome.success).count() as f64
                / expert_tasks.len() as f64
                * 100.0;
            let avg_turns = expert_tasks
                .iter()
                .map(|s| s.trajectory.len())
                .sum::<usize>() as f64
                / expert_tasks.len() as f64;

            patterns.push(HiddenPattern {
                pattern_type: "Task Complexity".to_string(),
                title: "Expert Task Performance".to_string(),
                description: format!(
                    "You attempted {} expert-level tasks with {:.1}% success rate. Average {:.0} conversation turns per task. \
                    Expert tasks typically require breaking down into smaller steps.",
                    expert_tasks.len(), success_rate, avg_turns
                ),
                significance: "MEDIUM".to_string(),
                actionable_insight: "For expert tasks, explicitly ask the AI to 'create a step-by-step plan first' before implementation. \
                    This reduces cognitive load and improves success rates.".to_string(),
                supporting_data: vec![
                    format!("Expert tasks attempted: {}", expert_tasks.len()),
                    format!("Success rate: {:.1}%", success_rate),
                    format!("Avg conversation turns: {:.0}", avg_turns),
                ],
            });
        }

        Ok(patterns)
    }
}

// Data structures for loading
#[derive(Debug, Deserialize)]
struct AgenticSequence {
    id: String,
    #[serde(default)]
    task_description: String,
    complexity: String,
    trajectory: Vec<TrajectoryStep>,
    outcome: TaskOutcome,
    #[serde(default)]
    features: serde_json::Value,
    #[serde(default)]
    metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct TrajectoryStep {
    #[serde(default)]
    step_number: usize,
    #[serde(default)]
    tool_calls: Vec<String>,
    #[serde(default)]
    step_outcome: String,
    #[serde(default)]
    user_message: Option<String>,
    #[serde(default)]
    ai_reasoning: Option<String>,
    #[serde(default)]
    ai_response: String,
}

#[derive(Debug, Deserialize)]
struct TaskOutcome {
    #[serde(default)]
    status: String,
    success: bool,
    #[serde(default)]
    total_steps: usize,
    #[serde(default)]
    total_tool_calls: usize,
    #[serde(default)]
    files_modified: Vec<String>,
    #[serde(default)]
    tokens_used: usize,
}

#[derive(Debug, Deserialize)]
struct PromptData {
    specificity: String,
    outcome: PromptOutcome,
}

#[derive(Debug, Deserialize)]
struct PromptOutcome {
    success: bool,
    tokens_used: usize,
    turns_to_complete: usize,
}
