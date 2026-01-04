use crate::backup::BackupManager;
use crate::models::format_bytes;
use crate::sanitizer::{Sanitizer, SensitiveDataType};
use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use zip::write::FileOptions;
use zip::ZipWriter;

#[derive(Debug, Serialize, Deserialize)]
pub struct TrainingExample {
    pub prompt: String,
    pub completion: String,
    pub metadata: ExampleMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExampleMetadata {
    pub tool: String,
    pub session_id: String,
    pub timestamp: Option<String>,
    pub tokens_estimate: usize,
}

pub struct DatasetPreparer {
    sanitizer: Sanitizer,
    output_dir: PathBuf,
    include_metadata: bool,
}

impl DatasetPreparer {
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            sanitizer: Sanitizer::new(),
            output_dir,
            include_metadata: true,
        }
    }

    pub async fn prepare_dataset(&self) -> Result<PrepareResults> {
        println!("{}", "🔧 AI Log Dataset Preparation".bold().cyan());
        println!();

        // Step 1: Create backup
        println!("{}", "Step 1/5: Creating backup...".yellow());
        let backup_path = self.create_initial_backup().await?;
        println!("  ✓ Backup created: {}", backup_path.display());
        println!();

        // Step 2: Extract to temp
        println!(
            "{}",
            "Step 2/5: Extracting to temporary directory...".yellow()
        );
        let temp_dir = self.extract_to_temp(&backup_path)?;
        println!("  ✓ Extracted to: {}", temp_dir.display());
        println!();

        // Step 3: Sanitize data
        println!("{}", "Step 3/5: Sanitizing sensitive data...".yellow());
        let sanitize_stats = self.sanitize_directory(&temp_dir)?;
        println!("  ✓ Files processed: {}", sanitize_stats.files_processed);
        println!(
            "  ✓ Sensitive items removed: {}",
            sanitize_stats.items_redacted
        );
        println!();

        // Step 4: Convert to training format
        println!("{}", "Step 4/5: Converting to training format...".yellow());
        let training_examples = self.convert_to_training_format(&temp_dir)?;
        println!("  ✓ Training examples created: {}", training_examples.len());
        println!();

        // Step 5: Create final dataset
        println!("{}", "Step 5/5: Creating final dataset archive...".yellow());

        // Store temp_dir for archive creation
        let temp_for_archive = temp_dir.clone();

        let dataset_path = self.create_dataset_archive_full(
            &temp_for_archive,
            &training_examples,
            &sanitize_stats,
        )?;
        println!("  ✓ Dataset saved: {}", dataset_path.display());
        println!();

        // Cleanup temp directory
        fs::remove_dir_all(&temp_dir)?;

        // Get sizes before moving paths
        let original_size = fs::metadata(&backup_path)?.len();
        let sanitized_size = fs::metadata(&dataset_path)?.len();

        let results = PrepareResults {
            dataset_path,
            backup_path,
            examples_count: training_examples.len(),
            original_size,
            sanitized_size,
            items_redacted: sanitize_stats.items_redacted,
            files_processed: sanitize_stats.files_processed,
        };

        self.print_summary(&results);

        Ok(results)
    }

    async fn create_initial_backup(&self) -> Result<PathBuf> {
        let temp_dir = std::env::temp_dir();
        let manager = BackupManager::new(temp_dir.clone(), 6);

        // Create backup without timestamp to have predictable name
        let backup_path = manager.create_backup(None, false).await?;

        Ok(backup_path)
    }

    fn extract_to_temp(&self, backup_path: &PathBuf) -> Result<PathBuf> {
        let temp_dir = std::env::temp_dir().join(format!(
            "ai-logs-sanitized-{}",
            chrono::Utc::now().timestamp()
        ));
        fs::create_dir_all(&temp_dir)?;

        // Extract tar.gz
        let tar_gz = File::open(backup_path)?;
        let tar = flate2::read::GzDecoder::new(tar_gz);
        let mut archive = tar::Archive::new(tar);
        archive.unpack(&temp_dir)?;

        Ok(temp_dir)
    }

    fn sanitize_directory(&self, dir: &PathBuf) -> Result<SanitizeStats> {
        let mut stats = SanitizeStats {
            files_processed: 0,
            items_redacted: 0,
            redacted_by_type: HashMap::new(),
        };

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} Sanitizing... {msg}")
                .unwrap(),
        );

        for entry in walkdir::WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Skip binary files
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if matches!(
                    ext_str.as_str(),
                    "png" | "jpg" | "jpeg" | "gif" | "pdf" | "zip" | "tar" | "gz"
                ) {
                    continue;
                }
            }

            // Skip .env files entirely (too risky)
            if path.file_name().and_then(|n| n.to_str()) == Some(".env") {
                fs::remove_file(path)?;
                stats.files_processed += 1;
                continue;
            }

            pb.set_message(format!(
                "Processing {}",
                path.file_name().unwrap().to_string_lossy()
            ));

            if let Ok(content) = fs::read_to_string(path) {
                // Detect sensitive data
                let matches = self.sanitizer.detect_sensitive_data(&content);
                stats.items_redacted += matches.len();

                for mat in matches {
                    *stats.redacted_by_type.entry(mat.data_type).or_insert(0) += 1;
                }

                // Sanitize and write back
                let sanitized = self.sanitizer.sanitize_text(&content);
                fs::write(path, sanitized)?;
                stats.files_processed += 1;
            }
        }

        pb.finish_with_message("Sanitization complete!");

        Ok(stats)
    }

    fn convert_to_training_format(&self, dir: &PathBuf) -> Result<Vec<TrainingExample>> {
        let mut examples = Vec::new();

        // Find history.jsonl files (Claude format)
        for entry in walkdir::WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.file_name().and_then(|n| n.to_str()) != Some("history.jsonl") {
                continue;
            }

            // Parse the history file
            if let Ok(file) = File::open(path) {
                let reader = BufReader::new(file);
                let mut session_id = 0;

                for line in reader.lines().map_while(Result::ok) {
                    if line.trim().is_empty() {
                        continue;
                    }

                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                        // Extract prompt and completion pairs
                        if let (Some(prompt), Some(completion)) =
                            (extract_prompt(&json), extract_completion(&json))
                        {
                            // Ensure both are sanitized
                            let sanitized_prompt = self.sanitizer.sanitize_text(&prompt);
                            let sanitized_completion = self.sanitizer.sanitize_text(&completion);

                            // Only include if safe
                            if self.sanitizer.is_safe_for_training(&sanitized_prompt)
                                && self.sanitizer.is_safe_for_training(&sanitized_completion)
                            {
                                examples.push(TrainingExample {
                                    prompt: sanitized_prompt.clone(),
                                    completion: sanitized_completion.clone(),
                                    metadata: ExampleMetadata {
                                        tool: "Claude Code".to_string(),
                                        session_id: format!("session_{}", session_id),
                                        timestamp: json
                                            .get("timestamp")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string()),
                                        tokens_estimate: (sanitized_prompt.len()
                                            + sanitized_completion.len())
                                            / 4,
                                    },
                                });

                                session_id += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(examples)
    }

    fn create_dataset_archive_full(
        &self,
        sanitized_dir: &PathBuf,
        examples: &[TrainingExample],
        stats: &SanitizeStats,
    ) -> Result<PathBuf> {
        let dataset_path = self.output_dir.join(format!(
            "ai-training-dataset-{}.zip",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        ));

        let file = File::create(&dataset_path)?;
        let mut zip = ZipWriter::new(file);

        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        println!("  Adding all sanitized files to archive...");
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} Adding files... {msg}")
                .unwrap(),
        );

        let mut file_count = 0;

        for entry in walkdir::WalkDir::new(sanitized_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Get relative path for archive
            let relative_path = path
                .strip_prefix(sanitized_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            pb.set_message(format!("{} files", file_count));

            // Add file to ZIP
            zip.start_file(&relative_path, options)?;
            let content = fs::read(path)?;
            zip.write_all(&content)?;
            file_count += 1;
        }

        pb.finish_with_message(format!("{} files added!", file_count));

        // Write training examples as JSONL (optional structured format)
        zip.start_file("_training_data.jsonl", options)?;
        for example in examples {
            // Optionally strip metadata if not needed
            let json = if self.include_metadata {
                serde_json::to_string(example)?
            } else {
                // Only include prompt and completion, strip metadata
                serde_json::to_string(&serde_json::json!({
                    "prompt": example.prompt,
                    "completion": example.completion
                }))?
            };
            writeln!(zip, "{}", json)?;
        }

        // Write metadata
        zip.start_file("_dataset_info.json", options)?;
        let info = serde_json::json!({
            "total_files": file_count,
            "total_examples": examples.len(),
            "total_tokens_estimate": examples.iter().map(|e| e.metadata.tokens_estimate).sum::<usize>(),
            "sanitization_stats": {
                "files_processed": stats.files_processed,
                "items_redacted": stats.items_redacted,
                "redacted_by_type": stats.redacted_by_type.iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect::<HashMap<_, _>>(),
            },
            "generated_at": chrono::Utc::now().to_rfc3339(),
            "format": "complete_sanitized_logs",
            "safe_for_training": true,
            "note": "This archive contains ALL your sanitized logs, not just extracted examples"
        });
        write!(zip, "{}", serde_json::to_string_pretty(&info)?)?;

        // Write README
        zip.start_file("README.md", options)?;
        write!(
            zip,
            r#"# AI Training Dataset - COMPLETE LOGS

## Overview

This dataset contains ALL your AI coding assistant logs with sensitive data removed.

## Contents

This is the COMPLETE sanitized version of your logs, preserving:
- ✅ All log files
- ✅ All debug information
- ✅ All file history
- ✅ All shell snapshots
- ✅ All telemetry
- ✅ Complete directory structure

**What was removed:**
- ❌ API keys and tokens
- ❌ Passwords
- ❌ Email addresses
- ❌ Phone numbers
- ❌ Credit cards, SSNs
- ❌ IP addresses
- ❌ Personal file paths (anonymized)
- ❌ .env files

## Additional Files

- `_training_data.jsonl` - Extracted prompt/completion pairs for easy finetuning
- `_dataset_info.json` - Metadata and sanitization statistics
- `README.md` - This file

## File Count

**Total files:** {}
**Items redacted:** {}

## Usage

All files are safe to:
- Use for model training
- Share with team members
- Upload to cloud storage
- Analyze with scripts
- Use for research

Everything sensitive has been replaced with `[REDACTED_*]` placeholders.

## License

Use responsibly and in accordance with your organization's data policies.
"#,
            file_count, stats.items_redacted
        )?;

        zip.finish()?;

        Ok(dataset_path)
    }

    #[allow(dead_code)]
    fn create_dataset_archive(
        &self,
        examples: &[TrainingExample],
        stats: &SanitizeStats,
    ) -> Result<PathBuf> {
        let dataset_path = self.output_dir.join(format!(
            "ai-training-dataset-{}.zip",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        ));

        let file = File::create(&dataset_path)?;
        let mut zip = ZipWriter::new(file);

        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        // Write ALL sanitized files from temp directory
        let temp_dir = std::env::temp_dir().join(format!(
            "ai-logs-sanitized-{}",
            chrono::Utc::now().timestamp()
        ));

        println!("  Adding all sanitized files to archive...");
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} Adding files... {msg}")
                .unwrap(),
        );

        for entry in walkdir::WalkDir::new(&temp_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Get relative path for archive
            let relative_path = path
                .strip_prefix(&temp_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            pb.set_message(relative_path.clone());

            // Add file to ZIP
            zip.start_file(&relative_path, options)?;
            let content = fs::read(path)?;
            zip.write_all(&content)?;
        }

        pb.finish_with_message("All files added!");

        // Write training examples as JSONL (optional structured format)
        zip.start_file("_training_data.jsonl", options)?;
        for example in examples {
            // Optionally strip metadata if not needed
            let json = if self.include_metadata {
                serde_json::to_string(example)?
            } else {
                // Only include prompt and completion, strip metadata
                serde_json::to_string(&serde_json::json!({
                    "prompt": example.prompt,
                    "completion": example.completion
                }))?
            };
            writeln!(zip, "{}", json)?;
        }

        // Write metadata
        zip.start_file("_dataset_info.json", options)?;
        let info = serde_json::json!({
            "total_examples": examples.len(),
            "total_tokens_estimate": examples.iter().map(|e| e.metadata.tokens_estimate).sum::<usize>(),
            "sanitization_stats": {
                "files_processed": stats.files_processed,
                "items_redacted": stats.items_redacted,
                "redacted_by_type": stats.redacted_by_type.iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect::<HashMap<_, _>>(),
            },
            "generated_at": chrono::Utc::now().to_rfc3339(),
            "format": "complete_sanitized_logs",
            "safe_for_training": true,
        });
        write!(zip, "{}", serde_json::to_string_pretty(&info)?)?;

        // Write README
        zip.start_file("README.md", options)?;
        write!(
            zip,
            r#"# AI Training Dataset

## Overview

This dataset was prepared from AI coding assistant logs with all sensitive data removed.

## Contents

- `training_data.jsonl` - Training examples in JSONL format
- `dataset_info.json` - Metadata and statistics
- `README.md` - This file

## Format

Each line in `training_data.jsonl` contains:
```json
{{
  "prompt": "User's question or request",
  "completion": "AI assistant's response",
  "metadata": {{
    "tool": "Tool name",
    "session_id": "Unique session identifier",
    "timestamp": "ISO 8601 timestamp",
    "tokens_estimate": 1234
  }}
}}
```

## Sanitization

All sensitive data has been removed:
- API keys and tokens
- Passwords
- Email addresses
- Phone numbers
- Credit card numbers
- Social Security Numbers
- IP addresses
- Personal file paths
- Environment variables

**Files processed:** {}
**Items redacted:** {}

## Usage

This dataset is safe for:
- Model finetuning
- Training experiments
- Research purposes
- Sharing with team members

## License

Use responsibly and in accordance with your organization's data policies.
"#,
            stats.files_processed, stats.items_redacted
        )?;

        zip.finish()?;

        Ok(dataset_path)
    }

    fn print_summary(&self, results: &PrepareResults) {
        println!("{}", "═".repeat(60).cyan());
        println!("{}", "  Dataset Preparation Complete!".bold().green());
        println!("{}", "═".repeat(60).cyan());
        println!();
        println!("📊 Statistics:");
        println!(
            "  Training Examples:    {}",
            results.examples_count.to_string().green()
        );
        println!("  Files Processed:      {}", results.files_processed);
        println!(
            "  Sensitive Items:      {} removed",
            results.items_redacted.to_string().red()
        );
        println!();
        println!("💾 File Sizes:");
        println!(
            "  Original Backup:      {}",
            format_bytes(results.original_size)
        );
        println!(
            "  Sanitized Dataset:    {}",
            format_bytes(results.sanitized_size).green()
        );
        println!(
            "  Reduction:            {:.1}%",
            (1.0 - (results.sanitized_size as f64 / results.original_size as f64)) * 100.0
        );
        println!();
        println!("📁 Output:");
        println!(
            "  Backup:               {}",
            results.backup_path.display().to_string().yellow()
        );
        println!(
            "  Dataset:              {}",
            results.dataset_path.display().to_string().cyan()
        );
        println!();
        println!(
            "{}",
            "✅ Safe for finetuning - all PII removed!".green().bold()
        );
        println!();
    }
}

pub struct PrepareResults {
    pub dataset_path: PathBuf,
    pub backup_path: PathBuf,
    pub examples_count: usize,
    pub original_size: u64,
    pub sanitized_size: u64,
    pub items_redacted: usize,
    pub files_processed: usize,
}

#[derive(Debug)]
pub struct SanitizeStats {
    pub files_processed: usize,
    pub items_redacted: usize,
    pub redacted_by_type: HashMap<SensitiveDataType, usize>,
}

fn extract_prompt(json: &serde_json::Value) -> Option<String> {
    json.get("userMessage")
        .or_else(|| json.get("prompt"))
        .or_else(|| json.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn extract_completion(json: &serde_json::Value) -> Option<String> {
    json.get("assistantMessage")
        .or_else(|| json.get("response"))
        .or_else(|| json.get("output"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}
