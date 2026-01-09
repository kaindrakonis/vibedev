use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, BufReader};
use std::path::{Path, PathBuf};

use crate::models::LogLocation;

/// Metadata about the search index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// Schema version
    pub version: String,

    /// When the index was last updated
    pub last_indexed: DateTime<Utc>,

    /// Metadata for each indexed location
    pub indexed_locations: Vec<LocationMetadata>,

    /// Total number of documents in the index
    pub total_docs: u64,

    /// Whether semantic search is enabled
    pub semantic_enabled: bool,
}

/// Metadata for a single log location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationMetadata {
    /// File path
    pub path: PathBuf,

    /// Content hash (MD5 of first 1MB + last modified time)
    pub hash: String,

    /// File size in bytes
    pub size_bytes: u64,

    /// Last modified timestamp
    pub last_modified: DateTime<Utc>,

    /// Number of documents indexed from this location
    pub doc_count: u64,
}

impl IndexMetadata {
    /// Creates new empty metadata
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            last_indexed: Utc::now(),
            indexed_locations: Vec::new(),
            total_docs: 0,
            semantic_enabled: false,
        }
    }

    /// Loads metadata from a file
    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path).context("Failed to open metadata file")?;
        let reader = BufReader::new(file);
        let metadata: IndexMetadata =
            serde_json::from_reader(reader).context("Failed to parse metadata JSON")?;
        Ok(metadata)
    }

    /// Saves metadata to a file
    pub fn save(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = File::create(path).context("Failed to create metadata file")?;
        serde_json::to_writer_pretty(file, self).context("Failed to write metadata JSON")?;
        Ok(())
    }

    /// Finds a location by path
    pub fn find_location(&self, path: &Path) -> Option<&LocationMetadata> {
        self.indexed_locations
            .iter()
            .find(|loc| loc.path == path)
    }

    /// Updates or adds a location
    pub fn upsert_location(&mut self, location: LocationMetadata) {
        if let Some(pos) = self
            .indexed_locations
            .iter()
            .position(|loc| loc.path == location.path)
        {
            self.indexed_locations[pos] = location;
        } else {
            self.indexed_locations.push(location);
        }
    }

    /// Removes a location by path
    pub fn remove_location(&mut self, path: &Path) {
        self.indexed_locations.retain(|loc| loc.path != path);
    }

    /// Updates the total document count
    pub fn update_total_docs(&mut self) {
        self.total_docs = self.indexed_locations.iter().map(|loc| loc.doc_count).sum();
    }
}

impl Default for IndexMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl LocationMetadata {
    /// Creates metadata from a LogLocation
    pub fn from_log_location(location: &LogLocation) -> Result<Self> {
        let hash = compute_location_hash(&location.path)?;
        let last_modified = fs::metadata(&location.path)?
            .modified()?
            .into();

        Ok(Self {
            path: location.path.clone(),
            hash,
            size_bytes: location.size_bytes,
            last_modified,
            doc_count: 0,  // Will be updated during indexing
        })
    }

    /// Checks if this location has changed compared to current state
    pub fn has_changed(&self) -> Result<bool> {
        // Check if file still exists
        if !self.path.exists() {
            return Ok(true);
        }

        // Compute current hash
        let current_hash = compute_location_hash(&self.path)?;

        // Compare hashes
        Ok(current_hash != self.hash)
    }
}

/// Computes a hash for a log location to detect changes
/// Hash is MD5(first_1mb || last_modified_timestamp)
pub fn compute_location_hash(path: &Path) -> Result<String> {
    let mut hasher = Md5::new();

    // Get file metadata
    let metadata = fs::metadata(path).context("Failed to read file metadata")?;
    let last_modified: DateTime<Utc> = metadata.modified()?.into();

    // Read first 1MB of file (or whole file if smaller)
    let max_bytes = 1_048_576; // 1MB
    let file = File::open(path).context("Failed to open file for hashing")?;
    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; max_bytes];
    let bytes_read = reader.read(&mut buffer).context("Failed to read file")?;

    // Hash the content + timestamp
    hasher.update(&buffer[..bytes_read]);
    hasher.update(last_modified.to_rfc3339().as_bytes());

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Detects which locations have changed since the last index
pub fn detect_changes(
    metadata: &IndexMetadata,
    discovered_locations: &[LogLocation],
) -> Result<Vec<LogLocation>> {
    let mut changed = Vec::new();

    // Build a map of existing locations for fast lookup
    let existing: HashMap<PathBuf, &LocationMetadata> = metadata
        .indexed_locations
        .iter()
        .map(|loc| (loc.path.clone(), loc))
        .collect();

    for location in discovered_locations {
        let needs_reindex = if let Some(existing_loc) = existing.get(&location.path) {
            // Check if it has changed
            existing_loc.has_changed().unwrap_or(true)
        } else {
            // New location
            true
        };

        if needs_reindex {
            changed.push(location.clone());
        }
    }

    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::io::Write;

    #[test]
    fn test_metadata_new() {
        let metadata = IndexMetadata::new();
        assert_eq!(metadata.version, "1.0");
        assert_eq!(metadata.total_docs, 0);
        assert_eq!(metadata.indexed_locations.len(), 0);
        assert!(!metadata.semantic_enabled);
    }

    #[test]
    fn test_metadata_save_load() {
        let dir = tempdir().unwrap();
        let metadata_path = dir.path().join("metadata.json");

        let mut metadata = IndexMetadata::new();
        metadata.total_docs = 100;
        metadata.semantic_enabled = true;

        // Save
        metadata.save(&metadata_path).unwrap();

        // Load
        let loaded = IndexMetadata::load(&metadata_path).unwrap();
        assert_eq!(loaded.version, metadata.version);
        assert_eq!(loaded.total_docs, 100);
        assert!(loaded.semantic_enabled);
    }

    #[test]
    fn test_compute_location_hash() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        // Create a test file
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Test content").unwrap();
        drop(file);

        // Compute hash
        let hash1 = compute_location_hash(&file_path).unwrap();
        assert!(!hash1.is_empty());

        // Hash should be consistent for same file
        let hash2 = compute_location_hash(&file_path).unwrap();
        assert_eq!(hash1, hash2);

        // Modify file
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Modified content").unwrap();
        drop(file);

        // Hash should change
        let hash3 = compute_location_hash(&file_path).unwrap();
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_upsert_location() {
        let mut metadata = IndexMetadata::new();

        let loc1 = LocationMetadata {
            path: PathBuf::from("/test/path1.log"),
            hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: Utc::now(),
            doc_count: 10,
        };

        // Add new location
        metadata.upsert_location(loc1.clone());
        assert_eq!(metadata.indexed_locations.len(), 1);

        // Update existing location
        let mut loc1_updated = loc1.clone();
        loc1_updated.doc_count = 20;
        metadata.upsert_location(loc1_updated);
        assert_eq!(metadata.indexed_locations.len(), 1);
        assert_eq!(metadata.indexed_locations[0].doc_count, 20);
    }

    #[test]
    fn test_update_total_docs() {
        let mut metadata = IndexMetadata::new();

        metadata.indexed_locations.push(LocationMetadata {
            path: PathBuf::from("/test/1.log"),
            hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: Utc::now(),
            doc_count: 50,
        });

        metadata.indexed_locations.push(LocationMetadata {
            path: PathBuf::from("/test/2.log"),
            hash: "hash2".to_string(),
            size_bytes: 200,
            last_modified: Utc::now(),
            doc_count: 75,
        });

        metadata.update_total_docs();
        assert_eq!(metadata.total_docs, 125);
    }
}
