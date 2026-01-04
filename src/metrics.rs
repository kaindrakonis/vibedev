use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use tokio::time;

pub async fn live_stats(interval_secs: u64) -> Result<()> {
    use colored::*;

    println!("{}", "📊 Real-time AI Tool Statistics".bold().cyan());
    println!();

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );

    let mut interval = time::interval(Duration::from_secs(interval_secs));

    loop {
        interval.tick().await;

        // Collect current stats
        let base_dir = dirs::home_dir().expect("Could not determine home directory");
        let discovery = crate::discovery::LogDiscovery::new(base_dir, true);

        match discovery.scan() {
            Ok(findings) => {
                pb.set_message(format!(
                    "Storage: {} | Files: {} | Tools: {}",
                    crate::models::format_bytes(findings.total_size_bytes).green(),
                    findings.total_files.to_string().cyan(),
                    findings.tools_found.len().to_string().yellow()
                ));
            }
            Err(e) => {
                pb.set_message(format!("Error: {}", e).red().to_string());
            }
        }

        pb.tick();
    }
}
