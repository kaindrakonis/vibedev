use anyhow::Result;
use plotters::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use crate::comprehensive_analyzer::ComprehensiveAnalysis;

// OpenSVM Brand Colors
const OPENSVM_PRIMARY: RGBColor = RGBColor(139, 92, 246); // Purple
const OPENSVM_SECONDARY: RGBColor = RGBColor(59, 130, 246); // Blue
const OPENSVM_ACCENT: RGBColor = RGBColor(16, 185, 129); // Green
const OPENSVM_DARK: RGBColor = RGBColor(17, 24, 39); // Dark bg
const OPENSVM_LIGHT: RGBColor = RGBColor(249, 250, 251); // Light bg
const OPENSVM_TEXT: RGBColor = RGBColor(243, 244, 246); // Text

pub struct InfographicGenerator {
    output_dir: PathBuf,
}

impl InfographicGenerator {
    pub fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }

    pub fn generate_all(&self, insights: &ComprehensiveAnalysis) -> Result<Vec<PathBuf>> {
        let mut generated = Vec::new();

        println!("🎨 Generating OpenSVM-branded infographics...\n");

        // 1. Language Distribution
        println!("📊 [1/8] Creating language distribution chart...");
        generated.push(self.generate_language_chart(insights)?);

        // 2. Activity Heatmap
        println!("📅 [2/8] Creating activity heatmap...");
        generated.push(self.generate_activity_heatmap(insights)?);

        // 3. Topic Bar Chart
        println!("🏷️  [3/8] Creating topic frequency chart...");
        generated.push(self.generate_topic_chart(insights)?);

        // 4. Error Breakdown
        println!("🐛 [4/8] Creating error breakdown chart...");
        generated.push(self.generate_error_chart(insights)?);

        // 5. Hourly Productivity
        println!("⏰ [5/8] Creating hourly productivity chart...");
        generated.push(self.generate_hourly_chart(insights)?);

        // 6. Project Allocation
        println!("📁 [6/8] Creating project allocation chart...");
        generated.push(self.generate_project_chart(insights)?);

        // 7. Stats Card
        println!("🏆 [7/8] Creating ultimate stats card...");
        generated.push(self.generate_stats_card(insights)?);

        // 8. Tool Comparison
        println!("🛠️  [8/8] Creating tool comparison chart...");
        generated.push(self.generate_tool_chart(insights)?);

        println!("\n✅ All infographics generated!");

        Ok(generated)
    }

    fn generate_language_chart(&self, insights: &ComprehensiveAnalysis) -> Result<PathBuf> {
        let path = self.output_dir.join("01_language_distribution.png");
        let root = BitMapBackend::new(&path.clone(), (1200, 800)).into_drawing_area();

        root.fill(&OPENSVM_DARK)?;

        let title_style = ("sans-serif", 40, &OPENSVM_TEXT).into_text_style(&root);
        root.draw_text(
            "Programming Languages Used",
            &title_style,
            (600, 40),
        )?;

        // Get language data
        let langs = &insights.advanced.language_stats.by_language;
        let mut lang_data: Vec<_> = langs.iter()
            .map(|(name, metrics)| (name.clone(), metrics.mentions as f64))
            .collect();
        lang_data.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        lang_data.truncate(10); // Top 10

        let total: f64 = lang_data.iter().map(|(_, v)| v).sum();

        // Create bar chart
        let mut chart = ChartBuilder::on(&root)
            .margin(20)
            .x_label_area_size(60)
            .y_label_area_size(80)
            .build_cartesian_2d(0f64..total * 1.1, 0..lang_data.len())?;

        chart
            .configure_mesh()
            .disable_x_mesh()
            .y_desc("Language")
            .x_desc("Mentions")
            .label_style(("sans-serif", 20, &OPENSVM_TEXT))
            .draw()?;

        for (idx, (lang, count)) in lang_data.iter().enumerate() {
            let color = match idx {
                0 => &OPENSVM_PRIMARY,
                1 => &OPENSVM_SECONDARY,
                2 => &OPENSVM_ACCENT,
                _ => &RGBColor(100, 100, 100),
            };

            chart.draw_series(std::iter::once(Rectangle::new(
                [(0.0, idx), (*count, idx + 1)],
                color.filled(),
            )))?;

            // Add label
            root.draw_text(
                lang,
                &("sans-serif", 18, &OPENSVM_TEXT).into_text_style(&root),
                (90, 120 + idx * 60),
            )?;
        }

        root.present()?;
        Ok(path)
    }

    fn generate_activity_heatmap(&self, insights: &ComprehensiveAnalysis) -> Result<PathBuf> {
        let path = self.output_dir.join("02_activity_heatmap.png");
        let root = BitMapBackend::new(&path.clone(), (1400, 400)).into_drawing_area();

        root.fill(&OPENSVM_DARK)?;

        let title_style = ("sans-serif", 36, &OPENSVM_TEXT).into_text_style(&root);
        root.draw_text(
            "Activity Heatmap (GitHub-style)",
            &title_style,
            (700, 30),
        )?;

        // Get calendar data
        let calendar = &insights.advanced.activity_heatmap.contribution_calendar;

        // Draw heatmap cells (simplified version)
        let start_x = 50;
        let start_y = 80;
        let cell_size = 12;
        let cell_gap = 2;

        for (idx, day) in calendar.iter().enumerate() {
            let week = idx / 7;
            let day_of_week = idx % 7;

            let x = start_x + week * (cell_size + cell_gap);
            let y = start_y + day_of_week * (cell_size + cell_gap);

            let color = match day.intensity {
                4 => OPENSVM_PRIMARY,
                3 => OPENSVM_SECONDARY,
                2 => OPENSVM_ACCENT,
                1 => RGBColor(50, 50, 50),
                _ => RGBColor(20, 20, 20),
            };

            root.draw(&Rectangle::new(
                [(x, y), (x + cell_size, y + cell_size)],
                color.filled(),
            ))?;
        }

        // Add legend
        let legend_y = 300;
        let legend_text = vec!["Less", "", "", "", "More"];
        for (idx, text) in legend_text.iter().enumerate() {
            let color = match idx {
                4 => OPENSVM_PRIMARY,
                3 => OPENSVM_SECONDARY,
                2 => OPENSVM_ACCENT,
                1 => RGBColor(50, 50, 50),
                _ => RGBColor(20, 20, 20),
            };

            let x = 50 + idx * 30;
            root.draw(&Rectangle::new(
                [(x, legend_y), (x + 12, legend_y + 12)],
                color.filled(),
            ))?;

            if !text.is_empty() {
                root.draw_text(
                    text,
                    &("sans-serif", 12, &OPENSVM_TEXT).into_text_style(&root),
                    (x + 15, legend_y),
                )?;
            }
        }

        root.present()?;
        Ok(path)
    }

    fn generate_topic_chart(&self, insights: &ComprehensiveAnalysis) -> Result<PathBuf> {
        let path = self.output_dir.join("03_top_topics.png");
        let root = BitMapBackend::new(&path.clone(), (1200, 800)).into_drawing_area();

        root.fill(&OPENSVM_DARK)?;

        let title_style = ("sans-serif", 40, &OPENSVM_TEXT).into_text_style(&root);
        root.draw_text(
            "Top 10 Most Discussed Topics",
            &title_style,
            (600, 40),
        )?;

        // Get top topics
        let topics = &insights.advanced.topic_clusters.word_frequencies;
        let top_10: Vec<_> = topics.iter().take(10).cloned().collect();

        let max_count = top_10.first().map(|(_, c)| *c).unwrap_or(1) as f64;

        let mut chart = ChartBuilder::on(&root)
            .margin(20)
            .x_label_area_size(60)
            .y_label_area_size(100)
            .build_cartesian_2d(0f64..max_count * 1.1, 0..top_10.len())?;

        chart
            .configure_mesh()
            .disable_x_mesh()
            .y_desc("Topic")
            .x_desc("Mentions")
            .label_style(("sans-serif", 20, &OPENSVM_TEXT))
            .draw()?;

        for (idx, (topic, count)) in top_10.iter().enumerate() {
            let color = match idx {
                0 => &OPENSVM_PRIMARY,
                1..=2 => &OPENSVM_SECONDARY,
                3..=5 => &OPENSVM_ACCENT,
                _ => &RGBColor(100, 100, 100),
            };

            chart.draw_series(std::iter::once(Rectangle::new(
                [(0.0, idx), (*count as f64, idx + 1)],
                color.filled(),
            )))?;

            // Add label with count
            root.draw_text(
                &format!("{} ({})", topic, count),
                &("sans-serif", 18, &OPENSVM_TEXT).into_text_style(&root),
                (110, 120 + idx * 60),
            )?;
        }

        root.present()?;
        Ok(path)
    }

    fn generate_error_chart(&self, insights: &ComprehensiveAnalysis) -> Result<PathBuf> {
        let path = self.output_dir.join("04_error_breakdown.png");
        let root = BitMapBackend::new(&path.clone(), (1200, 800)).into_drawing_area();

        root.fill(&OPENSVM_DARK)?;

        let title_style = ("sans-serif", 40, &OPENSVM_TEXT).into_text_style(&root);
        root.draw_text(
            "Error Breakdown (110,129 Total)",
            &title_style,
            (600, 40),
        )?;

        // Get error categories
        let errors = &insights.advanced.error_patterns.by_category;
        let mut error_data: Vec<_> = errors.iter()
            .map(|(name, count)| (name.clone(), *count as f64))
            .collect();
        error_data.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let total: f64 = error_data.iter().map(|(_, v)| v).sum();

        // Create pie chart (simplified as bars for now)
        let mut chart = ChartBuilder::on(&root)
            .margin(20)
            .x_label_area_size(60)
            .y_label_area_size(120)
            .build_cartesian_2d(0f64..total * 1.1, 0..error_data.len())?;

        chart
            .configure_mesh()
            .disable_x_mesh()
            .y_desc("Error Type")
            .x_desc("Count")
            .label_style(("sans-serif", 20, &OPENSVM_TEXT))
            .draw()?;

        for (idx, (error_type, count)) in error_data.iter().enumerate() {
            let percentage = (count / total * 100.0) as usize;

            chart.draw_series(std::iter::once(Rectangle::new(
                [(0.0, idx), (*count, idx + 1)],
                &OPENSVM_SECONDARY.mix(0.8),
            )))?;

            root.draw_text(
                &format!("{} - {}% ({:.0})", error_type, percentage, count),
                &("sans-serif", 16, &OPENSVM_TEXT).into_text_style(&root),
                (130, 120 + idx * 80),
            )?;
        }

        root.present()?;
        Ok(path)
    }

    fn generate_hourly_chart(&self, insights: &ComprehensiveAnalysis) -> Result<PathBuf> {
        let path = self.output_dir.join("05_hourly_productivity.png");
        let root = BitMapBackend::new(&path.clone(), (1400, 600)).into_drawing_area();

        root.fill(&OPENSVM_DARK)?;

        let title_style = ("sans-serif", 36, &OPENSVM_TEXT).into_text_style(&root);
        root.draw_text(
            "Productivity by Hour of Day",
            &title_style,
            (700, 30),
        )?;

        let hours_data: Vec<(usize, f64)> = (0..24)
            .map(|h| {
                let hours = insights.work_hours.hours_by_hour_of_day.get(&h).unwrap_or(&0.0);
                (h, *hours)
            })
            .collect();

        let max_hours = hours_data.iter().map(|(_, h)| h).fold(0.0f64, |a, b| a.max(*b));

        let mut chart = ChartBuilder::on(&root)
            .margin(40)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(0usize..23, 0f64..(max_hours * 1.2))?;

        chart
            .configure_mesh()
            .x_desc("Hour of Day")
            .y_desc("Hours Coded")
            .label_style(("sans-serif", 18, &OPENSVM_TEXT))
            .draw()?;

        chart.draw_series(
            hours_data.iter().map(|(hour, hours)| {
                let color = if *hour >= 22 || *hour <= 5 {
                    &OPENSVM_ACCENT // Late night
                } else if *hour >= 9 && *hour <= 17 {
                    &OPENSVM_PRIMARY // Work hours
                } else {
                    &OPENSVM_SECONDARY // Other
                };

                Rectangle::new([(*hour, 0.0), (*hour + 1, *hours)], color.filled())
            }),
        )?;

        root.present()?;
        Ok(path)
    }

    fn generate_project_chart(&self, insights: &ComprehensiveAnalysis) -> Result<PathBuf> {
        let path = self.output_dir.join("06_project_allocation.png");
        let root = BitMapBackend::new(&path.clone(), (1200, 800)).into_drawing_area();

        root.fill(&OPENSVM_DARK)?;

        let title_style = ("sans-serif", 40, &OPENSVM_TEXT).into_text_style(&root);
        root.draw_text(
            "Top 10 Projects by Time",
            &title_style,
            (600, 40),
        )?;

        // Get top projects
        let projects = &insights.advanced.project_allocation.by_project;
        let mut project_data: Vec<_> = projects.iter()
            .map(|(name, metrics)| (name.clone(), metrics.hours))
            .collect();
        project_data.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        project_data.truncate(10);

        let max_hours = project_data.first().map(|(_, h)| *h).unwrap_or(1.0);

        let mut chart = ChartBuilder::on(&root)
            .margin(20)
            .x_label_area_size(60)
            .y_label_area_size(120)
            .build_cartesian_2d(0f64..(max_hours * 1.1), 0..project_data.len())?;

        chart
            .configure_mesh()
            .disable_x_mesh()
            .y_desc("Project")
            .x_desc("Hours")
            .label_style(("sans-serif", 20, &OPENSVM_TEXT))
            .draw()?;

        for (idx, (project, hours)) in project_data.iter().enumerate() {
            chart.draw_series(std::iter::once(Rectangle::new(
                [(0.0, idx), (*hours, idx + 1)],
                &OPENSVM_ACCENT.mix(0.8),
            )))?;

            root.draw_text(
                &format!("{} ({:.1}h)", project, hours),
                &("sans-serif", 16, &OPENSVM_TEXT).into_text_style(&root),
                (130, 120 + idx * 60),
            )?;
        }

        root.present()?;
        Ok(path)
    }

    fn generate_stats_card(&self, insights: &ComprehensiveAnalysis) -> Result<PathBuf> {
        let path = self.output_dir.join("07_ultimate_stats_card.png");
        let root = BitMapBackend::new(&path.clone(), (1200, 1600)).into_drawing_area();

        root.fill(&OPENSVM_DARK)?;

        // OpenSVM branding
        let brand_style = ("sans-serif", 50).bold().into_text_style(&root).color(&OPENSVM_PRIMARY);
        root.draw_text("OpenSVM", &brand_style, (600, 50))?;

        let subtitle_style = ("sans-serif", 28, &OPENSVM_TEXT).into_text_style(&root);
        root.draw_text(
            "AI Coding Wrapped 2026",
            &subtitle_style,
            (600, 110),
        )?;

        // Stats
        let stats = vec![
            ("💬 Conversations", format!("{}", insights.conversations.total_conversations)),
            ("💬 Messages", format!("{:,}", insights.conversations.total_messages)),
            ("🎯 Tokens", format!("{}M", insights.token_usage.total_tokens / 1_000_000)),
            ("💰 Cost", format!("${:.2}", insights.cost_analysis.total_cost_usd)),
            ("⏱️  Hours Coded", format!("{:.0}h", insights.work_hours.total_hours)),
            ("📁 Projects", format!("{}", insights.advanced.project_allocation.total_projects)),
            ("💻 Languages", format!("{}", insights.advanced.language_stats.by_language.len())),
            ("🐛 Errors Fixed", format!("{:,}", insights.advanced.error_patterns.total_errors)),
            ("✅ Tasks Done", format!("{:,}", insights.advanced.task_completion.completed_tasks)),
            ("🔄 Context Switches", format!("{}", insights.advanced.project_allocation.context_switches)),
            ("😤 Frustrations", format!("{}", insights.viral_insights.behavior_patterns.frustration_count)),
            ("🏃 Longest Session", format!("{:.1}h", insights.work_hours.longest_session_hours)),
        ];

        let mut y = 200;
        for (label, value) in stats {
            let label_style = ("sans-serif", 24, &OPENSVM_TEXT).into_text_style(&root);
            let value_style = ("sans-serif", 32).bold().into_text_style(&root).color(&OPENSVM_ACCENT);

            root.draw_text(label, &label_style, (200, y))?;
            root.draw_text(&value, &value_style, (800, y))?;

            y += 100;
        }

        // Footer
        let footer_style = ("sans-serif", 20, &RGBColor(150, 150, 150)).into_text_style(&root);
        root.draw_text(
            "Generated with ai-log-analyzer • OpenSVM Analytics",
            &footer_style,
            (600, 1550),
        )?;

        root.present()?;
        Ok(path)
    }

    fn generate_tool_chart(&self, insights: &ComprehensiveAnalysis) -> Result<PathBuf> {
        let path = self.output_dir.join("08_tool_comparison.png");
        let root = BitMapBackend::new(&path.clone(), (1200, 600)).into_drawing_area();

        root.fill(&OPENSVM_DARK)?;

        let title_style = ("sans-serif", 40, &OPENSVM_TEXT).into_text_style(&root);
        root.draw_text(
            "Tool Usage by Hours",
            &title_style,
            (600, 40),
        )?;

        // Get tool hours
        let tools_data: Vec<(String, f64)> = insights.work_hours.hours_by_tool.iter()
            .map(|(name, hours)| (name.clone(), *hours))
            .collect();

        let max_hours = tools_data.iter().map(|(_, h)| h).fold(0.0f64, |a, b| a.max(*b));

        let mut chart = ChartBuilder::on(&root)
            .margin(40)
            .x_label_area_size(100)
            .y_label_area_size(60)
            .build_cartesian_2d(0..tools_data.len(), 0f64..(max_hours * 1.2))?;

        chart
            .configure_mesh()
            .x_desc("Tool")
            .y_desc("Hours")
            .label_style(("sans-serif", 18, &OPENSVM_TEXT))
            .draw()?;

        for (idx, (_tool, hours)) in tools_data.iter().enumerate() {
            let color = match idx {
                0 => &OPENSVM_PRIMARY,
                1 => &OPENSVM_SECONDARY,
                2 => &OPENSVM_ACCENT,
                _ => &RGBColor(100, 100, 100),
            };

            chart.draw_series(std::iter::once(Rectangle::new(
                [(idx, 0.0), (idx + 1, *hours)],
                color.filled(),
            )))?;
        }

        root.present()?;
        Ok(path)
    }
}
