# Desktop CLI

A Windows desktop automation tool optimized for LLM agents. Control complex applications like Altium Designer, CAD tools, and other Windows software through UI Automation APIs.

## Features

- **LLM-Optimized Output**: Compact, categorized UI summaries that maximize signal-to-noise ratio
- **Enhanced Query Language**: Intuitive `@role` syntax designed for AI agents
- **Smart Window Targeting**: Target windows by name, title, index, or let the CLI auto-disambiguate using element selectors
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
# List windows with query hints
desktop windows

# Get summary of a window (by name, index, or title)
desktop summary notepad
desktop summary :1
desktop summary "title:PCB"

# Click a button
desktop click notepad "@button 'Save'"

# Type into a field
desktop type altium "#inputField" --value "Hello World"

# Smart disambiguation: finds the right window automatically
desktop click altium "@button 'Compile'"
```

## Window Targeting

Desktop CLI uses a powerful query syntax to target windows:

### Basic Queries

| Query | Description |
|-------|-------------|
| `:1`, `:2` | Window by index (from `desktop windows` list) |
| `notepad` | Match by executable name (substring) |
| `title:PCB` | Match by window title |
| `hwnd:0x1234` | Match by HWND |
| `pid:12345` | Match by process ID |

### Wildcards

| Pattern | Meaning |
|---------|---------|
| `title:*Draft*` | Title contains "Draft" |
| `title:*.docx` | Title ends with ".docx" |
| `title:Document*` | Title starts with "Document" |

### Smart Disambiguation

When multiple windows match, the CLI tries the element selector on each:

```bash
# 3 Altium windows exist, but only one has the "Compile" button
desktop click altium "@button 'Compile'"
# → Automatically finds and clicks in the correct window

# If ambiguous, shows helpful error
desktop click altium "@button 'File'"
# Error: Found "@button 'File'" in 3 windows:
#   [:1] Altium Designer - PCB1.PcbDoc
#   [:2] Altium Designer - Schematic1.SchDoc
#   [:3] Altium Designer - Project.PrjPcb
# Tip: Use ':1' or refine with 'title:...'
```

### Environment Variable

```bash
export DESKTOP_WINDOW="altium title:PCB"
desktop summary   # Uses env var
desktop click "@button 'OK'"   # Uses env var for window
```

## Commands

### Window Discovery

```bash
# List all windows
desktop windows

# Filter by exe or title
desktop windows --exe notepad
desktop windows --title "Draft"

# JSON output for agents
desktop windows --json

# Query suggestions for specific window
desktop windows --suggest 0x1234
```

### LLM-Optimized Commands

| Command | Description |
|---------|-------------|
| `summary` | Get compact, categorized UI state |
| `query` | Find elements with enhanced syntax |
| `do` | Perform action and return summary |

### Element Query Syntax

```
@button "Save"           - Button with name "Save"
@input:enabled           - All enabled input fields
#btnSave                 - Element with automation ID
@tab:nth(2)              - Second tab
~below("Label") @input   - Input below a label
```

### Actions

```bash
# Click
desktop click notepad "@button 'Save'"
desktop click :1 --coords 100,200

# Type
desktop type notepad "#editor" --value "Hello"

# Keys
desktop keys notepad "ctrl+s"

# Scroll
desktop scroll notepad up --amount 5
```

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

## For LLM Agents

See [AGENT.md](AGENT.md) for detailed instructions on using this CLI from an LLM agent context.

Key principles:
1. Use `summary` after every action
2. Use role-based queries (`@button`) over control types
3. Let the CLI disambiguate windows automatically
4. Use `desktop windows --json` for machine-readable window list

## Development

```bash
# Build
cargo build

# Test
cargo test

# Run
desktop windows
desktop summary notepad
```

## License

MIT
