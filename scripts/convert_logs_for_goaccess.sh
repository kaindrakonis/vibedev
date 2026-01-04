#!/bin/bash
# Convert AI tool logs to GoAccess-compatible format
# Output: Combined Log Format (CLF) style

OUTPUT_FILE="${1:-/tmp/ai-logs-combined.log}"

echo "Converting AI logs to GoAccess format..."
> "$OUTPUT_FILE"

# Process Claude Code logs
find ~/.claude -name "*.jsonl" 2>/dev/null | while read -r file; do
    project=$(basename "$(dirname "$file")")

    jq -r --arg proj "$project" '
        # Convert epoch ms to date
        def to_date: . / 1000 | strftime("%d/%b/%Y:%H:%M:%S +0000");

        # Get message type
        def msg_type:
            if .type then .type
            elif .display then "user"
            elif .message then "assistant"
            else "event"
            end;

        # Get content length
        def content_len:
            if .display then (.display | length)
            elif .message then (.message | length)
            else 0
            end;

        # Format as CLF-like: IP - user [date] "METHOD /path HTTP" status size
        "127.0.0.1 - claude [\(.timestamp | to_date)] \"POST /\($proj)/\(msg_type) HTTP/1.1\" 200 \(content_len)"
    ' "$file" 2>/dev/null >> "$OUTPUT_FILE"
done

# Count entries
LINES=$(wc -l < "$OUTPUT_FILE")
echo "Converted $LINES log entries to $OUTPUT_FILE"

# Generate GoAccess report
echo "Generating GoAccess HTML report..."
goaccess "$OUTPUT_FILE" \
    --log-format='%h - %^ [%d:%t %^] "%m %U %H" %s %b' \
    --date-format='%d/%b/%Y' \
    --time-format='%H:%M:%S' \
    -o /tmp/ai-logs-report.html \
    --ignore-crawlers \
    --anonymize-ip

echo "Report saved to: /tmp/ai-logs-report.html"
