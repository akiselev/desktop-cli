# Desktop CLI

A Windows desktop automation tool optimized for LLM agents. Control complex applications like Altium Designer, CAD tools, and other Windows software through UI Automation APIs.

## Features

- **LLM-Optimized Output**: Compact, categorized UI summaries that maximize signal-to-noise ratio
- **Enhanced Query Language**: Intuitive `@role` syntax designed for AI agents
- **Semantic Role Detection**: Automatic classification of UI elements (button, input, menu, etc.)
- **Smart Filtering**: Heuristics to prune noise from complex UI hierarchies
- **Spatial Queries**: Find elements by position relative to other elements
- **Post-Action Summaries**: Understand what changed after each interaction

## Installation

```bash
cargo build --release
```

## Quick Start

```bash
# Start the daemon
desktop start --foreground

# List windows
desktop window list

# Set target window
desktop window set-default 1

# Get UI summary
desktop call summary

# Click a button
desktop call do click "@button \"Save\""
```

## Commands

### LLM-Optimized Commands

| Command | Description |
|---------|-------------|
| `summary` | Get compact, categorized UI state |
| `query` | Find elements with enhanced syntax |
| `do` | Perform action and return summary |

### Example Usage

```bash
# Get UI overview
desktop call summary --format text

# Find all buttons
desktop call query "@button" --all

# Click Save button
desktop call do click "@button \"Save\""

# Type into search field
desktop call do type "@input \"Search\"" --value "hello"

# Focus on toolbar region only
desktop call summary --region "0,0,1920,50" --roles "button,menu"
```

## Query Language

The enhanced query language is designed for intuitive LLM use:

```
@button "Save"           - Button with name "Save"
@input:enabled           - All enabled input fields
#btnSave                 - Element with automation ID
@tab:nth(2)              - Second tab
~below("Label") @input   - Input field below a label
```

### Syntax Reference

| Pattern | Meaning |
|---------|---------|
| `@role` | Semantic role (button, input, menu, tab, etc.) |
| `"text"` | Exact name match |
| `"*text*"` | Contains text |
| `#id` | Automation ID |
| `:nth(N)` | Nth match (1-based) |
| `:enabled` | Only enabled elements |
| `~below(sel)` | Below anchor element |
| `~near(sel)` | Near anchor element |

## Output Formats

### Summary (JSON)
```json
{
  "window": "Altium Designer",
  "actions": [
    {"ref_id": "b1", "role": "button", "label": "Save", "action": "click"}
  ],
  "navigation": [
    {"ref_id": "m1", "role": "menu", "label": "File"}
  ],
  "stats": {"total_elements": 150, "actionable_elements": 12}
}
```

### Summary (Text)
```
# Altium Designer

## Actions
[b1] button: "Save" (click)
[i1] input: "Search" (type)

## Navigation
[m1] menu: "File" (click)

Stats: 150 total, 45 visible, 12 actionable
```

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌──────────────────┐
│   CLI       │────▶│  RPC Daemon │────▶│  UI Automation   │
│  (client)   │ TCP │  (server)   │     │  (Windows API)   │
└─────────────┘     └─────────────┘     └──────────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  Gemini API │
                    │  (optional) │
                    └─────────────┘
```

## For LLM Agents

See [AGENT.md](AGENT.md) for detailed instructions on using this CLI from an LLM agent context.

Key principles:
1. Use `summary` after every action
2. Use role-based queries (`@button`) over control types
3. Filter by region when working in specific UI areas
4. Use text format for maximum token efficiency

## Development

```bash
# Build
cargo build

# Test
cargo test

# Run in foreground (for debugging)
desktop start --foreground
```

## License

MIT
