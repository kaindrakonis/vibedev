/// Project Structure Recovery from Source Maps
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use regex::Regex;

use crate::sourcemap_unbundler::{SourceMap, SourceMapExtractor};
use crate::deobfuscator::JSDeobfuscator;
use crate::behavior_analyzer::BehaviorAnalyzer;

pub struct StructureRecovery {
    binary_path: PathBuf,
    extracted_code: String,
    deobfuscate: bool,
}

impl StructureRecovery {
    pub fn new(binary_path: PathBuf, extracted_code: String) -> Self {
        Self {
            binary_path,
            extracted_code,
            deobfuscate: false,
        }
    }

    pub fn with_deobfuscation(mut self, enabled: bool) -> Self {
        self.deobfuscate = enabled;
        self
    }

    /// Main recovery pipeline
    pub fn recover_structure(&self, output_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        println!("[*] Step 1: Extracting file paths from source maps...");
        let file_paths = self.extract_file_paths()?;

        println!("[*] Step 2: Creating directory structure...");
        self.create_directory_structure(&file_paths, output_dir)?;

        println!("[*] Step 3: Distributing code to files using heuristics...");
        self.distribute_code(&file_paths, output_dir)?;

        println!("[+] Recovery complete! Reconstructed {} files", file_paths.len());
        Ok(())
    }

    /// Extract all file paths from source map fragments
    fn extract_file_paths(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let extractor = SourceMapExtractor::new(self.binary_path.clone());
        let source_maps = extractor.extract_source_maps()?;

        let mut all_paths = HashSet::new();

        for map in source_maps {
            for source in map.sources {
                all_paths.insert(source);
            }
        }

        // Also extract from strings in binary
        let output = std::process::Command::new("strings")
            .arg(&self.binary_path)
            .output()?;

        let content = String::from_utf8_lossy(&output.stdout);

        // Pattern: src/path/to/file.tsx
        let path_re = Regex::new(r"(src/[\w/-]+\.(ts|tsx|js|jsx))")?;
        for caps in path_re.captures_iter(&content) {
            if let Some(path) = caps.get(1) {
                all_paths.insert(path.as_str().to_string());
            }
        }

        // Pattern: bun://Module/file.ts
        let bun_re = Regex::new(r"bun://[\w/]+\.(ts|js)")?;
        for caps in bun_re.captures_iter(&content) {
            all_paths.insert(caps.get(0).unwrap().as_str().to_string());
        }

        let mut paths: Vec<String> = all_paths.into_iter().collect();
        paths.sort();

        println!("  [+] Found {} unique file paths", paths.len());
        Ok(paths)
    }

    /// Create directory structure based on file paths
    fn create_directory_structure(
        &self,
        file_paths: &[String],
        output_dir: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for file_path in file_paths {
            let sanitized = self.sanitize_path(file_path);
            let full_path = output_dir.join(&sanitized);

            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Create empty placeholder file
            File::create(&full_path)?;
        }

        Ok(())
    }

    /// Distribute extracted code to files using heuristics
    fn distribute_code(
        &self,
        file_paths: &[String],
        output_dir: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Parse the bundled code to find module boundaries
        let modules = self.extract_modules(&self.extracted_code)?;

        println!("  [+] Identified {} code modules", modules.len());

        // Match modules to file paths using heuristics
        let assignments = self.match_modules_to_files(&modules, file_paths)?;

        // Write code to files, separating node_modules
        let mut node_modules_count = 0;
        let mut app_code_count = 0;
        let mut ai_generated_names = 0;
        let mut detected_dependencies: HashSet<String> = HashSet::new();

        for (file_path, module_code) in assignments {
            // Detect if this is node_modules code
            let is_dependency = self.is_node_module(&file_path, &module_code);

            // Extract dependency names
            if is_dependency {
                if let Some(dep_name) = self.extract_dependency_name(&file_path, &module_code) {
                    detected_dependencies.insert(dep_name);
                }
            }

            let sanitized = self.sanitize_path(&file_path);

            // Track AI-generated filenames
            if file_path.ends_with(".AI") {
                ai_generated_names += 1;
            }

            // Place node_modules code in separate directory
            let full_path = if is_dependency {
                node_modules_count += 1;
                output_dir.join("node_modules").join(&sanitized)
            } else {
                app_code_count += 1;
                output_dir.join(&sanitized)
            };

            // Ensure parent directory exists
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Optionally deobfuscate the code (skip for node_modules to save time)
            let final_code = if self.deobfuscate && !is_dependency {
                let mut deobfuscator = JSDeobfuscator::new(module_code.clone())
                    .with_verbosity(false);
                deobfuscator.deobfuscate()
            } else {
                module_code
            };

            let mut file = File::create(&full_path)?;
            file.write_all(final_code.as_bytes())?;
        }

        println!("  [+] Application code files: {}", app_code_count);
        println!("  [+] Node modules files: {}", node_modules_count);
        if ai_generated_names > 0 {
            println!("  [+] AI-generated semantic filenames: {} (marked with .AI suffix)", ai_generated_names);
        }

        // Generate package.json if dependencies were detected
        if !detected_dependencies.is_empty() {
            self.generate_package_json(output_dir, &detected_dependencies)?;
            println!("  [+] Generated package.json with {} dependencies", detected_dependencies.len());
        }

        // Create README explaining the recovery
        self.create_readme(output_dir, file_paths)?;

        Ok(())
    }

    /// Extract module boundaries from bundled code
    fn extract_modules(&self, code: &str) -> Result<Vec<CodeModule>, Box<dyn std::error::Error>> {
        let mut modules = Vec::new();

        // Pattern 1: __commonJS wrapped modules
        let commonjs_re = Regex::new(r"var\s+require_(\w+)\s*=\s*__commonJS\(\(([^)]+)\)\s*=>\s*\{([^}]+(?:\{[^}]*\})*[^}]*)\}\)")?;

        for caps in commonjs_re.captures_iter(code) {
            let name = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let params = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            let body = caps.get(3).map(|m| m.as_str().to_string()).unwrap_or_default();

            modules.push(CodeModule {
                name: name.clone(),
                inferred_path: self.infer_file_path(&name, &body),
                code: format!("// Module: {}\n{}", name, body),
                module_type: ModuleType::CommonJS,
            });
        }

        // Pattern 2: ES6 modules with export
        let export_re = Regex::new(r"export\s+(?:default\s+)?(?:class|function|const|let|var)\s+(\w+)")?;

        let mut current_pos = 0;
        for caps in export_re.captures_iter(code) {
            if let Some(name_match) = caps.get(1) {
                let name = name_match.as_str();
                let start = name_match.start();

                // Extract surrounding context (500 chars before and after)
                let context_start = start.saturating_sub(500);
                let context_end = (start + 500).min(code.len());
                let context = &code[context_start..context_end];

                modules.push(CodeModule {
                    name: name.to_string(),
                    inferred_path: self.infer_file_path(name, context),
                    code: context.to_string(),
                    module_type: ModuleType::ES6,
                });
            }
        }

        // Pattern 3: React components
        let react_re = Regex::new(r"function\s+([A-Z]\w+)\s*\([^)]*\)\s*\{[^}]*(?:return\s+.*?<|React\.createElement)")?;

        for caps in react_re.captures_iter(code) {
            if let Some(name_match) = caps.get(1) {
                let name = name_match.as_str();
                let start = name_match.start();

                let context_start = start.saturating_sub(300);
                let context_end = (start + 1000).min(code.len());
                let context = &code[context_start..context_end];

                modules.push(CodeModule {
                    name: name.to_string(),
                    inferred_path: format!("src/components/{}.tsx", name),
                    code: context.to_string(),
                    module_type: ModuleType::ReactComponent,
                });
            }
        }

        Ok(modules)
    }

    /// Match extracted modules to file paths
    fn match_modules_to_files(
        &self,
        modules: &[CodeModule],
        file_paths: &[String],
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut assignments: HashMap<String, String> = HashMap::new();
        let mut path_counters: HashMap<String, usize> = HashMap::new();

        for module in modules {
            // Try exact match first
            if file_paths.contains(&module.inferred_path) {
                assignments.entry(module.inferred_path.clone())
                    .or_insert_with(String::new)
                    .push_str(&format!("\n// Module: {}\n{}\n", module.name, module.code));
                continue;
            }

            // Try fuzzy matching
            let best_match = file_paths.iter()
                .filter(|p| self.path_similarity(&module.inferred_path, p) > 0.5)
                .max_by(|a, b| {
                    self.path_similarity(&module.inferred_path, a)
                        .partial_cmp(&self.path_similarity(&module.inferred_path, b))
                        .unwrap()
                });

            if let Some(matched_path) = best_match {
                assignments.entry(matched_path.clone())
                    .or_insert_with(String::new)
                    .push_str(&format!("\n// Module: {}\n{}\n", module.name, module.code));
            } else {
                // Create a new file for unmatched modules - use semantic naming if possible
                let mut fallback_path = if let Some(semantic_path) = self.infer_semantic_filename(&module.code) {
                    semantic_path
                } else {
                    // Fall back to generic name with .AI suffix to mark for manual review
                    format!("recovered/{}.js.AI", module.name)
                };

                // Handle duplicates by adding counter
                let base_path = fallback_path.clone();
                let counter = path_counters.entry(base_path.clone()).or_insert(0);
                if *counter > 0 {
                    // Add counter before .AI extension
                    fallback_path = fallback_path.replace(".AI", &format!("{}.AI", counter));
                }
                *counter += 1;

                assignments.insert(fallback_path, module.code.clone());
            }
        }

        Ok(assignments)
    }

    /// Infer file path from module name and code content
    fn infer_file_path(&self, name: &str, code: &str) -> String {
        // First try semantic analysis for meaningful names
        if let Some(semantic_path) = self.infer_semantic_filename(code) {
            return semantic_path;
        }

        // Fallback to basic pattern matching
        // Check for React components
        if code.contains("React") || code.contains("jsx") || code.contains("tsx") {
            if name.chars().next().unwrap_or('a').is_uppercase() {
                return format!("src/components/{}.tsx", name);
            }
        }

        // Check for utility functions
        if code.contains("export") && !code.contains("class") {
            return format!("src/utils/{}.ts", name);
        }

        // Check for hooks (React)
        if name.starts_with("use") && name.chars().nth(3).map(|c| c.is_uppercase()).unwrap_or(false) {
            return format!("src/hooks/{}.ts", name);
        }

        // Check for types/interfaces
        if code.contains("interface") || code.contains("type ") {
            return format!("src/types/{}.ts", name);
        }

        // Check for Node.js modules
        if code.contains("require(\"") || code.contains("module.exports") {
            return format!("src/lib/{}.js", name);
        }

        // Default
        format!("src/{}.ts", name)
    }

    /// Infer semantic filename based on ACTUAL PURPOSE using behavioral analysis
    /// Analyzes: state machines, event handlers, data flow, control flow
    fn infer_semantic_filename(&self, code: &str) -> Option<String> {
        // Try behavioral analysis first (new approach)
        let analyzer = BehaviorAnalyzer::new(code.to_string());
        if let Some(semantic_path) = analyzer.get_semantic_filename() {
            return Some(semantic_path);
        }

        // Fallback to pattern matching (old approach)
        let patterns = [
            // Purpose: API Key & Secret Sanitization (found: AKIA, AWS key, AIza patterns)
            (vec!["AKIA", "AWS key", "AIza", "[REDACTED", "sanitize"],
             "src/security/apiKeySanitizer.ts.AI"),

            // Purpose: Protocol Header Management (found: Claude-Escapes, Claude-Steers)
            (vec!["Claude-Escapes", "Claude-Steers", "Claude-Permission", "Claude-Plan"],
             "src/protocol/headerParser.ts.AI"),

            // Purpose: Telemetry & Event Tracking (found: recordDroppedEvent, tengu_)
            (vec!["recordDroppedEvent", "tengu_", "trackEvent", "analytics"],
             "src/telemetry/eventRecorder.ts.AI"),

            // Purpose: Input Validation (found: datetime, email validation patterns)
            (vec!["email({message:", "datetime({offset:", "validate", "schema"],
             "src/validation/inputValidator.ts.AI"),

            // Purpose: Plan Mode Workflow (found: "Plan mode is active", isSubAgent)
            (vec!["Plan mode is active", "isSubAgent", "planFilePath", "workflow"],
             "src/planning/planModeController.ts.AI"),

            // Purpose: Agent Lifecycle Management
            (vec!["isSubAgent", "agentName", "agent lifecycle", "spawn agent"],
             "src/agents/lifecycleManager.ts.AI"),

            // Purpose: Session & State Management
            (vec!["SESSION_ACCESS_TOKEN", "session state", "snapshot", "checkpoint"],
             "src/session/stateManager.ts.AI"),

            // Purpose: File Permission & Access Control
            (vec!["Read-only except", "permission", "access control", "allowedPaths"],
             "src/permissions/accessController.ts.AI"),

            // Purpose: MCP Server Communication
            (vec!["mcp serve", "MCP", "CLAUDE_CODE_ENTRYPOINT"],
             "src/mcp/serverCommunication.ts.AI"),

            // File Operations - Sync (discovered: 787 existsSync, 278 statSync)
            (vec!["existsSync", "statSync", "readFileSync", "mkdirSync"],
             "src/utils/fileSystemSync.ts.AI"),
            (vec!["readdirSync", "unlinkSync", "rmdirSync", "writeFileSync"],
             "src/utils/fileOps.ts.AI"),

            // DOM Manipulation (discovered: 11184 createElement!)
            (vec!["createElement", "appendChild", "removeChild", "textContent"],
             "src/dom/domManipulator.ts.AI"),
            (vec!["addEventListener", "removeEventListener", "querySelector"],
             "src/dom/eventHandlers.ts.AI"),

            // Data Parsing (discovered: 775 parseInt, 442 JSON.stringify)
            (vec!["JSON.parse", "JSON.stringify", "parseInt", "parseFloat"],
             "src/utils/dataParser.ts.AI"),
            (vec!["encodeURIComponent", "decodeURIComponent", "atob", "btoa"],
             "src/utils/encoding.ts.AI"),

            // Error Handling (discovered: 8632 throw, 8520 Error()!)
            (vec!["throw new", "Error(", "try{", "catch("],
             "src/errors/errorHandler.ts.AI"),
            (vec!["Promise.reject", "reject(", "finally"],
             "src/errors/promiseErrors.ts.AI"),

            // Async/Await (discovered: 6749 await, 3907 async)
            (vec!["async ", "await ", "Promise.resolve"],
             "src/async/asyncOps.ts.AI"),

            // React Components & Hooks (discovered: 990 useState, 482 useEffect)
            (vec!["useState", "useEffect", "useContext", "useMemo"],
             "src/hooks/reactHooks.ts.AI"),
            (vec!["React.createElement", "jsx", "Component"],
             "src/components/ReactComponent.tsx.AI"),

            // Process Management (discovered: 3026 process., 224 process.exit)
            (vec!["process.exit", "process.argv", "process.cwd", "process.env"],
             "src/process/processManager.ts.AI"),
            (vec!["child_process", "spawn", "exec", "fork"],
             "src/process/childProcess.ts.AI"),
            (vec!["process.stdin", "process.stdout", "process.stderr"],
             "src/process/stdio.ts.AI"),

            // CLI & Terminal (discovered: 90 process.argv, 36 commander)
            (vec!["process.argv", "commander", "yargs", "readline"],
             "src/cli/cliParser.ts.AI"),

            // Timing (discovered: 1000 Date.now, 660 setTimeout)
            (vec!["setTimeout", "clearTimeout", "setInterval", "clearInterval"],
             "src/utils/timers.ts.AI"),
            (vec!["Date.now", "performance.now", "requestAnimationFrame"],
             "src/utils/performance.ts.AI"),

            // String Operations (discovered: 2978 .join, 2498 .slice)
            (vec![".join(", ".slice(", ".replace(", ".split("],
             "src/utils/stringUtils.ts.AI"),
            (vec!["RegExp", ".test(", ".match(", ".trim("],
             "src/utils/regexUtils.ts.AI"),

            // Network & HTTP
            (vec!["http.createServer", "https.request", "listen"],
             "src/server/httpServer.ts.AI"),
            (vec!["socket.on", "socket.emit", "io.on"],
             "src/socket/socketHandler.ts.AI"),
            (vec!["fetch(", ".then(", ".catch("],
             "src/api/httpClient.ts.AI"),

            // Crypto & Security
            (vec!["crypto.createHash", "randomBytes", "encrypt", "decrypt"],
             "src/security/cryptoUtils.ts.AI"),

            // Streams & Buffers
            (vec!["stream.Readable", "pipe", "Transform"],
             "src/streams/streamHandler.ts.AI"),
            (vec!["Buffer.from", "Buffer.alloc"],
             "src/utils/bufferUtils.ts.AI"),

            // Date/Time Operations (discovered: weekStartsOn, getFullYear, setFullYear)
            (vec!["getFullYear", "setFullYear", "getMonth", "setMonth"],
             "src/utils/dateUtils.ts.AI"),
            (vec!["weekStartsOn", "getTime", "toISOString", "parseDate"],
             "src/utils/timeUtils.ts.AI"),

            // Image Processing (discovered: imagePasteIds, isLoading)
            (vec!["imagePasteIds", "image", "canvas", "drawImage"],
             "src/utils/imageHandler.ts.AI"),
            (vec!["isLoading", "loading", "spinner", "loadingState"],
             "src/components/LoadingState.tsx.AI"),

            // Bit Operations (discovered: getLowestSetBit, compareTo)
            (vec!["getLowestSetBit", "bitwise", "<<", ">>"],
             "src/utils/bitOperations.ts.AI"),

            // Object Utilities & Polyfills (discovered: prototype, hasOwnProperty, construct)
            (vec!["prototype", "hasOwnProperty", "Object.keys", "Object.values"],
             "src/utils/objectUtils.ts.AI"),
            (vec!["polyfill", "__proto__", "defineProperty"],
             "src/polyfills/objectPolyfill.ts.AI"),

            // Database
            (vec!["database", "db", "query", "connection", "pool"],
             "src/database/dbConnection.js.AI"),
            (vec!["mongoose", "schema", "model"],
             "src/models/schema.js.AI"),
            (vec!["sequelize", "findOne", "findAll"],
             "src/models/queries.js.AI"),

            // Validation & Middleware
            (vec!["validate", "validator", "schema", "yup", "joi"],
             "src/middleware/validation.js.AI"),
            (vec!["middleware", "next()", "req", "res"],
             "src/middleware/handler.js.AI"),

            // UI Components
            (vec!["Button", "onClick", "className", "props"],
             "src/components/Button.tsx.AI"),
            (vec!["Form", "input", "onSubmit", "formData"],
             "src/components/Form.tsx.AI"),
            (vec!["Modal", "dialog", "isOpen", "onClose"],
             "src/components/Modal.tsx.AI"),

            // State Management
            (vec!["useState", "useEffect", "useContext"],
             "src/hooks/useCustomHook.ts.AI"),
            (vec!["redux", "dispatch", "action", "reducer"],
             "src/store/reducer.js.AI"),
            (vec!["createSlice", "configureStore"],
             "src/store/slice.js.AI"),

            // Utils & Helpers
            (vec!["format", "parse", "convert", "transform"],
             "src/utils/formatter.js.AI"),
            (vec!["logger", "log", "console", "debug"],
             "src/utils/logger.js.AI"),
            (vec!["config", "env", "process.env"],
             "src/config/config.js.AI"),

            // Testing
            (vec!["test", "expect", "describe", "it("],
             "tests/spec.test.js.AI"),
            (vec!["mock", "jest", "spy"],
             "tests/mocks.js.AI"),

            // Error Handling
            (vec!["Error", "throw", "catch", "try"],
             "src/errors/errorHandler.js.AI"),

            // File Operations
            (vec!["fs", "readFile", "writeFile", "path"],
             "src/utils/fileHandler.js.AI"),

            // WebSocket & Real-time
            (vec!["socket", "io", "emit", "on("],
             "src/socket/socketHandler.js.AI"),
        ];

        for (keywords, path) in &patterns {
            let matches = keywords.iter()
                .filter(|k| code.to_lowercase().contains(&k.to_lowercase()))
                .count();

            // If 2+ keywords match, use this semantic path
            if matches >= 2 {
                return Some(path.to_string());
            }
        }

        None
    }

    /// Calculate similarity between two paths
    fn path_similarity(&self, path1: &str, path2: &str) -> f64 {
        let parts1: Vec<&str> = path1.split('/').collect();
        let parts2: Vec<&str> = path2.split('/').collect();

        let common_parts = parts1.iter()
            .zip(parts2.iter())
            .filter(|(a, b)| a == b)
            .count();

        let max_parts = parts1.len().max(parts2.len());
        if max_parts == 0 {
            return 0.0;
        }

        common_parts as f64 / max_parts as f64
    }

    /// Sanitize file paths
    fn sanitize_path(&self, path: &str) -> PathBuf {
        let cleaned = path
            .replace("bun://", "bun/")
            .replace("file://", "")
            .replace("node:", "node/")
            .replace("webpack://", "webpack/");

        PathBuf::from(cleaned)
    }

    /// Detect if code is from node_modules based on patterns
    fn is_node_module(&self, path: &str, code: &str) -> bool {
        // Check path first
        if path.contains("node_modules") {
            return true;
        }

        // Common npm package patterns in code
        let npm_patterns = [
            "module.exports =",
            "Object.defineProperty(exports",
            "__esModule",
            "use strict",
        ];

        let has_npm_pattern = npm_patterns.iter().filter(|p| code.contains(*p)).count() >= 2;

        // Check for common library names
        let common_libs = [
            "react", "lodash", "axios", "express", "moment",
            "webpack", "babel", "typescript", "esbuild", "rollup",
            "vue", "angular", "jquery", "underscore", "chalk",
        ];

        let has_lib_reference = common_libs.iter().any(|lib| {
            path.to_lowercase().contains(lib) || code.to_lowercase().contains(&format!("@{}", lib))
        });

        // Less strict: npm pattern OR lib reference OR node_modules in path
        has_npm_pattern || has_lib_reference
    }

    /// Extract package name from path or code
    fn extract_dependency_name(&self, path: &str, code: &str) -> Option<String> {
        // Try extracting from node_modules path
        if path.contains("node_modules/") {
            let parts: Vec<&str> = path.split("node_modules/").collect();
            if parts.len() > 1 {
                let pkg_path = parts[1];
                // Handle scoped packages (@org/package)
                if pkg_path.starts_with('@') {
                    let pkg_parts: Vec<&str> = pkg_path.splitn(3, '/').collect();
                    if pkg_parts.len() >= 2 {
                        return Some(format!("{}/{}", pkg_parts[0], pkg_parts[1]));
                    }
                } else {
                    // Regular package
                    return pkg_path.split('/').next().map(|s| s.to_string());
                }
            }
        }

        // Try detecting from common library patterns in code
        let common_libs = [
            ("react", vec!["React.createElement", "useState", "useEffect", "Component"]),
            ("lodash", vec!["lodash", "_.map", "_.forEach"]),
            ("axios", vec!["axios", ".get(", ".post("]),
            ("express", vec!["express", "app.get", "app.post"]),
            ("webpack", vec!["webpack", "__webpack_require__"]),
            ("esbuild", vec!["esbuild", "export{"]),
            ("typescript", vec!["typescript", "tsc"]),
            ("chalk", vec!["chalk", "ansi"]),
            ("commander", vec!["commander", ".command(", ".option("]),
        ];

        for (name, patterns) in &common_libs {
            let match_count = patterns.iter().filter(|p| code.contains(*p)).count();
            if match_count > 0 || path.to_lowercase().contains(name) {
                return Some(name.to_string());
            }
        }

        None
    }

    /// Generate package.json based on detected dependencies
    fn generate_package_json(
        &self,
        output_dir: &Path,
        dependencies: &HashSet<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut deps_json = String::from("  \"dependencies\": {\n");

        let mut sorted_deps: Vec<&String> = dependencies.iter().collect();
        sorted_deps.sort();

        for (i, dep) in sorted_deps.iter().enumerate() {
            let comma = if i < sorted_deps.len() - 1 { "," } else { "" };
            deps_json.push_str(&format!("    \"{}\": \"*\"{}\n", dep, comma));
        }
        deps_json.push_str("  }");

        let package_json = format!(
r#"{{
  "name": "recovered-project",
  "version": "1.0.0",
  "description": "Recovered from bundled JavaScript",
  "main": "src/index.js",
  "scripts": {{
    "test": "echo \"Error: no test specified\" && exit 1"
  }},
{},
  "keywords": [],
  "author": "",
  "license": "ISC"
}}
"#,
            deps_json
        );

        fs::write(output_dir.join("package.json"), package_json)?;
        Ok(())
    }

    /// Create README explaining the recovery
    fn create_readme(
        &self,
        output_dir: &Path,
        file_paths: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let deobfuscation_note = if self.deobfuscate {
            "- **Deobfuscation**: Applied (variables renamed using heuristics)"
        } else {
            "- **Deobfuscation**: Not applied (variable names remain minified)"
        };

        let limitations_note = if self.deobfuscate {
            "- Variable names were heuristically renamed (may not match originals)"
        } else {
            "- Variable names are minified (use --deobfuscate flag for readability)"
        };

        let readme_content = format!(
r#"# Recovered Project Structure

This directory was reconstructed from source map fragments and heuristic analysis.

## Recovery Summary

- **Total files**: {}
- **Recovery method**: Source map path extraction + code distribution heuristics
- **Completeness**: Partial (source maps were stripped)
{}

## Directory Structure

{}

## How It Works

1. **Path Extraction**: File paths were extracted from source map "sources" arrays
2. **Structure Creation**: Directory tree was recreated based on these paths
3. **Code Distribution**: Bundled code was distributed to files using:
   - Module name matching
   - Code pattern recognition (React, Node.js, etc.)
   - Path similarity scoring
   - Dependency detection (separates node_modules from app code)
   - **Semantic file naming**: AI analyzes code purpose and generates meaningful names
{}

## AI-Generated Filenames (.AI Suffix)

Files ending with `.AI` have been automatically named based on semantic code analysis.
These require **manual review** to:
- Verify the filename matches the actual code purpose
- Rename variables for better readability
- Adjust the filename if the AI guess was incorrect

**Example**: `src/auth/authentication.js.AI` → Review and rename to final name after validation

## Limitations

⚠️ **This is a best-effort reconstruction**:
- Original source code was NOT embedded in source maps
- Code distribution is based on heuristics and may be incorrect
- Some files may be empty or contain wrong code
{}

## Next Steps

1. Review and verify the recovered structure
2. Run deobfuscator on individual files
3. Manually reorganize misplaced code
4. Add missing imports and fix broken references

## Files by Category

{}
"#,
            file_paths.len(),
            deobfuscation_note,
            self.format_directory_tree(file_paths),
            if self.deobfuscate { "\n4. **Deobfuscation**: Variable names were renamed using pattern analysis" } else { "" },
            limitations_note,
            self.categorize_files(file_paths)
        );

        fs::write(output_dir.join("README.md"), readme_content)?;
        Ok(())
    }

    fn format_directory_tree(&self, paths: &[String]) -> String {
        let mut tree = String::new();
        let mut dirs: HashMap<String, Vec<String>> = HashMap::new();

        for path in paths {
            if let Some(dir) = Path::new(path).parent() {
                let dir_str = dir.to_string_lossy().to_string();
                dirs.entry(dir_str).or_insert_with(Vec::new).push(path.clone());
            }
        }

        for (dir, files) in dirs.iter().take(10) {
            tree.push_str(&format!("- {}/\n", dir));
            for file in files.iter().take(3) {
                let name = Path::new(file).file_name().unwrap().to_string_lossy();
                tree.push_str(&format!("  - {}\n", name));
            }
            if files.len() > 3 {
                tree.push_str(&format!("  ... and {} more\n", files.len() - 3));
            }
        }

        tree
    }

    fn categorize_files(&self, paths: &[String]) -> String {
        let mut categories: HashMap<&str, usize> = HashMap::new();

        for path in paths {
            if path.contains("/components/") {
                *categories.entry("Components").or_insert(0) += 1;
            } else if path.contains("/utils/") || path.contains("/lib/") {
                *categories.entry("Utilities").or_insert(0) += 1;
            } else if path.contains("/hooks/") {
                *categories.entry("Hooks").or_insert(0) += 1;
            } else if path.contains("/types/") {
                *categories.entry("Types").or_insert(0) += 1;
            } else if path.ends_with(".tsx") {
                *categories.entry("TypeScript React").or_insert(0) += 1;
            } else if path.ends_with(".ts") {
                *categories.entry("TypeScript").or_insert(0) += 1;
            } else if path.ends_with(".js") {
                *categories.entry("JavaScript").or_insert(0) += 1;
            }
        }

        let mut result = String::new();
        for (category, count) in categories {
            result.push_str(&format!("- **{}**: {} files\n", category, count));
        }

        result
    }
}

#[derive(Debug, Clone)]
struct CodeModule {
    name: String,
    inferred_path: String,
    code: String,
    module_type: ModuleType,
}

#[derive(Debug, Clone)]
enum ModuleType {
    CommonJS,
    ES6,
    ReactComponent,
}
