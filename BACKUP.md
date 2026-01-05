# Backup Feature

## Overview

The `vibedev backup` command now supports comprehensive backup of your coding logs and history across multiple sources.

## Features

### 1. Standard AI Logs Backup (existing)
- Backs up logs from all discovered AI coding tools
- Creates compressed tar.gz archive
- Supports tool filtering and compression levels

### 2. Claude Logs Backup (NEW - `--include-claude`)
- Creates full backup of `~/.claude/` directory
- Includes all conversation history, debug logs, file-history, and session data
- Typical size: 500-600MB compressed

### 3. Git Commit History (NEW - `--include-git`)
- Discovers all git repositories in home directory (up to depth 5)
- Exports commit history from each repo in parseable format
- Format: `hash|author_name|author_email|timestamp|subject`
- Creates compressed archive of all git logs
- Typical size: 1-5MB for hundreds of repos

## Usage

### Basic backup (AI logs only)
```bash
vibedev backup
```

### Full backup (AI logs + Claude + Git)
```bash
vibedev backup --include-claude --include-git
```

### With custom output directory
```bash
vibedev backup --include-claude --include-git --output ~/backups
```

### Filter specific tool
```bash
vibedev backup --tool claude --include-claude --include-git
```

### Control compression level (0-9, default 6)
```bash
vibedev backup --compression 9 --include-claude --include-git
```

### Without timestamp in filename
```bash
vibedev backup --no-timestamp --include-claude --include-git
```

## Output Files

When using all flags, you'll get 3 archives:

1. `ai-logs-backup-YYYYMMDD-HHMMSS.tar.gz` - Standard AI logs
2. `claude-logs-YYYYMMDD-HHMMSS.tar.gz` - Full Claude directory backup (578MB)
3. `git-logs-YYYYMMDD-HHMMSS.tar.gz` - Git commit history (1-5MB)

## Examples

### Full backup with all features
```bash
vibedev backup \
  --include-claude \
  --include-git \
  --output ~/important-backups \
  --compression 9
```

Output:
```
💾 Creating backup archive...
Scanning from: /home/user

📦 Backup Summary:
  Tools: 58
  Total Size: 52.50 GB
  Total Files: 202612

📁 Creating archive: ai-logs-backup-20260105-044535.tar.gz
✅ AI logs backup created: ~/important-backups/ai-logs-backup-20260105-044535.tar.gz

📁 Adding Claude logs to backup...
  ✓ Claude logs: 578.0 MB

📝 Exporting git commit history...
  ✓ Found 127 git repositories
  ✓ gnome-boxes
  ✓ gnome-builder
  ✓ vibecheck
  ✓ opensvm
  ✓ aldrin
  ...
  ✓ Git logs archive: 1.6 MB (127 repos)
```

## Git Log Format

Each `.gitlog` file contains commit history in pipe-delimited format:

```
hash|author_name|author_email|timestamp|subject
c6c9b30...|Felipe Borges|felipeborges@gnome.org|1756462017|Release 49.rc.1
11e8be7...|Felipe Borges|felipeborges@gnome.org|1756458575|flatpak: Fix build
```

This format is parseable and can be used for:
- Timeline reconstruction
- Contribution analysis
- Coding pattern detection
- Work hour estimation

## Implementation Details

### Claude Backup
- Uses `tar -czf` to create compressed archive of `~/.claude/` directory
- Handles files that change during backup (active Claude sessions)
- Preserves directory structure and permissions

### Git History Export
- Uses `TimelineAnalyzer.find_git_repos()` to discover all git repositories
- Runs `git log --all --pretty=format:"%H|%an|%ae|%at|%s" --no-merges` for each repo
- Creates individual `.gitlog` files, then archives them
- Cleans up temporary directory after archival

### Integration with Timeline Feature
The git repositories discovered during backup are the same ones used by the `timeline` command to show your coding journey, ensuring consistency across features.

## Safety Notes

1. **Active files**: If Claude Code is running during backup, some debug files may change during archival. This is safe - the archive will still be created.

2. **Large backups**: The full AI logs backup can be 50GB+. Consider using `--tool` filter if you only need specific tools.

3. **Disk space**: Ensure you have enough disk space for all three archives (AI logs + Claude + Git).

4. **Privacy**: The backups contain your full conversation history and git commits. Store them securely.

## Restoration

To restore Claude logs:
```bash
cd ~
tar -xzf claude-logs-YYYYMMDD-HHMMSS.tar.gz
```

To extract git logs for analysis:
```bash
tar -xzf git-logs-YYYYMMDD-HHMMSS.tar.gz
cd git-logs
# Each repo has its own .gitlog file
```

## Related Commands

- `vibedev timeline` - Analyze your coding journey using the same git repos
- `vibedev analyze` - Analyze AI tool usage from backed up logs
- `vibedev insights` - Generate comprehensive insights from logs
