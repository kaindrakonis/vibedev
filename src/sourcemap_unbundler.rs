/// Source Map-Aware Unbundler - Reconstructs original files from bundled code
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use regex::Regex;

/// Source Map v3 format
#[derive(Debug, Deserialize, Serialize)]
pub struct SourceMap {
    pub version: u32,
    pub sources: Vec<String>,
    #[serde(default)]
    pub names: Vec<String>,
    pub mappings: String,
    #[serde(rename = "sourcesContent")]
    #[serde(default)]
    pub sources_content: Option<Vec<Option<String>>>,
    #[serde(rename = "sourceRoot")]
    #[serde(default)]
    pub source_root: Option<String>,
}

pub struct SourceMapExtractor {
    binary_path: PathBuf,
}

impl SourceMapExtractor {
    pub fn new(binary_path: PathBuf) -> Self {
        Self { binary_path }
    }

    /// Extract all source maps from binary
    pub fn extract_source_maps(&self) -> Result<Vec<SourceMap>, Box<dyn std::error::Error>> {
        // Read binary directly to get better extraction
        let binary_bytes = std::fs::read(&self.binary_path)?;
        let binary_str = String::from_utf8_lossy(&binary_bytes);

        let mut source_maps = Vec::new();

        // Strategy 1: Look for complete JSON source maps
        let json_re = Regex::new(r#"\{"version"\s*:\s*3[^}]{20,2000}\}"#)?;

        for caps in json_re.captures_iter(&binary_str) {
            let json_str = caps.get(0).unwrap().as_str();
            if json_str.contains("sources") {
                // Try to find the complete JSON by extending the match
                if let Some(start) = binary_str.find(json_str) {
                    // Look for the complete object (might be longer)
                    let mut end = start + json_str.len();
                    let mut brace_count = 0;
                    let chars: Vec<char> = binary_str[start..].chars().collect();

                    for (i, c) in chars.iter().enumerate() {
                        match c {
                            '{' => brace_count += 1,
                            '}' => {
                                brace_count -= 1;
                                if brace_count == 0 {
                                    end = start + i + 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }

                    let complete_json = &binary_str[start..end];
                    if let Ok(map) = serde_json::from_str::<SourceMap>(complete_json) {
                        source_maps.push(map);
                    }
                }
            }
        }

        // Strategy 2: Use strings command for fragmented maps
        if source_maps.is_empty() {
            let output = std::process::Command::new("strings")
                .arg("-n")
                .arg("50")
                .arg(&self.binary_path)
                .output()?;

            let content = String::from_utf8_lossy(&output.stdout);

            for line in content.lines() {
                if line.contains(r#""version":3"#) || line.contains(r#""sources":"#) {
                    // Try to parse what we have
                    if let Ok(map) = serde_json::from_str::<SourceMap>(line) {
                        source_maps.push(map);
                    }
                }
            }
        }

        println!("[+] Found {} source maps", source_maps.len());
        Ok(source_maps)
    }

    /// Extract inline base64 source maps
    pub fn extract_inline_source_maps(&self) -> Result<Vec<SourceMap>, Box<dyn std::error::Error>> {
        let output = std::process::Command::new("strings")
            .arg(&self.binary_path)
            .output()?;

        let content = String::from_utf8_lossy(&output.stdout);
        let mut source_maps = Vec::new();

        // Match: //# sourceMappingURL=data:application/json;base64,<base64>
        let re = Regex::new(r"sourceMappingURL=data:application/json;base64,([A-Za-z0-9+/=]+)")?;

        for caps in re.captures_iter(&content) {
            if let Some(base64_str) = caps.get(1) {
                if let Ok(decoded) = base64::decode(base64_str.as_str()) {
                    if let Ok(json_str) = String::from_utf8(decoded) {
                        if let Ok(map) = serde_json::from_str::<SourceMap>(&json_str) {
                            source_maps.push(map);
                        }
                    }
                }
            }
        }

        println!("[+] Found {} inline source maps", source_maps.len());
        Ok(source_maps)
    }

    /// Reconstruct original file structure from source maps
    pub fn reconstruct_files(&self, output_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let source_maps = self.extract_source_maps()?;

        if source_maps.is_empty() {
            println!("[!] No source maps found - trying inline maps...");
            // Try inline maps if regular ones not found
            // Note: Would need base64 crate for this
        }

        let mut files_created = 0;

        for (idx, map) in source_maps.iter().enumerate() {
            println!("\n[*] Processing source map #{}", idx + 1);
            println!("    Sources: {}", map.sources.len());
            println!("    Names: {}", map.names.len());

            // If sourcesContent is embedded, we can extract the original files!
            if let Some(sources_content) = &map.sources_content {
                for (i, source_path) in map.sources.iter().enumerate() {
                    if let Some(Some(content)) = sources_content.get(i) {
                        let file_path = self.sanitize_path(source_path);
                        let full_path = output_dir.join(&file_path);

                        // Create parent directories
                        if let Some(parent) = full_path.parent() {
                            fs::create_dir_all(parent)?;
                        }

                        // Write original source
                        let mut file = File::create(&full_path)?;
                        file.write_all(content.as_bytes())?;

                        println!("    [+] Extracted: {}", file_path.display());
                        files_created += 1;
                    }
                }
            }

            // Save the source map itself for reference
            let map_path = output_dir.join(format!("sourcemap_{}.json", idx));
            let map_json = serde_json::to_string_pretty(&map)?;
            fs::write(&map_path, map_json)?;
        }

        println!("\n[+] Reconstructed {} original files", files_created);
        Ok(())
    }

    /// Sanitize source paths (remove protocols, fix paths)
    fn sanitize_path(&self, path: &str) -> PathBuf {
        let cleaned = path
            .replace("bun://", "bun/")
            .replace("file://", "")
            .replace("node:", "node/")
            .replace("webpack://", "webpack/");

        PathBuf::from(cleaned)
    }

    /// Create a mapping index for name recovery
    pub fn create_name_index(&self) -> Result<HashMap<String, Vec<String>>, Box<dyn std::error::Error>> {
        let source_maps = self.extract_source_maps()?;
        let mut name_index: HashMap<String, Vec<String>> = HashMap::new();

        for map in source_maps {
            for source in &map.sources {
                name_index.entry(source.clone())
                    .or_insert_with(Vec::new)
                    .extend(map.names.clone());
            }
        }

        Ok(name_index)
    }

    /// Generate a report of what's in the source maps
    pub fn generate_report(&self) -> Result<(), Box<dyn std::error::Error>> {
        let source_maps = self.extract_source_maps()?;

        println!("\n=== Source Map Analysis ===\n");
        println!("Total source maps found: {}", source_maps.len());

        let mut all_sources: Vec<String> = Vec::new();
        let mut all_names: Vec<String> = Vec::new();
        let mut has_content_count = 0;

        for (idx, map) in source_maps.iter().enumerate() {
            all_sources.extend(map.sources.clone());
            all_names.extend(map.names.clone());

            if map.sources_content.is_some() {
                has_content_count += 1;
            }

            if idx < 5 {  // Show details for first 5
                println!("\nSource Map #{}:", idx + 1);
                println!("  Version: {}", map.version);
                println!("  Sources: {}", map.sources.len());
                println!("  Names: {}", map.names.len());
                println!("  Has embedded content: {}", map.sources_content.is_some());

                if !map.sources.is_empty() {
                    println!("  Source files:");
                    for (i, src) in map.sources.iter().take(10).enumerate() {
                        println!("    {}: {}", i + 1, src);
                    }
                    if map.sources.len() > 10 {
                        println!("    ... and {} more", map.sources.len() - 10);
                    }
                }
            }
        }

        println!("\n=== Summary ===");
        println!("Total unique source files: {}", all_sources.len());
        println!("Total unique names: {}", all_names.len());
        println!("Maps with embedded content: {}/{}", has_content_count, source_maps.len());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_path() {
        let extractor = SourceMapExtractor::new(PathBuf::from("/tmp/test"));

        assert_eq!(
            extractor.sanitize_path("bun://Bun/runtime.ts"),
            PathBuf::from("bun/Bun/runtime.ts")
        );

        assert_eq!(
            extractor.sanitize_path("webpack://app/src/index.js"),
            PathBuf::from("webpack/app/src/index.js")
        );
    }
}
