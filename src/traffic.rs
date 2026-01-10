//! Traffic parsing and logging for Claude API monitoring
//!
//! Parses Claude API request/response format and tracks usage metrics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Maximum number of traffic entries to keep in memory
const MAX_TRAFFIC_HISTORY: usize = 1000;

/// Claude API pricing (per million tokens)
const CLAUDE_SONNET_INPUT_COST: f64 = 3.0;
const CLAUDE_SONNET_OUTPUT_COST: f64 = 15.0;
const CLAUDE_OPUS_INPUT_COST: f64 = 15.0;
const CLAUDE_OPUS_OUTPUT_COST: f64 = 75.0;
const CLAUDE_HAIKU_INPUT_COST: f64 = 0.25;
const CLAUDE_HAIKU_OUTPUT_COST: f64 = 1.25;

/// A single API traffic entry (request + response pair)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficEntry {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub request: ApiRequest,
    pub response: Option<ApiResponse>,
    pub latency_ms: Option<u64>,
    pub status: TrafficStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrafficStatus {
    Pending,
    Success,
    Error(String),
    Streaming,
}

/// Claude API request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    pub model: String,
    pub max_tokens: Option<u64>,
    pub messages: Vec<Message>,
    pub system: Option<String>,
    pub stream: bool,
    pub tools: Option<Vec<serde_json::Value>>,
    /// Raw request body for debugging
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_body: Option<String>,
}

/// Claude API response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub id: Option<String>,
    pub model: Option<String>,
    pub content: Vec<ContentBlock>,
    pub usage: Option<Usage>,
    pub stop_reason: Option<String>,
    /// Raw response body for debugging
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_body: Option<String>,
}

/// Message in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: MessageContent,
}

/// Message content can be string or array of content blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

/// Content block in message or response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
}

/// Token usage from API response
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

/// Aggregated traffic statistics
#[derive(Debug, Clone, Default)]
pub struct TrafficStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub streaming_requests: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cache_write_tokens: u64,
    pub total_cost_usd: f64,
    pub avg_latency_ms: f64,
    pub models_used: std::collections::HashMap<String, u64>,
}

/// Get the default log file path
pub fn get_log_file_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claudev")
        .join("logs.txt")
}

/// Thread-safe traffic log
#[derive(Clone)]
pub struct TrafficLog {
    entries: Arc<Mutex<VecDeque<TrafficEntry>>>,
    next_id: Arc<Mutex<u64>>,
    stats: Arc<Mutex<TrafficStats>>,
    log_file: Arc<Mutex<Option<File>>>,
    log_path: PathBuf,
}

impl TrafficLog {
    pub fn new() -> Self {
        let log_path = get_log_file_path();
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_TRAFFIC_HISTORY))),
            next_id: Arc::new(Mutex::new(1)),
            stats: Arc::new(Mutex::new(TrafficStats::default())),
            log_file: Arc::new(Mutex::new(None)),
            log_path,
        }
    }

    /// Enable file logging
    pub fn enable_file_logging(&self) -> std::io::Result<()> {
        // Create directory if needed
        if let Some(parent) = self.log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        let mut log_file = self.log_file.lock().unwrap();
        *log_file = Some(file);

        tracing::info!("Logging traffic to {:?}", self.log_path);
        Ok(())
    }

    /// Write a line to the log file
    fn write_log(&self, line: &str) {
        if let Ok(mut guard) = self.log_file.lock() {
            if let Some(file) = guard.as_mut() {
                let _ = writeln!(file, "{}", line);
                let _ = file.flush();
            }
        }
    }

    /// Get the log file path
    pub fn get_log_path(&self) -> &PathBuf {
        &self.log_path
    }

    /// Start tracking a new request, returns entry ID
    pub fn start_request(&self, request: ApiRequest) -> u64 {
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;

        let timestamp = Utc::now();
        let entry = TrafficEntry {
            id,
            timestamp,
            request: request.clone(),
            response: None,
            latency_ms: None,
            status: TrafficStatus::Pending,
        };

        // Log to file
        self.write_log(&format!(
            "[{}] REQUEST #{} model={} stream={} messages={}",
            timestamp.format("%Y-%m-%d %H:%M:%S"),
            id,
            request.model,
            request.stream,
            request.messages.len()
        ));

        let mut entries = self.entries.lock().unwrap();
        if entries.len() >= MAX_TRAFFIC_HISTORY {
            entries.pop_front();
        }
        entries.push_back(entry);

        let mut stats = self.stats.lock().unwrap();
        stats.total_requests += 1;

        id
    }

    /// Complete a request with response
    pub fn complete_request(&self, id: u64, response: ApiResponse, latency_ms: u64) {
        // First, get info for logging without holding the lock
        let log_info = {
            let entries = self.entries.lock().unwrap();
            entries.iter().find(|e| e.id == id).map(|entry| {
                let usage = response.usage.as_ref();
                let model = response.model.as_deref().unwrap_or(&entry.request.model);
                let cost = usage.map(|u| calculate_cost(model, u)).unwrap_or(0.0);
                (
                    model.to_string(),
                    usage.map(|u| u.input_tokens).unwrap_or(0),
                    usage.map(|u| u.output_tokens).unwrap_or(0),
                    cost,
                )
            })
        };

        // Log to file
        if let Some((model, input, output, cost)) = log_info {
            self.write_log(&format!(
                "[{}] RESPONSE #{} model={} in={} out={} latency={}ms cost=${:.6}",
                Utc::now().format("%Y-%m-%d %H:%M:%S"),
                id,
                model,
                input,
                output,
                latency_ms,
                cost
            ));
        }

        // Now update entries and stats
        let mut entries = self.entries.lock().unwrap();
        if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
            entry.response = Some(response.clone());
            entry.latency_ms = Some(latency_ms);
            entry.status = TrafficStatus::Success;
        }

        // Update stats
        let mut stats = self.stats.lock().unwrap();
        stats.successful_requests += 1;

        if let Some(usage) = &response.usage {
            stats.total_input_tokens += usage.input_tokens;
            stats.total_output_tokens += usage.output_tokens;
            stats.total_cache_read_tokens += usage.cache_read_input_tokens;
            stats.total_cache_write_tokens += usage.cache_creation_input_tokens;

            // Calculate cost based on model
            if let Some(entry) = entries.iter().find(|e| e.id == id) {
                let model = response.model.as_deref().unwrap_or(&entry.request.model);
                stats.total_cost_usd += calculate_cost(model, usage);
            }
        }

        // Update model usage
        if let Some(model) = response.model {
            if !model.is_empty() {
                *stats.models_used.entry(model).or_insert(0) += 1;
            }
        }

        // Update average latency
        let total_latency: u64 = entries
            .iter()
            .filter_map(|e| e.latency_ms)
            .sum();
        let count = entries.iter().filter(|e| e.latency_ms.is_some()).count();
        if count > 0 {
            stats.avg_latency_ms = total_latency as f64 / count as f64;
        }
    }

    /// Mark request as failed
    pub fn fail_request(&self, id: u64, error: String) {
        // Log to file
        self.write_log(&format!(
            "[{}] ERROR #{} {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            id,
            error
        ));

        let mut entries = self.entries.lock().unwrap();
        if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
            entry.status = TrafficStatus::Error(error);

            let mut stats = self.stats.lock().unwrap();
            stats.failed_requests += 1;
        }
    }

    /// Mark request as streaming
    pub fn mark_streaming(&self, id: u64) {
        let mut entries = self.entries.lock().unwrap();
        if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
            entry.status = TrafficStatus::Streaming;

            let mut stats = self.stats.lock().unwrap();
            stats.streaming_requests += 1;
        }
    }

    /// Get recent entries (newest first)
    pub fn get_recent(&self, limit: usize) -> Vec<TrafficEntry> {
        let entries = self.entries.lock().unwrap();
        entries.iter().rev().take(limit).cloned().collect()
    }

    /// Get current stats
    pub fn get_stats(&self) -> TrafficStats {
        self.stats.lock().unwrap().clone()
    }

    /// Export all entries as JSON
    pub fn export_jsonl(&self) -> String {
        let entries = self.entries.lock().unwrap();
        entries
            .iter()
            .map(|e| serde_json::to_string(e).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Default for TrafficLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate cost for a request based on model and usage
pub fn calculate_cost(model: &str, usage: &Usage) -> f64 {
    let (input_rate, output_rate) = if model.contains("opus") {
        (CLAUDE_OPUS_INPUT_COST, CLAUDE_OPUS_OUTPUT_COST)
    } else if model.contains("haiku") {
        (CLAUDE_HAIKU_INPUT_COST, CLAUDE_HAIKU_OUTPUT_COST)
    } else {
        // Default to Sonnet pricing
        (CLAUDE_SONNET_INPUT_COST, CLAUDE_SONNET_OUTPUT_COST)
    };

    let input_cost = (usage.input_tokens as f64 / 1_000_000.0) * input_rate;
    let output_cost = (usage.output_tokens as f64 / 1_000_000.0) * output_rate;

    input_cost + output_cost
}

/// Parse a request body into ApiRequest
pub fn parse_request(body: &str) -> Result<ApiRequest, serde_json::Error> {
    let mut request: ApiRequest = serde_json::from_str(body)?;
    request.raw_body = Some(body.to_string());
    Ok(request)
}

/// Parse a response body into ApiResponse
pub fn parse_response(body: &str) -> Result<ApiResponse, serde_json::Error> {
    let mut response: ApiResponse = serde_json::from_str(body)?;
    response.raw_body = Some(body.to_string());
    Ok(response)
}

/// Parse streaming response chunks
pub fn parse_stream_event(line: &str) -> Option<StreamEvent> {
    if !line.starts_with("data: ") {
        return None;
    }

    let data = &line[6..];
    if data == "[DONE]" {
        return Some(StreamEvent::Done);
    }

    serde_json::from_str(data).ok().map(StreamEvent::Data)
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Data(StreamData),
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamData {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub index: u64,
    pub delta: Option<StreamDelta>,
    pub usage: Option<Usage>,
    pub message: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDelta {
    #[serde(rename = "type")]
    pub delta_type: Option<String>,
    pub text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request() {
        let body = r#"{
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 8096,
            "messages": [{"role": "user", "content": "Hello"}],
            "stream": true
        }"#;

        let request = parse_request(body).unwrap();
        assert_eq!(request.model, "claude-sonnet-4-20250514");
        assert_eq!(request.max_tokens, Some(8096));
        assert!(request.stream);
    }

    #[test]
    fn test_calculate_cost() {
        let usage = Usage {
            input_tokens: 1000,
            output_tokens: 500,
            ..Default::default()
        };

        let cost = calculate_cost("claude-sonnet-4-20250514", &usage);
        // 1000/1M * 3 + 500/1M * 15 = 0.003 + 0.0075 = 0.0105
        assert!((cost - 0.0105).abs() < 0.0001);
    }

    #[test]
    fn test_traffic_log() {
        let log = TrafficLog::new();

        let request = ApiRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: Some(1000),
            messages: vec![],
            system: None,
            stream: false,
            tools: None,
            raw_body: None,
        };

        let id = log.start_request(request);
        assert_eq!(id, 1);

        let response = ApiResponse {
            id: Some("msg_123".to_string()),
            model: Some("claude-sonnet-4-20250514".to_string()),
            content: vec![],
            usage: Some(Usage {
                input_tokens: 100,
                output_tokens: 50,
                ..Default::default()
            }),
            stop_reason: Some("end_turn".to_string()),
            raw_body: None,
        };

        log.complete_request(id, response, 500);

        let stats = log.get_stats();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful_requests, 1);
        assert_eq!(stats.total_input_tokens, 100);
        assert_eq!(stats.total_output_tokens, 50);
    }
}
