use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AiTool {
    ClaudeCode,
    Cline,
    Cursor,
    Kiro,
    RooCode,
    Kilo,
    VSCode,
    Copilot,
    Tabnine,
    CodeWhisperer,
    Windsurf,
    Continue,
    Aider,
    Cody,
    CodeGPT,
    BitoAI,
    AmazonQ,
    Supermaven,
    Other(String),
}

impl AiTool {
    pub fn from_path(path: &Path) -> Option<Self> {
        let path_str = path.to_string_lossy().to_lowercase();

        if path_str.contains(".claude") {
            Some(AiTool::ClaudeCode)
        } else if path_str.contains("cline") {
            Some(AiTool::Cline)
        } else if path_str.contains("cursor") {
            Some(AiTool::Cursor)
        } else if path_str.contains("kiro") {
            Some(AiTool::Kiro)
        } else if path_str.contains("roo") || path_str.contains("roocode") {
            Some(AiTool::RooCode)
        } else if path_str.contains("kilo") {
            Some(AiTool::Kilo)
        } else if path_str.contains(".vscode") {
            Some(AiTool::VSCode)
        } else if path_str.contains("copilot") {
            Some(AiTool::Copilot)
        } else if path_str.contains("tabnine") {
            Some(AiTool::Tabnine)
        } else if path_str.contains("codewhisperer") || path_str.contains("code-whisperer") {
            Some(AiTool::CodeWhisperer)
        } else if path_str.contains("windsurf") {
            Some(AiTool::Windsurf)
        } else if path_str.contains("continue") {
            Some(AiTool::Continue)
        } else if path_str.contains("aider") {
            Some(AiTool::Aider)
        } else if path_str.contains("cody") || path_str.contains("sourcegraph") {
            Some(AiTool::Cody)
        } else if path_str.contains("codegpt") {
            Some(AiTool::CodeGPT)
        } else if path_str.contains("bito") {
            Some(AiTool::BitoAI)
        } else if path_str.contains("amazonq") || path_str.contains("amazon-q") {
            Some(AiTool::AmazonQ)
        } else if path_str.contains("supermaven") {
            Some(AiTool::Supermaven)
        } else {
            None
        }
    }

    pub fn name(&self) -> &str {
        match self {
            AiTool::ClaudeCode => "Claude Code",
            AiTool::Cline => "Cline",
            AiTool::Cursor => "Cursor",
            AiTool::Kiro => "Kiro",
            AiTool::RooCode => "Roo Code",
            AiTool::Kilo => "Kilo",
            AiTool::VSCode => "VSCode",
            AiTool::Copilot => "GitHub Copilot",
            AiTool::Tabnine => "Tabnine",
            AiTool::CodeWhisperer => "AWS CodeWhisperer",
            AiTool::Windsurf => "Windsurf",
            AiTool::Continue => "Continue.dev",
            AiTool::Aider => "Aider",
            AiTool::Cody => "Sourcegraph Cody",
            AiTool::CodeGPT => "CodeGPT",
            AiTool::BitoAI => "Bito AI",
            AiTool::AmazonQ => "Amazon Q",
            AiTool::Supermaven => "Supermaven",
            AiTool::Other(name) => name,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLocation {
    pub tool: AiTool,
    pub path: PathBuf,
    pub log_type: LogType,
    pub size_bytes: u64,
    pub file_count: usize,
    pub oldest_entry: Option<DateTime<Utc>>,
    pub newest_entry: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogType {
    Debug,
    History,
    FileHistory,
    Session,
    Telemetry,
    ShellSnapshot,
    Todo,
    Cache,
    Plugin,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryFindings {
    pub locations: Vec<LogLocation>,
    pub total_size_bytes: u64,
    pub total_files: usize,
    pub tools_found: Vec<AiTool>,
}

impl DiscoveryFindings {
    pub fn print_summary(&self) {
        use colored::*;
        use comfy_table::presets::UTF8_FULL;
        use comfy_table::*;

        println!("{}", "📁 Discovered Log Locations:".bold());
        println!();

        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_header(vec!["Tool", "Type", "Path", "Size", "Files"]);

        for loc in &self.locations {
            table.add_row(vec![
                loc.tool.name(),
                format!("{:?}", loc.log_type).as_str(),
                loc.path.display().to_string().as_str(),
                &format_bytes(loc.size_bytes),
                &loc.file_count.to_string(),
            ]);
        }

        println!("{table}");

        println!();
        println!("{}", "📊 Summary:".bold());
        println!(
            "  Total Size:  {}",
            format_bytes(self.total_size_bytes).green()
        );
        println!("  Total Files: {}", self.total_files.to_string().cyan());
        println!(
            "  Tools Found: {}",
            self.tools_found.len().to_string().yellow()
        );

        println!();
        println!("{}", "🔧 Tools Detected:".bold());
        for tool in &self.tools_found {
            println!("  • {}", tool.name());
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResults {
    pub tools: HashMap<String, ToolAnalysis>,
    pub global_metrics: GlobalMetrics,
    pub recommendations: Vec<Recommendation>,
    pub cost_estimate: Option<CostEstimate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAnalysis {
    pub tool_name: String,
    pub total_size: u64,
    pub file_count: usize,
    pub session_count: usize,
    pub prompt_count: u64,
    pub avg_session_length: f64,
    pub date_range: (Option<DateTime<Utc>>, Option<DateTime<Utc>>),
    pub usage_patterns: UsagePatterns,
    pub issues: Vec<Issue>,
    pub storage_breakdown: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePatterns {
    pub hourly_distribution: HashMap<u8, u64>,
    pub daily_distribution: HashMap<String, u64>,
    pub top_commands: Vec<(String, u64)>,
    pub top_projects: Vec<(String, u64)>,
    pub avg_session_duration_mins: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub impact: String,
    pub fix: String,
    pub estimated_savings: Option<u64>, // bytes
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMetrics {
    pub total_storage: u64,
    pub compressible_bytes: u64,
    pub old_files_bytes: u64, // >30 days
    pub total_sessions: usize,
    pub total_prompts: u64,
    pub estimated_tokens: u64,
    pub peak_usage_hour: u8,
    pub most_used_tool: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub category: RecommendationCategory,
    pub priority: Priority,
    pub title: String,
    pub description: String,
    pub action: String,
    pub estimated_savings: Option<u64>, // bytes or cost in cents
    pub effort: Effort,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationCategory {
    Storage,
    Performance,
    Cost,
    UX,
    Configuration,
    Security,
}

impl std::fmt::Display for RecommendationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecommendationCategory::Storage => write!(f, "Storage"),
            RecommendationCategory::Performance => write!(f, "Performance"),
            RecommendationCategory::Cost => write!(f, "Cost"),
            RecommendationCategory::UX => write!(f, "User Experience"),
            RecommendationCategory::Configuration => write!(f, "Configuration"),
            RecommendationCategory::Security => write!(f, "Security"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Effort {
    Minutes,
    Hours,
    Days,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub monthly_cost_usd: f64,
    pub token_count: u64,
    pub breakdown_by_tool: HashMap<String, f64>,
    pub optimization_potential: f64,
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
