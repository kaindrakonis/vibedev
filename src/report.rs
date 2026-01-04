use crate::models::*;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

pub struct ReportGenerator {
    format: ReportFormat,
}

enum ReportFormat {
    Markdown,
    Json,
    Html,
    Text,
}

impl ReportGenerator {
    pub fn new(format: &str) -> Self {
        let format = match format.to_lowercase().as_str() {
            "json" => ReportFormat::Json,
            "html" => ReportFormat::Html,
            "text" => ReportFormat::Text,
            _ => ReportFormat::Markdown,
        };

        Self { format }
    }

    pub fn generate(&self, results: &AnalysisResults) -> Result<String> {
        match self.format {
            ReportFormat::Markdown => self.generate_markdown(results),
            ReportFormat::Json => self.generate_json(results),
            ReportFormat::Html => self.generate_html(results),
            ReportFormat::Text => self.generate_text(results),
        }
    }

    fn generate_markdown(&self, results: &AnalysisResults) -> Result<String> {
        let mut md = String::new();

        md.push_str("# AI Coding Tools - Comprehensive Analysis Report\n\n");
        md.push_str(&format!(
            "**Generated:** {}\n\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        md.push_str("---\n\n");
        md.push_str("## 📊 Global Metrics\n\n");
        md.push_str(&format!(
            "- **Total Storage:** {}\n",
            format_bytes(results.global_metrics.total_storage)
        ));
        md.push_str(&format!(
            "- **Total Sessions:** {}\n",
            results.global_metrics.total_sessions
        ));
        md.push_str(&format!(
            "- **Total Prompts:** {}\n",
            results.global_metrics.total_prompts
        ));
        md.push_str(&format!(
            "- **Estimated Tokens:** {}\n",
            results.global_metrics.estimated_tokens
        ));
        md.push_str(&format!(
            "- **Peak Usage Hour:** {}:00 UTC\n",
            results.global_metrics.peak_usage_hour
        ));
        md.push_str(&format!(
            "- **Most Used Tool:** {}\n",
            results.global_metrics.most_used_tool
        ));

        if let Some(ref cost) = results.cost_estimate {
            md.push_str("\n## 💰 Cost Estimate\n\n");
            md.push_str(&format!(
                "- **Monthly Cost:** ${:.2}\n",
                cost.monthly_cost_usd
            ));
            md.push_str(&format!(
                "- **Optimization Potential:** ${:.2}\n",
                cost.optimization_potential
            ));
        }

        md.push_str("\n## 🔧 Per-Tool Analysis\n\n");
        for (name, analysis) in &results.tools {
            md.push_str(&format!("### {}\n\n", name));
            md.push_str(&format!("- Size: {}\n", format_bytes(analysis.total_size)));
            md.push_str(&format!("- Sessions: {}\n", analysis.session_count));
            md.push_str(&format!("- Prompts: {}\n", analysis.prompt_count));
            md.push_str(&format!(
                "- Avg Session Length: {:.1}\n",
                analysis.avg_session_length
            ));
            md.push('\n');
        }

        md.push_str("## 💡 Recommendations\n\n");
        for rec in &results.recommendations {
            md.push_str(&format!(
                "### {:?} - {} - {}\n\n",
                rec.priority, rec.category, rec.title
            ));
            md.push_str(&format!("{}\n\n", rec.description));
            md.push_str(&format!("**Action:** {}\n\n", rec.action));
            if let Some(savings) = rec.estimated_savings {
                md.push_str(&format!(
                    "**Estimated Savings:** {}\n\n",
                    format_bytes(savings)
                ));
            }
        }

        Ok(md)
    }

    fn generate_json(&self, results: &AnalysisResults) -> Result<String> {
        Ok(serde_json::to_string_pretty(results)?)
    }

    fn generate_html(&self, _results: &AnalysisResults) -> Result<String> {
        // HTML generation would go here
        Ok("<html><body>HTML report not yet implemented</body></html>".to_string())
    }

    fn generate_text(&self, _results: &AnalysisResults) -> Result<String> {
        // Plain text generation
        Ok("Text report not yet implemented".to_string())
    }

    pub fn write_to_file(&self, content: &str, path: &PathBuf) -> Result<()> {
        fs::write(path, content)?;
        Ok(())
    }
}
