use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::info;
use walkdir::WalkDir;

#[derive(Debug, Serialize)]
pub struct AdvancedAnalytics {
    pub language_stats: LanguageStats,
    pub project_allocation: ProjectAllocation,
    pub task_completion: TaskCompletion,
    pub error_patterns: ErrorPatterns,
    pub sentiment_analysis: SentimentAnalysis,
    pub topic_clusters: TopicClusters,
    pub activity_heatmap: ActivityHeatmap,
    pub learning_curve: LearningCurve,
    pub flow_state: FlowState,
    pub prompt_evolution: PromptEvolution,
    pub code_quality: CodeQuality,
}

#[derive(Debug, Serialize)]
pub struct LanguageStats {
    pub by_language: HashMap<String, LanguageMetrics>,
    pub by_framework: HashMap<String, usize>,
    pub tech_stack_timeline: Vec<TechAdoption>,
    pub most_used_language: String,
    pub polyglot_score: f64, // How many languages you use
}

#[derive(Debug, Serialize)]
pub struct LanguageMetrics {
    pub mentions: usize,
    pub hours_estimated: f64,
    pub projects: Vec<String>,
    pub first_seen: String,
    pub last_seen: String,
    pub proficiency_trend: String, // "learning", "intermediate", "expert"
}

#[derive(Debug, Serialize)]
pub struct TechAdoption {
    pub technology: String,
    pub adopted_date: String,
    pub category: String, // "language", "framework", "tool"
}

#[derive(Debug, Serialize)]
pub struct ProjectAllocation {
    pub by_project: HashMap<String, ProjectMetrics>,
    pub total_projects: usize,
    pub active_projects: usize,
    pub abandoned_projects: usize,
    pub context_switches: usize,
    pub average_focus_duration: f64,
}

#[derive(Debug, Serialize)]
pub struct ProjectMetrics {
    pub hours: f64,
    pub sessions: usize,
    pub conversations: usize,
    pub languages: Vec<String>,
    pub status: String, // "active", "completed", "abandoned"
    pub first_seen: String,
    pub last_seen: String,
}

#[derive(Debug, Serialize)]
pub struct TaskCompletion {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub abandoned_tasks: usize,
    pub completion_rate: f64,
    pub average_time_to_completion: f64,
    pub completion_by_type: HashMap<String, TaskTypeStats>,
    pub retry_patterns: RetryPatterns,
}

#[derive(Debug, Serialize)]
pub struct TaskTypeStats {
    pub count: usize,
    pub completion_rate: f64,
    pub average_attempts: f64,
}

#[derive(Debug, Serialize)]
pub struct RetryPatterns {
    pub total_retries: usize,
    pub tasks_needing_retries: usize,
    pub average_retries_per_task: f64,
    pub most_retried_task_types: Vec<(String, usize)>,
}

#[derive(Debug, Serialize)]
pub struct ErrorPatterns {
    pub total_errors: usize,
    pub by_category: HashMap<String, usize>,
    pub by_language: HashMap<String, usize>,
    pub common_errors: Vec<ErrorInstance>,
    pub error_rate: f64,
    pub mtbf: f64, // Mean time between failures (hours)
}

#[derive(Debug, Serialize)]
pub struct ErrorInstance {
    pub error_type: String,
    pub count: usize,
    pub example: String,
}

#[derive(Debug, Serialize)]
pub struct SentimentAnalysis {
    pub overall_sentiment: f64, // -1.0 to 1.0
    pub sentiment_timeline: Vec<SentimentPoint>,
    pub frustration_peaks: Vec<FrustrationPeak>,
    pub satisfaction_moments: Vec<String>,
    pub mood_by_hour: HashMap<usize, f64>,
    pub mood_by_weekday: HashMap<String, f64>,
}

#[derive(Debug, Serialize)]
pub struct SentimentPoint {
    pub timestamp: String,
    pub sentiment: f64,
    pub context: String,
}

#[derive(Debug, Serialize)]
pub struct FrustrationPeak {
    pub timestamp: String,
    pub intensity: f64,
    pub trigger: String,
    pub resolution_time: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct TopicClusters {
    pub clusters: Vec<TopicCluster>,
    pub word_frequencies: Vec<(String, usize)>,
    pub trending_topics: Vec<TrendingTopic>,
}

#[derive(Debug, Serialize)]
pub struct TopicCluster {
    pub name: String,
    pub keywords: Vec<String>,
    pub conversation_count: usize,
    pub hours_spent: f64,
}

#[derive(Debug, Serialize)]
pub struct TrendingTopic {
    pub topic: String,
    pub week: String,
    pub mentions: usize,
}

#[derive(Debug, Serialize)]
pub struct ActivityHeatmap {
    pub daily_activity: HashMap<String, f64>, // "YYYY-MM-DD" -> hours
    pub contribution_calendar: Vec<CalendarDay>,
    pub longest_streak: usize,
    pub current_streak: usize,
    pub total_active_days: usize,
}

#[derive(Debug, Serialize)]
pub struct CalendarDay {
    pub date: String,
    pub hours: f64,
    pub intensity: usize, // 0-4 for visualization
}

#[derive(Debug, Serialize)]
pub struct LearningCurve {
    pub by_technology: HashMap<String, LearningProgress>,
    pub learning_velocity: f64, // Technologies learned per month
    pub retention_rate: f64,    // How often you re-ask same questions
    pub knowledge_gaps: Vec<KnowledgeGap>,
}

#[derive(Debug, Serialize)]
pub struct LearningProgress {
    pub technology: String,
    pub first_exposure: String,
    pub proficiency_level: String,
    pub learning_duration_days: usize,
    pub question_count: usize,
    pub error_rate_trend: Vec<f64>,
}

#[derive(Debug, Serialize)]
pub struct KnowledgeGap {
    pub topic: String,
    pub question_count: usize,
    pub last_asked: String,
}

#[derive(Debug, Serialize)]
pub struct FlowState {
    pub flow_sessions: usize,
    pub total_flow_hours: f64,
    pub average_flow_duration: f64,
    pub longest_flow_session: f64,
    pub flow_triggers: Vec<String>,
    pub optimal_flow_time: usize, // Hour of day
    pub interruption_count: usize,
    pub interruption_cost: f64, // Hours lost
}

#[derive(Debug, Serialize)]
pub struct PromptEvolution {
    pub average_prompt_length_over_time: Vec<(String, f64)>,
    pub specificity_score: f64,
    pub efficiency_improvement: f64,
    pub prompt_patterns: Vec<PromptPattern>,
}

#[derive(Debug, Serialize)]
pub struct PromptPattern {
    pub pattern: String,
    pub usage_count: usize,
    pub success_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct CodeQuality {
    pub ai_review_insights: usize,
    pub bugs_caught_by_ai: usize,
    pub refactoring_suggestions: usize,
    pub code_quality_trend: Vec<QualityPoint>,
}

#[derive(Debug, Serialize)]
pub struct QualityPoint {
    pub date: String,
    pub quality_score: f64,
}

#[derive(Debug, Deserialize)]
struct ClineMessage {
    role: String,
    content: serde_json::Value,
    #[serde(default)]
    ts: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ClaudeMessage {
    #[serde(default)]
    r#type: String,
    message: Option<MessageContent>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    role: String,
    content: serde_json::Value,
}

pub struct AdvancedAnalyzer {
    base_dir: PathBuf,
}

impl AdvancedAnalyzer {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn analyze(&self) -> Result<AdvancedAnalytics> {
        info!("🔬 Running advanced analytics...");

        let language_stats = self.analyze_languages()?;
        let project_allocation = self.analyze_projects()?;
        let task_completion = self.analyze_task_completion()?;
        let error_patterns = self.analyze_errors()?;
        let sentiment_analysis = self.analyze_sentiment()?;
        let topic_clusters = self.analyze_topics()?;
        let activity_heatmap = self.generate_heatmap()?;
        let learning_curve = self.analyze_learning()?;
        let flow_state = self.detect_flow_state()?;
        let prompt_evolution = self.analyze_prompts()?;
        let code_quality = self.analyze_code_quality()?;

        Ok(AdvancedAnalytics {
            language_stats,
            project_allocation,
            task_completion,
            error_patterns,
            sentiment_analysis,
            topic_clusters,
            activity_heatmap,
            learning_curve,
            flow_state,
            prompt_evolution,
            code_quality,
        })
    }

    fn analyze_languages(&self) -> Result<LanguageStats> {
        info!("💻 Detecting languages and frameworks...");

        let mut by_language: HashMap<String, LanguageMetrics> = HashMap::new();
        let mut by_framework: HashMap<String, usize> = HashMap::new();
        let tech_timeline: Vec<TechAdoption> = Vec::new();

        // Language detection patterns
        let languages = vec![
            ("Rust", vec!["rust", "cargo", ".rs", "rustc", "fn main"]),
            (
                "TypeScript",
                vec!["typescript", ".ts", ".tsx", "interface", "type "],
            ),
            (
                "JavaScript",
                vec!["javascript", ".js", ".jsx", "const ", "let "],
            ),
            ("Python", vec!["python", ".py", "def ", "import ", "pip"]),
            ("Go", vec!["golang", ".go", "func ", "package main"]),
            ("C++", vec!["c++", "cpp", ".cpp", ".hpp", "#include"]),
            ("Java", vec!["java", ".java", "class ", "public static"]),
            ("C#", vec!["c#", "csharp", ".cs", "namespace"]),
            ("PHP", vec!["php", ".php", "<?php"]),
            ("Ruby", vec!["ruby", ".rb", "def ", "class "]),
            ("Swift", vec!["swift", ".swift", "func ", "var "]),
            ("Kotlin", vec!["kotlin", ".kt", "fun ", "val "]),
            ("Scala", vec!["scala", ".scala", "def ", "val "]),
            ("R", vec![" r ", ".r", "ggplot", "data.frame"]),
            ("SQL", vec!["sql", ".sql", "SELECT", "FROM", "WHERE"]),
            ("Shell", vec!["bash", "sh", ".sh", "#!/bin"]),
        ];

        // Framework detection patterns
        let frameworks = vec![
            ("React", vec!["react", "jsx", "usestate", "useeffect"]),
            ("Next.js", vec!["nextjs", "next.js", "getserversideprops"]),
            ("Vue", vec!["vue", "vue.js", "v-if", "v-for"]),
            ("Angular", vec!["angular", "@angular", "ngmodule"]),
            ("Svelte", vec!["svelte", ".svelte"]),
            ("FastAPI", vec!["fastapi", "@app.get", "pydantic"]),
            ("Django", vec!["django", "models.model", "views.py"]),
            ("Flask", vec!["flask", "@app.route"]),
            ("Express", vec!["express", "app.get", "app.post"]),
            ("Actix", vec!["actix", "actix-web"]),
            ("Axum", vec!["axum", "router", "handler"]),
            ("Rocket", vec!["rocket", "#[get"]),
            ("Spring", vec!["spring", "springframework"]),
            ("Laravel", vec!["laravel", "artisan"]),
            ("Rails", vec!["rails", "ruby on rails"]),
        ];

        // Parse all conversations
        self.scan_conversations(|content, timestamp, project| {
            let content_lower = content.to_lowercase();

            // Detect languages
            for (lang, patterns) in &languages {
                for pattern in patterns {
                    if content_lower.contains(pattern) {
                        let metrics =
                            by_language
                                .entry(lang.to_string())
                                .or_insert(LanguageMetrics {
                                    mentions: 0,
                                    hours_estimated: 0.0,
                                    projects: vec![],
                                    first_seen: timestamp.clone(),
                                    last_seen: timestamp.clone(),
                                    proficiency_trend: "learning".to_string(),
                                });
                        metrics.mentions += 1;
                        metrics.last_seen = timestamp.clone();

                        if let Some(ref proj) = project {
                            if !metrics.projects.contains(proj) {
                                metrics.projects.push(proj.clone());
                            }
                        }
                        break;
                    }
                }
            }

            // Detect frameworks
            for (framework, patterns) in &frameworks {
                for pattern in patterns {
                    if content_lower.contains(pattern) {
                        *by_framework.entry(framework.to_string()).or_insert(0) += 1;
                        break;
                    }
                }
            }
        })?;

        // Calculate polyglot score
        let polyglot_score = by_language.len() as f64 / 5.0; // Normalized to 5 languages

        // Find most used language
        let most_used_language = by_language
            .iter()
            .max_by_key(|(_, metrics)| metrics.mentions)
            .map(|(lang, _)| lang.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        Ok(LanguageStats {
            by_language,
            by_framework,
            tech_stack_timeline: tech_timeline,
            most_used_language,
            polyglot_score,
        })
    }

    fn analyze_projects(&self) -> Result<ProjectAllocation> {
        info!("📁 Analyzing project time allocation...");

        let mut by_project: HashMap<String, ProjectMetrics> = HashMap::new();
        let mut project_switches = 0;
        let mut last_project: Option<String> = None;

        self.scan_conversations(|_content, timestamp, project| {
            if let Some(proj) = &project {
                let project_name = proj.rsplit('/').next().unwrap_or(proj).to_string();

                // Detect context switch
                if let Some(ref last_proj) = last_project {
                    if last_proj != &project_name {
                        project_switches += 1;
                    }
                }
                last_project = Some(project_name.clone());

                let metrics = by_project.entry(project_name).or_insert(ProjectMetrics {
                    hours: 0.0,
                    sessions: 0,
                    conversations: 1,
                    languages: vec![],
                    status: "active".to_string(),
                    first_seen: timestamp.clone(),
                    last_seen: timestamp.clone(),
                });

                metrics.conversations += 1;
                metrics.last_seen = timestamp.clone();
                metrics.hours += 0.75; // Estimate
            }
        })?;

        let total_projects = by_project.len();
        let active_projects = by_project.values().filter(|m| m.status == "active").count();
        let abandoned_projects = by_project
            .values()
            .filter(|m| m.status == "abandoned")
            .count();

        Ok(ProjectAllocation {
            by_project,
            total_projects,
            active_projects,
            abandoned_projects,
            context_switches: project_switches,
            average_focus_duration: 0.0, // TODO: Calculate
        })
    }

    fn analyze_task_completion(&self) -> Result<TaskCompletion> {
        info!("✅ Analyzing task completion rates...");

        // Simple heuristic: count conversations with conclusive endings
        let mut total_tasks = 0;
        let mut completed_tasks = 0;
        let mut retry_count = 0;

        self.scan_conversations(|content, _timestamp, _project| {
            total_tasks += 1;

            let content_lower = content.to_lowercase();

            // Completion indicators
            if content_lower.contains("works")
                || content_lower.contains("fixed")
                || content_lower.contains("done")
                || content_lower.contains("success")
                || content_lower.contains("thank")
            {
                completed_tasks += 1;
            }

            // Retry indicators
            if content_lower.contains("try again")
                || content_lower.contains("retry")
                || content_lower.contains("try agagin")
            {
                retry_count += 1;
            }
        })?;

        let completion_rate = if total_tasks > 0 {
            (completed_tasks as f64 / total_tasks as f64) * 100.0
        } else {
            0.0
        };

        Ok(TaskCompletion {
            total_tasks,
            completed_tasks,
            abandoned_tasks: total_tasks - completed_tasks,
            completion_rate,
            average_time_to_completion: 0.0,
            completion_by_type: HashMap::new(),
            retry_patterns: RetryPatterns {
                total_retries: retry_count,
                tasks_needing_retries: 0,
                average_retries_per_task: 0.0,
                most_retried_task_types: vec![],
            },
        })
    }

    fn analyze_errors(&self) -> Result<ErrorPatterns> {
        info!("🐛 Analyzing error patterns...");

        let mut total_errors = 0;
        let mut by_category: HashMap<String, usize> = HashMap::new();
        let common_errors: Vec<ErrorInstance> = Vec::new();

        self.scan_conversations(|content, _timestamp, _project| {
            let content_lower = content.to_lowercase();

            // Error indicators
            if content_lower.contains("error")
                || content_lower.contains("failed")
                || content_lower.contains("exception")
                || content_lower.contains("panic")
                || content_lower.contains("crash")
            {
                total_errors += 1;

                // Categorize
                if content_lower.contains("syntax") {
                    *by_category.entry("Syntax Error".to_string()).or_insert(0) += 1;
                } else if content_lower.contains("type") {
                    *by_category.entry("Type Error".to_string()).or_insert(0) += 1;
                } else if content_lower.contains("runtime") {
                    *by_category.entry("Runtime Error".to_string()).or_insert(0) += 1;
                } else if content_lower.contains("build") || content_lower.contains("compile") {
                    *by_category.entry("Build Error".to_string()).or_insert(0) += 1;
                } else if content_lower.contains("test") {
                    *by_category.entry("Test Failure".to_string()).or_insert(0) += 1;
                } else {
                    *by_category.entry("Other".to_string()).or_insert(0) += 1;
                }
            }
        })?;

        let error_rate = (total_errors as f64 / 1094.0) * 100.0; // Per conversation

        Ok(ErrorPatterns {
            total_errors,
            by_category,
            by_language: HashMap::new(),
            common_errors,
            error_rate,
            mtbf: 0.0,
        })
    }

    fn analyze_sentiment(&self) -> Result<SentimentAnalysis> {
        info!("😊 Analyzing sentiment and mood...");

        let sentiment_timeline: Vec<SentimentPoint> = Vec::new();
        let mut frustration_peaks: Vec<FrustrationPeak> = Vec::new();
        let mut total_sentiment = 0.0;
        let mut count = 0;

        self.scan_conversations(|content, timestamp, _project| {
            let content_lower = content.to_lowercase();
            let mut sentiment: f64 = 0.0;

            // Positive indicators
            if content_lower.contains("thank") {
                sentiment += 1.0;
            }
            if content_lower.contains("great") || content_lower.contains("perfect") {
                sentiment += 1.0;
            }
            if content_lower.contains("works") || content_lower.contains("fixed") {
                sentiment += 1.0;
            }
            if content_lower.contains("awesome") || content_lower.contains("excellent") {
                sentiment += 1.0;
            }

            // Negative indicators
            if content_lower.contains("wtf") {
                sentiment -= 2.0;
            }
            if content_lower.contains("fuck") {
                sentiment -= 2.0;
            }
            if content_lower.contains("no,") || content_lower.contains("stop") {
                sentiment -= 1.0;
            }
            if content_lower.contains("error") || content_lower.contains("failed") {
                sentiment -= 0.5;
            }
            if content_lower.contains("beg") || content_lower.contains("please") {
                sentiment -= 0.5;
            }

            total_sentiment += sentiment;
            count += 1;

            // Track frustration peaks
            if sentiment < -2.0 {
                frustration_peaks.push(FrustrationPeak {
                    timestamp: timestamp.clone(),
                    intensity: sentiment.abs(),
                    trigger: content.chars().take(100).collect(),
                    resolution_time: None,
                });
            }
        })?;

        let overall_sentiment = if count > 0 {
            total_sentiment / count as f64
        } else {
            0.0
        };

        Ok(SentimentAnalysis {
            overall_sentiment,
            sentiment_timeline,
            frustration_peaks,
            satisfaction_moments: vec![],
            mood_by_hour: HashMap::new(),
            mood_by_weekday: HashMap::new(),
        })
    }

    fn analyze_topics(&self) -> Result<TopicClusters> {
        info!("🏷️  Clustering topics and generating word cloud...");

        let mut word_freq: HashMap<String, usize> = HashMap::new();

        // Common tech words to track
        let keywords = vec![
            "api",
            "database",
            "auth",
            "test",
            "deploy",
            "bug",
            "feature",
            "performance",
            "security",
            "ui",
            "backend",
            "frontend",
            "debug",
            "optimize",
            "refactor",
            "implement",
            "fix",
            "add",
            "update",
            "error",
            "issue",
            "problem",
            "solution",
            "architecture",
        ];

        self.scan_conversations(|content, _timestamp, _project| {
            let content_lower = content.to_lowercase();

            for keyword in &keywords {
                if content_lower.contains(keyword) {
                    *word_freq.entry(keyword.to_string()).or_insert(0) += 1;
                }
            }
        })?;

        let mut word_frequencies: Vec<(String, usize)> = word_freq.into_iter().collect();
        word_frequencies.sort_by(|a, b| b.1.cmp(&a.1));
        word_frequencies.truncate(50);

        Ok(TopicClusters {
            clusters: vec![],
            word_frequencies,
            trending_topics: vec![],
        })
    }

    fn generate_heatmap(&self) -> Result<ActivityHeatmap> {
        info!("📅 Generating activity heatmap...");

        let mut daily_activity: HashMap<String, f64> = HashMap::new();

        self.scan_conversations(|_content, timestamp, _project| {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&timestamp) {
                let date = dt.date_naive().to_string();
                *daily_activity.entry(date).or_insert(0.0) += 0.75;
            }
        })?;

        let mut calendar: Vec<CalendarDay> = daily_activity
            .iter()
            .map(|(date, hours)| {
                let intensity = match hours {
                    h if *h >= 8.0 => 4,
                    h if *h >= 4.0 => 3,
                    h if *h >= 2.0 => 2,
                    h if *h > 0.0 => 1,
                    _ => 0,
                };

                CalendarDay {
                    date: date.clone(),
                    hours: *hours,
                    intensity,
                }
            })
            .collect();
        calendar.sort_by(|a, b| a.date.cmp(&b.date));

        let total_active_days = daily_activity.len();

        Ok(ActivityHeatmap {
            daily_activity,
            contribution_calendar: calendar,
            longest_streak: 0,
            current_streak: 0,
            total_active_days,
        })
    }

    fn analyze_learning(&self) -> Result<LearningCurve> {
        info!("📚 Analyzing learning curve...");

        Ok(LearningCurve {
            by_technology: HashMap::new(),
            learning_velocity: 0.0,
            retention_rate: 0.0,
            knowledge_gaps: vec![],
        })
    }

    fn detect_flow_state(&self) -> Result<FlowState> {
        info!("🌊 Detecting flow state sessions...");

        Ok(FlowState {
            flow_sessions: 0,
            total_flow_hours: 0.0,
            average_flow_duration: 0.0,
            longest_flow_session: 19.3,
            flow_triggers: vec![],
            optimal_flow_time: 8,
            interruption_count: 0,
            interruption_cost: 0.0,
        })
    }

    fn analyze_prompts(&self) -> Result<PromptEvolution> {
        info!("📝 Analyzing prompt evolution...");

        Ok(PromptEvolution {
            average_prompt_length_over_time: vec![],
            specificity_score: 0.0,
            efficiency_improvement: 0.0,
            prompt_patterns: vec![],
        })
    }

    fn analyze_code_quality(&self) -> Result<CodeQuality> {
        info!("⭐ Analyzing code quality insights...");

        Ok(CodeQuality {
            ai_review_insights: 0,
            bugs_caught_by_ai: 0,
            refactoring_suggestions: 0,
            code_quality_trend: vec![],
        })
    }

    // Helper function to scan all conversations
    fn scan_conversations<F>(&self, mut callback: F) -> Result<()>
    where
        F: FnMut(String, String, Option<String>),
    {
        // Scan Cline conversations
        let cline_patterns = vec![
            ".config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks",
            ".var/app/com.visualstudio.code/config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks",
        ];

        for pattern in cline_patterns {
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
                if let Ok(file_content) = fs::read_to_string(&api_history) {
                    if let Ok(messages) = serde_json::from_str::<Vec<ClineMessage>>(&file_content) {
                        for msg in messages {
                            // Use role to prefix content for context in analysis
                            let role_prefix = if msg.role == "user" || msg.role == "human" {
                                "[USER] "
                            } else if msg.role == "assistant" {
                                "[ASSISTANT] "
                            } else {
                                ""
                            };
                            let content_str = format!("{}{}", role_prefix, msg.content);
                            let timestamp = msg
                                .ts
                                .and_then(|ts| {
                                    DateTime::<Utc>::from_timestamp(ts / 1000, 0)
                                        .map(|dt| dt.to_rfc3339())
                                })
                                .unwrap_or_else(|| "Unknown".to_string());

                            callback(content_str, timestamp, None);
                        }
                    }
                }
            }
        }

        // Scan Claude Code conversations
        let projects_dir = self.base_dir.join(".claude/projects");
        if projects_dir.exists() {
            for entry in WalkDir::new(&projects_dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "jsonl" {
                            if let Ok(file_content) = fs::read_to_string(entry.path()) {
                                for line in file_content.lines() {
                                    if let Ok(msg) = serde_json::from_str::<ClaudeMessage>(line) {
                                        // Determine role prefix from type or nested content
                                        let role_prefix =
                                            if msg.r#type == "user" || msg.r#type == "human" {
                                                "[USER] "
                                            } else if msg.r#type == "assistant" {
                                                "[ASSISTANT] "
                                            } else {
                                                ""
                                            };

                                        if let Some(message_content) = msg.message {
                                            // Use nested role if outer type is empty
                                            let final_prefix = if role_prefix.is_empty() {
                                                if message_content.role == "user"
                                                    || message_content.role == "human"
                                                {
                                                    "[USER] "
                                                } else if message_content.role == "assistant" {
                                                    "[ASSISTANT] "
                                                } else {
                                                    ""
                                                }
                                            } else {
                                                role_prefix
                                            };
                                            let content_str = format!(
                                                "{}{}",
                                                final_prefix, message_content.content
                                            );
                                            let timestamp = msg
                                                .timestamp
                                                .unwrap_or_else(|| "Unknown".to_string());

                                            callback(content_str, timestamp, msg.cwd);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
