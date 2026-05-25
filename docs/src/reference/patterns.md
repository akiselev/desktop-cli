# Pattern Reference

Guide to UI Automation patterns and pattern operations in desktop-cli.

## What Are Patterns?

In UI Automation (UIA), **patterns** are interfaces that expose specific functionality of UI elements. Each pattern provides a set of operations for interacting with elements that support that pattern.

For example:
- A button supports the **Invoke** pattern (for clicking)
- A text box supports the **Value** pattern (for getting/setting text)
- A checkbox supports the **Toggle** pattern (for checking/unchecking)

Desktop-cli exposes these patterns through two commands:
- `invoke` - Low-level pattern operations
- `do` - High-level action commands (recommended)

## Pattern Overview

| Pattern | Purpose | Elements | Operations |
|---------|---------|----------|------------|
| [Invoke](#invoke-pattern) | Activate/click elements | Buttons, menu items, links | invoke |
| [Value](#value-pattern) | Get/set text values | Text inputs, edit boxes | get-value, set-value |
| [Toggle](#toggle-pattern) | Toggle binary state | Checkboxes, toggle buttons | toggle |
| [Selection](#selection-pattern) | Select items | List items, tabs, radio buttons | select |
| [Scroll](#scroll-pattern) | Scroll containers | Scrollable areas, lists | scroll |
| [ExpandCollapse](#expandcollapse-pattern) | Expand/collapse nodes | Tree items, menu items | expand, collapse |

## Invoke Pattern

The Invoke pattern activates elements that perform a single, unambiguous action (typically clicking or activating).

### Supported Elements
- Buttons
- Menu items
- Links
- Toolbar buttons

### Operations

#### invoke

Activates the element (equivalent to clicking).

**Using `invoke` command:**
```bash
desktop invoke notepad "Button#save" --pattern invoke
```

**Using `do` command (recommended):**
```bash
desktop do notepad click "@button 'Save'"
```

### Examples

Click Save button:
```bash
desktop do notepad click "@button 'Save'"
```

Activate menu item:
```bash
desktop do notepad click "@menuitem 'File'"
```

Click toolbar button:
```bash
desktop do vscode click "@button 'Run'"
```

## Value Pattern

The Value pattern gets or sets the text value of controls like text boxes and edit fields.

### Supported Elements
- Text input fields (Edit controls)
- Text areas
- Numeric inputs
- Some combo boxes

### Operations

#### get-value

Retrieves the current text value.

**Using `invoke` command:**
```bash
desktop invoke notepad "Edit#editor" --pattern get-value
```

**Using `do` command:**
```bash
desktop do notepad get "#editor"
```

Returns JSON:
```json
{
  "pattern": "Value",
  "operation": "get-value",
  "result": "Current text content"
}
```

#### set-value

Sets the text value (replaces current content).

**Using `invoke` command:**
```bash
desktop invoke notepad "Edit#editor" --pattern set-value --value "Hello World"
```

**Using `do` command (recommended):**
```bash
desktop do notepad type "@input 'File name'" --value "document.txt"
```

### Examples

Read value from input:
```bash
desktop do notepad get "@input 'File name'"
```

Set value in text box:
```bash
desktop do altium type "@input 'Component Name'" --value "Resistor"
```

Clear field (set to empty):
```bash
desktop do notepad type "@input 'Search'" --value ""
```

## Toggle Pattern

The Toggle pattern toggles elements between two or three states (on/off, or on/indeterminate/off).

### Supported Elements
- Checkboxes
- Toggle buttons
- Some toolbar buttons (like Bold, Italic)

### Operations

#### toggle

Cycles to the next toggle state:
- Off → On
- On → Off (for binary toggles)
- On → Indeterminate → Off (for three-state toggles)

**Using `invoke` command:**
```bash
desktop invoke :1 "CheckBox#chkEnable" --pattern toggle
```

**Using `do` command (recommended):**
```bash
desktop do notepad toggle "@checkbox 'Word Wrap'"
```

### Examples

Toggle checkbox:
```bash
desktop do notepad toggle "@checkbox 'Word Wrap'"
```

Toggle toolbar button:
```bash
desktop do vscode toggle "@button 'Toggle Sidebar'"
```

Check multiple options:
```bash
desktop do settings toggle "@checkbox 'Enable notifications'"
desktop do settings toggle "@checkbox 'Start on boot'"
```

## Selection Pattern

The Selection pattern selects items within containers like lists, tabs, and radio button groups.

### Supported Elements
- List items
- Tab items
- Radio buttons
- Combo box items
- Tree items (sometimes)

### Operations

#### select

Selects the target item (may deselect others depending on container type).

**Using `invoke` command:**
```bash
desktop invoke vscode "TabItem#main.rs" --pattern select
```

**Using `do` command (recommended):**
```bash
desktop do vscode select "@tab 'main.rs'"
```

### Examples

Select tab:
```bash
desktop do browser select "@tab 'GitHub'"
```

Select list item:
```bash
desktop do explorer select "@listitem 'document.txt'"
```

Select radio button:
```bash
desktop do settings select "@radiobutton 'Dark theme'"
```

Select from dropdown (combo box):
```bash
desktop do notepad select "@combobox 'Font'"
desktop do notepad select "@listitem 'Arial'"
```

## Scroll Pattern

The Scroll pattern scrolls containers vertically or horizontally.

### Supported Elements
- Scrollable areas
- Lists
- Document areas
- Tree views

### Operations

#### scroll

Scrolls the container. Typically exposed through the `scroll` command rather than pattern invocation.

**Using scroll command (recommended):**
```bash
desktop scroll notepad up
desktop scroll notepad down --amount 5
```

**Using pattern (advanced):**
```bash
desktop invoke notepad "Document#content" --pattern scroll --value "down"
```

### Examples

Scroll down in document:
```bash
desktop scroll notepad down --amount 3
```

Scroll up in list:
```bash
desktop scroll explorer up --amount 5
```

## ExpandCollapse Pattern

The ExpandCollapse pattern expands or collapses nodes in hierarchical structures.

### Supported Elements
- Tree items
- Expandable menu items
- Accordion sections
- Collapsible panels

### Operations

#### expand

Expands a collapsed node to show children.

**Using `invoke` command:**
```bash
desktop invoke explorer "TreeItem#Documents" --pattern expand
```

**Using `do` command (recommended):**
```bash
desktop do explorer expand "@treeitem 'Documents'"
```

#### collapse

Collapses an expanded node to hide children.

**Using `invoke` command:**
```bash
desktop invoke explorer "TreeItem#Documents" --pattern collapse
```

**Using `do` command (recommended):**
```bash
desktop do explorer collapse "@treeitem 'Documents'"
```

### Examples

Expand folder in tree:
```bash
desktop do explorer expand "@treeitem 'Downloads'"
```

Collapse folder:
```bash
desktop do explorer collapse "@treeitem 'Downloads'"
```

Navigate hierarchy:
```bash
desktop do explorer expand "@treeitem 'Documents'"
desktop do explorer expand "@treeitem 'Projects'"
desktop do explorer select "@treeitem 'desktop-cli'"
```

Expand all top-level nodes:
```bash
for item in Documents Downloads Pictures; do
  desktop do explorer expand "@treeitem '$item'"
done
```

## The `do` Command

The `do` command provides a simpler, more intuitive interface to patterns. It automatically maps actions to patterns.

### Action Mappings

| Action | Pattern | Description |
|--------|---------|-------------|
| `click` | Invoke | Click/activate element |
| `type`, `input`, `set` | Value.SetValue | Set text value |
| `get`, `read` | Value.GetValue | Get text value |
| `toggle`, `check`, `uncheck` | Toggle | Toggle checkbox/button |
| `expand`, `open` | ExpandCollapse.Expand | Expand node |
| `collapse`, `close` | ExpandCollapse.Collapse | Collapse node |
| `select`, `choose` | Selection.Select | Select item |

### Why Use `do`?

**Instead of:**
```bash
desktop invoke notepad "Button#save" --pattern invoke
```

**Write:**
```bash
desktop do notepad click "@button 'Save'"
```

Benefits:
- More intuitive action verbs
- Shorter syntax
- LLM-friendly
- Combines query + action in one command

### `do` Examples

Click button:
```bash
desktop do notepad click "@button 'Save'"
```

Type text:
```bash
desktop do notepad type "@input 'File name'" --value "document.txt"
```

Toggle checkbox:
```bash
desktop do settings toggle "@checkbox 'Dark mode'"
```

Select tab:
```bash
desktop do vscode select "@tab 'main.rs'"
```

Expand tree:
```bash
desktop do explorer expand "@treeitem 'Projects'"
```

## When to Use `invoke` vs `do`

### Use `do` when:
- You want readable, intuitive commands
- You're writing automation scripts
- You're using LLM agents
- You want action-oriented semantics

### Use `invoke` when:
- You need fine-grained pattern control
- You're debugging pattern support
- You need to specify exact pattern and operation
- You're working with custom or uncommon patterns

## Pattern Support Detection

Not all elements support all patterns. To check what patterns an element supports:

```bash
desktop dump-tree notepad --json | jq '.patterns'
```

Or use the `summary` command:
```bash
desktop summary notepad --format json | jq '.elements[].patterns'
```

Common patterns in output:
```json
{
  "patterns": ["Invoke", "Value", "Toggle"]
}
```

## Error Handling

### "Pattern not supported"

The element doesn't support the requested pattern.

**Solutions:**
- Check element type (buttons don't support Value pattern)
- Verify pattern support with `dump-tree`
- Try alternative approach (use `type` command instead of Value pattern)

### "Element not actionable"

Element is disabled or offscreen.

**Solutions:**
- Check element state: `desktop query notepad "@button:enabled"`
- Scroll element into view first
- Wait for element to become enabled

### "Operation failed"

Pattern operation failed.

**Solutions:**
- Verify element is in correct state (can't collapse already-collapsed node)
- Check permissions (some operations require elevated access)
- Ensure application is responsive

## Best Practices

1. **Prefer `do` over `invoke`** - More readable and maintainable
2. **Check element state before acting** - Use `:enabled` and `:visible` selectors
3. **Use appropriate patterns** - Don't try to "click" a text input; use `type`
4. **Handle missing pattern support** - Not all buttons support Toggle
5. **Combine with `summary`** - Use after actions to verify state changes
6. **Use pattern-specific commands** - `click`, `type`, `scroll` are optimized for their patterns

## Advanced Pattern Usage

### Chaining Operations

Perform multiple pattern operations in sequence:

```bash
# Expand tree, select item, invoke action
desktop do explorer expand "@treeitem 'Documents'"
desktop do explorer expand "@treeitem 'Projects'"
desktop do explorer select "@treeitem 'desktop-cli'"
desktop do explorer click "@button 'Open'"
```

### Conditional Operations

Check value before setting:

```bash
# Get current value
current=$(desktop do notepad get "@input 'File name'" | jq -r '.result')

# Set only if different
if [ "$current" != "newfile.txt" ]; then
  desktop do notepad type "@input 'File name'" --value "newfile.txt"
fi
```

### Pattern-Based State Verification

Use `get-value` to verify action results:

```bash
# Type text
desktop do notepad type "@input 'Search'" --value "test"

# Verify it was set
desktop do notepad get "@input 'Search'"
```

## Summary

Patterns are the foundation of UI Automation. The `do` command provides an intuitive interface to common patterns, while `invoke` offers fine-grained control. Choose the right tool for your automation needs.

**Quick reference:**
- Click things → `do ... click`
- Type text → `do ... type ... --value`
- Toggle checkboxes → `do ... toggle`
- Select items → `do ... select`
- Expand/collapse → `do ... expand/collapse`
- Read values → `do ... get`
