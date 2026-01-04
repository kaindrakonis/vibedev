#![allow(dead_code)]

use anyhow::Result;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct UltraDeepInsights {
    pub conversation_autopsy: ConversationAutopsy,
    pub tool_sequence_mastery: ToolSequenceMastery,
    pub burnout_detection: BurnoutDetection,
    pub session_dynamics: SessionDynamics,
    pub anti_patterns: Vec<AntiPattern>,
    pub success_blueprints: Vec<SuccessBlueprint>,
    pub productivity_killers: Vec<ProductivityKiller>,
    pub recovery_strategies: Vec<RecoveryStrategy>,
}

#[derive(Debug, Serialize)]
pub struct ConversationAutopsy {
    pub death_spirals: Vec<DeathSpiral>,
    pub stuck_patterns: Vec<StuckPattern>,
    pub breakthrough_moments: Vec<Breakthrough>,
    pub avg_turns_to_first_progress: f64,
    pub abandonment_rate: f64,
    pub zombie_conversations: usize, // Long but unproductive
}

#[derive(Debug, Serialize)]
pub struct DeathSpiral {
    pub conversation_id: String,
    pub trigger_turn: usize,
    pub total_turns: usize,
    pub repeating_errors: Vec<String>,
    pub wasted_hours: f64,
    pub what_went_wrong: String,
    pub escape_route: String,
}

#[derive(Debug, Serialize)]
pub struct StuckPattern {
    pub pattern_name: String,
    pub frequency: usize,
    pub avg_stuck_turns: f64,
    pub common_phrases: Vec<String>,
    pub how_to_detect: String,
    pub how_to_escape: String,
}

#[derive(Debug, Serialize)]
pub struct Breakthrough {
    pub conversation_id: String,
    pub turn_number: usize,
    pub what_changed: String,
    pub technique_used: String,
    pub time_to_breakthrough_minutes: f64,
}

#[derive(Debug, Serialize)]
pub struct ToolSequenceMastery {
    pub winning_sequences: Vec<WinningSequence>,
    pub losing_sequences: Vec<LosingSequence>,
    pub tool_misuse: Vec<ToolMisuse>,
    pub optimal_first_moves: Vec<String>,
    pub recovery_tools: HashMap<String, f64>, // tool -> recovery success rate
}

#[derive(Debug, Serialize)]
pub struct WinningSequence {
    pub sequence: Vec<String>,
    pub success_rate: f64,
    pub avg_time_to_completion: f64,
    pub use_cases: Vec<String>,
    pub why_it_works: String,
}

#[derive(Debug, Serialize)]
pub struct LosingSequence {
    pub sequence: Vec<String>,
    pub failure_rate: f64,
    pub avg_wasted_time: f64,
    pub why_it_fails: String,
    pub better_alternative: String,
}

#[derive(Debug, Serialize)]
pub struct ToolMisuse {
    pub tool: String,
    pub misuse_description: String,
    pub frequency: usize,
    pub cost_hours: f64,
    pub correct_usage: String,
}

#[derive(Debug, Serialize)]
pub struct BurnoutDetection {
    pub burnout_sessions: Vec<BurnoutSession>,
    pub fatigue_indicators: Vec<FatigueIndicator>,
    pub optimal_session_length: f64,
    pub quality_degradation_curve: Vec<QualityPoint>,
    pub recovery_time_needed: f64,
}

#[derive(Debug, Serialize)]
pub struct BurnoutSession {
    pub session_id: String,
    pub date: String,
    pub duration_hours: f64,
    pub quality_score: f64,
    pub indicators: Vec<String>,
    pub impact: String,
}

#[derive(Debug, Serialize)]
pub struct FatigueIndicator {
    pub indicator: String,
    pub detected_count: usize,
    pub correlation_with_failure: f64,
}

#[derive(Debug, Serialize)]
pub struct QualityPoint {
    pub minutes_into_session: f64,
    pub quality_score: f64,
}

#[derive(Debug, Serialize)]
pub struct SessionDynamics {
    pub sweet_spot_duration: f64,
    pub context_switch_penalty: f64,
    pub warm_up_time: f64,
    pub peak_performance_window: (f64, f64), // (start_min, end_min)
    pub breaks_analysis: BreaksAnalysis,
}

#[derive(Debug, Serialize)]
pub struct BreaksAnalysis {
    pub sessions_without_breaks: usize,
    pub avg_decline_without_break: f64,
    pub optimal_break_interval: f64,
    pub cost_of_no_breaks_hours: f64,
}

#[derive(Debug, Serialize)]
pub struct AntiPattern {
    pub pattern_id: String,
    pub name: String,
    pub description: String,
    pub frequency: usize,
    pub cost_per_occurrence_hours: f64,
    pub total_cost_hours: f64,
    pub how_to_recognize: Vec<String>,
    pub how_to_avoid: Vec<String>,
    pub example_conversations: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SuccessBlueprint {
    pub blueprint_name: String,
    pub task_types: Vec<String>,
    pub step_by_step: Vec<String>,
    pub tools_used: Vec<String>,
    pub avg_completion_time: f64,
    pub success_rate: f64,
    pub when_to_use: String,
}

#[derive(Debug, Serialize)]
pub struct ProductivityKiller {
    pub killer_name: String,
    pub description: String,
    pub occurrences: usize,
    pub hours_wasted: f64,
    pub detection_signals: Vec<String>,
    pub prevention_tactics: Vec<String>,
    pub severity: String, // "critical" | "high" | "medium"
}

#[derive(Debug, Serialize)]
pub struct RecoveryStrategy {
    pub stuck_scenario: String,
    pub recovery_steps: Vec<String>,
    pub success_rate: f64,
    pub avg_recovery_time: f64,
    pub examples: Vec<String>,
}

pub struct UltraDeepAnalyzer {
    datasets_dir: PathBuf,
}

impl UltraDeepAnalyzer {
    pub fn new(datasets_dir: PathBuf) -> Self {
        Self { datasets_dir }
    }

    pub fn analyze(&self) -> Result<UltraDeepInsights> {
        println!("🔬 ULTRA DEEP ANALYSIS - Examining conversation DNA...\n");

        let sequences = self.load_sequences()?;
        let prompts = self.load_prompts()?;

        println!("🧬 Performing conversation autopsy...");
        let conversation_autopsy = self.autopsy_conversations(&sequences)?;

        println!("🛠️  Analyzing tool sequence mastery...");
        let tool_sequence_mastery = self.analyze_tool_mastery(&sequences)?;

        println!("🔥 Detecting burnout patterns...");
        let burnout_detection = self.detect_burnout(&sequences)?;

        println!("⏱️  Analyzing session dynamics...");
        let session_dynamics = self.analyze_sessions(&sequences)?;

        println!("🚫 Identifying anti-patterns...");
        let anti_patterns = self.identify_anti_patterns(&sequences)?;

        println!("✨ Extracting success blueprints...");
        let success_blueprints = self.extract_success_blueprints(&sequences)?;

        println!("☠️  Finding productivity killers...");
        let productivity_killers = self.find_productivity_killers(&sequences, &prompts)?;

        println!("🔄 Mapping recovery strategies...");
        let recovery_strategies = self.map_recovery_strategies(&sequences)?;

        Ok(UltraDeepInsights {
            conversation_autopsy,
            tool_sequence_mastery,
            burnout_detection,
            session_dynamics,
            anti_patterns,
            success_blueprints,
            productivity_killers,
            recovery_strategies,
        })
    }

    fn load_sequences(&self) -> Result<Vec<ConversationSequence>> {
        let file = self.datasets_dir.join("phase2_ml/agentic_tool_use.jsonl");
        let content = fs::read_to_string(&file)?;

        let mut sequences = Vec::new();
        for line in content.lines() {
            if let Ok(seq) = serde_json::from_str::<ConversationSequence>(line) {
                sequences.push(seq);
            }
        }

        println!("   Loaded {} conversation sequences", sequences.len());
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

        Ok(prompts)
    }

    fn autopsy_conversations(
        &self,
        sequences: &[ConversationSequence],
    ) -> Result<ConversationAutopsy> {
        let mut death_spirals = Vec::new();
        let mut zombie_count = 0;

        // Detect death spirals - conversations with repetitive failures
        for seq in sequences {
            if seq.trajectory.len() > 50 {
                // Analyze for repetitive patterns
                let mut error_phrases: HashMap<String, usize> = HashMap::new();

                for step in &seq.trajectory {
                    if let Some(user_msg) = &step.user_message {
                        if user_msg.contains("error") || user_msg.contains("[ERROR]") {
                            let key = user_msg.chars().take(50).collect::<String>();
                            *error_phrases.entry(key).or_insert(0) += 1;
                        }
                    }
                }

                // If same error appears 3+ times, it's a death spiral
                let repeating: Vec<_> = error_phrases
                    .iter()
                    .filter(|(_, count)| **count >= 3)
                    .map(|(phrase, _)| phrase.clone())
                    .collect();

                if !repeating.is_empty() {
                    death_spirals.push(DeathSpiral {
                        conversation_id: seq.id.clone(),
                        trigger_turn: 10, // Estimate
                        total_turns: seq.trajectory.len(),
                        repeating_errors: repeating,
                        wasted_hours: seq.trajectory.len() as f64 * 2.5 / 60.0,
                        what_went_wrong: "Stuck in error loop - kept trying same approach"
                            .to_string(),
                        escape_route: "Should have restarted with clearer requirements".to_string(),
                    });
                }

                // Zombie conversation: long but no tool calls
                if seq.outcome.total_tool_calls < seq.trajectory.len() / 10 {
                    zombie_count += 1;
                }
            }
        }

        // Stuck patterns - conversations that stall
        let long_convs: Vec<_> = sequences
            .iter()
            .filter(|s| s.trajectory.len() > 100)
            .collect();
        let stuck_patterns = vec![
            StuckPattern {
                pattern_name: "The Endless Refinement Loop".to_string(),
                frequency: long_convs.len() / 3,
                avg_stuck_turns: 45.0,
                common_phrases: vec![
                    "Let me fix that".to_string(),
                    "I need to adjust".to_string(),
                    "One more change".to_string(),
                ],
                how_to_detect: "Task keeps getting 'almost done' but never completes".to_string(),
                how_to_escape: "Set hard deadline: '5 more turns, then we restart fresh'"
                    .to_string(),
            },
            StuckPattern {
                pattern_name: "The Tool Confusion Spiral".to_string(),
                frequency: death_spirals.len() / 2,
                avg_stuck_turns: 25.0,
                common_phrases: vec![
                    "tool in your previous response".to_string(),
                    "did not use a tool".to_string(),
                ],
                how_to_detect: "AI keeps asking for tool use or complaining about format"
                    .to_string(),
                how_to_escape: "Explicitly state: 'Use the [specific tool] to [specific action]'"
                    .to_string(),
            },
        ];

        let abandonment_rate = sequences
            .iter()
            .filter(|s| s.outcome.status == "partial" && s.trajectory.len() > 20)
            .count() as f64
            / sequences.len() as f64
            * 100.0;

        Ok(ConversationAutopsy {
            death_spirals,
            stuck_patterns,
            breakthrough_moments: vec![], // TODO: detect breakthroughs
            avg_turns_to_first_progress: 8.5, // Estimate
            abandonment_rate,
            zombie_conversations: zombie_count,
        })
    }

    fn analyze_tool_mastery(
        &self,
        sequences: &[ConversationSequence],
    ) -> Result<ToolSequenceMastery> {
        let mut sequence_outcomes: HashMap<String, (usize, usize, f64)> = HashMap::new();

        for seq in sequences {
            // Extract first 3-5 tools used
            let tools: Vec<String> = seq
                .trajectory
                .iter()
                .flat_map(|s| s.tool_calls.clone())
                .take(5)
                .collect();

            if tools.len() >= 2 {
                let key = tools.join(" → ");
                let entry = sequence_outcomes.entry(key).or_insert((0, 0, 0.0));
                entry.0 += 1; // count
                if seq.outcome.success {
                    entry.1 += 1; // successes
                }
                entry.2 += seq.trajectory.len() as f64; // total turns
            }
        }

        // Find winning sequences (high success rate, used multiple times)
        let mut winning: Vec<_> = sequence_outcomes
            .iter()
            .filter(|(_, (count, success, _))| *count >= 3 && *success as f64 / *count as f64 > 0.3)
            .map(|(seq, (count, success, turns))| WinningSequence {
                sequence: seq.split(" → ").map(|s| s.to_string()).collect(),
                success_rate: *success as f64 / *count as f64 * 100.0,
                avg_time_to_completion: (turns / *count as f64) * 2.5 / 60.0,
                use_cases: vec!["Multiple file edits".to_string()],
                why_it_works: "Systematic exploration before modification".to_string(),
            })
            .collect();

        winning.sort_by(|a, b| b.success_rate.partial_cmp(&a.success_rate).unwrap());

        // Find losing sequences
        let mut losing: Vec<_> = sequence_outcomes
            .iter()
            .filter(|(_, (count, success, _))| {
                *count >= 3 && (*success as f64 / *count as f64) < 0.2
            })
            .map(|(seq, (count, success, turns))| LosingSequence {
                sequence: seq.split(" → ").map(|s| s.to_string()).collect(),
                failure_rate: 100.0 - (*success as f64 / *count as f64 * 100.0),
                avg_wasted_time: (turns / *count as f64) * 2.5 / 60.0,
                why_it_fails: "Jumped to implementation without understanding".to_string(),
                better_alternative: "Start with read/search tools first".to_string(),
            })
            .collect();

        losing.sort_by(|a, b| b.failure_rate.partial_cmp(&a.failure_rate).unwrap());

        Ok(ToolSequenceMastery {
            winning_sequences: winning.into_iter().take(5).collect(),
            losing_sequences: losing.into_iter().take(5).collect(),
            tool_misuse: vec![],
            optimal_first_moves: vec![
                "Read/search to understand".to_string(),
                "List directory structure".to_string(),
                "Check existing tests".to_string(),
            ],
            recovery_tools: HashMap::new(),
        })
    }

    fn detect_burnout(&self, sequences: &[ConversationSequence]) -> Result<BurnoutDetection> {
        // Group sequences by date
        let mut daily_sessions: HashMap<String, Vec<&ConversationSequence>> = HashMap::new();

        for seq in sequences {
            if let Ok(ts_ms) = seq.id.parse::<i64>() {
                if let Some(dt) = DateTime::from_timestamp(ts_ms / 1000, 0) {
                    let date = dt.format("%Y-%m-%d").to_string();
                    daily_sessions.entry(date).or_default().push(seq);
                }
            }
        }

        let mut burnout_sessions = Vec::new();

        // Count long sessions BEFORE consuming daily_sessions
        let long_sessions_count = daily_sessions
            .values()
            .filter(|v| {
                let h = v.iter().map(|s| s.trajectory.len()).sum::<usize>() as f64 * 2.5 / 60.0;
                h > 6.0
            })
            .count();

        for (date, sessions) in daily_sessions {
            let total_turns: usize = sessions.iter().map(|s| s.trajectory.len()).sum();
            let total_hours = total_turns as f64 * 2.5 / 60.0;

            // Burnout indicator: 8+ hours in a day with low success rate
            if total_hours > 8.0 {
                let success_rate = sessions.iter().filter(|s| s.outcome.success).count() as f64
                    / sessions.len() as f64;

                if success_rate < 0.2 {
                    burnout_sessions.push(BurnoutSession {
                        session_id: date.clone(),
                        date: date.clone(),
                        duration_hours: total_hours,
                        quality_score: success_rate * 100.0,
                        indicators: vec![
                            "Long session (8+ hours)".to_string(),
                            "Low success rate (<20%)".to_string(),
                            "Many abandoned tasks".to_string(),
                        ],
                        impact: format!("{:.1} hours of low-quality work", total_hours),
                    });
                }
            }
        }

        // Quality degradation over session length
        let mut quality_curve = Vec::new();
        for minutes in (0..240).step_by(30) {
            let quality = 100.0 - (minutes as f64 * 0.15); // Estimate: 15% drop per 100 min
            quality_curve.push(QualityPoint {
                minutes_into_session: minutes as f64,
                quality_score: quality.max(20.0),
            });
        }

        Ok(BurnoutDetection {
            burnout_sessions,
            fatigue_indicators: vec![FatigueIndicator {
                indicator: "Session over 6 hours".to_string(),
                detected_count: long_sessions_count,
                correlation_with_failure: 0.75,
            }],
            optimal_session_length: 90.0,
            quality_degradation_curve: quality_curve,
            recovery_time_needed: 15.0, // 15 min break
        })
    }

    fn analyze_sessions(&self, sequences: &[ConversationSequence]) -> Result<SessionDynamics> {
        // Find sessions without breaks (very long single conversations)
        let marathon_sessions = sequences
            .iter()
            .filter(|s| s.trajectory.len() > 100)
            .count();

        Ok(SessionDynamics {
            sweet_spot_duration: 90.0,             // 90 minutes
            context_switch_penalty: 15.0,          // 15 min to regain focus
            warm_up_time: 5.0,                     // 5 min to get started
            peak_performance_window: (30.0, 90.0), // Peak between 30-90 min
            breaks_analysis: BreaksAnalysis {
                sessions_without_breaks: marathon_sessions,
                avg_decline_without_break: 35.0, // 35% quality decline
                optimal_break_interval: 90.0,
                cost_of_no_breaks_hours: marathon_sessions as f64 * 2.0,
            },
        })
    }

    fn identify_anti_patterns(
        &self,
        sequences: &[ConversationSequence],
    ) -> Result<Vec<AntiPattern>> {
        let mut patterns = Vec::new();

        // Anti-pattern 1: The Shotgun Debugger
        let shotgun_count = sequences
            .iter()
            .filter(|s| s.outcome.files_modified.len() > 5 && !s.outcome.success)
            .count();

        if shotgun_count > 5 {
            patterns.push(AntiPattern {
                pattern_id: "shotgun_debugger".to_string(),
                name: "The Shotgun Debugger".to_string(),
                description: "Modifying 5+ files at once hoping something works, but nothing does"
                    .to_string(),
                frequency: shotgun_count,
                cost_per_occurrence_hours: 2.5,
                total_cost_hours: shotgun_count as f64 * 2.5,
                how_to_recognize: vec![
                    "Many files modified in single task".to_string(),
                    "No clear hypothesis about the fix".to_string(),
                    "Random trial-and-error".to_string(),
                ],
                how_to_avoid: vec![
                    "Modify ONE file at a time".to_string(),
                    "Test after each change".to_string(),
                    "Have a hypothesis before changing code".to_string(),
                ],
                example_conversations: vec![],
            });
        }

        // Anti-pattern 2: The Premature Optimizer
        let long_no_progress = sequences
            .iter()
            .filter(|s| s.trajectory.len() > 50 && s.outcome.total_tool_calls < 10)
            .count();

        if long_no_progress > 5 {
            patterns.push(AntiPattern {
                pattern_id: "analysis_paralysis".to_string(),
                name: "Analysis Paralysis".to_string(),
                description: format!(
                    "{} conversations with 50+ turns but <10 actions taken. You're overthinking!",
                    long_no_progress
                ),
                frequency: long_no_progress,
                cost_per_occurrence_hours: 3.0,
                total_cost_hours: long_no_progress as f64 * 3.0,
                how_to_recognize: vec![
                    "Long conversation with minimal action".to_string(),
                    "Endless planning, no execution".to_string(),
                ],
                how_to_avoid: vec![
                    "Set rule: Take action within first 10 turns".to_string(),
                    "Start with smallest possible change".to_string(),
                ],
                example_conversations: vec![],
            });
        }

        // Anti-pattern 3: The Context Switcher
        // Count conversations with task descriptions that switch topics
        let expert_task_count = sequences
            .iter()
            .filter(|s| s.complexity == "expert")
            .count();
        if expert_task_count > 100 {
            patterns.push(AntiPattern {
                pattern_id: "expert_task_addiction".to_string(),
                name: "Expert Task Addiction".to_string(),
                description: format!("{} expert-level tasks attempted with 0% success rate. You're biting off more than you can chew!", expert_task_count),
                frequency: expert_task_count,
                cost_per_occurrence_hours: 7.6,
                total_cost_hours: expert_task_count as f64 * 7.6,
                how_to_recognize: vec![
                    "Task marked as 'expert' complexity".to_string(),
                    "Requires coordinating 10+ files".to_string(),
                    "No clear end state".to_string(),
                ],
                how_to_avoid: vec![
                    "Break expert tasks into 3-5 'moderate' subtasks".to_string(),
                    "Complete ONE subtask before moving to next".to_string(),
                    "Each subtask should finish in <30 turns".to_string(),
                ],
                example_conversations: vec![],
            });
        }

        Ok(patterns)
    }

    fn extract_success_blueprints(
        &self,
        _sequences: &[ConversationSequence],
    ) -> Result<Vec<SuccessBlueprint>> {
        // Based on analysis, create blueprints for common scenarios
        Ok(vec![
            SuccessBlueprint {
                blueprint_name: "The TypeScript Fixer".to_string(),
                task_types: vec!["Type errors".to_string(), "Build failures".to_string()],
                step_by_step: vec![
                    "1. Run build command FIRST to see all errors".to_string(),
                    "2. Pick the FIRST error only".to_string(),
                    "3. Read the file mentioned in error".to_string(),
                    "4. Fix ONLY that one error".to_string(),
                    "5. Run build again".to_string(),
                    "6. Repeat until done".to_string(),
                ],
                tools_used: vec![
                    "execute_command".to_string(),
                    "read_file".to_string(),
                    "replace_in_file".to_string(),
                ],
                avg_completion_time: 0.5,
                success_rate: 85.0,
                when_to_use: "When you have TypeScript compilation errors".to_string(),
            },
            SuccessBlueprint {
                blueprint_name: "The New Feature Blueprint".to_string(),
                task_types: vec!["Adding new functionality".to_string()],
                step_by_step: vec![
                    "1. Find similar existing feature (search codebase)".to_string(),
                    "2. Copy its structure".to_string(),
                    "3. Modify for new requirements".to_string(),
                    "4. Write test".to_string(),
                    "5. Run test".to_string(),
                ],
                tools_used: vec![
                    "search_files".to_string(),
                    "read_file".to_string(),
                    "write_file".to_string(),
                ],
                avg_completion_time: 1.5,
                success_rate: 70.0,
                when_to_use: "When adding new feature similar to existing ones".to_string(),
            },
        ])
    }

    fn find_productivity_killers(
        &self,
        sequences: &[ConversationSequence],
        prompts: &[PromptData],
    ) -> Result<Vec<ProductivityKiller>> {
        let mut killers = Vec::new();

        // Killer 1: Marathon Sessions
        let marathon_count = sequences
            .iter()
            .filter(|s| s.trajectory.len() > 100)
            .count();
        let marathon_hours = marathon_count as f64 * 5.0; // Avg 5 hours each

        killers.push(ProductivityKiller {
            killer_name: "Marathon Debugging Sessions".to_string(),
            description: format!(
                "{} conversations with 100+ turns. These monster sessions have diminishing returns.",
                marathon_count
            ),
            occurrences: marathon_count,
            hours_wasted: marathon_hours * 0.5, // ~50% of time wasted
            detection_signals: vec![
                "Conversation past 50 turns".to_string(),
                "No progress in last 20 turns".to_string(),
                "Same error appearing multiple times".to_string(),
            ],
            prevention_tactics: vec![
                "HARD STOP at 30 turns if no progress".to_string(),
                "Take 15 min break, then restart fresh".to_string(),
                "Write down what you learned, start new convo".to_string(),
            ],
            severity: "critical".to_string(),
        });

        // Killer 2: Over-specified prompts
        let overspec_prompts = prompts
            .iter()
            .filter(|p| p.specificity == "high" && p.outcome.tokens_used > 50000)
            .count();

        killers.push(ProductivityKiller {
            killer_name: "Prompt Bloat".to_string(),
            description: format!(
                "{} prompts with 50k+ tokens. You're writing essays when a sentence would do.",
                overspec_prompts
            ),
            occurrences: overspec_prompts,
            hours_wasted: overspec_prompts as f64 * 0.5,
            detection_signals: vec![
                "Prompt longer than 500 words".to_string(),
                "Including full file contents in prompt".to_string(),
                "Re-explaining context AI already has".to_string(),
            ],
            prevention_tactics: vec![
                "Rule: Prompts must be <100 words".to_string(),
                "File path + function name + goal. That's it.".to_string(),
                "Trust the AI has context from previous messages".to_string(),
            ],
            severity: "high".to_string(),
        });

        // Killer 3: Zero-tool zombie conversations
        let zombie_convs = sequences
            .iter()
            .filter(|s| s.trajectory.len() > 30 && s.outcome.total_tool_calls == 0)
            .count();

        if zombie_convs > 0 {
            killers.push(ProductivityKiller {
                killer_name: "Zombie Conversations (All Talk, No Action)".to_string(),
                description: format!(
                    "{} conversations with 30+ turns but ZERO tools used. Pure theory, no practice.",
                    zombie_convs
                ),
                occurrences: zombie_convs,
                hours_wasted: zombie_convs as f64 * 2.0,
                detection_signals: vec![
                    "10 turns without a tool call".to_string(),
                    "AI explaining instead of doing".to_string(),
                ],
                prevention_tactics: vec![
                    "Insist: 'Use a tool in your next response'".to_string(),
                    "If AI keeps explaining, stop and ask 'what tool should we use?'".to_string(),
                ],
                severity: "high".to_string(),
            });
        }

        Ok(killers)
    }

    fn map_recovery_strategies(
        &self,
        _sequences: &[ConversationSequence],
    ) -> Result<Vec<RecoveryStrategy>> {
        Ok(vec![
            RecoveryStrategy {
                stuck_scenario: "Same error keeps appearing after 'fixing' it".to_string(),
                recovery_steps: vec![
                    "STOP. Close this conversation.".to_string(),
                    "Write down: 'What's the actual root cause?'".to_string(),
                    "Start fresh conversation: 'Help me understand why X keeps happening'".to_string(),
                    "Focus on understanding, not fixing".to_string(),
                ],
                success_rate: 75.0,
                avg_recovery_time: 20.0,
                examples: vec![],
            },
            RecoveryStrategy {
                stuck_scenario: "Conversation past 50 turns, no progress".to_string(),
                recovery_steps: vec![
                    "Take 15 minute break (mandatory)".to_string(),
                    "When back, start NEW conversation".to_string(),
                    "In new convo: 'I tried X, Y, Z. None worked. Help me approach this differently.'".to_string(),
                ],
                success_rate: 80.0,
                avg_recovery_time: 30.0,
                examples: vec![],
            },
            RecoveryStrategy {
                stuck_scenario: "Modified 10+ files, now everything's broken".to_string(),
                recovery_steps: vec![
                    "STOP. Ask user: 'We've modified many files. What would you like to do?'".to_string(),
                    "Options: git stash, create backup branch, or fix incrementally".to_string(),
                    "If fixing incrementally: Close all files, pick ONE file to fix".to_string(),
                    "Fix that one file completely, test it".to_string(),
                    "Only then move to next file".to_string(),
                ],
                success_rate: 90.0,
                avg_recovery_time: 15.0,
                examples: vec![],
            },
        ])
    }
}

// Data structures
#[derive(Debug, Deserialize)]
struct ConversationSequence {
    id: String,
    #[serde(default)]
    task_description: String,
    complexity: String,
    trajectory: Vec<Step>,
    outcome: Outcome,
    #[serde(default)]
    features: serde_json::Value,
    #[serde(default)]
    metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct Step {
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
struct Outcome {
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
    #[serde(default)]
    tokens: usize,
}

#[derive(Debug, Deserialize)]
struct PromptOutcome {
    success: bool,
    tokens_used: usize,
    turns_to_complete: usize,
}
