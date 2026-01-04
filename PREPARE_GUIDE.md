# AI Log Dataset Preparation Guide

## Overview

The `prepare` command creates a sanitized, finetuning-ready dataset from your AI coding assistant logs by:
1. Backing up your logs
2. Extracting to a temporary directory
3. Removing ALL sensitive data (PII, API keys, passwords, etc.)
4. Converting to training format (JSONL)
5. Creating a compressed ZIP dataset

## Usage

```bash
# Prepare dataset in home directory (default)
./target/release/ai-log-analyzer prepare

# Specify output directory
./target/release/ai-log-analyzer prepare --output /path/to/output
```

## What Gets Removed

### 🔒 API Keys & Tokens
- OpenAI/Anthropic keys (`sk-*`, `sk-ant-*`)
- GitHub tokens (`ghp_*`, `github_pat_*`)
- GitLab tokens (`glpat-*`)
- Slack tokens (`xox*`)
- AWS access keys (`AKIA*`)
- Google API keys and OAuth tokens
- Bearer tokens
- Generic API tokens

### 🔑 Passwords & Credentials
- Password fields (`password=`, `passwd=`, `pwd=`, `pass=`)
- URLs with authentication (`https://user:pass@`)
- Environment variables (`.env` file contents)

### 👤 Personal Identifiable Information (PII)
- Email addresses
- Phone numbers
- Social Security Numbers
- Credit card numbers
- IP addresses
- Personal file paths (`/home/username` → `/home/[USER]`)

## Output Format

The prepared dataset is a ZIP file containing:

### 1. `training_data.jsonl`
JSONL format with prompt/completion pairs:

```jsonl
{
  "prompt": "How do I implement authentication in Express?",
  "completion": "Here's how to implement authentication in Express using JWT...",
  "metadata": {
    "tool": "Claude Code",
    "session_id": "session_123",
    "timestamp": "2024-01-03T10:00:00Z",
    "tokens_estimate": 450
  }
}
```

### 2. `dataset_info.json`
Metadata and statistics:

```json
{
  "total_examples": 1523,
  "total_tokens_estimate": 685900,
  "sanitization_stats": {
    "files_processed": 245,
    "items_redacted": 1834,
    "redacted_by_type": {
      "API Key": 87,
      "Email": 432,
      "File Path": 1201,
      "Password": 43,
      "IP Address": 71
    }
  },
  "generated_at": "2024-01-03T10:30:00Z",
  "format": "jsonl",
  "safe_for_training": true
}
```

### 3. `README.md`
Documentation about the dataset, sanitization process, and usage instructions.

## Example Output

```
🔧 AI Log Dataset Preparation

Step 1/5: Creating backup...
  ✓ Backup created: /tmp/ai-logs-raw-backup.tar.gz

Step 2/5: Extracting to temporary directory...
  ✓ Extracted to: /tmp/ai-logs-sanitized-1704277200

Step 3/5: Sanitizing sensitive data...
  ✓ Files processed: 245
  ✓ Sensitive items removed: 1834

Step 4/5: Converting to training format...
  ✓ Training examples created: 1523

Step 5/5: Creating final dataset archive...
  ✓ Dataset saved: ~/ai-training-dataset-20240103-103000.zip

════════════════════════════════════════════════════════════
  Dataset Preparation Complete!
════════════════════════════════════════════════════════════

📊 Statistics:
  Training Examples:    1523
  Files Processed:      245
  Sensitive Items:      1834 removed

💾 File Sizes:
  Original Backup:      775.04 MB
  Sanitized Dataset:    89.34 MB
  Reduction:            88.5%

📁 Output:
  Dataset:              /home/user/ai-training-dataset-20240103-103000.zip

✅ Safe for finetuning - all PII removed!
```

## Use Cases

### 1. Model Finetuning
Train custom models on your coding patterns:
```python
import json

# Load training data
with open('training_data.jsonl', 'r') as f:
    examples = [json.loads(line) for line in f]

# Use with your finetuning framework
for example in examples:
    train_model(
        prompt=example['prompt'],
        completion=example['completion']
    )
```

### 2. Dataset Analysis
Analyze your AI usage patterns:
```python
import json
import pandas as pd

# Load into pandas
data = pd.read_json('training_data.jsonl', lines=True)

# Analyze token usage
print(f"Average tokens: {data['metadata'].apply(lambda x: x['tokens_estimate']).mean()}")

# Count by tool
print(data['metadata'].apply(lambda x: x['tool']).value_counts())
```

### 3. Transfer Learning
Use sanitized logs from multiple AI tools to train a unified model that understands different coding assistant patterns.

### 4. Research & Benchmarking
Create safe, shareable datasets for:
- AI coding assistant research
- Prompt engineering studies
- Model performance benchmarking
- Team knowledge sharing

## Safety Guarantees

✅ **No API Keys** - All authentication tokens removed
✅ **No Passwords** - All password fields redacted
✅ **No PII** - Emails, phones, SSNs removed
✅ **No Personal Paths** - File paths anonymized
✅ **No Environment Variables** - .env files excluded

## Best Practices

1. **Review Before Sharing**
   - Always review `dataset_info.json` to see what was redacted
   - Spot-check `training_data.jsonl` for any missed sensitive data

2. **Backup Original Data**
   - The raw backup is saved temporarily
   - Consider keeping encrypted backups of original logs

3. **Regular Updates**
   - Run `prepare` periodically to capture new logs
   - Merge datasets for larger training sets

4. **Version Control**
   - Tag dataset versions with timestamps
   - Track which logs were included in each dataset

## Technical Details

### Sanitization Algorithm

1. **Pattern Matching**: Uses regex patterns to detect 40+ types of sensitive data
2. **Replacement**: Sensitive data replaced with `[REDACTED_TYPE]` tokens
3. **Validation**: Double-checks all content is safe before including
4. **Metadata Preservation**: Keeps timestamps and tool info for analysis

### Performance

- Processes ~1GB of logs in under 2 minutes
- Sanitizes ~250 files/second
- Memory-efficient streaming for large files
- Parallel processing where safe

## Troubleshooting

### "No training examples created"
- Ensure you have history.jsonl files
- Check that logs contain prompt/completion pairs
- Currently supports Claude Code format best

### "Dataset too small"
- Run more AI coding sessions first
- Merge multiple dataset runs
- Adjust filtering criteria if needed

### "Custom patterns needed"
- Add custom regex patterns for domain-specific secrets
- Extend the Sanitizer class with additional patterns

## Future Enhancements

Planned features:
- Support for more log formats (Cursor, Cline detailed parsing)
- Custom sanitization rules configuration
- Differential privacy techniques
- Encryption at rest
- Cloud storage integration
- Automated finetuning pipeline

---

**Remember:** This tool helps prepare safe datasets, but always review output before sharing externally. When in doubt, redact more rather than less!
