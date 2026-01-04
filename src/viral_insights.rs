use anyhow::Result;
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::info;
use walkdir::WalkDir;

#[derive(Debug, Serialize)]
pub struct ViralInsights {
    pub fun_facts: FunFacts,
    pub behavior_patterns: BehaviorPatterns,
    pub time_analytics: TimeAnalytics,
    pub code_velocity: CodeVelocity,
    pub achievements: Vec<Achievement>,
    pub comparisons: Comparisons,
    pub records: Records,
}

#[derive(Debug, Serialize)]
pub struct FunFacts {
    pub tokens_in_books: f64,     // Equivalent number of novels
    pub tokens_in_wikipedia: f64, // Percentage of Wikipedia
    pub pages_if_printed: usize,  // If printed as book
    pub carbon_footprint_kg: f64, // CO2 equivalent
    pub reading_time_hours: f64,  // Human reading time
    pub cost_in_coffee: usize,    // Equivalent Starbucks coffees
}

#[derive(Debug, Serialize)]
pub struct BehaviorPatterns {
    pub frustration_count: usize,                  // "wtf", "no,", "please"
    pub go_on_count: usize,                        // Just "go on"
    pub retry_count: usize,                        // "try again"
    pub typo_count: usize,                         // Common typos detected
    pub command_spam_events: usize,                // Rapid repeated commands
    pub politeness_score: f64,                     // How often user says please/thanks
    pub most_common_prompts: Vec<(String, usize)>, // Top 10 repeated prompts
    pub frustration_examples: Vec<String>,         // Actual frustrated messages
}

#[derive(Debug, Serialize)]
pub struct TimeAnalytics {
    pub hourly_heatmap: HashMap<usize, usize>, // Messages by hour (0-23)
    pub daily_heatmap: HashMap<String, usize>, // Messages by weekday
    pub late_night_sessions: usize,            // Sessions after midnight
    pub binge_coding_sessions: usize,          // 8+ hour sessions
    pub most_productive_hour: usize,           // Peak hour
    pub most_productive_day: String,           // Peak weekday
    pub earliest_session: String,              // Timestamp of earliest coding
    pub latest_session: String,                // Timestamp of latest coding
    pub average_session_gap_hours: f64,        // Time between sessions
}

#[derive(Debug, Serialize)]
pub struct CodeVelocity {
    pub lines_per_hour: f64,
    pub files_per_session: f64,
    pub biggest_refactor_lines: usize,
    pub most_edited_file: String,
    pub most_edited_file_times: usize,
    pub total_edits: usize,
    pub total_new_files: usize,
    pub edit_to_create_ratio: f64,
}

#[derive(Debug, Serialize)]
pub struct Achievement {
    pub name: String,
    pub description: String,
    pub unlocked: bool,
    pub emoji: String,
}

#[derive(Debug, Serialize)]
pub struct Comparisons {
    pub war_and_peace_equivalent: f64, // How many War & Peace novels
    pub harry_potter_series: f64,      // How many HP series
    pub movie_scripts: f64,            // Equivalent movie scripts
    pub tweets: usize,                 // Equivalent tweets (280 chars)
    pub stackoverflow_answers: usize,  // Avg 500 tokens each
}

#[derive(Debug, Serialize)]
pub struct Records {
    pub longest_session_messages: usize,
    pub longest_session_tokens: u64,
    pub most_messages_in_hour: usize,
    pub fastest_bug_fix_minutes: f64,
    pub biggest_file_edited_mb: f64,
    pub most_expensive_conversation_usd: f64,
}

#[derive(Debug, Deserialize)]
struct ClineMessage {
    role: String,
    content: serde_json::Value,
    #[serde(default)]
    ts: Option<i64>,
}

pub struct ViralAnalyzer {
    base_dir: PathBuf,
    total_tokens: u64,
    total_cost: f64,
}

impl ViralAnalyzer {
    pub fn new(base_dir: PathBuf, total_tokens: u64, total_cost: f64) -> Self {
        Self {
            base_dir,
            total_tokens,
            total_cost,
        }
    }

    pub fn analyze(&self) -> Result<ViralInsights> {
        info!("🎉 Generating viral insights...");

        let fun_facts = self.generate_fun_facts();
        let behavior_patterns = self.analyze_behavior()?;
        let time_analytics = self.analyze_time_patterns()?;
        let code_velocity = self.analyze_code_velocity()?;
        let achievements = self.unlock_achievements(&behavior_patterns, &time_analytics);
        let comparisons = self.generate_comparisons();
        let records = self.find_records()?;

        Ok(ViralInsights {
            fun_facts,
            behavior_patterns,
            time_analytics,
            code_velocity,
            achievements,
            comparisons,
            records,
        })
    }

    fn generate_fun_facts(&self) -> FunFacts {
        // Average novel: 100,000 tokens
        let novels = self.total_tokens as f64 / 100_000.0;

        // Wikipedia: ~4 billion tokens
        let wikipedia_pct = (self.total_tokens as f64 / 4_000_000_000.0) * 100.0;

        // Pages: 500 tokens per page
        let pages = (self.total_tokens as f64 / 500.0) as usize;

        // Carbon: ~0.0001 kg CO2 per 1000 tokens (GPT-3 estimate)
        let carbon = (self.total_tokens as f64 / 1000.0) * 0.0001;

        // Reading: 250 tokens per minute
        let reading_hours = (self.total_tokens as f64 / 250.0) / 60.0;

        // Coffee: $5 per coffee
        let coffees = (self.total_cost / 5.0) as usize;

        FunFacts {
            tokens_in_books: novels,
            tokens_in_wikipedia: wikipedia_pct,
            pages_if_printed: pages,
            carbon_footprint_kg: carbon,
            reading_time_hours: reading_hours,
            cost_in_coffee: coffees,
        }
    }

    fn analyze_behavior(&self) -> Result<BehaviorPatterns> {
        info!("🧠 Analyzing user behavior patterns...");

        let mut frustration_count = 0;
        let mut go_on_count = 0;
        let mut retry_count = 0;
        let mut typo_count = 0;
        let mut please_thanks_count = 0;
        let mut total_user_messages = 0;
        let mut prompt_frequency: HashMap<String, usize> = HashMap::new();
        let mut frustration_examples = Vec::new();

        let frustration_patterns = vec![
            r"wtf",
            r"what the fuck",
            r"no,",
            r"stop",
            r"please",
            r"beg",
            r"come on",
            r"fucking",
            r"you must",
            r"you just deleted",
        ];
        let politeness_patterns = vec![r"please", r"thank", r"thanks"];

        let patterns = vec![
            ("Cline", ".config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks"),
            ("Cline (Flatpak)", ".var/app/com.visualstudio.code/config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks"),
            ("Kilo", ".config/Code/User/globalStorage/kilocode.kilo-code/tasks"),
            ("Roo-Cline", ".config/Code/User/globalStorage/rooveterinaryinc.roo-cline/tasks"),
        ];

        for (_, pattern) in patterns {
            let path = self.base_dir.join(pattern);
            if !path.exists() {
                continue;
            }

            for entry in WalkDir::new(&path)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let api_history = entry.path().join("api_conversation_history.json");
                if !api_history.exists() {
                    continue;
                }

                if let Ok(content) = fs::read_to_string(&api_history) {
                    if let Ok(messages) = serde_json::from_str::<Vec<ClineMessage>>(&content) {
                        for msg in &messages {
                            if msg.role != "user" {
                                continue;
                            }

                            total_user_messages += 1;
                            let text = msg.content.to_string().to_lowercase();

                            // Track prompt frequency (use char_indices to avoid UTF-8 issues)
                            let short_text = text.chars().take(50).collect::<String>();
                            *prompt_frequency.entry(short_text.clone()).or_insert(0) += 1;

                            // Detect "go on"
                            if text.contains("go on") {
                                go_on_count += 1;
                            }

                            // Detect retries
                            if text.contains("try again") || text.contains("try agagin") {
                                retry_count += 1;
                                typo_count += text.matches("agagin").count();
                            }

                            // Detect frustration
                            for pattern in &frustration_patterns {
                                if text.contains(pattern) {
                                    frustration_count += 1;
                                    if frustration_examples.len() < 5 {
                                        let original = msg.content.to_string();
                                        if original.len() < 100 {
                                            frustration_examples.push(original);
                                        }
                                    }
                                    break;
                                }
                            }

                            // Detect politeness
                            for pattern in &politeness_patterns {
                                if text.contains(pattern) {
                                    please_thanks_count += 1;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Top prompts
        let mut sorted_prompts: Vec<_> = prompt_frequency.into_iter().collect();
        sorted_prompts.sort_by(|a, b| b.1.cmp(&a.1));
        let most_common_prompts = sorted_prompts.into_iter().take(10).collect();

        let politeness_score = if total_user_messages > 0 {
            (please_thanks_count as f64 / total_user_messages as f64) * 100.0
        } else {
            0.0
        };

        Ok(BehaviorPatterns {
            frustration_count,
            go_on_count,
            retry_count,
            typo_count,
            command_spam_events: 0, // TODO: Implement
            politeness_score,
            most_common_prompts,
            frustration_examples,
        })
    }

    fn analyze_time_patterns(&self) -> Result<TimeAnalytics> {
        info!("⏰ Analyzing time patterns...");

        let mut hourly: HashMap<usize, usize> = HashMap::new();
        let mut daily: HashMap<String, usize> = HashMap::new();
        let mut late_night_sessions = 0;
        let mut binge_sessions = 0;
        let mut earliest: Option<i64> = None;
        let mut latest: Option<i64> = None;
        let mut session_timestamps: Vec<i64> = Vec::new();

        let patterns = vec![
            ("Cline", ".config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks"),
            ("Cline (Flatpak)", ".var/app/com.visualstudio.code/config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks"),
        ];

        for (_, pattern) in patterns {
            let path = self.base_dir.join(pattern);
            if !path.exists() {
                continue;
            }

            for entry in WalkDir::new(&path)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let api_history = entry.path().join("api_conversation_history.json");
                if !api_history.exists() {
                    continue;
                }

                if let Ok(content) = fs::read_to_string(&api_history) {
                    if let Ok(messages) = serde_json::from_str::<Vec<ClineMessage>>(&content) {
                        let mut session_start: Option<i64> = None;
                        let mut session_end: Option<i64> = None;

                        for msg in &messages {
                            if let Some(ts) = msg.ts {
                                // Track session times
                                if session_start.is_none() {
                                    session_start = Some(ts);
                                }
                                session_end = Some(ts);

                                // Convert to DateTime
                                if let Some(dt) = DateTime::<Utc>::from_timestamp(ts / 1000, 0) {
                                    let hour = dt.hour() as usize;
                                    let weekday = dt.weekday().to_string();

                                    *hourly.entry(hour).or_insert(0) += 1;
                                    *daily.entry(weekday).or_insert(0) += 1;

                                    // Late night (00:00-05:00)
                                    if hour < 5 {
                                        late_night_sessions += 1;
                                    }

                                    // Track earliest/latest
                                    if earliest.is_none_or(|e| ts < e) {
                                        earliest = Some(ts);
                                    }
                                    if latest.is_none_or(|l| ts > l) {
                                        latest = Some(ts);
                                    }
                                }
                            }
                        }

                        // Check for binge sessions (8+ hours)
                        if let (Some(start), Some(end)) = (session_start, session_end) {
                            let duration_hours = (end - start) as f64 / (1000.0 * 3600.0);
                            if duration_hours >= 8.0 {
                                binge_sessions += 1;
                            }
                            session_timestamps.push(start);
                        }
                    }
                }
            }
        }

        // Find most productive hour/day
        let most_productive_hour = hourly
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(&hour, _)| hour)
            .unwrap_or(12);
        let most_productive_day = daily
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(day, _)| day.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // Calculate average session gap
        session_timestamps.sort();
        let mut gaps = Vec::new();
        for i in 1..session_timestamps.len() {
            let gap =
                (session_timestamps[i] - session_timestamps[i - 1]) as f64 / (1000.0 * 3600.0);
            gaps.push(gap);
        }
        let avg_gap = if !gaps.is_empty() {
            gaps.iter().sum::<f64>() / gaps.len() as f64
        } else {
            0.0
        };

        let earliest_str = earliest
            .and_then(|ts| DateTime::<Utc>::from_timestamp(ts / 1000, 0).map(|dt| dt.to_rfc3339()))
            .unwrap_or_else(|| "Unknown".to_string());

        let latest_str = latest
            .and_then(|ts| DateTime::<Utc>::from_timestamp(ts / 1000, 0).map(|dt| dt.to_rfc3339()))
            .unwrap_or_else(|| "Unknown".to_string());

        Ok(TimeAnalytics {
            hourly_heatmap: hourly,
            daily_heatmap: daily,
            late_night_sessions,
            binge_coding_sessions: binge_sessions,
            most_productive_hour,
            most_productive_day,
            earliest_session: earliest_str,
            latest_session: latest_str,
            average_session_gap_hours: avg_gap,
        })
    }

    fn analyze_code_velocity(&self) -> Result<CodeVelocity> {
        info!("🚀 Analyzing code velocity...");

        // TODO: Parse file-history for detailed metrics
        Ok(CodeVelocity {
            lines_per_hour: 0.0,
            files_per_session: 0.0,
            biggest_refactor_lines: 0,
            most_edited_file: "Unknown".to_string(),
            most_edited_file_times: 0,
            total_edits: 0,
            total_new_files: 0,
            edit_to_create_ratio: 0.0,
        })
    }

    fn unlock_achievements(
        &self,
        behavior: &BehaviorPatterns,
        time: &TimeAnalytics,
    ) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        // "Night Owl" - Late night sessions
        achievements.push(Achievement {
            name: "Night Owl".to_string(),
            description: format!("Coded after midnight {} times", time.late_night_sessions),
            unlocked: time.late_night_sessions > 10,
            emoji: "🦉".to_string(),
        });

        // "Marathon Coder" - Binge sessions
        achievements.push(Achievement {
            name: "Marathon Coder".to_string(),
            description: format!(
                "{} coding sessions over 8 hours",
                time.binge_coding_sessions
            ),
            unlocked: time.binge_coding_sessions > 0,
            emoji: "🏃".to_string(),
        });

        // "Patience Tester" - Go on count
        achievements.push(Achievement {
            name: "Patience Tester".to_string(),
            description: format!("Said 'go on' {} times", behavior.go_on_count),
            unlocked: behavior.go_on_count > 100,
            emoji: "⏭️".to_string(),
        });

        // "Rage Coder" - Frustration count
        achievements.push(Achievement {
            name: "Rage Coder".to_string(),
            description: format!("Frustrated moments: {}", behavior.frustration_count),
            unlocked: behavior.frustration_count > 50,
            emoji: "😤".to_string(),
        });

        // "Polite Developer" - High politeness
        achievements.push(Achievement {
            name: "Polite Developer".to_string(),
            description: format!("{:.1}% politeness score", behavior.politeness_score),
            unlocked: behavior.politeness_score > 20.0,
            emoji: "🙏".to_string(),
        });

        // "Token Millionaire" - 100M+ tokens
        achievements.push(Achievement {
            name: "Token Millionaire".to_string(),
            description: "Consumed over 100 million tokens".to_string(),
            unlocked: self.total_tokens > 100_000_000,
            emoji: "💰".to_string(),
        });

        // "Big Spender" - $500+ spent
        achievements.push(Achievement {
            name: "Big Spender".to_string(),
            description: format!("Spent ${:.2} on AI", self.total_cost),
            unlocked: self.total_cost > 500.0,
            emoji: "💸".to_string(),
        });

        achievements
    }

    fn generate_comparisons(&self) -> Comparisons {
        // War and Peace: 587,287 words = ~700k tokens
        let war_and_peace = self.total_tokens as f64 / 700_000.0;

        // Harry Potter series: 1,084,170 words = ~1.3M tokens
        let harry_potter = self.total_tokens as f64 / 1_300_000.0;

        // Average movie script: 20k tokens
        let movie_scripts = self.total_tokens as f64 / 20_000.0;

        // Tweet: 70 tokens average
        let tweets = (self.total_tokens as f64 / 70.0) as usize;

        // StackOverflow answer: 500 tokens
        let stackoverflow = (self.total_tokens as f64 / 500.0) as usize;

        Comparisons {
            war_and_peace_equivalent: war_and_peace,
            harry_potter_series: harry_potter,
            movie_scripts,
            tweets,
            stackoverflow_answers: stackoverflow,
        }
    }

    fn find_records(&self) -> Result<Records> {
        info!("🏆 Finding records...");

        let mut longest_messages = 0;
        let mut longest_tokens = 0u64;
        let mut most_expensive_conv = 0.0;

        let patterns = vec![
            ("Cline", ".config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks"),
            ("Cline (Flatpak)", ".var/app/com.visualstudio.code/config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks"),
        ];

        for (_, pattern) in patterns {
            let path = self.base_dir.join(pattern);
            if !path.exists() {
                continue;
            }

            for entry in WalkDir::new(&path)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let api_history = entry.path().join("api_conversation_history.json");
                if !api_history.exists() {
                    continue;
                }

                if let Ok(content) = fs::read_to_string(&api_history) {
                    if let Ok(messages) = serde_json::from_str::<Vec<ClineMessage>>(&content) {
                        let msg_count = messages.len();
                        if msg_count > longest_messages {
                            longest_messages = msg_count;
                        }

                        // Calculate tokens
                        let char_count: usize =
                            messages.iter().map(|m| m.content.to_string().len()).sum();
                        let tokens = (char_count / 4) as u64;

                        if tokens > longest_tokens {
                            longest_tokens = tokens;
                        }

                        // Estimate cost (rough)
                        let cost = (tokens as f64 / 1_000_000.0) * 9.0; // ~$9 per million
                        if cost > most_expensive_conv {
                            most_expensive_conv = cost;
                        }
                    }
                }
            }
        }

        Ok(Records {
            longest_session_messages: longest_messages,
            longest_session_tokens: longest_tokens,
            most_messages_in_hour: 0,     // TODO
            fastest_bug_fix_minutes: 0.0, // TODO
            biggest_file_edited_mb: 0.0,  // TODO
            most_expensive_conversation_usd: most_expensive_conv,
        })
    }
}
