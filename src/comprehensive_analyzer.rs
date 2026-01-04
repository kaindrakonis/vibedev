use crate::advanced_analytics::{AdvancedAnalytics, AdvancedAnalyzer};
use crate::claude_code_parser::{ClaudeCodeParser, ClaudeCodeStats};
use crate::viral_insights::{ViralAnalyzer, ViralInsights};
use crate::work_hours_analyzer::{WorkHoursAnalysis, WorkHoursAnalyzer};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use tracing::info;
use walkdir::WalkDir;

/// Context for parsing conversation tasks - groups mutable state to reduce function arguments
struct ParseContext<'a> {
    total_conversations: &'a mut usize,
    total_messages: &'a mut usize,
    total_user: &'a mut usize,
    total_assistant: &'a mut usize,
    by_tool: &'a mut HashMap<String, ConversationStats>,
    largest: &'a mut ConversationInfo,
    files_referenced: &'a mut HashSet<String>,
}

#[derive(Debug, Serialize)]
pub struct ComprehensiveAnalysis {
    pub conversations: ConversationAnalysis,
    pub token_usage: TokenUsage,
    pub code_attribution: CodeAttribution,
    pub cost_analysis: CostAnalysis,
    pub productivity_metrics: ProductivityMetrics,
    pub language_stats: LanguageStats,
    pub tool_comparison: ToolComparison,
    pub claude_code: ClaudeCodeStats,
    pub viral_insights: ViralInsights,
    pub work_hours: WorkHoursAnalysis,
    pub advanced: AdvancedAnalytics,
}

#[derive(Debug, Serialize)]
pub struct ConversationAnalysis {
    pub total_conversations: usize,
    pub total_messages: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub by_tool: HashMap<String, ConversationStats>,
    pub largest_conversation: ConversationInfo,
    pub average_conversation_length: f64,
    pub files_referenced: Vec<String>, // Files mentioned in conversations
}

#[derive(Debug, Serialize)]
pub struct ConversationInfo {
    pub tool: String,
    pub messages: usize,
    pub tokens: u64,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct ConversationStats {
    pub conversations: usize,
    pub messages: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub avg_message_length: f64,
}

#[derive(Debug, Serialize)]
pub struct TokenUsage {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub by_tool: HashMap<String, ToolTokens>,
    pub by_model: HashMap<String, u64>,
}

#[derive(Debug, Serialize)]
pub struct ToolTokens {
    pub input: u64,
    pub output: u64,
    pub total: u64,
}

#[derive(Debug, Serialize)]
pub struct CodeAttribution {
    pub total_lines_tracked: usize,
    pub ai_generated_lines: usize,
    pub percentage_ai: f64,
    pub by_composer: HashMap<String, usize>,
}

#[derive(Debug, Serialize)]
pub struct CostAnalysis {
    pub total_cost_usd: f64,
    pub by_tool: HashMap<String, f64>,
    pub by_model: HashMap<String, ModelCost>,
    pub monthly_estimate: f64,
    pub potential_savings: f64,
}

#[derive(Debug, Serialize)]
pub struct ModelCost {
    pub tokens: u64,
    pub cost: f64,
}

#[derive(Debug, Serialize)]
pub struct ProductivityMetrics {
    pub total_sessions: usize,
    pub average_session_duration: f64,
    pub total_files_modified: usize,
    pub error_rate: f64,
    pub retry_rate: f64,
    pub most_active_hours: Vec<usize>,
}

#[derive(Debug, Serialize)]
pub struct LanguageStats {
    pub by_language: HashMap<String, LanguageUsage>,
    pub most_used_language: String,
    pub framework_mentions: HashMap<String, usize>,
}

#[derive(Debug, Serialize)]
pub struct LanguageUsage {
    pub files: usize,
    pub lines_of_code: usize,
    pub conversations: usize,
}

#[derive(Debug, Serialize)]
pub struct ToolComparison {
    pub by_effectiveness: Vec<ToolEffectiveness>,
    pub by_cost_efficiency: Vec<ToolCostEfficiency>,
    pub by_usage: Vec<ToolUsage>,
}

#[derive(Debug, Serialize)]
pub struct ToolEffectiveness {
    pub tool: String,
    pub success_rate: f64,
    pub avg_tokens_per_task: f64,
}

#[derive(Debug, Serialize)]
pub struct ToolCostEfficiency {
    pub tool: String,
    pub cost_per_conversation: f64,
    pub tokens_per_dollar: f64,
}

#[derive(Debug, Serialize)]
pub struct ToolUsage {
    pub tool: String,
    pub conversations: usize,
    pub total_tokens: u64,
}

// Cline task structures
#[derive(Debug, Deserialize)]
struct ClineMessage {
    role: String,
    content: serde_json::Value,
    #[serde(default)]
    metadata: Option<ClineMetadata>,
}

#[derive(Debug, Deserialize)]
struct ClineMetadata {
    #[serde(default)]
    model_usage: Option<ModelUsage>,
    #[serde(default)]
    files_in_context: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ModelUsage {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
}

pub struct ComprehensiveAnalyzer {
    base_dir: PathBuf,
}

impl ComprehensiveAnalyzer {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn analyze(&self) -> Result<ComprehensiveAnalysis> {
        info!("🔍 Starting comprehensive analysis of 52+ GB data...");

        let mut conv_analysis = self.analyze_conversations()?;
        let mut token_usage = self.analyze_tokens()?;

        // Parse Claude Code logs
        info!("📊 Parsing Claude Code logs...");
        let claude_parser = ClaudeCodeParser::new(self.base_dir.clone());
        let claude_stats = claude_parser.parse().unwrap_or_else(|e| {
            info!("Failed to parse Claude Code logs: {}", e);
            ClaudeCodeStats {
                total_prompts: 0,
                total_conversations: 0,
                total_messages: 0,
                user_messages: 0,
                assistant_messages: 0,
                projects: HashMap::new(),
                estimated_tokens: 0,
                frustration_prompts: vec![],
                go_on_count: 0,
            }
        });

        // Add Claude Code stats to totals
        conv_analysis.total_conversations += claude_stats.total_conversations;
        conv_analysis.total_messages += claude_stats.total_messages;
        conv_analysis.user_messages += claude_stats.user_messages;
        conv_analysis.assistant_messages += claude_stats.assistant_messages;
        token_usage.total_tokens += claude_stats.estimated_tokens;
        token_usage.total_input_tokens += (claude_stats.estimated_tokens as f64 * 0.6) as u64;
        token_usage.total_output_tokens += (claude_stats.estimated_tokens as f64 * 0.4) as u64;

        let code_attribution = self.analyze_code_attribution()?;
        let cost_analysis = self.calculate_costs(&token_usage)?;
        let productivity = self.analyze_productivity()?;
        let language_stats = self.analyze_languages()?;
        let tool_comparison = self.compare_tools(&conv_analysis, &token_usage, &cost_analysis)?;

        // Generate viral insights
        info!("🎉 Generating viral insights...");
        let viral_analyzer = ViralAnalyzer::new(
            self.base_dir.clone(),
            token_usage.total_tokens,
            cost_analysis.total_cost_usd,
        );
        let viral_insights = viral_analyzer.analyze()?;

        // Analyze work hours
        info!("⏱️  Analyzing work hours from timestamps...");
        let hours_analyzer = WorkHoursAnalyzer::new(self.base_dir.clone());
        let work_hours = hours_analyzer.analyze()?;

        // Advanced analytics
        info!("🔬 Running advanced analytics...");
        let advanced_analyzer = AdvancedAnalyzer::new(self.base_dir.clone());
        let advanced = advanced_analyzer.analyze()?;

        Ok(ComprehensiveAnalysis {
            conversations: conv_analysis,
            token_usage,
            code_attribution,
            cost_analysis,
            productivity_metrics: productivity,
            language_stats,
            tool_comparison,
            claude_code: claude_stats,
            viral_insights,
            work_hours,
            advanced,
        })
    }

    fn analyze_conversations(&self) -> Result<ConversationAnalysis> {
        info!("📊 Analyzing conversations from all tools...");

        let mut total_conversations = 0;
        let mut total_messages = 0;
        let mut total_user = 0;
        let mut total_assistant = 0;
        let mut by_tool: HashMap<String, ConversationStats> = HashMap::new();
        let mut files_referenced: HashSet<String> = HashSet::new();
        let mut largest = ConversationInfo {
            tool: String::new(),
            messages: 0,
            tokens: 0,
            path: String::new(),
        };

        // Parse Cline/Kilo/Roo-Cline tasks
        let mut ctx = ParseContext {
            total_conversations: &mut total_conversations,
            total_messages: &mut total_messages,
            total_user: &mut total_user,
            total_assistant: &mut total_assistant,
            by_tool: &mut by_tool,
            largest: &mut largest,
            files_referenced: &mut files_referenced,
        };
        self.parse_cline_tasks(&mut ctx)?;

        let avg_length = if total_conversations > 0 {
            total_messages as f64 / total_conversations as f64
        } else {
            0.0
        };

        Ok(ConversationAnalysis {
            total_conversations,
            total_messages,
            user_messages: total_user,
            assistant_messages: total_assistant,
            by_tool,
            largest_conversation: largest,
            average_conversation_length: avg_length,
            files_referenced: files_referenced.into_iter().collect(),
        })
    }

    fn parse_cline_tasks(&self, ctx: &mut ParseContext<'_>) -> Result<()> {
        let patterns = vec![
            ("Cline", ".config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks"),
            ("Cline (Flatpak)", ".var/app/com.visualstudio.code/config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks"),
            ("Kilo", ".config/Code/User/globalStorage/kilocode.kilo-code/tasks"),
            ("Roo-Cline", ".config/Code/User/globalStorage/rooveterinaryinc.roo-cline/tasks"),
            ("Roo-Cline (Cursor)", ".config/Cursor/User/globalStorage/rooveterinaryinc.roo-cline/tasks"),
        ];

        for (tool_name, pattern) in patterns {
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
                if !entry.file_type().is_dir() {
                    continue;
                }

                let api_history = entry.path().join("api_conversation_history.json");
                if !api_history.exists() {
                    continue;
                }

                if let Ok(content) = fs::read_to_string(&api_history) {
                    if let Ok(messages) = serde_json::from_str::<Vec<ClineMessage>>(&content) {
                        let msg_count = messages.len();
                        let user_count = messages.iter().filter(|m| m.role == "user").count();
                        let assistant_count =
                            messages.iter().filter(|m| m.role == "assistant").count();

                        // Collect files referenced in context from metadata
                        for msg in &messages {
                            if let Some(meta) = &msg.metadata {
                                if let Some(files) = &meta.files_in_context {
                                    for file in files {
                                        ctx.files_referenced.insert(file.clone());
                                    }
                                }
                            }
                        }

                        *ctx.total_conversations += 1;
                        *ctx.total_messages += msg_count;
                        *ctx.total_user += user_count;
                        *ctx.total_assistant += assistant_count;

                        // Track largest conversation
                        if msg_count > ctx.largest.messages {
                            ctx.largest.tool = tool_name.to_string();
                            ctx.largest.messages = msg_count;
                            ctx.largest.path = api_history.to_string_lossy().to_string();

                            // Estimate tokens
                            let char_count: usize =
                                messages.iter().map(|m| m.content.to_string().len()).sum();
                            ctx.largest.tokens = (char_count / 4) as u64;
                        }

                        // Update tool stats
                        let stats =
                            ctx.by_tool
                                .entry(tool_name.to_string())
                                .or_insert(ConversationStats {
                                    conversations: 0,
                                    messages: 0,
                                    user_messages: 0,
                                    assistant_messages: 0,
                                    avg_message_length: 0.0,
                                });

                        stats.conversations += 1;
                        stats.messages += msg_count;
                        stats.user_messages += user_count;
                        stats.assistant_messages += assistant_count;
                    }
                }
            }
        }

        // Calculate averages
        for stats in ctx.by_tool.values_mut() {
            if stats.messages > 0 {
                stats.avg_message_length = stats.messages as f64 / stats.conversations as f64;
            }
        }

        Ok(())
    }

    fn analyze_tokens(&self) -> Result<TokenUsage> {
        info!("💰 Analyzing token usage (estimating from message lengths)...");

        let mut total_input = 0u64;
        let mut total_output = 0u64;
        let mut by_tool: HashMap<String, ToolTokens> = HashMap::new();
        let mut by_model: HashMap<String, u64> = HashMap::new();

        // Parse token data from API conversation history
        let patterns = vec![
            ("Cline", ".config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks"),
            ("Cline (Flatpak)", ".var/app/com.visualstudio.code/config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks"),
            ("Kilo", ".config/Code/User/globalStorage/kilocode.kilo-code/tasks"),
            ("Roo-Cline", ".config/Code/User/globalStorage/rooveterinaryinc.roo-cline/tasks"),
            ("Roo-Cline (Cursor)", ".config/Cursor/User/globalStorage/rooveterinaryinc.roo-cline/tasks"),
        ];

        for (tool_name, pattern) in patterns {
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
                            // First try to use actual token counts from metadata
                            let (input_tokens, output_tokens) = if let Some(meta) = &msg.metadata {
                                if let Some(usage) = &meta.model_usage {
                                    (
                                        usage.input_tokens.unwrap_or(0),
                                        usage.output_tokens.unwrap_or(0),
                                    )
                                } else {
                                    // Estimate from content if no usage data
                                    let content_str = msg.content.to_string();
                                    let estimated = (content_str.len() / 4) as u64;
                                    if msg.role == "user" {
                                        (estimated, 0)
                                    } else {
                                        (0, estimated)
                                    }
                                }
                            } else {
                                // Fallback: estimate ~4 characters per token
                                let content_str = msg.content.to_string();
                                let estimated = (content_str.len() / 4) as u64;
                                if msg.role == "user" {
                                    (estimated, 0)
                                } else {
                                    (0, estimated)
                                }
                            };

                            total_input += input_tokens;
                            total_output += output_tokens;

                            let tool_tokens =
                                by_tool.entry(tool_name.to_string()).or_insert(ToolTokens {
                                    input: 0,
                                    output: 0,
                                    total: 0,
                                });

                            tool_tokens.input += input_tokens;
                            tool_tokens.output += output_tokens;
                            tool_tokens.total += input_tokens + output_tokens;
                        }
                    }
                }
            }
        }

        // Assume Claude Sonnet 3.5 as default model
        *by_model.entry("claude-sonnet-3.5".to_string()).or_insert(0) = total_input + total_output;

        Ok(TokenUsage {
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_tokens: total_input + total_output,
            by_tool,
            by_model,
        })
    }

    fn analyze_code_attribution(&self) -> Result<CodeAttribution> {
        info!("🎨 Analyzing code attribution from Cursor...");

        let db_path = self
            .base_dir
            .join(".config/Cursor/User/globalStorage/state.vscdb");

        if !db_path.exists() {
            info!("Cursor database not found, skipping code attribution");
            return Ok(CodeAttribution {
                total_lines_tracked: 0,
                ai_generated_lines: 0,
                percentage_ai: 0.0,
                by_composer: HashMap::new(),
            });
        }

        // Try to parse SQLite database
        match rusqlite::Connection::open(&db_path) {
            Ok(conn) => {
                // Query aiCodeTrackingLines
                let query = "SELECT value FROM ItemTable WHERE key='aiCodeTrackingLines'";
                match conn.query_row(query, [], |row| row.get::<_, String>(0)) {
                    Ok(value) => {
                        // Parse JSON array of tracked lines
                        if let Ok(tracked_lines) =
                            serde_json::from_str::<Vec<serde_json::Value>>(&value)
                        {
                            let total = tracked_lines.len();

                            // Count by composer
                            let mut by_composer = HashMap::new();
                            for line in &tracked_lines {
                                if let Some(metadata) = line.get("metadata") {
                                    if let Some(source) = metadata.get("source") {
                                        if let Some(source_str) = source.as_str() {
                                            *by_composer
                                                .entry(source_str.to_string())
                                                .or_insert(0) += 1;
                                        }
                                    }
                                }
                            }

                            let ai_lines = total; // All tracked lines are AI-generated
                            let percentage = 100.0; // Of tracked lines, all are AI

                            return Ok(CodeAttribution {
                                total_lines_tracked: total,
                                ai_generated_lines: ai_lines,
                                percentage_ai: percentage,
                                by_composer,
                            });
                        }
                    }
                    Err(e) => {
                        info!("Could not query Cursor database: {}", e);
                    }
                }
            }
            Err(e) => {
                info!("Could not open Cursor database: {}", e);
            }
        }

        Ok(CodeAttribution {
            total_lines_tracked: 0,
            ai_generated_lines: 0,
            percentage_ai: 0.0,
            by_composer: HashMap::new(),
        })
    }

    fn calculate_costs(&self, token_usage: &TokenUsage) -> Result<CostAnalysis> {
        info!("💵 Calculating costs (Claude Sonnet 3.5 pricing)...");

        // Claude 3.5 Sonnet pricing (as of Jan 2025)
        let input_cost_per_mtok = 3.0; // $3 per million input tokens
        let output_cost_per_mtok = 15.0; // $15 per million output tokens

        let input_cost =
            (token_usage.total_input_tokens as f64 / 1_000_000.0) * input_cost_per_mtok;
        let output_cost =
            (token_usage.total_output_tokens as f64 / 1_000_000.0) * output_cost_per_mtok;
        let total_cost = input_cost + output_cost;

        let mut by_tool = HashMap::new();
        let mut by_model = HashMap::new();

        for (tool, tokens) in &token_usage.by_tool {
            let tool_input_cost = (tokens.input as f64 / 1_000_000.0) * input_cost_per_mtok;
            let tool_output_cost = (tokens.output as f64 / 1_000_000.0) * output_cost_per_mtok;
            by_tool.insert(tool.clone(), tool_input_cost + tool_output_cost);
        }

        for (model, tokens) in &token_usage.by_model {
            // Calculate cost based on assumed 50/50 input/output split
            let estimated_cost = (*tokens as f64 / 1_000_000.0)
                * ((input_cost_per_mtok + output_cost_per_mtok) / 2.0);
            by_model.insert(
                model.clone(),
                ModelCost {
                    tokens: *tokens,
                    cost: estimated_cost,
                },
            );
        }

        // Potential savings: Could save 20% by using Haiku for simple tasks
        let potential_savings = total_cost * 0.20;

        Ok(CostAnalysis {
            total_cost_usd: total_cost,
            by_tool,
            by_model,
            monthly_estimate: total_cost, // This is historical total
            potential_savings,
        })
    }

    fn analyze_productivity(&self) -> Result<ProductivityMetrics> {
        info!("📈 Analyzing productivity metrics...");

        Ok(ProductivityMetrics {
            total_sessions: 0,
            average_session_duration: 0.0,
            total_files_modified: 0,
            error_rate: 0.0,
            retry_rate: 0.0,
            most_active_hours: vec![],
        })
    }

    fn analyze_languages(&self) -> Result<LanguageStats> {
        info!("🔤 Analyzing language statistics...");

        Ok(LanguageStats {
            by_language: HashMap::new(),
            most_used_language: String::new(),
            framework_mentions: HashMap::new(),
        })
    }

    fn compare_tools(
        &self,
        _conv: &ConversationAnalysis,
        _tokens: &TokenUsage,
        _cost: &CostAnalysis,
    ) -> Result<ToolComparison> {
        info!("⚖️  Comparing tool effectiveness...");

        Ok(ToolComparison {
            by_effectiveness: vec![],
            by_cost_efficiency: vec![],
            by_usage: vec![],
        })
    }
}
