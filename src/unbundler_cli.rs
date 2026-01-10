/// JavaScript Unbundler - Extract and analyze bundled JS from binaries
use std::fs::{File, self};
use std::io::Write;
use std::path::PathBuf;
use regex::Regex;
use clap::{Parser, Subcommand};

mod sourcemap_unbundler;
use sourcemap_unbundler::SourceMapExtractor;

mod deobfuscator;
use deobfuscator::JSDeobfuscator;

mod behavior_analyzer;

mod structure_recovery;
use structure_recovery::StructureRecovery;

#[derive(Parser)]
#[command(name = "unbundle")]
#[command(about = "Extract and analyze bundled JavaScript from binaries")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract JavaScript strings from a binary
    Extract {
        /// Path to binary file
        #[arg(short, long)]
        binary: PathBuf,

        /// Output directory
        #[arg(short, long, default_value = "./unbundled")]
        output: PathBuf,

        /// Minimum string length to extract
        #[arg(short, long, default_value = "50")]
        min_length: usize,
    },

    /// Analyze bundle format (esbuild/webpack/rollup)
    Analyze {
        /// Path to binary file
        #[arg(short, long)]
        binary: PathBuf,
    },

    /// Beautify extracted minified code
    Beautify {
        /// Path to minified JS file
        #[arg(short, long)]
        input: PathBuf,

        /// Output file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Extract and analyze source maps
    SourceMaps {
        /// Path to binary file
        #[arg(short, long)]
        binary: PathBuf,

        /// Generate report only (don't extract files)
        #[arg(short, long)]
        report_only: bool,

        /// Output directory for reconstructed files
        #[arg(short, long, default_value = "./reconstructed")]
        output: PathBuf,
    },

    /// Deobfuscate minified JavaScript using heuristics
    Deobfuscate {
        /// Path to minified JS file
        #[arg(short, long)]
        input: PathBuf,

        /// Output file for deobfuscated code
        #[arg(short, long)]
        output: PathBuf,

        /// Show rename report
        #[arg(short, long)]
        report: bool,
    },

    /// Recover project structure from source map fragments
    RecoverStructure {
        /// Path to binary file
        #[arg(short, long)]
        binary: PathBuf,

        /// Path to extracted code file
        #[arg(short, long)]
        extracted_code: PathBuf,

        /// Output directory for recovered structure
        #[arg(short, long, default_value = "./recovered_project")]
        output: PathBuf,

        /// Apply heuristic deobfuscation to recovered files
        #[arg(short, long)]
        deobfuscate: bool,
    },
}

struct JsExtractor {
    binary_path: PathBuf,
    min_length: usize,
}

impl JsExtractor {
    fn new(binary_path: PathBuf, min_length: usize) -> Self {
        Self { binary_path, min_length }
    }

    /// Extract all strings from binary using `strings` command
    fn extract_strings(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let output = std::process::Command::new("strings")
            .arg("-n")
            .arg(self.min_length.to_string())
            .arg(&self.binary_path)
            .output()?;

        let content = String::from_utf8_lossy(&output.stdout);
        Ok(content.lines().map(|s| s.to_string()).collect())
    }

    /// Detect if a string is JavaScript code
    fn is_javascript(&self, s: &str) -> bool {
        // JavaScript patterns
        let patterns = [
            "function ",
            "const ",
            "let ",
            "var ",
            "class ",
            "export ",
            "import ",
            "=>",
            ".prototype.",
            "module.exports",
            "require(",
        ];

        patterns.iter().any(|&p| s.contains(p))
    }

    /// Extract JavaScript code from strings
    fn extract_javascript(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let strings = self.extract_strings()?;

        Ok(strings.into_iter()
            .filter(|s| self.is_javascript(s))
            .collect())
    }

    /// Detect bundler type from code patterns
    fn detect_bundler(&self, code: &[String]) -> String {
        let full_code = code.join("\n");

        if full_code.contains("__webpack_require__") || full_code.contains("webpackJsonp") {
            "webpack".to_string()
        } else if full_code.contains("__rollup__") || full_code.contains("createCommonjsModule") {
            "rollup".to_string()
        } else if full_code.contains("export{") && full_code.contains("var ") {
            "esbuild".to_string()
        } else if full_code.contains("define.amd") {
            "requirejs".to_string()
        } else if full_code.contains("System.register") {
            "systemjs".to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// Save extracted code to files
    fn save_to_files(&self, code: Vec<String>, output_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(output_dir)?;

        // Save all JavaScript code
        let mut all_js = File::create(output_dir.join("all_extracted.js"))?;
        for line in &code {
            writeln!(all_js, "{}", line)?;
        }

        // Save individual functions/modules
        for (idx, line) in code.iter().enumerate() {
            if line.len() > 1000 {  // Large chunks likely to be modules
                let mut file = File::create(output_dir.join(format!("module_{}.js", idx)))?;
                writeln!(file, "{}", line)?;
            }
        }

        println!("[+] Extracted {} JavaScript strings", code.len());
        println!("[+] Saved to: {}", output_dir.display());

        Ok(())
    }
}

struct BundleAnalyzer {
    binary_path: PathBuf,
}

impl BundleAnalyzer {
    fn new(binary_path: PathBuf) -> Self {
        Self { binary_path }
    }

    fn analyze(&self) -> Result<(), Box<dyn std::error::Error>> {
        let extractor = JsExtractor::new(self.binary_path.clone(), 50);
        let js_code = extractor.extract_javascript()?;

        println!("=== Bundle Analysis ===\n");
        println!("Binary: {}", self.binary_path.display());
        println!("JavaScript strings found: {}", js_code.len());

        let bundler = extractor.detect_bundler(&js_code);
        println!("Detected bundler: {}", bundler);

        // Count patterns
        let full_code = js_code.join("\n");
        let function_re = Regex::new(r"\bfunction\s+\w+").unwrap();
        let class_re = Regex::new(r"\bclass\s+\w+").unwrap();
        let export_re = Regex::new(r"\bexport\s+").unwrap();
        let import_re = Regex::new(r"\bimport\s+").unwrap();

        let function_count = function_re.find_iter(&full_code).count();
        let class_count = class_re.find_iter(&full_code).count();
        let export_count = export_re.find_iter(&full_code).count();
        let import_count = import_re.find_iter(&full_code).count();

        println!("\nCode statistics:");
        println!("  Functions: {}", function_count);
        println!("  Classes: {}", class_count);
        println!("  Exports: {}", export_count);
        println!("  Imports: {}", import_count);

        // Find source file references
        let file_re = Regex::new(r"[\w-]+\.(js|ts|jsx|tsx)").unwrap();
        let source_files: Vec<&str> = file_re
            .find_iter(&full_code)
            .map(|m| m.as_str())
            .collect();

        if !source_files.is_empty() {
            println!("\nSource files referenced: {}", source_files.len());
            for file in source_files.iter().take(10) {
                println!("  - {}", file);
            }
            if source_files.len() > 10 {
                let remaining = source_files.len() - 10;
                println!("  ... and {} more", remaining);
            }
        }

        Ok(())
    }
}

struct CodeBeautifier;

impl CodeBeautifier {
    /// Basic JavaScript beautifier (splits minified code into readable format)
    fn beautify(input: &str) -> String {
        let mut result = String::new();
        let mut indent = 0;
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];

            match c {
                '{' => {
                    result.push(c);
                    result.push('\n');
                    indent += 2;
                    result.push_str(&" ".repeat(indent));
                }
                '}' => {
                    indent = indent.saturating_sub(2);
                    result.push('\n');
                    result.push_str(&" ".repeat(indent));
                    result.push(c);
                }
                ';' => {
                    result.push(c);
                    if i + 1 < chars.len() && chars[i + 1] != '}' {
                        result.push('\n');
                        result.push_str(&" ".repeat(indent));
                    }
                }
                _ => result.push(c),
            }

            i += 1;
        }

        result
    }

    fn beautify_file(input: PathBuf, output: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let content = fs::read_to_string(&input)?;
        let beautified = Self::beautify(&content);

        let mut file = File::create(&output)?;
        write!(file, "{}", beautified)?;

        println!("[+] Beautified {} -> {}", input.display(), output.display());

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Extract { binary, output, min_length } => {
            println!("Extracting JavaScript from: {}", binary.display());
            let extractor = JsExtractor::new(binary, min_length);
            let js_code = extractor.extract_javascript()?;
            extractor.save_to_files(js_code, &output)?;
        }

        Commands::Analyze { binary } => {
            let analyzer = BundleAnalyzer::new(binary);
            analyzer.analyze()?;
        }

        Commands::Beautify { input, output } => {
            CodeBeautifier::beautify_file(input, output)?;
        }

        Commands::SourceMaps { binary, report_only, output } => {
            let extractor = SourceMapExtractor::new(binary);

            if report_only {
                extractor.generate_report()?;
            } else {
                println!("Extracting source maps and reconstructing files...");
                extractor.generate_report()?;
                extractor.reconstruct_files(&output)?;
            }
        }

        Commands::Deobfuscate { input, output, report } => {
            println!("Deobfuscating: {}", input.display());
            let mut deobfuscator = JSDeobfuscator::from_file(&input)?;
            let deobfuscated = deobfuscator.deobfuscate();

            if report {
                deobfuscator.print_report();
            }

            fs::write(&output, deobfuscated)?;
            println!("[+] Saved deobfuscated code to: {}", output.display());
        }

        Commands::RecoverStructure { binary, extracted_code, output, deobfuscate } => {
            println!("Recovering project structure from source maps...");
            if deobfuscate {
                println!("[*] Deobfuscation enabled - variables will be renamed using heuristics");
            }
            let code = fs::read_to_string(&extracted_code)?;
            let recovery = StructureRecovery::new(binary, code)
                .with_deobfuscation(deobfuscate);
            recovery.recover_structure(&output)?;
        }
    }

    Ok(())
}
