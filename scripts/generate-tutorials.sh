#!/bin/bash
# Tutorial generation script
# Parses .tmpl files with {{COMMAND:...}} and {{OUTPUT}} markers,
# executes commands in the current environment, and substitutes output.
#
# Usage: ./scripts/generate-tutorials.sh [template_dir] [output_dir]
#
# Template markers:
#   {{COMMAND:desktop windows}}  - Runs the command and captures output
#   {{OUTPUT}}                   - Replaced with the output of the preceding COMMAND

set -euo pipefail

TEMPLATE_DIR="${1:-docs/templates}"
OUTPUT_DIR="${2:-docs/src/tutorials}"

if [ ! -d "$TEMPLATE_DIR" ]; then
    echo "ERROR: Template directory not found: $TEMPLATE_DIR" >&2
    exit 1
fi

mkdir -p "$OUTPUT_DIR"

process_template() {
    local tmpl="$1"
    local basename
    basename=$(basename "$tmpl" .tmpl)
    local output="$OUTPUT_DIR/$basename"

    echo "Processing: $tmpl -> $output"

    local pending_command=""
    local has_error=0

    while IFS= read -r line || [ -n "$line" ]; do
        # Check for COMMAND marker
        if [[ "$line" =~ \{\{COMMAND:(.+)\}\} ]]; then
            local cmd="${BASH_REMATCH[1]}"
            # Output the line as-is (template should format it as a code block)
            echo "$line" | sed "s/{{COMMAND:.*}}/\`$cmd\`/"
            pending_command="$cmd"
            continue
        fi

        # Check for OUTPUT marker
        if [[ "$line" =~ \{\{OUTPUT\}\} ]]; then
            if [ -z "$pending_command" ]; then
                echo "ERROR: {{OUTPUT}} without preceding {{COMMAND:...}} in $tmpl" >&2
                has_error=1
                echo "$line"
            else
                echo '```'
                # Execute the command and capture output
                if ! eval "$pending_command" 2>&1; then
                    echo "WARNING: Command failed: $pending_command" >&2
                fi
                echo '```'
                pending_command=""
            fi
            continue
        fi

        # Regular line - pass through
        echo "$line"
    done < "$tmpl" > "$output"

    if [ -n "$pending_command" ]; then
        echo "ERROR: {{COMMAND:...}} without matching {{OUTPUT}} in $tmpl" >&2
        has_error=1
    fi

    return $has_error
}

errors=0
for tmpl in "$TEMPLATE_DIR"/*.md.tmpl; do
    [ -f "$tmpl" ] || continue
    if ! process_template "$tmpl"; then
        errors=$((errors + 1))
    fi
done

if [ "$errors" -gt 0 ]; then
    echo "ERROR: $errors template(s) had errors" >&2
    exit 1
fi

echo "Tutorial generation complete."
