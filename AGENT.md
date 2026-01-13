# Desktop CLI - Agent Instructions

This document provides instructions for LLM agents controlling desktop applications via the Desktop CLI.

## Quick Start

```bash
# 1. Start the daemon
desktop start --foreground

# 2. List available windows
desktop window list

# 3. Set default target window
desktop window set-default 1  # Use index from list

# 4. Get UI summary (recommended first step)
desktop call summary

# 5. Perform an action
desktop call do click "@button \"Save\""
```

## Core Philosophy: Signal over Noise

This CLI is designed to **maximize signal-to-noise ratio** for LLM consumption:

1. **Use `summary` after every action** - Get a compact categorized view of the UI
2. **Use the enhanced query syntax** - More intuitive than raw CSS selectors
3. **Use `do` for combined actions** - Click + summary in one call
4. **Filter by role/region** - Reduce output to relevant elements

## Commands Reference

### LLM-Optimized Commands (Recommended)

#### `summary` - Get UI State
Returns a compact, categorized view of visible UI elements.

```bash
# Basic summary
desktop call summary

# Text format (even more compact)
desktop call summary --format text

# Focus on toolbar area only
desktop call summary --region "0,0,800,50"

# Only show buttons and inputs
desktop call summary --roles "button,input"

# Include element coordinates
desktop call summary --bounds
```

**Output Structure:**
```json
{
  "window": "Altium Designer - PCB1",
  "actions": [
    {"ref_id": "b1", "role": "button", "label": "Save", "action": "click"},
    {"ref_id": "i1", "role": "input", "label": "Search", "action": "type"}
  ],
  "navigation": [
    {"ref_id": "m1", "role": "menu", "label": "File", "action": "click"}
  ],
  "stats": {"total_elements": 150, "visible_elements": 45, "actionable_elements": 12}
}
```

#### `query` - Find Elements
Find elements using the enhanced query language.

```bash
# Find Save button
desktop call query "@button \"Save\""

# Find all enabled input fields
desktop call query "@input:enabled" --all

# Find second tab
desktop call query "@tab:nth(2)"

# Find button below a label
desktop call query "~below(\"Username\") @button"
```

#### `do` - Perform Action + Get Summary
Combines finding, acting, and summarizing in one call.

```bash
# Click a button
desktop call do click "@button \"Save\""

# Type into an input
desktop call do type "@input \"Search\"" --value "component123"

# Toggle a checkbox
desktop call do toggle "@checkbox \"Enable\""

# Expand a tree item
desktop call do expand "@treeitem \"Libraries\""
```

### Query Language Reference

The enhanced query language is designed to be intuitive for LLMs:

| Syntax | Description | Example |
|--------|-------------|---------|
| `@role` | Find by semantic role | `@button`, `@input`, `@menu` |
| `"text"` | Match by name (exact) | `"Save"` |
| `"*text*"` | Match by name (contains) | `"*Save*"` |
| `#id` | Match by automation ID | `#btnSave` |
| `:nth(N)` | Nth match (1-based) | `@tab:nth(2)` |
| `:first` | First match | `@button:first` |
| `:last` | Last match | `@button:last` |
| `:enabled` | Only enabled elements | `@input:enabled` |
| `:disabled` | Only disabled elements | `@button:disabled` |
| `~below(sel)` | Below anchor element | `~below("Label") @input` |
| `~near(sel)` | Near anchor element | `~near(#header) @button` |
| `~inside(sel)` | Inside container | `~inside(#toolbar) @button` |

**Available Roles:**
- `@button` - Buttons, clickable elements
- `@input` - Text inputs, edit fields
- `@checkbox` - Checkboxes
- `@radio` - Radio buttons
- `@dropdown` - Combo boxes, dropdowns
- `@menu` - Menu items
- `@tab` - Tab items
- `@link` - Hyperlinks
- `@list` - List items
- `@tree` - Tree items
- `@slider` - Sliders, spinners
- `@table` - Tables, grids

### Legacy Commands

These are still available but produce more verbose output:

```bash
# Full tree dump (verbose)
desktop call dump-tree --depth 3

# CSS-style selector (less intuitive)
desktop call find-element "Button[name=\"Save\"]"

# Direct pattern invocation
desktop call invoke "Button#save" --pattern invoke
```

## Workflow Patterns

### Pattern 1: Explore and Act

```bash
# 1. Get overview
desktop call summary

# 2. Find specific element
desktop call query "@button \"Place\""

# 3. Perform action
desktop call do click "@button \"Place\""

# 4. Check result (summary again)
desktop call summary
```

### Pattern 2: Form Filling

```bash
# Fill multiple fields efficiently
desktop call do type "@input \"Name\"" --value "Component1"
desktop call do type "@input \"Value\"" --value "10k"
desktop call do click "@button \"OK\""
```

### Pattern 3: Menu Navigation

```bash
# Navigate menu hierarchy
desktop call do click "@menu \"File\""
desktop call summary  # See submenu items
desktop call do click "@menuitem \"Open\""
```

### Pattern 4: Focused Region Work

```bash
# Work only in toolbar area
desktop call summary --region "0,0,1920,50" --roles "button"

# Work only in properties panel (right side)
desktop call summary --region "1400,100,500,800"
```

## Tips for LLMs

### DO:
- Always call `summary` after actions to verify state changes
- Use role-based queries (`@button`) over control types (`Button`)
- Use `:nth()` for repeated elements instead of guessing
- Filter by region when working in specific UI areas
- Check `stats.actionable_elements` to know how many elements you can interact with

### DON'T:
- Don't use `dump-tree` for normal operations (too verbose)
- Don't guess automation IDs - query first
- Don't assume element positions - use spatial queries
- Don't send coordinates - use semantic selectors

### Handling Complex UIs (like Altium):

1. **Start with summary** to understand the layout
2. **Focus on regions** - toolbars, panels, dialogs separately
3. **Use paths** for deeply nested elements (`--paths` flag)
4. **Check menu state** before clicking menu items
5. **Wait after actions** - complex UIs may have loading states

## Error Handling

```json
// Element not found
{"count": 0, "matches": [], "suggestions": ["@button:contains(Save)"]}

// Action failed
{"success": false, "error": "Element is disabled"}

// Pattern not supported
{"success": false, "error": "Element does not support Invoke pattern"}
```

When errors occur:
1. Re-query to verify element exists
2. Check element state (`:enabled`)
3. Try alternative selectors from suggestions
4. Use `summary` to understand current UI state

## Token Efficiency

Approximate token usage per command:

| Command | Typical Output Tokens |
|---------|----------------------|
| `summary` (JSON) | 200-500 |
| `summary --format text` | 100-300 |
| `query` (single) | 50-100 |
| `do` | 100-200 |
| `dump-tree` | 1000-5000+ |

For maximum efficiency:
- Use text format for summaries
- Filter by roles when possible
- Use `--region` to focus on relevant areas
- Avoid `dump-tree` except for debugging
