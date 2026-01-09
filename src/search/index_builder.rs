use anyhow::{Context, Result};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexWriter};

use crate::discovery::LogDiscovery;
use crate::models::{DiscoveryFindings, LogLocation};
use crate::models::{format_bytes, LogType};
use crate::parsers::claude::ClaudeParser;
use crate::parsers::cline::ClineParser;
use crate::parsers::cursor::CursorParser;
use crate::parsers::generic::GenericParser;
use crate::parsers::LogParser;

use super::metadata::{detect_changes, IndexMetadata, LocationMetadata};
use super::schema::{build_schema, LogEntryDocument, FIELD_FILE_PATH};

const BATCH_SIZE: usize = 10_000;
const MEMORY_BUDGET_MB: usize = 500;

/// Statistics about indexing operation
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub total_docs: u64,
    pub total_files: usize,
    pub total_bytes: u64,
    pub index_size_bytes: u64,
    pub duration_secs: f64,
}

/// Builds and manages the Tantivy search index
pub struct IndexBuilder {
    index_path: PathBuf,
    metadata_path: PathBuf,
    index: Index,
    schema: Schema,
}

impl IndexBuilder {
    /// Creates a new IndexBuilder
    pub fn new(index_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(index_dir)?;

        let schema = build_schema();
        let index_path = index_dir.to_path_buf();
        let metadata_path = index_dir.join("metadata.json");

        // Try to open existing index, or create new one
        let index = if index_path.join("meta.json").exists() {
            Index::open_in_dir(&index_path).context("Failed to open existing index")?
        } else {
            Index::create_in_dir(&index_path, schema.clone())
                .context("Failed to create new index")?
        };

        Ok(Self {
            index_path,
            metadata_path,
            index,
            schema,
        })
    }

    /// Builds the initial index from discovered logs
    pub fn build_initial_index(&mut self) -> Result<IndexStats> {
        let start = std::time::Instant::now();

        println!("\n{}", "📊 Discovering logs...".cyan());

        // Run discovery
        let home_dir = dirs::home_dir().context("Could not determine home directory")?;
        let discovery = LogDiscovery::new(home_dir, true);
        let findings = discovery.scan()?;

        println!(
            "   Found: {} tools, {} locations, {}",
            findings.tools_found.len().to_string().yellow(),
            findings.locations.len().to_string().yellow(),
            format_bytes(findings.total_size_bytes).cyan()
        );

        println!("\n{}", "🏗️  Building search index...".green().bold());

        // Index all locations
        let stats = self.index_locations(&findings.locations, true)?;

        // Save metadata
        let mut metadata = IndexMetadata::new();
        for location in &findings.locations {
            if let Ok(loc_meta) = LocationMetadata::from_log_location(location) {
                metadata.upsert_location(loc_meta);
            }
        }
        metadata.update_total_docs();
        metadata.save(&self.metadata_path)?;

        let elapsed = start.elapsed().as_secs_f64();
        let mut final_stats = stats;
        final_stats.duration_secs = elapsed;

        // Print summary
        self.print_stats(&final_stats);

        Ok(final_stats)
    }

    /// Updates the index with new/changed logs
    pub fn update_index(&mut self) -> Result<IndexStats> {
        let start = std::time::Instant::now();

        println!("\n{}", "🔄 Checking for updates...".cyan());

        // Load existing metadata
        let metadata = if self.metadata_path.exists() {
            IndexMetadata::load(&self.metadata_path)?
        } else {
            return self.build_initial_index();
        };

        // Run discovery
        let home_dir = dirs::home_dir().context("Could not determine home directory")?;
        let discovery = LogDiscovery::new(home_dir, true);
        let findings = discovery.scan()?;

        // Detect changes
        let changed = detect_changes(&metadata, &findings.locations)?;

        if changed.is_empty() {
            println!("   {}", "No changes detected".green());
            return Ok(IndexStats {
                total_docs: metadata.total_docs,
                total_files: metadata.indexed_locations.len(),
                total_bytes: metadata
                    .indexed_locations
                    .iter()
                    .map(|l| l.size_bytes)
                    .sum(),
                index_size_bytes: self.get_index_size()?,
                duration_secs: 0.0,
            });
        }

        println!(
            "   Found {} new/changed files",
            changed.len().to_string().yellow()
        );

        // Delete old docs for changed locations
        self.delete_docs_by_paths(&changed)?;

        // Reindex changed locations
        let stats = self.index_locations(&changed, false)?;

        // Update metadata
        let mut updated_metadata = metadata;
        for location in &changed {
            if let Ok(mut loc_meta) = LocationMetadata::from_log_location(location) {
                loc_meta.doc_count = stats.total_docs;
                updated_metadata.upsert_location(loc_meta);
            }
        }
        updated_metadata.last_indexed = chrono::Utc::now();
        updated_metadata.update_total_docs();
        updated_metadata.save(&self.metadata_path)?;

        let elapsed = start.elapsed().as_secs_f64();
        let mut final_stats = stats;
        final_stats.duration_secs = elapsed;

        self.print_stats(&final_stats);

        Ok(final_stats)
    }

    /// Indexes a list of log locations
    fn index_locations(
        &mut self,
        locations: &[LogLocation],
        show_progress: bool,
    ) -> Result<IndexStats> {
        let total_files = locations.len();
        let total_bytes: u64 = locations.iter().map(|l| l.size_bytes).sum();

        // Create progress bar
        let pb = if show_progress {
            let pb = ProgressBar::new(total_files as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(
                        "   [{bar:40.cyan/blue}] {pos}/{len} files ({msg}) ETA: {eta}",
                    )
                    .unwrap()
                    .progress_chars("█▓▒░ "),
            );
            Some(pb)
        } else {
            None
        };

        // Create index writer
        let mut writer: IndexWriter<TantivyDocument> = self
            .index
            .writer(MEMORY_BUDGET_MB * 1_024 * 1_024)
            .context("Failed to create index writer")?;

        // Track stats
        let doc_counter = Arc::new(AtomicU64::new(0));
        let doc_id_counter = Arc::new(AtomicU64::new(self.get_current_max_doc_id()? + 1));

        // Parsers chain
        let parsers: Vec<Box<dyn LogParser>> = vec![
            Box::new(ClaudeParser),
            Box::new(ClineParser),
            Box::new(CursorParser),
            Box::new(GenericParser),
        ];

        let mut bytes_processed = 0u64;

        for location in locations {
            // Try each parser
            let parsed = parsers
                .iter()
                .find_map(|parser| {
                    if parser.can_parse(&location.path) {
                        parser.parse(&location.path).ok()
                    } else {
                        None
                    }
                });

            if let Some(parsed_log) = parsed {
                // Index entries in batches
                let mut batch = Vec::new();

                for entry in &parsed_log.entries {
                    let doc_id = doc_id_counter.fetch_add(1, Ordering::SeqCst);

                    let log_entry_doc = LogEntryDocument::from_log_entry(
                        entry,
                        doc_id,
                        &parsed_log.tool,
                        &format!("{:?}", location.log_type),
                        &location.path,
                    );

                    batch.push(log_entry_doc);

                    // Commit batch
                    if batch.len() >= BATCH_SIZE {
                        for doc in &batch {
                            writer.add_document(doc.to_tantivy_document(&self.schema))?;
                        }
                        doc_counter.fetch_add(batch.len() as u64, Ordering::SeqCst);
                        batch.clear();
                    }
                }

                // Commit remaining
                if !batch.is_empty() {
                    for doc in &batch {
                        writer.add_document(doc.to_tantivy_document(&self.schema))?;
                    }
                    doc_counter.fetch_add(batch.len() as u64, Ordering::SeqCst);
                }
            }

            bytes_processed += location.size_bytes;

            if let Some(ref pb) = pb {
                pb.inc(1);
                pb.set_message(format_bytes(bytes_processed));
            }
        }

        if let Some(pb) = pb {
            pb.finish_with_message("Done!");
        }

        // Commit index
        writer
            .commit()
            .context("Failed to commit index")?;

        let total_docs = doc_counter.load(Ordering::SeqCst);
        let index_size = self.get_index_size()?;

        Ok(IndexStats {
            total_docs,
            total_files,
            total_bytes,
            index_size_bytes: index_size,
            duration_secs: 0.0, // Set by caller
        })
    }

    /// Deletes documents by file paths (for incremental updates)
    fn delete_docs_by_paths(&mut self, locations: &[LogLocation]) -> Result<()> {
        let mut writer: IndexWriter<TantivyDocument> = self
            .index
            .writer(MEMORY_BUDGET_MB * 1_024 * 1_024)?;

        let file_path_field = self.schema.get_field(FIELD_FILE_PATH)?;

        for location in locations {
            let path_str = location.path.to_string_lossy().to_string();
            let term = Term::from_field_text(file_path_field, &path_str);
            writer.delete_term(term);
        }

        writer.commit()?;
        Ok(())
    }

    /// Gets the current maximum doc_id in the index
    fn get_current_max_doc_id(&self) -> Result<u64> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();

        // Simple approach: return the number of documents
        // This assumes doc_ids are sequential
        Ok(searcher.num_docs())
    }

    /// Gets the size of the index on disk
    fn get_index_size(&self) -> Result<u64> {
        let mut size = 0u64;
        for entry in walkdir::WalkDir::new(&self.index_path) {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() {
                    size += entry.metadata()?.len();
                }
            }
        }
        Ok(size)
    }

    /// Prints indexing statistics
    fn print_stats(&self, stats: &IndexStats) {
        println!();
        println!(
            "   Indexed {} log entries in {:.1}s",
            stats.total_docs.to_string().green().bold(),
            stats.duration_secs
        );
        println!(
            "   Index size: {}",
            format_bytes(stats.index_size_bytes).green()
        );
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_index_builder_new() {
        let dir = tempdir().unwrap();
        let index_dir = dir.path().join("test_index");

        let builder = IndexBuilder::new(&index_dir);
        assert!(builder.is_ok());

        // Verify index was created
        assert!(index_dir.exists());
        assert!(index_dir.join("meta.json").exists());
    }
}
