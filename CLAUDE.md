# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**ai-log-analyzer** - A Rust CLI tool that analyzes logs from AI coding assistants (Claude Code, Cursor, Cline, Kiro, Copilot, etc.) to generate insights on usage patterns, costs, storage, and productivity.

## Build and Run Commands

```bash
# Build release binary
cargo build --release

# Run tests (3 tests in sanitizer.rs)
cargo test

# Run with debug logging
./target/release/ai-log-analyzer --verbose <subcommand>

# Install globally
cargo install --path .
```

**Binary location:** `target/release/ai-log-analyzer` (~8MB)

## CLI Subcommands

| Command | Description | Key Flags |
|---------|-------------|-----------|
| `discover` | Scan system for AI tool logs | `--base-dir`, `--hidden` |
| `analyze` | Generate analysis reports | `--format` (text/json/html/markdown), `--output`, `--tool`, `--days`, `--skip-compression` |
| `backup` | Create compressed backup archive | `--output`, `--tool`, `--compression` (0-9), `--timestamp` |
| `prepare` | Prepare sanitized finetuning dataset | `--output` |
| `insights` | Comprehensive insights + HTML dashboard | `--output`, `--html`, `--html-output`, `--infographics` |
| `extract-datasets` | Extract 37 dataset types from backup | `--backup` (required), `--output` |
| `analyze-datasets` | Generate reports from datasets | `--datasets-dir`, `--output` |
| `deep-analysis` | Temporal patterns, learning curves | `--datasets-dir`, `--output` |
| `ultra-deep` | Anti-patterns, productivity killers | `--datasets-dir`, `--output` |
| `stats` | Real-time statistics monitor | `--interval` (seconds) |
| `compare` | Compare multiple AI tools | `--format` (table) |

## Architecture

### Core Modules

- `main.rs` - CLI entry point using clap for argument parsing
- `models.rs` - Core data structures: `AiTool` enum (18 tools), `LogLocation`, `LogType`, analysis result types
- `discovery.rs` - Scans home directory for AI tool logs in known locations
- `analysis.rs` - Metrics calculation, cost estimation, recommendations engine

### Parser System (`src/parsers/`)

All parsers implement the `LogParser` trait:

```rust
pub trait LogParser: Send + Sync {
    fn can_parse(&self, path: &PathBuf) -> bool;
    fn parse(&self, path: &PathBuf) -> Result<ParsedLog>;
}
```

**ParsedLog structure:**
- `tool: AiTool` - Which AI tool generated the log
- `entries: Vec<LogEntry>` - Parsed log entries with timestamp, level, message, category
- `metadata: LogMetadata` - File size, entry count, date range

**EntryCategory enum:** `UserPrompt`, `AssistantResponse`, `SystemEvent`, `Error`, `ToolUse`, `FileOperation`, `Unknown`

**LogLevel enum:** `Debug`, `Info`, `Warn`, `Error`, `Unknown`

#### ClaudeParser (`claude.rs`)
- **Path detection:** Contains `.claude`
- **Files parsed:** `history.jsonl`
- **Format:** JSONL with RFC3339 timestamps
- **Field detection:** `userMessage`/`prompt` → UserPrompt, `assistantMessage`/`response` → AssistantResponse, `tool_use`/`toolUse` → ToolUse
- **Additional:** `analyze_claude_logs()` function counts sessions, prompts, responses, estimates tokens (1 token ≈ 4 chars), counts debug/file-history files

#### ClineParser (`cline.rs`)
- **Path detection:** Contains `cline`
- **Files parsed:** `cline.log`, `main.log`, `history.jsonl`
- **Formats:** JSONL and plain text
- **JSON fields:** `timestamp`/`ts`/`time`, `type`/`role` for categorization, `message`/`content`/`text`
- **Text parsing:** Detects `user:`/`assistant:`/`tool:` prefixes, log levels from keywords

#### CursorParser (`cursor.rs`)
- **Path detection:** Contains `cursor`
- **Files parsed:** `main.log`, `renderer.log`, `extensionHost.log`, `chat.log`
- **Formats:** JSON and VSCode-style text logs `[timestamp] [level] message`
- **JSON fields:** `category: "chat"`, `type: "user_input"/"ai_response"/"code_edit"/"file_operation"`
- **Note:** rusqlite dependency available for Cursor SQLite database parsing

#### GenericParser (`generic.rs`)
- **Path detection:** Always returns true (fallback parser)
- **Limits:** Max 10,000 lines to prevent memory issues
- **Keyword detection:** Searches for `error`/`warn`/`debug`, `user`/`prompt`, `assistant`/`response`, `tool`/`function`, `file`/`edit`
- **Timestamp:** Basic ISO 8601 detection (contains 'T' and ':')
- **Tool identification:** Uses `AiTool::from_path()` or falls back to filename

#### Adding a New Parser

1. Create `src/parsers/toolname.rs`:
```rust
use super::{LogParser, ParsedLog, LogEntry, LogLevel, EntryCategory, LogMetadata};
use crate::models::AiTool;

pub struct ToolnameParser;

impl LogParser for ToolnameParser {
    fn can_parse(&self, path: &PathBuf) -> bool {
        path.to_string_lossy().contains("toolname")
    }

    fn parse(&self, path: &PathBuf) -> Result<ParsedLog> {
        // Parse log files, extract entries
        Ok(ParsedLog {
            tool: AiTool::Other("Toolname".to_string()),
            entries: vec![],
            metadata: LogMetadata { ... },
        })
    }
}
```

2. Add to `src/parsers/mod.rs`:
```rust
pub mod toolname;
pub use toolname::ToolnameParser;
```

3. Add variant to `AiTool` enum in `src/models.rs` if needed

### Discovery System (`src/discovery.rs`)

`LogDiscovery` scans the home directory for AI tool logs using 80+ predefined search patterns.

```rust
let discovery = LogDiscovery::new(base_dir, include_hidden);
let findings: DiscoveryFindings = discovery.scan()?;
```

**Search pattern categories:**
- Claude Code: `.claude`, `Library/Application Support/Claude`, `AppData/Roaming/Claude`
- Cursor: `.cursor`, `.cursor/extensions`, `Library/Application Support/Cursor`
- VSCode extensions: `.config/Code/User/globalStorage/saoudrizwan.claude-dev` (Cline), `github.copilot-chat`, `continue.continue`, `sourcegraph.cody-ai`
- Cursor extensions: `.config/Cursor/User/globalStorage/*`
- Kiro: `.config/Kiro`, `.kiro/extensions`
- Flatpak: `.var/app/com.visualstudio.code/config/Code/...`
- JetBrains: `.local/share/JetBrains`, `.AndroidStudio`, `.IntelliJIdea`, etc.

**Subdirectory classification** (maps to `LogType`):
- `debug`, `logs` → Debug
- `file-history`, `checkpoints` → FileHistory
- `history.jsonl` → History
- `sessions`, `session-env`, `tasks` → Session
- `telemetry` → Telemetry
- `state.vscdb` → Session (Cursor/VSCode state databases)

### Analyzer Hierarchy (3 levels)

**Why 3 analyzers?** Different depths of analysis:

| Analyzer | Module | Purpose | Used By |
|----------|--------|---------|---------|
| `Analyzer` | `analysis.rs` | Basic metrics, storage, cost | `analyze` command |
| `ConversationAnalyzer` | `analyzer.rs` | Cline/Kilo task parsing | Internal |
| `ComprehensiveAnalyzer` | `comprehensive_analyzer.rs` | Full insights, work hours, viral stats | `insights` command |

### Analysis Engine (`src/analysis.rs`)

`Analyzer` uses builder pattern for configuration:

```rust
let analyzer = Analyzer::new()
    .with_tool_filter(Some("claude".to_string()))  // Filter specific tool
    .with_time_range(Some(30))                      // Last 30 days
    .with_compression_check(true);                  // Check compressibility

let results: AnalysisResults = analyzer.analyze().await?;
```

**Cost estimation** (Claude Sonnet pricing):
- Input: $3/M tokens (60% of total)
- Output: $15/M tokens (40% of total)
- Optimization potential: 30% estimated savings

**Recommendations generated for:**
- Storage > 500MB → Suggests backup
- Compressible logs (Debug, FileHistory) → 50% compression estimate
- Old files (>30 days) → Cleanup suggestions

### Analysis Pipeline

1. **Discovery** - `LogDiscovery::scan()` finds logs across 80+ paths
2. **Parsing** - Tool-specific parsers extract sessions, prompts, tokens
3. **Analysis** - `Analyzer` computes metrics, costs, recommendations
4. **Reporting** - `ReportGenerator` outputs markdown/json/html

### Deep Analysis Modules

#### ComprehensiveAnalyzer (`comprehensive_analyzer.rs`)

Primary analysis engine producing `ComprehensiveAnalysis`:

```rust
let analyzer = ComprehensiveAnalyzer::new(home_dir);
let insights: ComprehensiveAnalysis = analyzer.analyze()?;
```

**Output structure:**
- `conversations` - Total conversations/messages, by-tool breakdown, largest conversation
- `token_usage` - Input/output tokens, by-tool and by-model breakdown
- `code_attribution` - AI-generated lines count, percentage, by-composer
- `cost_analysis` - Total cost, monthly estimate, potential savings
- `productivity_metrics` - Efficiency scores
- `work_hours` - Session tracking from `WorkHoursAnalyzer`
- `viral_insights` - Shareable statistics from `ViralAnalyzer`
- `advanced` - Deep metrics from `AdvancedAnalyzer`

#### DeepAnalyzer (`deep_insights.rs`)

Produces `DeepInsights` for temporal and behavioral patterns:

**TemporalPatterns:**
- `burnout_indicators` - Periods with burnout signs (mild/moderate/severe)
- `peak_performance_windows` - Best hours with efficiency scores
- `error_clusters` - Days with high error counts and causes
- `context_switch_costs` - Project switch recovery times

**ConversationIntelligence:**
- `successful_patterns` - Patterns that work (sequence, success rate, avg tokens)
- `failed_patterns` - Patterns that fail
- `common_derailments` - What causes conversations to go off track
- `retry_analysis` - Retry frequency and causes

**TaskComplexityAnalysis:**
- `complexity_vs_outcome` - Success rates by task complexity

#### UltraDeepAnalyzer (`ultra_deep.rs`)

Most advanced analysis for anti-patterns and productivity killers:

- `conversation_autopsy` - Death spirals, zombie conversations, abandonment rate
- `anti_patterns` - Named anti-patterns with hours wasted and avoidance tactics
- `productivity_killers` - Severity-ranked issues with prevention strategies
- `success_blueprints` - Step-by-step patterns that consistently work
- `tool_sequence_mastery` - Winning vs losing tool call sequences
- `burnout_detection` - Burnout sessions, optimal session length
- `recovery_strategies` - How to escape stuck scenarios

#### WorkHoursAnalyzer (`work_hours_analyzer.rs`)

Session and time tracking:
- Hours by hour-of-day (24-hour distribution)
- Hours by weekday
- Hours by tool
- Work-life balance score
- Busiest day/hour detection

### Dataset Extraction (`src/extractors/`)

Extractors take `ComprehensiveAnalysis` + `Vec<Conversation>` and produce specialized JSONL datasets:

```rust
// Example extractor pattern
impl BugPatternsExtractor {
    pub fn extract(insights: &ComprehensiveAnalysis, conversations: &[Conversation]) -> Result<BugPatternsDataset>
}
```

**Available extractors:**
- `BugPatternsExtractor` - Groups errors by pattern, tracks time-to-fix, learning status
- `PromptEngineeringExtractor` - Analyzes prompt effectiveness and success rates
- `AgenticToolUseExtractor` - Sequences of tool calls and their outcomes
- `CodeDebuggingExtractor` - Error resolution patterns and fix strategies
- `PersonalStyleExtractor` - User coding patterns and preferences

### Data Pipeline Modules

#### DatasetPreparer (`prepare.rs`)

5-step pipeline for creating finetuning datasets:

```rust
let preparer = DatasetPreparer::new(output_dir);
let results = preparer.prepare_dataset().await?;
```

1. Create backup → 2. Extract to temp → 3. Sanitize → 4. Convert to JSONL → 5. Create ZIP

**Output format** (`TrainingExample`):
```json
{"prompt": "...", "completion": "...", "metadata": {"tool": "Claude Code", "session_id": "...", "tokens_estimate": 450}}
```

#### BackupManager (`backup.rs`)

Creates compressed tar.gz archives:
- Compression levels 0-9 (default 6)
- Optional tool filtering
- Timestamp in filename
- Uses flate2 for gzip compression

#### DatasetExtractor (`dataset_extractor.rs`)

Master orchestrator for extracting 37 datasets into 4 phases:
- `phase1_immediate/` - Quick wins
- `phase2_ml/` - ML training data
- `phase3_advanced/` - Deep analysis
- `huggingface/` - HuggingFace-ready formats

#### Extraction Utilities (`extraction_utils.rs`)

Core data structures for extraction pipeline:

```rust
struct Conversation { id, tool, timestamp, messages, file_path }
struct Message { role, content, timestamp, tool_calls, tokens }
struct ToolCall { tool, parameters, result, success }
struct ErrorInstance { error_type, message, file, line, language, fix }
```

Functions: `load_all_conversations()`, `load_cline_conversations()`, `extract_errors()`

### Analytics Modules

#### ViralAnalyzer (`viral_insights.rs`)

Generates shareable/fun statistics:

- **FunFacts**: tokens as books/Wikipedia pages, carbon footprint, cost in coffees
- **BehaviorPatterns**: frustration count ("wtf", "no"), "go on" count, politeness score, typo count
- **TimeAnalytics**: hourly/daily heatmaps, late night sessions, binge coding detection
- **Achievements**: Unlockable badges based on usage
- **Records**: Personal bests

#### ClaudeCodeParser (`claude_code_parser.rs`)

**Why two Claude parsers?**
- `parsers/claude.rs` → Generic `LogParser` trait for pipeline
- `claude_code_parser.rs` → Specialized stats extraction with project breakdown

```rust
struct ClaudeCodeStats {
    total_prompts, total_conversations, total_messages,
    user_messages, assistant_messages, projects: HashMap,
    estimated_tokens, frustration_prompts, go_on_count
}
```

Parses `~/.claude/projects/*/history.jsonl` with project-level granularity.

#### ConversationAnalyzer (`analyzer.rs`)

Analyzes Cline/Kilo/Roo-Cline tasks and Claude Code history:

```rust
let analyzer = ConversationAnalyzer::new(home_dir);
let stats: ConversationStats = analyzer.analyze()?;
```

**Paths scanned:**
- `.config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks/` (Cline)
- `.config/Code/User/globalStorage/kilocode.kilo-code/tasks/` (Kilo)
- `.config/Code/User/globalStorage/rooveterinaryinc.roo-cline/tasks/` (Roo-Cline)
- `.claude/history.jsonl` (Claude Code)

**Parses:** `api_conversation_history.json`, `task_metadata.json` for token counts.

#### AdvancedAnalyzer (`advanced_analytics.rs`)

Deep metrics for `ComprehensiveAnalysis.advanced` field.

#### ReportAnalyzer (`report_analyzer.rs`)

Analyzes extracted datasets and generates markdown/JSON reports.

### Supporting Modules

- `sanitizer.rs` - 27 regex patterns for PII/credential removal
- `html_report.rs` - D3.js interactive dashboards
- `report.rs` - Formatted output (markdown/json/html tables)
- `metrics.rs` - Real-time statistics (incomplete implementation)
- `infographics.rs` - PNG generation (disabled - compilation errors)

## Key Data Structures

```rust
// Tool identification (18 variants)
enum AiTool { ClaudeCode, Cline, Cursor, Kiro, RooCode, Kilo, VSCode,
              Copilot, Tabnine, CodeWhisperer, Windsurf, Continue,
              Aider, Cody, CodeGPT, BitoAI, AmazonQ, Supermaven, Other(String) }

// Log categorization
enum LogType { Debug, History, FileHistory, Session, Telemetry,
               ShellSnapshot, Todo, Cache, Plugin, Unknown }

// Discovery
struct LogLocation { tool, path, log_type, size_bytes, file_count, oldest_entry, newest_entry }
struct DiscoveryFindings { locations: Vec<LogLocation>, total_size_bytes, total_files, tools_found }

// Analysis (basic)
struct AnalysisResults { tools: HashMap<String, ToolAnalysis>, global_metrics, recommendations, cost_estimate }
struct GlobalMetrics { total_storage, compressible_bytes, total_sessions, total_prompts, estimated_tokens, peak_usage_hour, most_used_tool }

// Comprehensive analysis
struct ComprehensiveAnalysis { conversations, token_usage, code_attribution, cost_analysis,
                               productivity_metrics, language_stats, tool_comparison,
                               claude_code, viral_insights, work_hours, advanced }

// Deep insights
struct DeepInsights { temporal_patterns, conversation_intelligence, learning_curves,
                      productivity_rhythms, tool_effectiveness, task_complexity_analysis, hidden_patterns }

// Extraction
struct Conversation { id, tool, timestamp, messages: Vec<Message>, file_path }
struct Message { role, content, timestamp, tool_calls: Vec<ToolCall>, tokens }
struct ToolCall { tool, parameters: Value, result, success }
```

## Dependencies

- **CLI**: clap 4.5 with derive feature
- **Async**: tokio (full), rayon for parallelism
- **Parsing**: serde_json, regex, chrono
- **Output**: comfy-table, colored, indicatif (progress bars)
- **Compression**: flate2, tar, zip
- **Database**: rusqlite (bundled) for Cursor DB parsing
- **Visualization**: plotters, image, imageproc

## Common Workflows

```bash
# Full analysis pipeline
./target/release/ai-log-analyzer discover
./target/release/ai-log-analyzer analyze --output report.md
./target/release/ai-log-analyzer insights --html

# Create finetuning dataset (sanitizes PII, API keys, passwords)
./target/release/ai-log-analyzer backup
./target/release/ai-log-analyzer prepare --output ~/datasets

# Deep productivity analysis
./target/release/ai-log-analyzer extract-datasets --backup ~/ai-logs-backup.zip
./target/release/ai-log-analyzer analyze-datasets
./target/release/ai-log-analyzer ultra-deep
```

## Log Locations Scanned (80+ paths)

**Native installs:**
- `~/.claude/` - Claude Code (history.jsonl, debug/, file-history/)
- `~/.cursor/`, `~/.cursor/extensions/` - Cursor AI
- `~/.config/Kiro/`, `~/.kiro/` - Kiro
- `~/.continue/` - Continue.dev
- `~/.aider/` - Aider
- `~/.windsurf/` - Windsurf
- `~/.cody/` - Sourcegraph Cody
- `~/.tabnine/` - Tabnine
- `~/.supermaven/` - Supermaven

**VSCode extensions (globalStorage):**
- `~/.config/Code/User/globalStorage/saoudrizwan.claude-dev/` - Cline
- `~/.config/Code/User/globalStorage/rooveterinaryinc.roo-cline/` - Roo-Cline
- `~/.config/Code/User/globalStorage/github.copilot-chat/` - Copilot
- `~/.config/Code/User/globalStorage/kilocode.kilo-code/` - Kilo
- `~/.config/Code/User/globalStorage/continue.continue/` - Continue
- `~/.config/Code/User/globalStorage/sourcegraph.cody-ai/` - Cody

**Flatpak (can be 40+ GB):**
- `~/.var/app/com.visualstudio.code/config/Code/User/globalStorage/...`
- `~/.var/app/com.cursor.Cursor/config/Cursor/User/globalStorage/...`

**macOS/Windows:**
- `~/Library/Application Support/Claude|Cursor|Kiro/...`
- `%APPDATA%/Claude|Cursor/...`

**JetBrains:** `~/.local/share/JetBrains/`, `~/.AndroidStudio/`, `~/.IntelliJIdea/`, etc.

## Sanitization Patterns (`src/sanitizer.rs`)

The `Sanitizer` class uses **27 regex patterns** across 5 categories:

**API Keys (13 patterns):**
- OpenAI/Anthropic: `sk-*`, `sk-ant-*`
- GitHub: `ghp_*`, `gho_*`, `github_pat_*`
- GitLab: `glpat-*`
- Slack: `xox[baprs]-*`
- AWS: `AKIA*`
- Google: `ya29.*`, `AIza*`, OAuth client IDs
- Generic: `Bearer *`, `token=*`

**Passwords (4 patterns):** `password=`, `passwd=`, `pwd=`, `pass=`

**PII (5 patterns):** emails, phone numbers, SSNs, credit cards, IP addresses

**Paths (3 patterns):** `/home/[user]`, `/Users/[user]`, `C:\Users\[user]`

**Other (2 patterns):** URLs with auth (`https://user:pass@`), environment variables

**Replacement tokens:** `[REDACTED_API_KEY]`, `[REDACTED_EMAIL]`, `[REDACTED_CC]`, `[REDACTED_SSN]`, `[REDACTED_PHONE]`, `[REDACTED_IP]`, `/home/[USER]`

## Quick Reference

```bash
# Most common commands
ai-log-analyzer discover                    # Find all AI logs
ai-log-analyzer analyze --format markdown   # Basic report
ai-log-analyzer insights --html             # Full HTML dashboard
ai-log-analyzer backup                      # Create backup archive
ai-log-analyzer prepare --output ~/data     # Sanitized finetuning dataset

# Deep analysis pipeline
ai-log-analyzer extract-datasets --backup ~/ai-logs-backup.tar.gz
ai-log-analyzer deep-analysis
ai-log-analyzer ultra-deep
```

## Tool Detection Logic (`AiTool::from_path`)

Path substring matching (case-insensitive):
- `.claude` → ClaudeCode
- `cline` → Cline
- `cursor` → Cursor
- `kiro` → Kiro
- `roo`/`roocode` → RooCode
- `kilo` → Kilo
- `.vscode` → VSCode
- `copilot` → Copilot
- `tabnine` → Tabnine
- `codewhisperer`/`code-whisperer` → CodeWhisperer
- `windsurf` → Windsurf
- `continue` → Continue
- `aider` → Aider
- `cody`/`sourcegraph` → Cody
- `codegpt` → CodeGPT
- `bito` → BitoAI
- `amazonq`/`amazon-q` → AmazonQ
- `supermaven` → Supermaven

## Known Limitations

1. Token estimation is approximate (1 token ≈ 4 chars)
2. `infographics.rs` is disabled due to compilation errors (use `--html` flag instead)
3. Real-time stats (`stats` command) has incomplete implementation
4. Cursor SQLite database parsing not yet implemented (rusqlite available but unused)
5. Parsers rely on known file locations; custom paths may not be detected
6. `AiTool::from_path()` uses substring matching - may misidentify paths containing tool names
