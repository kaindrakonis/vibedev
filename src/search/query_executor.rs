use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, Query, QueryParser, RegexQuery, TermQuery};
use tantivy::schema::*;
use tantivy::{Index, TantivyDocument};

use super::schema::*;

/// Search query parameters
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// Text query
    pub text: String,

    /// Filter by tool name
    pub tool: Option<String>,

    /// Filter by log type
    pub log_type: Option<String>,

    /// Filter by category
    pub category: Option<String>,

    /// Filter by log level
    pub level: Option<String>,

    /// Filter by project
    pub project: Option<String>,

    /// Start date (inclusive)
    pub from_date: Option<DateTime<Utc>>,

    /// End date (inclusive)
    pub to_date: Option<DateTime<Utc>>,

    /// Use regex matching
    pub regex: bool,

    /// Maximum number of results
    pub limit: usize,

    /// Offset for pagination
    pub offset: usize,

    /// Output format
    pub format: OutputFormat,

    /// Number of context lines before/after match
    pub context: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Table,
    Json,
    Markdown,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            text: String::new(),
            tool: None,
            log_type: None,
            category: None,
            level: None,
            project: None,
            from_date: None,
            to_date: None,
            regex: false,
            limit: 100,
            offset: 0,
            format: OutputFormat::Table,
            context: 0,
        }
    }
}

/// Single search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub doc_id: u64,
    pub tool: String,
    pub log_type: String,
    pub timestamp: Option<String>,
    pub level: String,
    pub category: String,
    pub message: String,
    pub file_path: String,
    pub project: String,
    pub score: f32,
}

/// Search results wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub query: String,
    pub total_found: usize,
    pub showing: usize,
    pub offset: usize,
    pub results: Vec<SearchResult>,
    pub search_time_ms: u64,
}

/// Executes search queries against the Tantivy index
pub struct QueryExecutor {
    index: Index,
    schema: Schema,
}

impl QueryExecutor {
    /// Creates a new QueryExecutor
    pub fn new(index_path: &Path) -> Result<Self> {
        let schema = build_schema();
        let index = Index::open_in_dir(index_path).context("Failed to open index")?;

        Ok(Self { index, schema })
    }

    /// Executes a search query
    pub fn execute(&self, query: &SearchQuery) -> Result<SearchResults> {
        let start = std::time::Instant::now();

        let reader = self.index.reader()?;

        let searcher = reader.searcher();

        // Build the query
        let tantivy_query = self.build_query(query)?;

        // Execute search
        let top_docs = searcher.search(
            &*tantivy_query,
            &TopDocs::with_limit(query.limit + query.offset),
        )?;

        // Skip offset
        let results_to_show: Vec<_> = top_docs.into_iter().skip(query.offset).collect();

        // Convert to SearchResult
        let mut results = Vec::new();
        for (score, doc_address) in results_to_show {
            let retrieved_doc = searcher.doc(doc_address)?;
            let result = self.doc_to_search_result(&retrieved_doc, score)?;
            results.push(result);
        }

        let search_time_ms = start.elapsed().as_millis() as u64;

        Ok(SearchResults {
            query: query.text.clone(),
            total_found: results.len(),
            showing: results.len(),
            offset: query.offset,
            results,
            search_time_ms,
        })
    }

    /// Builds a Tantivy query from SearchQuery
    fn build_query(&self, query: &SearchQuery) -> Result<Box<dyn Query>> {
        let message_field = self.schema.get_field(FIELD_MESSAGE)?;

        // Main text query
        let mut subqueries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        if !query.text.is_empty() {
            if query.regex {
                // Regex query
                let regex_query = RegexQuery::from_pattern(&query.text, message_field)?;
                subqueries.push((Occur::Must, Box::new(regex_query)));
            } else {
                // Standard full-text query
                let query_parser = QueryParser::for_index(&self.index, vec![message_field]);
                let parsed = query_parser.parse_query(&query.text)?;
                subqueries.push((Occur::Must, parsed));
            }
        }

        // Add filters
        if let Some(ref tool) = query.tool {
            let tool_field = self.schema.get_field(FIELD_TOOL)?;
            let term = Term::from_field_text(tool_field, tool);
            subqueries.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
        }

        if let Some(ref log_type) = query.log_type {
            let log_type_field = self.schema.get_field(FIELD_LOG_TYPE)?;
            let term = Term::from_field_text(log_type_field, log_type);
            subqueries.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
        }

        if let Some(ref category) = query.category {
            let category_field = self.schema.get_field(FIELD_CATEGORY)?;
            let term = Term::from_field_text(category_field, category);
            subqueries.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
        }

        if let Some(ref level) = query.level {
            let level_field = self.schema.get_field(FIELD_LEVEL)?;
            let term = Term::from_field_text(level_field, level);
            subqueries.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
        }

        if let Some(ref project) = query.project {
            let project_field = self.schema.get_field(FIELD_PROJECT)?;
            let term = Term::from_field_text(project_field, project);
            subqueries.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
        }

        // Date range filter (simplified - would need proper range query in production)
        // TODO: Implement proper date range query using RangeQuery

        if subqueries.is_empty() {
            // Match all query if no filters
            let all_query_parser = QueryParser::for_index(&self.index, vec![message_field]);
            Ok(all_query_parser.parse_query("*")?)
        } else if subqueries.len() == 1 {
            Ok(subqueries.into_iter().next().unwrap().1)
        } else {
            Ok(Box::new(BooleanQuery::new(subqueries)))
        }
    }

    /// Converts a Tantivy document to SearchResult
    fn doc_to_search_result(&self, doc: &TantivyDocument, score: f32) -> Result<SearchResult> {
        let schema = &self.schema;

        let doc_id = doc
            .get_first(schema.get_field(FIELD_DOC_ID)?)
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let tool = doc
            .get_first(schema.get_field(FIELD_TOOL)?)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let log_type = doc
            .get_first(schema.get_field(FIELD_LOG_TYPE)?)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let timestamp = doc
            .get_first(schema.get_field(FIELD_TIMESTAMP)?)
            .and_then(|v| v.as_datetime())
            .map(|dt| {
                // Convert tantivy DateTime to timestamp and then to string
                let micros = dt.into_timestamp_micros();
                let secs = micros / 1_000_000;
                let nanos = ((micros % 1_000_000) * 1000) as u32;
                chrono::DateTime::from_timestamp(secs, nanos)
                    .unwrap_or_default()
                    .to_rfc3339()
            });

        let level = doc
            .get_first(schema.get_field(FIELD_LEVEL)?)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let category = doc
            .get_first(schema.get_field(FIELD_CATEGORY)?)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let message = doc
            .get_first(schema.get_field(FIELD_MESSAGE)?)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let file_path = doc
            .get_first(schema.get_field(FIELD_FILE_PATH)?)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let project = doc
            .get_first(schema.get_field(FIELD_PROJECT)?)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(SearchResult {
            doc_id,
            tool,
            log_type,
            timestamp,
            level,
            category,
            message,
            file_path,
            project,
            score,
        })
    }

    /// Formats search results for display
    pub fn format_results(&self, results: &SearchResults, format: OutputFormat) -> String {
        match format {
            OutputFormat::Json => self.format_json(results),
            OutputFormat::Markdown => self.format_markdown(results),
            OutputFormat::Table => self.format_table(results),
        }
    }

    fn format_json(&self, results: &SearchResults) -> String {
        serde_json::to_string_pretty(results).unwrap_or_default()
    }

    fn format_markdown(&self, results: &SearchResults) -> String {
        let mut output = String::new();
        output.push_str(&format!("# Search Results\n\n"));
        output.push_str(&format!("Query: `{}`\n", results.query));
        output.push_str(&format!(
            "Found {} results in {}ms\n\n",
            results.total_found, results.search_time_ms
        ));

        for (i, result) in results.results.iter().enumerate() {
            output.push_str(&format!("## Result {}\n\n", i + 1));
            output.push_str(&format!("- **Tool**: {}\n", result.tool));
            output.push_str(&format!("- **Category**: {}\n", result.category));
            output.push_str(&format!("- **Level**: {}\n", result.level));
            if let Some(ref ts) = result.timestamp {
                output.push_str(&format!("- **Time**: {}\n", ts));
            }
            output.push_str(&format!("- **Project**: {}\n", result.project));
            output.push_str(&format!("\n```\n{}\n```\n\n", result.message));
        }

        output
    }

    fn format_table(&self, results: &SearchResults) -> String {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("Tool").fg(Color::Cyan),
                Cell::new("Date").fg(Color::Cyan),
                Cell::new("Category").fg(Color::Cyan),
                Cell::new("Message").fg(Color::Cyan),
            ]);

        for result in &results.results {
            let date = result
                .timestamp
                .as_ref()
                .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "N/A".to_string());

            let message = if result.message.len() > 60 {
                format!("{}...", &result.message[..57])
            } else {
                result.message.clone()
            };

            table.add_row(vec![
                Cell::new(&result.tool),
                Cell::new(date),
                Cell::new(&result.category),
                Cell::new(message),
            ]);
        }

        table.to_string()
    }
}

/// Parses a relative date string like "7d" or "1m" into DateTime
pub fn parse_relative_date(s: &str) -> Result<DateTime<Utc>> {
    let now = Utc::now();

    if s.ends_with('d') {
        let days: i64 = s[..s.len() - 1].parse()?;
        Ok(now - chrono::Duration::days(days))
    } else if s.ends_with('m') {
        let months: i64 = s[..s.len() - 1].parse()?;
        Ok(now - chrono::Duration::days(months * 30))
    } else if s.ends_with('y') {
        let years: i64 = s[..s.len() - 1].parse()?;
        Ok(now - chrono::Duration::days(years * 365))
    } else {
        // Try parsing as date YYYY-MM-DD
        let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")?;
        Ok(DateTime::from_naive_utc_and_offset(
            date.and_hms_opt(0, 0, 0).unwrap(),
            Utc,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_relative_date() {
        // Basic test
        let result = parse_relative_date("7d");
        assert!(result.is_ok());

        let result = parse_relative_date("2m");
        assert!(result.is_ok());

        let result = parse_relative_date("2026-01-01");
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_query_default() {
        let query = SearchQuery::default();
        assert_eq!(query.limit, 100);
        assert_eq!(query.offset, 0);
        assert!(!query.regex);
    }
}
