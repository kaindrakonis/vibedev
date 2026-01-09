use anyhow::{Context, Result};
use clap::Args;
use colored::*;
use std::path::PathBuf;

use super::index_builder::IndexBuilder;
use super::metadata::IndexMetadata;
use super::query_executor::{parse_relative_date, OutputFormat, QueryExecutor, SearchQuery};

/// Search command arguments
#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Search query text
    #[arg(value_name = "QUERY")]
    pub query: String,

    /// Filter by AI tool (e.g., claude, cursor, cline)
    #[arg(long)]
    pub tool: Option<String>,

    /// Filter by log type (e.g., history, debug, session)
    #[arg(long)]
    pub log_type: Option<String>,

    /// Filter by category (e.g., prompt, response, error)
    #[arg(long)]
    pub category: Option<String>,

    /// Filter by log level (e.g., debug, info, warn, error)
    #[arg(long)]
    pub level: Option<String>,

    /// Filter by project name
    #[arg(long)]
    pub project: Option<String>,

    /// Start date (YYYY-MM-DD or relative: 7d, 1m, 1y)
    #[arg(long)]
    pub from: Option<String>,

    /// End date (YYYY-MM-DD or relative: 7d, 1m, 1y)
    #[arg(long)]
    pub to: Option<String>,

    /// Treat query as regex pattern
    #[arg(long)]
    pub regex: bool,

    /// Maximum number of results
    #[arg(long, default_value = "100")]
    pub limit: usize,

    /// Offset for pagination
    #[arg(long, default_value = "0")]
    pub offset: usize,

    /// Output format (table, json, markdown)
    #[arg(long, default_value = "table")]
    pub format: String,

    /// Number of context lines before/after match
    #[arg(long, default_value = "0")]
    pub context: usize,

    /// Force rebuild index
    #[arg(long)]
    pub rebuild: bool,

    /// Update index before searching
    #[arg(long)]
    pub update: bool,
}

/// Handles the search command
pub fn handle_search(args: SearchArgs) -> Result<()> {
    let cache_dir = dirs::cache_dir()
        .context("Could not determine cache directory")?
        .join("vibedev")
        .join("search_index");

    let metadata_path = cache_dir.join("metadata.json");

    // Check if index exists
    let index_exists = cache_dir.join("meta.json").exists();

    // Build or update index
    if args.rebuild || !index_exists {
        println!("{}", "🔍 No search index found. Building index...".yellow());
        let mut builder = IndexBuilder::new(&cache_dir)?;
        builder.build_initial_index()?;
    } else if args.update {
        let mut builder = IndexBuilder::new(&cache_dir)?;
        builder.update_index()?;
    } else {
        // Show index info
        if let Ok(metadata) = IndexMetadata::load(&metadata_path) {
            let age = chrono::Utc::now()
                .signed_duration_since(metadata.last_indexed)
                .num_minutes();

            let age_str = if age < 60 {
                format!("{}m ago", age)
            } else if age < 1440 {
                format!("{}h ago", age / 60)
            } else {
                format!("{}d ago", age / 1440)
            };

            println!(
                "{} Using cached index ({} docs, last updated {})",
                "✓".green(),
                metadata.total_docs.to_string().cyan(),
                age_str.yellow()
            );
        }
    }

    // Build search query
    let from_date = args
        .from
        .as_ref()
        .map(|s| parse_relative_date(s))
        .transpose()?;

    let to_date = args
        .to
        .as_ref()
        .map(|s| parse_relative_date(s))
        .transpose()?;

    let format = match args.format.to_lowercase().as_str() {
        "json" => OutputFormat::Json,
        "markdown" | "md" => OutputFormat::Markdown,
        _ => OutputFormat::Table,
    };

    let search_query = SearchQuery {
        text: args.query.clone(),
        tool: args.tool,
        log_type: args.log_type,
        category: args.category,
        level: args.level,
        project: args.project,
        from_date,
        to_date,
        regex: args.regex,
        limit: args.limit,
        offset: args.offset,
        format,
        context: args.context,
    };

    // Execute search
    println!("\n{} Searching...", "🔍".cyan());

    let executor = QueryExecutor::new(&cache_dir)?;
    let results = executor.execute(&search_query)?;

    println!(
        "   Found {} results in {}ms\n",
        results.total_found.to_string().green(),
        results.search_time_ms
    );

    // Format and display results
    let formatted = executor.format_results(&results, format);
    println!("{}", formatted);

    // Show pagination info
    if results.total_found > results.showing {
        let remaining = results.total_found - results.showing;
        println!(
            "\n{} Showing {}/{} results. Use --limit and --offset for more.",
            "ℹ".cyan(),
            results.showing,
            results.total_found
        );
        if remaining > 0 {
            let next_offset = args.offset + args.limit;
            println!(
                "   Next page: --offset {} --limit {}",
                next_offset, args.limit
            );
        }
    }

    Ok(())
}
