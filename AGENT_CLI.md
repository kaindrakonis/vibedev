# Agent-Friendly CLI Usage

## Overview

The CLI now supports machine-readable output modes that make it easy for AI agents and automation tools to consume results.

## Global Flags

### `--json`
Output all results in machine-readable JSON format with structured progress updates.

```bash
vibedev --json backup --include-claude
```

Output format:
```json
{"stage":"info","message":"Creating backup...","current":null,"total":null,"percentage":null,"status":"started"}
{"stage":"scan","message":"Processing 1/100","current":1,"total":100,"percentage":1.0,"status":"running"}
...
{"stage":"complete","message":"Backup created","current":100,"total":100,"percentage":100.0,"status":"completed"}
```

### `--plain`
Disable colors and emojis for piping to files or other tools.

```bash
vibedev --plain timeline --cluster | grep "Completed"
```

Output:
```
[OK] Found 127 git repositories
[OK] Analyzed 2,453 sessions
  Completed: 1,234 | Abandoned: 567 | Resumed: 432 | Ongoing: 220
```

### Auto-Detection
When output is piped or redirected, plain mode is automatically enabled:

```bash
vibedev backup > backup.log  # Automatically uses plain mode
```

## Structured Output Examples

### JSON Mode - Timeline Command

```bash
vibedev --json timeline --months 3
```

```json
{
  "success": true,
  "command": "timeline",
  "duration_ms": 1523,
  "output": {
    "total_sessions": 2453,
    "stats": {
      "completed": 1234,
      "abandoned": 567,
      "resumed": 432,
      "ongoing": 220,
      "completion_rate": 50.3,
      "avg_session_hours": 2.4,
      "most_worked_project": "vibecheck",
      "context_switches": 89
    },
    "sessions": [...]
  },
  "errors": [],
  "warnings": ["High abandonment rate detected"]
}
```

### JSON Mode - AI Impact Analysis

```bash
vibedev --json backup --analyze-impact --output ~/backups
```

Streaming progress updates:
```json
{"stage":"backup","message":"Scanning AI logs","current":null,"total":null,"percentage":null,"status":"started"}
{"stage":"backup","message":"Found 58 tools","current":null,"total":null,"percentage":null,"status":"running"}
{"stage":"analysis","message":"Loading Claude conversations","current":null,"total":null,"percentage":null,"status":"running"}
{"stage":"analysis","message":"Analyzing 1234 commits","current":1234,"total":null,"percentage":null,"status":"running"}
{"stage":"complete","message":"Analysis saved","current":null,"total":null,"percentage":null,"status":"completed"}
```

Final result:
```json
{
  "success": true,
  "command": "backup",
  "duration_ms": 45231,
  "output": {
    "ai_impact_analysis": {
      "ai_assisted_commits": 842,
      "solo_commits": 392,
      "ai_assistance_rate": 68.3,
      "velocity_improvement": 42.5,
      "lines_written_with_ai": 125430,
      "most_ai_assisted_language": "Rust",
      "pair_programming_sessions": 156,
      "copy_paste_incidents": 23
    },
    "backup_files": [
      "ai-logs-backup-20260105-080000.tar.gz",
      "claude-logs-20260105-080000.tar.gz",
      "git-logs-20260105-080000.tar.gz",
      "ai-impact-analysis-20260105-080000.json"
    ]
  },
  "errors": [],
  "warnings": []
}
```

## Agent Integration Examples

### Python Agent

```python
import subprocess
import json

# Run command in JSON mode
result = subprocess.run(
    ["vibedev", "--json", "backup", "--analyze-impact"],
    capture_output=True,
    text=True
)

# Parse progress updates (streaming)
for line in result.stdout.splitlines():
    update = json.loads(line)
    if update["status"] == "running":
        print(f"Progress: {update['message']}")
    elif update["status"] == "completed":
        print(f"✓ {update['stage']} completed")

# Get final result
final_result = json.loads(result.stdout.splitlines()[-1])
if final_result["success"]:
    print(f"✓ Backup completed in {final_result['duration_ms']}ms")
    print(f"AI assisted {final_result['output']['ai_impact_analysis']['ai_assistance_rate']}% of commits")
```

### Shell Script

```bash
#!/bin/bash

# Run analysis and capture JSON output
OUTPUT=$(vibedev --json timeline --months 6)

# Extract specific metrics
COMPLETED=$(echo "$OUTPUT" | jq -r '.output.stats.completed')
ABANDONED=$(echo "$OUTPUT" | jq -r '.output.stats.abandoned')
COMPLETION_RATE=$(echo "$OUTPUT" | jq -r '.output.stats.completion_rate')

echo "Completion Rate: $COMPLETION_RATE%"

# Alert if completion rate is low
if (( $(echo "$COMPLETION_RATE < 50" | bc -l) )); then
    echo "⚠ Warning: Low completion rate!"
fi
```

### JavaScript/Node.js

```javascript
const { execSync } = require('child_process');

// Run command
const output = execSync('vibedev --json backup --analyze-impact', {
    encoding: 'utf-8'
});

// Parse streaming updates
const lines = output.trim().split('\n');
const progressUpdates = lines.slice(0, -1).map(line => JSON.parse(line));
const finalResult = JSON.parse(lines[lines.length - 1]);

// Process results
if (finalResult.success) {
    const analysis = finalResult.output.ai_impact_analysis;
    console.log(`AI Assistance Rate: ${analysis.ai_assistance_rate}%`);
    console.log(`Velocity Improvement: ${analysis.velocity_improvement}%`);
    console.log(`Most AI-Assisted Language: ${analysis.most_ai_assisted_language}`);
}
```

## Progress Updates

In JSON mode, progress updates are streamed to stdout as they happen:

```json
{"stage":"scan","message":"Scanning directories","current":1,"total":10,"percentage":10.0,"status":"running"}
{"stage":"scan","message":"Scanning directories","current":2,"total":10,"percentage":20.0,"status":"running"}
...
{"stage":"scan","message":"Scanning directories","current":10,"total":10,"percentage":100.0,"status":"completed"}
```

Agents can:
1. Parse each line as it arrives
2. Update UI with progress percentage
3. Cancel long-running operations
4. Handle errors immediately

## Error Handling

Errors are included in structured output:

```json
{
  "success": false,
  "command": "backup",
  "duration_ms": 234,
  "output": null,
  "errors": [
    "Failed to create backup directory: Permission denied",
    "Git repository at /home/user/project is corrupted"
  ],
  "warnings": [
    "Some files were skipped due to size limits"
  ]
}
```

Exit codes:
- `0`: Success
- `1`: General error
- `2`: Invalid arguments
- `3`: Permission denied
- `4`: File not found

## Environment Variables

### `VIBEDEV_JSON=1`
Force JSON output mode (equivalent to --json flag)

```bash
export VIBEDEV_JSON=1
vibedev timeline  # Outputs JSON
```

### `NO_COLOR=1`
Disable colors (equivalent to --plain flag)

```bash
export NO_COLOR=1
vibedev backup  # Plain text output
```

## Best Practices for Agents

1. **Always use --json flag** for programmatic access
2. **Parse streaming updates** line-by-line as they arrive
3. **Check success field** in final result before processing output
4. **Handle errors gracefully** by checking errors array
5. **Set timeouts** for long-running operations (backup can take minutes)
6. **Validate JSON** before parsing (use try-catch)
7. **Check exit codes** in addition to JSON output

## Comparison: Human vs Agent Output

### Human Output (default)
```
🔬 Analyzing AI Impact on Productivity...
  ✓ Loaded Claude conversation history
  ✓ Loaded git commit history with detailed stats

📊 AI Impact Analysis Results
═══════════════════════════════

  Productivity Metrics
    • AI-Assisted Commits: 842 (68.3%)
    • Solo Commits: 392
    • Velocity Improvement: 42.5% faster with AI! 🚀

  AI Dependency Trend
    2025-10 │████████████████ 65%
    2025-11 │██████████████████ 70%
    2025-12 │████████████████████ 75%
```

### Agent Output (--json)
```json
{
  "stage": "analysis",
  "message": "Analyzing AI Impact on Productivity",
  "status": "started"
}
{
  "stage": "analysis",
  "message": "Loaded Claude conversation history",
  "status": "running"
}
{
  "stage": "complete",
  "message": "Analysis saved",
  "status": "completed"
}
{
  "success": true,
  "command": "backup",
  "duration_ms": 4532,
  "output": {
    "ai_assisted_commits": 842,
    "solo_commits": 392,
    "ai_assistance_rate": 68.3,
    "velocity_improvement": 42.5,
    "learning_curve": [
      {"month": "2025-10", "dependency_percentage": 65.0},
      {"month": "2025-11", "dependency_percentage": 70.0},
      {"month": "2025-12", "dependency_percentage": 75.0}
    ]
  },
  "errors": [],
  "warnings": []
}
```

## Supported Commands

All commands support `--json` and `--plain` flags:

- ✅ `discover` - Scan system for AI logs
- ✅ `analyze` - Generate analysis reports
- ✅ `backup` - Create backup archives
- ✅ `timeline` - Generate coding journey timeline
- ✅ `insights` - Comprehensive insights
- ✅ `stats` - Real-time statistics
- ✅ `compare` - Compare multiple AI tools
