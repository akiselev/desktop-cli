----
name: desktop-cli
description: CLI for automating and interacting desktop applications
----

## Quick Start

```bash
# 1. List available windows
desktop windows

# 2. Get UI summary (use window name, index, or title)
desktop summary notepad
desktop summary :1
desktop summary "title:PCB"

# 3. Perform an action
desktop click notepad "@button 'Save'"
```

## Core Philosophy: Signal over Noise

This CLI is designed to **maximize signal-to-noise ratio** for LLM consumption:

1. **Use `summary` after every action** - Get a compact categorized view of the UI
2. **Use the enhanced query syntax** - More intuitive than raw CSS selectors
3. **Let smart disambiguation work for you** - Specify elements, let the CLI find the right window
4. **Filter by role/region** - Reduce output to relevant elements

## Window Targeting

### Query Syntax

Target windows using intuitive queries:

| Syntax | Description | Example |
|--------|-------------|---------|
| `:N` | By index from window list | `:1`, `:2`, `:last` |
| `name` | By executable (substring) | `notepad`, `altium` |
| `title:X` | By window title | `title:PCB`, `title:*Draft*` |
| `hwnd:X` | By HWND | `hwnd:0x1234` |
| `pid:N` | By process ID | `pid:12345` |

### Smart Disambiguation

When targeting by exe name matches multiple windows, the CLI tries the element selector on each:

```bash
# Multiple Altium windows exist
desktop click altium "@button 'Compile'"

# The CLI will:
# 1. Find all windows matching "altium"
# 2. Check which ones have "@button 'Compile'"
# 3. Click on the one window that has it
# 4. Error only if 0 or 2+ windows have the element
```

### Window Discovery

```bash
# List windows with query suggestions
desktop windows

# JSON output for parsing
desktop windows --json

# Get unique queries for a specific window
desktop windows --suggest 0x1234
```

Output includes helpful query hints:
```
Windows (3 found):
  [:1] altium | title:"Altium Designer - PCB1.PcbDoc" | hwnd:0x1234
  [:2] altium | title:"Altium Designer - Schematic1.SchDoc" | hwnd:0x5678
  [:3] notepad | title:"Untitled - Notepad" | hwnd:0x9ABC

Query examples:
  :1                    → Altium PCB1
  notepad               → Untitled - Notepad
  altium title:PCB      → Altium PCB1
```

## Commands Reference

### LLM-Optimized Commands (Recommended)

#### `summary` - Get UI State
Returns a compact, categorized view of visible UI elements.

```bash
# Basic summary
desktop summary notepad

# Text format (even more compact)
desktop summary :1 --format text

# Focus on toolbar area only
desktop summary altium --region "0,0,800,50"

# Only show buttons and inputs
desktop summary notepad --roles "button,input"

# Include element coordinates
desktop summary :1 --bounds
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
desktop query notepad "@button 'Save'"

# Find all enabled input fields
desktop query altium "@input:enabled" --all

# Find second tab
desktop query :1 "@tab:nth(2)"

# Find button below a label
desktop query notepad "~below('Username') @button"
```

#### `click` - Click an Element

```bash
# Click a button
desktop click notepad "@button 'Save'"

# Click at coordinates
desktop click :1 --coords 100,200

# Right-click
desktop click altium "@menu 'File'" --kind right

# Double-click
desktop click notepad "@listitem 'Document'" --kind double
```

#### `type` - Type Text

```bash
# Type into an input
desktop type notepad "#editor" --value "Hello World"

# Type into a named field
desktop type altium "@input 'Search'" --value "component123"
```

#### `keys` - Send Key Combinations

```bash
desktop keys notepad "ctrl+s"     # Save
desktop keys :1 "ctrl+c"          # Copy
desktop keys altium "alt+f4"      # Close
desktop keys notepad "enter"      # Enter
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

## Workflow Patterns

### Pattern 1: Explore and Act

```bash
# 1. List windows to find your target
desktop windows

# 2. Get overview of the window
desktop summary notepad

# 3. Find specific element
desktop query notepad "@button 'Place'"

# 4. Perform action
desktop click notepad "@button 'Place'"

# 5. Check result (summary again)
desktop summary notepad
```

### Pattern 2: Smart Targeting

```bash
# Let disambiguation find the right window
desktop click altium "@button 'Compile'"
# → Automatically finds the Altium window with the Compile button

# If you need to be specific
desktop click "altium title:PCB" "@button 'Compile'"
```

### Pattern 3: Form Filling

```bash
# Fill multiple fields efficiently
desktop type notepad "@input 'Name'" --value "Component1"
desktop type notepad "@input 'Value'" --value "10k"
desktop click notepad "@button 'OK'"
```

### Pattern 4: Menu Navigation

```bash
# Navigate menu hierarchy
desktop click altium "@menu 'File'"
desktop summary altium  # See submenu items
desktop click altium "@menuitem 'Open'"
```

### Pattern 5: Environment Variable

```bash
# Set window for session
export DESKTOP_WINDOW="altium title:PCB"

# Now commands use that window automatically
desktop summary
desktop click "@button 'Save'"
desktop keys "ctrl+s"
```

## Tips for LLMs

### DO:
- Always call `summary` after actions to verify state changes
- Use role-based queries (`@button`) over control types (`Button`)
- Use `:nth()` for repeated elements instead of guessing
- Filter by region when working in specific UI areas
- Let smart disambiguation work for you - specify element, not window
- Check `stats.actionable_elements` to know how many elements you can interact with

### DON'T:
- Don't use `dump-tree` for normal operations (too verbose)
- Don't guess automation IDs - query first
- Don't assume element positions - use semantic selectors
- Don't over-specify windows when disambiguation will work

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

// Ambiguous window
Error: Found "@button 'File'" in 3 windows:
  [:1] Altium Designer - PCB1.PcbDoc
  [:2] Altium Designer - Schematic1.SchDoc
  [:3] Altium Designer - Project.PrjPcb
Tip: Use ':1' or refine with 'title:...'
```

When errors occur:
1. Re-query to verify element exists
2. Check element state (`:enabled`)
3. Try alternative selectors from suggestions
4. Use `summary` to understand current UI state
5. Refine window target with `title:...` if ambiguous

## Token Efficiency

Approximate token usage per command:

| Command | Typical Output Tokens |
|---------|----------------------|
| `summary` (JSON) | 200-500 |
| `summary --format text` | 100-300 |
| `query` (single) | 50-100 |
| `click` | 20-50 |
| `dump-tree` | 1000-5000+ |
| `windows --json` | 300-800 |

For maximum efficiency:
- Use text format for summaries
- Filter by roles when possible
- Use `--region` to focus on relevant areas
- Avoid `dump-tree` except for debugging
