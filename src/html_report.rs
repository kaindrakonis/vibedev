use crate::comprehensive_analyzer::ComprehensiveAnalysis;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

pub struct HtmlReportGenerator {
    output_path: PathBuf,
}

impl HtmlReportGenerator {
    pub fn new(output_path: PathBuf) -> Self {
        Self { output_path }
    }

    pub fn generate(&self, insights: &ComprehensiveAnalysis) -> Result<()> {
        println!("🎨 Generating interactive HTML report with D3.js visualizations...");

        let html = self.generate_html(insights)?;
        fs::write(&self.output_path, html)?;

        println!("✅ Interactive HTML report generated!");
        println!("📁 Saved to: {}", self.output_path.display());
        println!("🌐 Open it in your browser to view!");

        Ok(())
    }

    fn generate_html(&self, insights: &ComprehensiveAnalysis) -> Result<String> {
        let insights_json = serde_json::to_string_pretty(insights)?;

        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>OpenSVM AI Coding Wrapped 2026</title>
    <script src="https://d3js.org/d3.v7.min.js"></script>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        body {{
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
            background: linear-gradient(135deg, #0f172a 0%, #1e293b 100%);
            color: #f1f5f9;
            line-height: 1.6;
            overflow-x: hidden;
        }}

        .container {{
            max-width: 1400px;
            margin: 0 auto;
            padding: 40px 20px;
        }}

        .header {{
            text-align: center;
            margin-bottom: 60px;
            padding: 60px 20px;
            background: linear-gradient(135deg, #8b5cf6 0%, #3b82f6 100%);
            border-radius: 20px;
            box-shadow: 0 20px 60px rgba(139, 92, 246, 0.3);
        }}

        .header h1 {{
            font-size: 4rem;
            font-weight: 800;
            margin-bottom: 10px;
            background: linear-gradient(135deg, #fff 0%, #e0e7ff 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
        }}

        .header .subtitle {{
            font-size: 1.5rem;
            color: #e0e7ff;
            font-weight: 300;
        }}

        .stats-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin-bottom: 60px;
        }}

        .stat-card {{
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid rgba(139, 92, 246, 0.2);
            border-radius: 15px;
            padding: 30px;
            backdrop-filter: blur(10px);
            transition: all 0.3s ease;
        }}

        .stat-card:hover {{
            transform: translateY(-5px);
            box-shadow: 0 10px 30px rgba(139, 92, 246, 0.3);
            border-color: rgba(139, 92, 246, 0.5);
        }}

        .stat-card .icon {{
            font-size: 2.5rem;
            margin-bottom: 10px;
        }}

        .stat-card .value {{
            font-size: 2.5rem;
            font-weight: 700;
            color: #a78bfa;
            margin-bottom: 5px;
        }}

        .stat-card .label {{
            font-size: 1rem;
            color: #cbd5e1;
            text-transform: uppercase;
            letter-spacing: 1px;
        }}

        .chart-container {{
            background: rgba(255, 255, 255, 0.03);
            border: 1px solid rgba(139, 92, 246, 0.15);
            border-radius: 20px;
            padding: 40px;
            margin-bottom: 40px;
            backdrop-filter: blur(10px);
        }}

        .chart-title {{
            font-size: 2rem;
            font-weight: 700;
            margin-bottom: 30px;
            color: #e0e7ff;
        }}

        .chart {{
            width: 100%;
            height: 500px;
        }}

        .tooltip {{
            position: absolute;
            background: rgba(15, 23, 42, 0.95);
            border: 1px solid #8b5cf6;
            border-radius: 8px;
            padding: 12px;
            font-size: 14px;
            pointer-events: none;
            opacity: 0;
            transition: opacity 0.2s;
            box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
        }}

        .legend {{
            display: flex;
            flex-wrap: wrap;
            gap: 20px;
            margin-top: 20px;
            justify-content: center;
        }}

        .legend-item {{
            display: flex;
            align-items: center;
            gap: 8px;
        }}

        .legend-color {{
            width: 20px;
            height: 20px;
            border-radius: 4px;
        }}

        .footer {{
            text-align: center;
            margin-top: 80px;
            padding: 40px;
            color: #64748b;
        }}

        .footer a {{
            color: #8b5cf6;
            text-decoration: none;
        }}

        .heatmap-cell {{
            stroke: #1e293b;
            stroke-width: 2;
        }}

        .bar {{
            transition: opacity 0.2s;
        }}

        .bar:hover {{
            opacity: 0.8;
        }}

        .achievement {{
            display: inline-block;
            background: linear-gradient(135deg, #10b981 0%, #059669 100%);
            color: white;
            padding: 8px 16px;
            border-radius: 20px;
            margin: 5px;
            font-size: 0.9rem;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 OpenSVM</h1>
            <div class="subtitle">AI Coding Wrapped 2026</div>
        </div>

        <div class="stats-grid" id="statsGrid"></div>

        <div class="chart-container">
            <div class="chart-title">📊 Programming Languages Distribution</div>
            <div id="languageChart" class="chart"></div>
        </div>

        <div class="chart-container">
            <div class="chart-title">🏷️ Top Topics Discussed</div>
            <div id="topicsChart" class="chart"></div>
        </div>

        <div class="chart-container">
            <div class="chart-title">🐛 Error Breakdown</div>
            <div id="errorChart" class="chart"></div>
        </div>

        <div class="chart-container">
            <div class="chart-title">⏰ Productivity by Hour</div>
            <div id="hourlyChart" class="chart"></div>
        </div>

        <div class="chart-container">
            <div class="chart-title">📅 Activity Heatmap</div>
            <div id="heatmapChart" class="chart"></div>
        </div>

        <div class="chart-container">
            <div class="chart-title">📁 Project Time Allocation</div>
            <div id="projectChart" class="chart"></div>
        </div>

        <div class="chart-container">
            <div class="chart-title">🛠️ Tool Comparison</div>
            <div id="toolChart" class="chart"></div>
        </div>

        <div class="footer">
            <p>Generated with <a href="https://osvm.ai" target="_blank">OpenSVM</a> AI Log Analyzer</p>
            <p>Analyzed 52.42 GB of data • {} conversations • {} hours coded</p>
        </div>
    </div>

    <div class="tooltip" id="tooltip"></div>

    <script>
        const data = {};

        // OpenSVM brand colors
        const colors = {{
            primary: '#8b5cf6',
            secondary: '#3b82f6',
            accent: '#10b981',
            error: '#ef4444',
            warning: '#f59e0b',
            gradient: ['#8b5cf6', '#3b82f6', '#10b981', '#f59e0b', '#ef4444']
        }};

        // Create stats cards
        function createStatsCards() {{
            const stats = [
                {{ icon: '💬', value: data.conversations.total_conversations.toLocaleString(), label: 'Conversations' }},
                {{ icon: '💭', value: data.conversations.total_messages.toLocaleString(), label: 'Messages' }},
                {{ icon: '🎯', value: Math.round(data.token_usage.total_tokens / 1000000) + 'M', label: 'Tokens' }},
                {{ icon: '💰', value: '$' + data.cost_analysis.total_cost_usd.toFixed(2), label: 'Total Cost' }},
                {{ icon: '⏱️', value: Math.round(data.work_hours.total_hours) + 'h', label: 'Hours Coded' }},
                {{ icon: '📁', value: data.advanced.project_allocation.total_projects, label: 'Projects' }},
                {{ icon: '💻', value: Object.keys(data.advanced.language_stats.by_language).length, label: 'Languages' }},
                {{ icon: '🐛', value: data.advanced.error_patterns.total_errors.toLocaleString(), label: 'Errors Fixed' }},
            ];

            const grid = d3.select('#statsGrid');
            stats.forEach(stat => {{
                const card = grid.append('div').attr('class', 'stat-card');
                card.append('div').attr('class', 'icon').text(stat.icon);
                card.append('div').attr('class', 'value').text(stat.value);
                card.append('div').attr('class', 'label').text(stat.label);
            }});
        }}

        // Language distribution chart
        function createLanguageChart() {{
            const languages = Object.entries(data.advanced.language_stats.by_language)
                .map(([name, metrics]) => ({{ name, mentions: metrics.mentions }}))
                .sort((a, b) => b.mentions - a.mentions)
                .slice(0, 10);

            const width = document.getElementById('languageChart').clientWidth;
            const height = 500;
            const margin = {{ top: 20, right: 30, bottom: 60, left: 100 }};

            const svg = d3.select('#languageChart')
                .append('svg')
                .attr('width', width)
                .attr('height', height);

            const x = d3.scaleLinear()
                .domain([0, d3.max(languages, d => d.mentions)])
                .range([margin.left, width - margin.right]);

            const y = d3.scaleBand()
                .domain(languages.map(d => d.name))
                .range([margin.top, height - margin.bottom])
                .padding(0.2);

            const colorScale = d3.scaleOrdinal()
                .domain(languages.map(d => d.name))
                .range(colors.gradient);

            svg.selectAll('rect')
                .data(languages)
                .join('rect')
                .attr('class', 'bar')
                .attr('x', margin.left)
                .attr('y', d => y(d.name))
                .attr('width', d => x(d.mentions) - margin.left)
                .attr('height', y.bandwidth())
                .attr('fill', d => colorScale(d.name))
                .attr('rx', 8)
                .on('mouseover', function(event, d) {{
                    showTooltip(event, `${{d.name}}: ${{d.mentions.toLocaleString()}} mentions`);
                }})
                .on('mouseout', hideTooltip);

            svg.append('g')
                .attr('transform', `translate(0,${{height - margin.bottom}})`)
                .call(d3.axisBottom(x).ticks(5))
                .attr('color', '#94a3b8');

            svg.append('g')
                .attr('transform', `translate(${{margin.left}},0)`)
                .call(d3.axisLeft(y))
                .attr('color', '#94a3b8');
        }}

        // Topics chart
        function createTopicsChart() {{
            const topics = data.advanced.topic_clusters.word_frequencies.slice(0, 10);

            const width = document.getElementById('topicsChart').clientWidth;
            const height = 500;
            const margin = {{ top: 20, right: 30, bottom: 60, left: 120 }};

            const svg = d3.select('#topicsChart')
                .append('svg')
                .attr('width', width)
                .attr('height', height);

            const x = d3.scaleLinear()
                .domain([0, d3.max(topics, d => d[1])])
                .range([margin.left, width - margin.right]);

            const y = d3.scaleBand()
                .domain(topics.map(d => d[0]))
                .range([margin.top, height - margin.bottom])
                .padding(0.2);

            svg.selectAll('rect')
                .data(topics)
                .join('rect')
                .attr('class', 'bar')
                .attr('x', margin.left)
                .attr('y', d => y(d[0]))
                .attr('width', d => x(d[1]) - margin.left)
                .attr('height', y.bandwidth())
                .attr('fill', colors.secondary)
                .attr('rx', 8)
                .on('mouseover', function(event, d) {{
                    showTooltip(event, `${{d[0]}}: ${{d[1].toLocaleString()}} mentions`);
                }})
                .on('mouseout', hideTooltip);

            svg.append('g')
                .attr('transform', `translate(0,${{height - margin.bottom}})`)
                .call(d3.axisBottom(x).ticks(5))
                .attr('color', '#94a3b8');

            svg.append('g')
                .attr('transform', `translate(${{margin.left}},0)`)
                .call(d3.axisLeft(y))
                .attr('color', '#94a3b8');
        }}

        // Error chart (pie chart)
        function createErrorChart() {{
            const errors = Object.entries(data.advanced.error_patterns.by_category)
                .map(([name, count]) => ({{ name, count }}))
                .sort((a, b) => b.count - a.count);

            const width = document.getElementById('errorChart').clientWidth;
            const height = 500;
            const radius = Math.min(width, height) / 2 - 40;

            const svg = d3.select('#errorChart')
                .append('svg')
                .attr('width', width)
                .attr('height', height)
                .append('g')
                .attr('transform', `translate(${{width / 2}},${{height / 2}})`);

            const color = d3.scaleOrdinal()
                .domain(errors.map(d => d.name))
                .range(colors.gradient);

            const pie = d3.pie()
                .value(d => d.count)
                .sort(null);

            const arc = d3.arc()
                .innerRadius(radius * 0.5)
                .outerRadius(radius);

            svg.selectAll('path')
                .data(pie(errors))
                .join('path')
                .attr('d', arc)
                .attr('fill', d => color(d.data.name))
                .attr('stroke', '#1e293b')
                .attr('stroke-width', 2)
                .on('mouseover', function(event, d) {{
                    const pct = ((d.data.count / d3.sum(errors, d => d.count)) * 100).toFixed(1);
                    showTooltip(event, `${{d.data.name}}: ${{d.data.count.toLocaleString()}} (${{pct}}%)`);
                }})
                .on('mouseout', hideTooltip);

            // Legend
            const legend = d3.select('#errorChart')
                .append('div')
                .attr('class', 'legend');

            errors.forEach(error => {{
                const item = legend.append('div').attr('class', 'legend-item');
                item.append('div')
                    .attr('class', 'legend-color')
                    .style('background', color(error.name));
                item.append('span').text(error.name);
            }});
        }}

        // Hourly productivity chart
        function createHourlyChart() {{
            const hourlyData = Array.from({{ length: 24 }}, (_, i) => ({{
                hour: i,
                hours: data.work_hours.hours_by_hour_of_day[i] || 0
            }}));

            const width = document.getElementById('hourlyChart').clientWidth;
            const height = 500;
            const margin = {{ top: 20, right: 30, bottom: 60, left: 60 }};

            const svg = d3.select('#hourlyChart')
                .append('svg')
                .attr('width', width)
                .attr('height', height);

            const x = d3.scaleBand()
                .domain(hourlyData.map(d => d.hour))
                .range([margin.left, width - margin.right])
                .padding(0.1);

            const y = d3.scaleLinear()
                .domain([0, d3.max(hourlyData, d => d.hours)])
                .range([height - margin.bottom, margin.top]);

            svg.selectAll('rect')
                .data(hourlyData)
                .join('rect')
                .attr('class', 'bar')
                .attr('x', d => x(d.hour))
                .attr('y', d => y(d.hours))
                .attr('width', x.bandwidth())
                .attr('height', d => y(0) - y(d.hours))
                .attr('fill', d => {{
                    if (d.hour >= 22 || d.hour <= 5) return colors.accent;
                    if (d.hour >= 9 && d.hour <= 17) return colors.primary;
                    return colors.secondary;
                }})
                .attr('rx', 4)
                .on('mouseover', function(event, d) {{
                    showTooltip(event, `${{d.hour}}:00 - ${{d.hours.toFixed(1)}}h coded`);
                }})
                .on('mouseout', hideTooltip);

            svg.append('g')
                .attr('transform', `translate(0,${{height - margin.bottom}})`)
                .call(d3.axisBottom(x))
                .attr('color', '#94a3b8');

            svg.append('g')
                .attr('transform', `translate(${{margin.left}},0)`)
                .call(d3.axisLeft(y))
                .attr('color', '#94a3b8');
        }}

        // Activity heatmap
        function createHeatmap() {{
            const calendar = data.advanced.activity_heatmap.contribution_calendar;

            const width = document.getElementById('heatmapChart').clientWidth;
            const cellSize = 15;
            const weeksToShow = Math.min(Math.ceil(calendar.length / 7), Math.floor(width / cellSize));
            const height = cellSize * 8;

            const svg = d3.select('#heatmapChart')
                .append('svg')
                .attr('width', width)
                .attr('height', height);

            const colorScale = d3.scaleLinear()
                .domain([0, 1, 2, 3, 4])
                .range(['#1e293b', '#475569', colors.accent, colors.secondary, colors.primary]);

            const recentData = calendar.slice(-weeksToShow * 7);

            svg.selectAll('rect')
                .data(recentData)
                .join('rect')
                .attr('class', 'heatmap-cell')
                .attr('x', (d, i) => (Math.floor(i / 7)) * cellSize)
                .attr('y', (d, i) => (i % 7) * cellSize)
                .attr('width', cellSize - 2)
                .attr('height', cellSize - 2)
                .attr('rx', 2)
                .attr('fill', d => colorScale(d.intensity))
                .on('mouseover', function(event, d) {{
                    showTooltip(event, `${{d.date}}: ${{d.hours.toFixed(1)}}h`);
                }})
                .on('mouseout', hideTooltip);
        }}

        // Project allocation
        function createProjectChart() {{
            const projects = Object.entries(data.advanced.project_allocation.by_project)
                .map(([name, metrics]) => ({{ name, hours: metrics.hours }}))
                .sort((a, b) => b.hours - a.hours)
                .slice(0, 10);

            const width = document.getElementById('projectChart').clientWidth;
            const height = 500;
            const margin = {{ top: 20, right: 30, bottom: 60, left: 150 }};

            const svg = d3.select('#projectChart')
                .append('svg')
                .attr('width', width)
                .attr('height', height);

            const x = d3.scaleLinear()
                .domain([0, d3.max(projects, d => d.hours)])
                .range([margin.left, width - margin.right]);

            const y = d3.scaleBand()
                .domain(projects.map(d => d.name))
                .range([margin.top, height - margin.bottom])
                .padding(0.2);

            svg.selectAll('rect')
                .data(projects)
                .join('rect')
                .attr('class', 'bar')
                .attr('x', margin.left)
                .attr('y', d => y(d.name))
                .attr('width', d => x(d.hours) - margin.left)
                .attr('height', y.bandwidth())
                .attr('fill', colors.accent)
                .attr('rx', 8)
                .on('mouseover', function(event, d) {{
                    showTooltip(event, `${{d.name}}: ${{d.hours.toFixed(1)}}h`);
                }})
                .on('mouseout', hideTooltip);

            svg.append('g')
                .attr('transform', `translate(0,${{height - margin.bottom}})`)
                .call(d3.axisBottom(x).ticks(5))
                .attr('color', '#94a3b8');

            svg.append('g')
                .attr('transform', `translate(${{margin.left}},0)`)
                .call(d3.axisLeft(y))
                .attr('color', '#94a3b8');
        }}

        // Tool comparison
        function createToolChart() {{
            const tools = Object.entries(data.work_hours.hours_by_tool)
                .map(([name, hours]) => ({{ name, hours }}))
                .sort((a, b) => b.hours - a.hours);

            const width = document.getElementById('toolChart').clientWidth;
            const height = 500;
            const margin = {{ top: 20, right: 30, bottom: 100, left: 60 }};

            const svg = d3.select('#toolChart')
                .append('svg')
                .attr('width', width)
                .attr('height', height);

            const x = d3.scaleBand()
                .domain(tools.map(d => d.name))
                .range([margin.left, width - margin.right])
                .padding(0.2);

            const y = d3.scaleLinear()
                .domain([0, d3.max(tools, d => d.hours)])
                .range([height - margin.bottom, margin.top]);

            const colorScale = d3.scaleOrdinal()
                .domain(tools.map(d => d.name))
                .range(colors.gradient);

            svg.selectAll('rect')
                .data(tools)
                .join('rect')
                .attr('class', 'bar')
                .attr('x', d => x(d.name))
                .attr('y', d => y(d.hours))
                .attr('width', x.bandwidth())
                .attr('height', d => y(0) - y(d.hours))
                .attr('fill', d => colorScale(d.name))
                .attr('rx', 8)
                .on('mouseover', function(event, d) {{
                    showTooltip(event, `${{d.name}}: ${{d.hours.toFixed(1)}}h`);
                }})
                .on('mouseout', hideTooltip);

            svg.append('g')
                .attr('transform', `translate(0,${{height - margin.bottom}})`)
                .call(d3.axisBottom(x))
                .selectAll('text')
                .attr('transform', 'rotate(-45)')
                .style('text-anchor', 'end')
                .attr('color', '#94a3b8');

            svg.append('g')
                .attr('transform', `translate(${{margin.left}},0)`)
                .call(d3.axisLeft(y))
                .attr('color', '#94a3b8');
        }}

        // Tooltip functions
        function showTooltip(event, text) {{
            const tooltip = d3.select('#tooltip');
            tooltip.style('opacity', 1)
                .html(text)
                .style('left', (event.pageX + 10) + 'px')
                .style('top', (event.pageY - 10) + 'px');
        }}

        function hideTooltip() {{
            d3.select('#tooltip').style('opacity', 0);
        }}

        // Initialize all charts
        createStatsCards();
        createLanguageChart();
        createTopicsChart();
        createErrorChart();
        createHourlyChart();
        createHeatmap();
        createProjectChart();
        createToolChart();
    </script>
</body>
</html>"#,
            insights.conversations.total_conversations,
            insights.work_hours.total_hours.round() as u64,
            insights_json
        );

        Ok(html)
    }
}
