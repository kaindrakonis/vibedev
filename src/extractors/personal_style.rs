use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::extraction_utils::Conversation;

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeStyleExample {
    pub id: String,
    pub instruction: String,
    pub code_generated: String,
    pub language: String,
    pub style_features: StyleFeatures,
    pub conversation_context: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StyleFeatures {
    pub naming_convention: String,
    pub comment_style: String,
    pub error_handling_pattern: String,
    pub test_preference: String,
    pub architecture_pattern: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StylePattern {
    pub category: String,
    pub pattern: String,
    pub frequency: usize,
    pub examples: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersonalStyleDataset {
    pub total_examples: usize,
    pub examples: Vec<CodeStyleExample>,
    pub style_patterns: Vec<StylePattern>,
    pub preferences: HashMap<String, String>,
    pub by_language: HashMap<String, usize>,
}

pub struct PersonalStyleExtractor;

impl PersonalStyleExtractor {
    pub fn extract(conversations: &[Conversation]) -> Result<PersonalStyleDataset> {
        println!(
            "💻 Extracting personal coding style from {} conversations...",
            conversations.len()
        );

        let mut examples = Vec::new();

        for (conv_idx, conv) in conversations.iter().enumerate() {
            // Look for assistant messages with code
            for (msg_idx, msg) in conv.messages.iter().enumerate() {
                if msg.role == "assistant"
                    && (msg.content.contains("```")
                        || msg.content.contains("fn ")
                        || msg.content.contains("function "))
                {
                    if let Some(example) = Self::extract_code_example(msg, conv, conv_idx, msg_idx)
                    {
                        examples.push(example);
                    }
                }
            }
        }

        println!("   Extracted {} code style examples", examples.len());

        let style_patterns = Self::identify_patterns(&examples);
        let preferences = Self::extract_preferences(&examples);
        let by_language = Self::group_by_language(&examples);

        Ok(PersonalStyleDataset {
            total_examples: examples.len(),
            examples,
            style_patterns,
            preferences,
            by_language,
        })
    }

    fn extract_code_example(
        msg: &crate::extraction_utils::Message,
        conv: &Conversation,
        conv_idx: usize,
        msg_idx: usize,
    ) -> Option<CodeStyleExample> {
        let content = &msg.content;

        // Extract code from markdown code blocks
        let code = if content.contains("```") {
            Self::extract_code_block(content)?
        } else {
            content.chars().take(500).collect()
        };

        let language = Self::detect_language(&code);

        // Get the user's request from previous message
        let instruction = conv
            .messages
            .get(msg_idx.saturating_sub(1))
            .filter(|m| m.role == "user")
            .map(|m| m.content.chars().take(200).collect())
            .unwrap_or_else(|| "Generate code".to_string());

        Some(CodeStyleExample {
            id: format!("style_{}_{}", conv_idx, msg_idx),
            instruction,
            code_generated: code.clone(),
            language: language.clone(),
            style_features: Self::analyze_style(&code, &language),
            conversation_context: conv
                .messages
                .iter()
                .take(msg_idx)
                .map(|m| m.content.chars().take(100).collect())
                .collect(),
        })
    }

    fn extract_code_block(content: &str) -> Option<String> {
        // Extract code from ```language\n code \n```
        if let Some(start) = content.find("```") {
            if let Some(code_start) = content[start + 3..].find('\n') {
                if let Some(end) = content[start + 3 + code_start..].find("```") {
                    return Some(
                        content[start + 3 + code_start..start + 3 + code_start + end]
                            .trim()
                            .to_string(),
                    );
                }
            }
        }
        None
    }

    fn detect_language(code: &str) -> String {
        if code.contains("fn ") && code.contains("->") {
            "rust".to_string()
        } else if code.contains("function ") || code.contains("const ") || code.contains("=>") {
            "typescript".to_string()
        } else if code.contains("def ") || code.contains("import ") {
            "python".to_string()
        } else {
            "unknown".to_string()
        }
    }

    fn analyze_style(code: &str, language: &str) -> StyleFeatures {
        // Language-specific naming convention detection
        let naming_convention = match language {
            "rust" | "python" | "ruby" => {
                // These languages conventionally use snake_case
                if code.contains("_") {
                    "snake_case"
                } else {
                    "mixed"
                }
            }
            "javascript" | "typescript" | "java" | "c#" => {
                // These languages conventionally use camelCase
                if code.contains("_") && !code.contains("CONST_") {
                    "mixed"
                } else {
                    "camelCase"
                }
            }
            "go" => {
                // Go uses mixedCaps for exported and unexported names
                "mixedCaps"
            }
            _ => {
                if code.contains("_") {
                    "snake_case"
                } else {
                    "camelCase"
                }
            }
        }
        .to_string();

        // Language-specific comment style detection
        let comment_style = match language {
            "rust" => {
                if code.contains("///") || code.contains("//!") {
                    "doc_comments"
                } else if code.contains("//") {
                    "inline"
                } else {
                    "minimal"
                }
            }
            "python" => {
                if code.contains("\"\"\"") || code.contains("'''") {
                    "docstrings"
                } else if code.contains("#") {
                    "inline"
                } else {
                    "minimal"
                }
            }
            "javascript" | "typescript" => {
                if code.contains("/**") {
                    "jsdoc"
                } else if code.contains("//") {
                    "inline"
                } else {
                    "minimal"
                }
            }
            _ => {
                if code.contains("//") {
                    "inline"
                } else {
                    "minimal"
                }
            }
        }
        .to_string();

        // Language-specific error handling detection
        let error_handling_pattern = match language {
            "rust" => {
                if code.contains("Result") || code.contains("?") {
                    "result_type"
                } else if code.contains("unwrap") {
                    "panic_on_error"
                } else {
                    "basic"
                }
            }
            "go" => {
                if code.contains("if err != nil") {
                    "explicit_error_check"
                } else {
                    "basic"
                }
            }
            _ => {
                if code.contains("try") || code.contains("catch") {
                    "try_catch"
                } else {
                    "basic"
                }
            }
        }
        .to_string();

        // Language-specific test detection
        let test_preference = match language {
            "rust" => {
                if code.contains("#[test]") || code.contains("#[cfg(test)]") {
                    "test_driven"
                } else {
                    "manual"
                }
            }
            "javascript" | "typescript" => {
                if code.contains("describe(") || code.contains("it(") || code.contains("test(") {
                    "test_driven"
                } else {
                    "manual"
                }
            }
            "python" => {
                if code.contains("def test_")
                    || code.contains("unittest")
                    || code.contains("pytest")
                {
                    "test_driven"
                } else {
                    "manual"
                }
            }
            _ => {
                if code.contains("test") {
                    "test_driven"
                } else {
                    "manual"
                }
            }
        }
        .to_string();

        StyleFeatures {
            naming_convention,
            comment_style,
            error_handling_pattern,
            test_preference,
            architecture_pattern: "modular".to_string(),
        }
    }

    fn identify_patterns(examples: &[CodeStyleExample]) -> Vec<StylePattern> {
        let mut patterns = Vec::new();

        // Naming convention pattern
        let snake_case_count = examples
            .iter()
            .filter(|e| e.style_features.naming_convention == "snake_case")
            .count();

        if snake_case_count > 0 {
            patterns.push(StylePattern {
                category: "naming".to_string(),
                pattern: "snake_case".to_string(),
                frequency: snake_case_count,
                examples: examples
                    .iter()
                    .filter(|e| e.style_features.naming_convention == "snake_case")
                    .take(3)
                    .map(|e| e.code_generated.chars().take(100).collect())
                    .collect(),
            });
        }

        patterns
    }

    fn extract_preferences(examples: &[CodeStyleExample]) -> HashMap<String, String> {
        let mut prefs = HashMap::new();

        // Determine most common error handling
        let error_handling: HashMap<String, usize> =
            examples.iter().fold(HashMap::new(), |mut acc, e| {
                *acc.entry(e.style_features.error_handling_pattern.clone())
                    .or_insert(0) += 1;
                acc
            });

        if let Some((pattern, _)) = error_handling.iter().max_by_key(|(_, &count)| count) {
            prefs.insert("error_handling".to_string(), pattern.clone());
        }

        prefs.insert("style".to_string(), "consistent".to_string());

        prefs
    }

    fn group_by_language(examples: &[CodeStyleExample]) -> HashMap<String, usize> {
        let mut map = HashMap::new();
        for example in examples {
            *map.entry(example.language.clone()).or_insert(0) += 1;
        }
        map
    }

    pub fn save_to_file(dataset: &PersonalStyleDataset, output_path: &Path) -> Result<()> {
        // Save as JSONL for fine-tuning
        let mut lines = Vec::new();
        for example in &dataset.examples {
            // Format as instruction-response pairs for fine-tuning
            let training_example = serde_json::json!({
                "instruction": example.instruction,
                "input": "",
                "output": example.code_generated,
                "language": example.language,
                "style": example.style_features,
            });
            lines.push(serde_json::to_string(&training_example)?);
        }
        fs::write(output_path.join("personal_style.jsonl"), lines.join("\n"))?;

        // Save for HuggingFace (instruction format)
        fs::write(
            output_path.join("../huggingface/personal_style.jsonl"),
            lines.join("\n"),
        )?;

        // Save style guide
        let style_guide = serde_json::json!({
            "total_examples": dataset.total_examples,
            "preferences": dataset.preferences,
            "patterns": dataset.style_patterns,
            "by_language": dataset.by_language,
        });

        fs::write(
            output_path.join("personal_style_guide.json"),
            serde_json::to_string_pretty(&style_guide)?,
        )?;

        println!(
            "✅ Saved {} code style examples to {}",
            dataset.examples.len(),
            output_path.display()
        );
        println!("   Languages covered: {:?}", dataset.by_language.keys());
        println!(
            "   Style patterns identified: {}",
            dataset.style_patterns.len()
        );

        Ok(())
    }
}
