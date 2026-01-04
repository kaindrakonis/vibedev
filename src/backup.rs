use crate::discovery::LogDiscovery;
use crate::models::format_bytes;
use anyhow::{Context, Result};
use chrono::Utc;
use colored::*;
use flate2::write::GzEncoder;
use flate2::Compression;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::{self, File};
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct BackupManager {
    output_dir: PathBuf,
    compression_level: u32,
}

impl BackupManager {
    pub fn new(output_dir: PathBuf, compression_level: u32) -> Self {
        Self {
            output_dir,
            compression_level: compression_level.min(9),
        }
    }

    pub async fn create_backup(
        &self,
        tool_filter: Option<String>,
        include_timestamp: bool,
    ) -> Result<PathBuf> {
        // Discover logs
        let home_dir = dirs::home_dir().context("Could not determine home directory")?;
        let discovery = LogDiscovery::new(home_dir.clone(), true);
        let findings = discovery.scan()?;

        // Filter by tool if specified
        let locations = if let Some(tool_name) = tool_filter.as_ref() {
            findings
                .locations
                .into_iter()
                .filter(|loc| {
                    loc.tool
                        .name()
                        .to_lowercase()
                        .contains(&tool_name.to_lowercase())
                })
                .collect()
        } else {
            findings.locations
        };

        if locations.is_empty() {
            println!("⚠️  No logs found to backup");
            return Err(anyhow::anyhow!("No logs found"));
        }

        // Calculate total size
        let total_size: u64 = locations.iter().map(|loc| loc.size_bytes).sum();
        let total_files: usize = locations.iter().map(|loc| loc.file_count).sum();

        println!("\n📦 Backup Summary:");
        println!("  Tools: {}", locations.len());
        println!("  Total Size: {}", format_bytes(total_size).cyan());
        println!("  Total Files: {}", total_files.to_string().cyan());
        println!();

        // Generate backup filename
        let backup_filename = self.generate_filename(tool_filter.as_deref(), include_timestamp);
        let backup_path = self.output_dir.join(&backup_filename);

        println!("📁 Creating archive: {}\n", backup_filename.yellow());

        // Create progress bar
        let pb = ProgressBar::new(total_files as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({msg})")
                .unwrap()
                .progress_chars("#>-"),
        );

        // Create tar.gz archive
        let tar_gz = File::create(&backup_path)?;
        let enc = GzEncoder::new(tar_gz, Compression::new(self.compression_level));
        let mut tar = tar::Builder::new(enc);

        let mut files_added = 0u64;
        let mut bytes_added = 0u64;

        for location in &locations {
            let tool_name = location.tool.name().replace(" ", "_");
            let log_type = format!("{:?}", location.log_type).to_lowercase();

            if location.path.is_file() {
                // Add single file
                let archive_path = format!(
                    "{}/{}",
                    tool_name,
                    location.path.file_name().unwrap().to_string_lossy()
                );

                if let Ok(mut file) = File::open(&location.path) {
                    tar.append_file(&archive_path, &mut file)?;
                    files_added += 1;
                    bytes_added += location.size_bytes;
                    pb.inc(1);
                    pb.set_message(format_bytes(bytes_added));
                }
            } else if location.path.is_dir() {
                // Add directory recursively
                for entry in WalkDir::new(&location.path)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let path = entry.path();
                    if path.is_file() {
                        let relative = path.strip_prefix(&home_dir).unwrap_or(path);
                        let archive_path =
                            format!("{}/{}/{}", tool_name, log_type, relative.display());

                        if let Ok(mut file) = File::open(path) {
                            if let Ok(metadata) = file.metadata() {
                                tar.append_file(&archive_path, &mut file)?;
                                files_added += 1;
                                bytes_added += metadata.len();
                                pb.inc(1);
                                pb.set_message(format_bytes(bytes_added));
                            }
                        }
                    }
                }
            }
        }

        tar.finish()?;
        pb.finish_with_message("Done!");

        let backup_size = fs::metadata(&backup_path)?.len();
        let compression_ratio = (1.0 - (backup_size as f64 / total_size as f64)) * 100.0;

        println!();
        println!("{}", "✅ Backup Complete!".green().bold());
        println!();
        println!("  Location: {}", backup_path.display().to_string().cyan());
        println!("  Files Archived: {}", files_added.to_string().green());
        println!("  Original Size: {}", format_bytes(total_size));
        println!("  Backup Size: {}", format_bytes(backup_size).green());
        println!("  Compression: {:.1}%", compression_ratio);
        println!();

        Ok(backup_path)
    }

    fn generate_filename(&self, tool_filter: Option<&str>, include_timestamp: bool) -> String {
        let mut parts = vec!["ai-logs-backup".to_string()];

        if let Some(tool) = tool_filter {
            parts.push(tool.to_lowercase().replace(" ", "-"));
        }

        if include_timestamp {
            let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
            parts.push(timestamp.to_string());
        }

        format!("{}.tar.gz", parts.join("-"))
    }
}

/// Restore backup from archive
pub fn restore_backup(backup_path: &PathBuf, output_dir: Option<PathBuf>) -> Result<()> {
    let restore_dir = output_dir.unwrap_or_else(|| {
        dirs::home_dir()
            .expect("Could not determine home directory")
            .join("ai-logs-restore")
    });

    println!("📂 Restoring backup to: {}", restore_dir.display());

    fs::create_dir_all(&restore_dir)?;

    let tar_gz = File::open(backup_path)?;
    let tar = flate2::read::GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(tar);

    archive.unpack(&restore_dir)?;

    println!("✅ Backup restored successfully!");
    println!("   Location: {}", restore_dir.display());

    Ok(())
}
