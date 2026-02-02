# Command Reference

Complete reference for all desktop-cli commands.

## Command Overview

| Command | Purpose |
|---------|---------|
| [windows](#windows) | List visible windows with query hints |
| [summary](#summary) | Get compact UI summary optimized for LLMs |
| [query](#query) | Find elements using enhanced selector syntax |
| [click](#click) | Click an element or coordinates |
| [type](#type) | Type text into an element |
| [keys](#keys) | Send key combinations |
| [scroll](#scroll) | Scroll up or down |
| [dump-tree](#dump-tree) | Dump the UIA element tree |
| [find-element](#find-element) | Find elements by CSS-style selector |
| [invoke](#invoke) | Invoke UIA pattern operations |
| [do](#do) | Combined action + invoke |

## Global Options

### `-t, --target <WINDOW>`

Override window target for any command. Useful for scripts that work with multiple windows.

**Example:**
```bash
desktop -t notepad query "@button 'Save'"
desktop --target ":1" click "@button 'OK'"
```

## Commands

### windows

List all visible windows with information for targeting.

**Synopsis:**
```bash
desktop windows [OPTIONS]
```

**Options:**

| Option | Type | Description |
|--------|------|-------------|
| `--exe <NAME>` | String | Filter by executable name (substring match) |
| `--title <TEXT>` | String | Filter by window title (substring match) |
| `--json` | Flag | Output as JSON for machine consumption |
| `--suggest <HWND>` | String | Show query suggestions for specific HWND |

**Examples:**

List all windows:
```bash
desktop windows
```

Find notepad windows:
```bash
desktop windows --exe notepad
```

Get JSON output for automation:
```bash
desktop windows --json
```

Show targeting suggestions for a specific window:
```bash
desktop windows --suggest 0x12345
```

---

### summary

Get a compact UI summary optimized for LLM consumption. Returns categorized elements (actions, navigation, content) with minimal noise. Use this after every action to understand UI state.

**Synopsis:**
```bash
desktop summary [WINDOW] [OPTIONS]
```

**Arguments:**

| Argument | Type | Description |
|----------|------|-------------|
| `WINDOW` | String (optional) | Window query (e.g., "notepad", ":1", "title:PCB") |

**Options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--format <FORMAT>` | String | json | Output format: json or text |
| `--bounds` | Flag | false | Include bounding boxes in output |
| `--paths` | Flag | false | Include full hierarchy paths |
| `--region <X,Y,W,H>` | String | - | Focus on region: x,y,width,height |
| `--depth <N>` | Number | 10 | Maximum traversal depth |
| `--roles <ROLES>` | String | - | Filter by roles (comma-separated) |

**Examples:**

Get summary for notepad window:
```bash
desktop summary notepad
```

Get summary with bounding boxes for positioning:
```bash
desktop summary :1 --bounds
```

Focus on specific region and filter to buttons only:
```bash
desktop summary notepad --region 100,200,300,400 --roles button
```

Human-readable text output:
```bash
desktop summary notepad --format text
```

---

### query

Find elements using enhanced LLM-friendly selector syntax. Supports @role selectors, #id selectors, pseudo-selectors, and more.

**Synopsis:**
```bash
desktop query <WINDOW> <SELECTOR> [OPTIONS]
```

**Arguments:**

| Argument | Type | Description |
|----------|------|-------------|
| `WINDOW` | String | Window query |
| `SELECTOR` | String | Element selector (see [Selectors](selectors.md)) |

**Options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--all` | Flag | false | Return all matches (default: first only) |
| `--format <FORMAT>` | String | compact | Output format: full, compact, or refs |

**Examples:**

Find button by role and name:
```bash
desktop query notepad "@button 'Save'"
```

Find all enabled input fields:
```bash
desktop query :1 "@input:enabled" --all
```

Find element by automation ID:
```bash
desktop query altium "#btnSave"
```

Find second tab with full details:
```bash
desktop query notepad "@tab:nth(2)" --format full
```

---

### click

Click an element using a selector or specific coordinates.

**Synopsis:**
```bash
desktop click <WINDOW> [SELECTOR] [OPTIONS]
```

**Arguments:**

| Argument | Type | Description |
|----------|------|-------------|
| `WINDOW` | String | Window query |
| `SELECTOR` | String (optional) | Element selector (required unless --coords used) |

**Options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `-k, --kind <KIND>` | String | left | Click type: left, right, or double |
| `-c, --coords <X,Y>` | String | - | Click at coordinates instead of selector |

**Examples:**

Click a button:
```bash
desktop click notepad "@button 'Save'"
```

Right-click on element:
```bash
desktop click altium "Button[name='Compile']" --kind right
```

Double-click at specific coordinates:
```bash
desktop click :1 --coords 100,200 --kind double
```

---

### type

Type text into an element. The element is focused before typing.

**Synopsis:**
```bash
desktop type <WINDOW> <SELECTOR> --value <TEXT>
```

**Arguments:**

| Argument | Type | Description |
|----------|------|-------------|
| `WINDOW` | String | Window query |
| `SELECTOR` | String | Element selector to focus |

**Options:**

| Option | Type | Description |
|--------|------|-------------|
| `--value <TEXT>` | String (required) | Text to type |

**Examples:**

Type into an editor:
```bash
desktop type notepad "#editor" --value "Hello World"
```

Type into a named input field:
```bash
desktop type altium "@input 'Name'" --value "Component1"
```

---

### keys

Send key combinations to a window. Supports modifiers (ctrl, alt, shift) and special keys.

**Synopsis:**
```bash
desktop keys <WINDOW> <KEYS>
```

**Arguments:**

| Argument | Type | Description |
|----------|------|-------------|
| `WINDOW` | String | Window query |
| `KEYS` | String | Key combination (e.g., "ctrl+s", "alt+f4", "enter") |

**Examples:**

Save with Ctrl+S:
```bash
desktop keys notepad "ctrl+s"
```

Close window with Alt+F4:
```bash
desktop keys :1 "alt+f4"
```

Press Enter:
```bash
desktop keys notepad "enter"
```

---

### scroll

Scroll a window up or down.

**Synopsis:**
```bash
desktop scroll <WINDOW> <DIRECTION> [OPTIONS]
```

**Arguments:**

| Argument | Type | Description |
|----------|------|-------------|
| `WINDOW` | String | Window query |
| `DIRECTION` | String | Direction: up or down |

**Options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `-n, --amount <N>` | Number | 3 | Number of scroll notches |

**Examples:**

Scroll up 3 notches (default):
```bash
desktop scroll notepad up
```

Scroll down 5 notches:
```bash
desktop scroll :1 down --amount 5
```

---

### dump-tree

Dump the complete UIA element tree for a window. Useful for understanding window structure and debugging selectors.

**Synopsis:**
```bash
desktop dump-tree <WINDOW> [OPTIONS]
```

**Arguments:**

| Argument | Type | Description |
|----------|------|-------------|
| `WINDOW` | String | Window query |

**Options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--depth <N>` | Number | 5 | Maximum traversal depth |
| `--json` | Flag | false | Output as JSON (default is compact text) |

**Examples:**

Dump window tree with default depth:
```bash
desktop dump-tree notepad
```

Dump deeper tree as JSON:
```bash
desktop dump-tree :1 --depth 10 --json
```

---

### find-element

Find UI elements using CSS-style selectors. This is the lower-level API underlying the `query` command.

**Synopsis:**
```bash
desktop find-element <WINDOW> <SELECTOR> [OPTIONS]
```

**Arguments:**

| Argument | Type | Description |
|----------|------|-------------|
| `WINDOW` | String | Window query |
| `SELECTOR` | String | CSS-style selector (e.g., "Button#save", "[name~='*OK*']") |

**Options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--all` | Flag | false | Find all matches (default: first only) |

**Examples:**

Find button by automation ID:
```bash
desktop find-element notepad "Button#save"
```

Find all elements with wildcard name match:
```bash
desktop find-element altium "[name~='*OK*']" --all
```

---

### invoke

Invoke a UIA pattern operation on an element. This is the low-level pattern API for advanced automation.

**Synopsis:**
```bash
desktop invoke <WINDOW> <SELECTOR> --pattern <OPERATION> [OPTIONS]
```

**Arguments:**

| Argument | Type | Description |
|----------|------|-------------|
| `WINDOW` | String | Window query |
| `SELECTOR` | String | CSS-style selector to find target element |

**Options:**

| Option | Type | Description |
|--------|------|-------------|
| `--pattern <OPERATION>` | String (required) | Pattern operation: invoke, get-value, set-value, toggle, select, expand, collapse |
| `--value <VALUE>` | String | Value for set operations |

**Examples:**

Invoke (click/activate) a button:
```bash
desktop invoke notepad "Button#save" --pattern invoke
```

Get value from input field:
```bash
desktop invoke altium "Edit#txtName" --pattern get-value
```

Set value in input field:
```bash
desktop invoke notepad "Edit#editor" --pattern set-value --value "Hello"
```

Toggle a checkbox:
```bash
desktop invoke :1 "CheckBox#chkEnable" --pattern toggle
```

Expand a tree node:
```bash
desktop invoke explorer "TreeItem#folder1" --pattern expand
```

---

### do

Perform an action on an element. This combines finding and acting in one call, providing a simpler alternative to `invoke`.

**Synopsis:**
```bash
desktop do <WINDOW> <ACTION> <TARGET> [OPTIONS]
```

**Arguments:**

| Argument | Type | Description |
|----------|------|-------------|
| `WINDOW` | String | Window query |
| `ACTION` | String | Action: click, type, toggle, expand, collapse, select |
| `TARGET` | String | Target element query (e.g., @button "Save", #inputField) |

**Options:**

| Option | Type | Description |
|--------|------|-------------|
| `--value <VALUE>` | String | Value for type/set operations |

**Action Mappings:**

| Action | Maps to Pattern |
|--------|----------------|
| click | invoke |
| type, input, set | set-value |
| toggle, check, uncheck | toggle |
| expand, open | expand |
| collapse, close | collapse |
| select, choose | select |
| get, read | get-value |

**Examples:**

Click a button:
```bash
desktop do notepad click "@button 'Save'"
```

Type into a field:
```bash
desktop do altium type "@input 'Name'" --value "Component1"
```

Toggle a checkbox:
```bash
desktop do :1 toggle "#chkEnable"
```

Expand a tree node:
```bash
desktop do explorer expand "@treeitem 'Documents'"
```

## Window Targeting Syntax

All commands that accept a window argument support these targeting formats:

| Format | Description | Example |
|--------|-------------|---------|
| `:N` | Window by index (from `windows` command) | `:1`, `:2` |
| `name` | Executable name (without .exe) | `notepad`, `chrome` |
| `title:TEXT` | Window title (substring match) | `title:Document1` |
| `title:*PATTERN*` | Window title with wildcards | `title:*Draft*` |
| `hwnd:0xHEX` | Window handle (HWND) | `hwnd:0x12345` |
| `pid:NUMBER` | Process ID | `pid:12345` |

**Examples:**
```bash
desktop summary :1                    # First window from list
desktop query notepad "@button 'Save'"    # Any notepad window
desktop click "title:*Draft*" "@button 'OK'"   # Title with wildcard
desktop type "hwnd:0x12345" "#input" --value "text"  # By HWND
```
