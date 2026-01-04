#![allow(dead_code)]

use anyhow::Result;
use regex::Regex;

lazy_static::lazy_static! {
    // API Keys and Tokens
    static ref API_KEY_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"sk-[a-zA-Z0-9]{20,}").unwrap(),                    // OpenAI/Anthropic keys
        Regex::new(r"sk-ant-[a-zA-Z0-9-]{20,}").unwrap(),               // Anthropic keys
        Regex::new(r"ghp_[a-zA-Z0-9]{36}").unwrap(),                    // GitHub tokens
        Regex::new(r"gho_[a-zA-Z0-9]{36}").unwrap(),                    // GitHub OAuth
        Regex::new(r"github_pat_[a-zA-Z0-9]{22}_[a-zA-Z0-9]{59}").unwrap(), // GitHub PAT
        Regex::new(r"glpat-[a-zA-Z0-9-]{20}").unwrap(),                // GitLab tokens
        Regex::new(r"xox[baprs]-[a-zA-Z0-9-]+").unwrap(),               // Slack tokens
        Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),                       // AWS access keys
        Regex::new(r"ya29\.[a-zA-Z0-9_-]+").unwrap(),                   // Google OAuth
        Regex::new(r"AIza[0-9A-Za-z_-]{35}").unwrap(),                  // Google API keys
        Regex::new(r"[0-9]+-[0-9A-Za-z_]{32}\.apps\.googleusercontent\.com").unwrap(), // Google OAuth client
        Regex::new(r"Bearer [a-zA-Z0-9_.=-]+").unwrap(),              // Bearer tokens
        Regex::new(r#"token[:=]\s*["']?[a-zA-Z0-9_.-]{20,}"#).unwrap(), // Generic tokens
    ];

    // Passwords
    static ref PASSWORD_PATTERNS: Vec<Regex> = vec![
        Regex::new(r#"password[:=]\s*["']([^"']+)["']"#).unwrap(),
        Regex::new(r#"passwd[:=]\s*["']([^"']+)["']"#).unwrap(),
        Regex::new(r#"pwd[:=]\s*["']([^"']+)["']"#).unwrap(),
        Regex::new(r#"pass[:=]\s*["']([^"']+)["']"#).unwrap(),
    ];

    // Personal Identifiable Information
    static ref EMAIL_REGEX: Regex = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();
    static ref PHONE_REGEX: Regex = Regex::new(r"\b(\+?1?[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap();
    static ref SSN_REGEX: Regex = Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap();
    static ref CREDIT_CARD_REGEX: Regex = Regex::new(r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b").unwrap();
    static ref IP_REGEX: Regex = Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap();

    // File paths
    static ref HOME_PATH_REGEX: Regex = Regex::new(r"/home/[^/\s]+").unwrap();
    static ref USERS_PATH_REGEX: Regex = Regex::new(r"/Users/[^/\s]+").unwrap();
    static ref WINDOWS_PATH_REGEX: Regex = Regex::new(r"C:\\Users\\[^\\]+").unwrap();

    // URLs with auth
    static ref URL_WITH_AUTH: Regex = Regex::new(r"https?://[^:]+:[^@]+@").unwrap();

    // Environment variables
    static ref ENV_VAR_PATTERN: Regex = Regex::new(r"(?m)^[A-Z_][A-Z0-9_]*=.+$").unwrap();
}

pub struct Sanitizer {
    redact_emails: bool,
    redact_paths: bool,
    redact_ips: bool,
    custom_patterns: Vec<Regex>,
}

impl Sanitizer {
    pub fn new() -> Self {
        Self {
            redact_emails: true,
            redact_paths: true,
            redact_ips: true,
            custom_patterns: Vec::new(),
        }
    }

    pub fn with_custom_pattern(mut self, pattern: &str) -> Result<Self> {
        self.custom_patterns.push(Regex::new(pattern)?);
        Ok(self)
    }

    pub fn sanitize_text(&self, text: &str) -> String {
        let mut sanitized = text.to_string();

        // Remove API keys and tokens
        for pattern in API_KEY_PATTERNS.iter() {
            sanitized = pattern
                .replace_all(&sanitized, "[REDACTED_API_KEY]")
                .to_string();
        }

        // Remove passwords
        for pattern in PASSWORD_PATTERNS.iter() {
            sanitized = pattern
                .replace_all(&sanitized, "password=\"[REDACTED]\"")
                .to_string();
        }

        // Remove credit cards
        sanitized = CREDIT_CARD_REGEX
            .replace_all(&sanitized, "[REDACTED_CC]")
            .to_string();

        // Remove SSNs
        sanitized = SSN_REGEX
            .replace_all(&sanitized, "[REDACTED_SSN]")
            .to_string();

        // Remove phone numbers
        sanitized = PHONE_REGEX
            .replace_all(&sanitized, "[REDACTED_PHONE]")
            .to_string();

        // Remove emails
        if self.redact_emails {
            sanitized = EMAIL_REGEX
                .replace_all(&sanitized, "[REDACTED_EMAIL]")
                .to_string();
        }

        // Remove IPs
        if self.redact_ips {
            sanitized = IP_REGEX
                .replace_all(&sanitized, "[REDACTED_IP]")
                .to_string();
        }

        // Remove URLs with authentication
        sanitized = URL_WITH_AUTH
            .replace_all(&sanitized, "https://[REDACTED_AUTH]@")
            .to_string();

        // Anonymize file paths
        if self.redact_paths {
            sanitized = HOME_PATH_REGEX
                .replace_all(&sanitized, "/home/[USER]")
                .to_string();
            sanitized = USERS_PATH_REGEX
                .replace_all(&sanitized, "/Users/[USER]")
                .to_string();
            sanitized = WINDOWS_PATH_REGEX
                .replace_all(&sanitized, "C:\\Users\\[USER]")
                .to_string();
        }

        // Remove environment variables that might contain secrets
        sanitized = ENV_VAR_PATTERN
            .replace_all(&sanitized, "[REDACTED_ENV_VAR]")
            .to_string();

        // Apply custom patterns
        for pattern in &self.custom_patterns {
            sanitized = pattern.replace_all(&sanitized, "[REDACTED]").to_string();
        }

        sanitized
    }

    pub fn detect_sensitive_data(&self, text: &str) -> Vec<SensitiveDataMatch> {
        let mut matches = Vec::new();

        // Check for API keys
        for pattern in API_KEY_PATTERNS.iter() {
            for mat in pattern.find_iter(text) {
                matches.push(SensitiveDataMatch {
                    data_type: SensitiveDataType::ApiKey,
                    position: mat.start(),
                    length: mat.len(),
                });
            }
        }

        // Check for passwords
        for pattern in PASSWORD_PATTERNS.iter() {
            for mat in pattern.find_iter(text) {
                matches.push(SensitiveDataMatch {
                    data_type: SensitiveDataType::Password,
                    position: mat.start(),
                    length: mat.len(),
                });
            }
        }

        // Check for emails
        for mat in EMAIL_REGEX.find_iter(text) {
            matches.push(SensitiveDataMatch {
                data_type: SensitiveDataType::Email,
                position: mat.start(),
                length: mat.len(),
            });
        }

        // Check for credit cards
        for mat in CREDIT_CARD_REGEX.find_iter(text) {
            matches.push(SensitiveDataMatch {
                data_type: SensitiveDataType::CreditCard,
                position: mat.start(),
                length: mat.len(),
            });
        }

        matches
    }

    pub fn is_safe_for_training(&self, text: &str) -> bool {
        self.detect_sensitive_data(text).is_empty()
    }
}

impl Default for Sanitizer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SensitiveDataMatch {
    pub data_type: SensitiveDataType,
    pub position: usize,
    pub length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SensitiveDataType {
    ApiKey,
    Password,
    Email,
    Phone,
    Ssn,
    CreditCard,
    IpAddress,
    FilePath,
    EnvironmentVariable,
}

impl std::fmt::Display for SensitiveDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensitiveDataType::ApiKey => write!(f, "API Key"),
            SensitiveDataType::Password => write!(f, "Password"),
            SensitiveDataType::Email => write!(f, "Email"),
            SensitiveDataType::Phone => write!(f, "Phone Number"),
            SensitiveDataType::Ssn => write!(f, "SSN"),
            SensitiveDataType::CreditCard => write!(f, "Credit Card"),
            SensitiveDataType::IpAddress => write!(f, "IP Address"),
            SensitiveDataType::FilePath => write!(f, "File Path"),
            SensitiveDataType::EnvironmentVariable => write!(f, "Environment Variable"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_redaction() {
        let sanitizer = Sanitizer::new();
        let text = "My API key is sk-1234567890abcdefghij";
        let result = sanitizer.sanitize_text(text);
        assert!(result.contains("[REDACTED_API_KEY]"));
        assert!(!result.contains("sk-1234567890abcdefghij"));
    }

    #[test]
    fn test_email_redaction() {
        let sanitizer = Sanitizer::new();
        let text = "Contact me at user@example.com";
        let result = sanitizer.sanitize_text(text);
        assert!(result.contains("[REDACTED_EMAIL]"));
        assert!(!result.contains("user@example.com"));
    }

    #[test]
    fn test_path_redaction() {
        let sanitizer = Sanitizer::new();
        let text = "File at /home/john/secret.txt";
        let result = sanitizer.sanitize_text(text);
        assert!(result.contains("/home/[USER]"));
        assert!(!result.contains("/home/john"));
    }
}
