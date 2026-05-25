# Selector Reference

Comprehensive guide to element selectors in desktop-cli.

## Overview

Desktop-cli provides a powerful, LLM-friendly selector syntax for finding UI elements. Selectors combine role-based queries, automation IDs, string matching, pseudo-selectors, and spatial relationships.

## Syntax Summary

```bnf
selector      ::= role_selector | id_selector | css_selector
role_selector ::= "@" role_name [string_match] [pseudo]
id_selector   ::= "#" automation_id
css_selector  ::= element_type ["#" automation_id] ["[" attribute "]"]
string_match  ::= '"' exact '"' | '"' prefix '*' '"' | '"' '*' substring '*' '"'
pseudo        ::= ":" pseudo_name ["(" argument ")"]
spatial       ::= "~" relation "(" selector ")"
```

## Role Selectors

The `@role` syntax finds elements by their accessibility role and optionally by name.

**Format:** `@role ["name"]`

### Supported Roles

| Role | Description | Example Elements |
|------|-------------|------------------|
| `button` | Clickable buttons | Save, OK, Cancel buttons |
| `input` | Text input fields | Edit boxes, text areas |
| `checkbox` | Checkboxes | Enable/disable options |
| `radiobutton` | Radio buttons | Single-choice options |
| `menu` | Menu items | File, Edit menu items |
| `menuitem` | Individual menu items | Save, Open commands |
| `tab` | Tab controls | Document tabs, ribbon tabs |
| `tabitem` | Individual tabs | Sheet1, Home tab |
| `list` | List controls | File lists, item lists |
| `listitem` | Individual list items | File names, options |
| `tree` | Tree controls | Folder trees, navigation |
| `treeitem` | Tree nodes | Folders, hierarchy items |
| `combobox` | Dropdown lists | Select controls |
| `link` | Hyperlinks | URL links, navigation links |
| `text` | Static text | Labels, descriptions |
| `group` | Container groups | Panels, groupboxes |
| `toolbar` | Toolbars | Button toolbars |
| `statusbar` | Status bars | Bottom status text |
| `window` | Windows | Dialog boxes, main windows |
| `pane` | Panes | Split panels |
| `document` | Document areas | Content areas, editors |
| `scrollbar` | Scroll bars | Vertical/horizontal scrolls |
| `slider` | Slider controls | Volume, brightness sliders |
| `progressbar` | Progress bars | Loading indicators |
| `spinner` | Numeric spinners | Up/down controls |
| `table` | Tables | Data grids |
| `cell` | Table cells | Grid cells |
| `image` | Images | Icons, pictures |
| `separator` | Separators | Dividers |

### Examples

Find any button:
```bash
desktop query notepad "@button"
```

Find specific button by name:
```bash
desktop query notepad "@button 'Save'"
```

Find input field:
```bash
desktop query notepad "@input 'File name'"
```

Find tab:
```bash
desktop query vscode "@tab 'main.rs'"
```

Find menu item:
```bash
desktop query notepad "@menuitem 'File'"
```

## Automation ID Selectors

The `#id` syntax finds elements by their automation ID, which is a stable identifier set by developers.

**Format:** `#automation_id`

### Examples

Find element by ID:
```bash
desktop query notepad "#btnSave"
```

Find input by ID:
```bash
desktop query altium "#txtComponentName"
```

## String Matching

String matching controls how names are compared. Supports exact matches, prefix wildcards, and substring wildcards.

### Exact Match

Use quotes without wildcards for exact matching:

```bash
desktop query notepad "@button 'Save'"
```

Matches: "Save"
Does not match: "Save As", "Save All"

### Prefix Wildcard

Use `*` at the end to match any suffix:

```bash
desktop query notepad "@button 'Save*'"
```

Matches: "Save", "Save As", "Save All"
Does not match: "QuickSave"

### Substring Wildcard

Use `*` on both sides to match anywhere:

```bash
desktop query notepad "@button '*Save*'"
```

Matches: "Save", "Save As", "QuickSave", "Autosave Options"

### Case Sensitivity

String matching is typically case-insensitive on Windows, but best practice is to match the actual casing shown in UI.

## Pseudo-Selectors

Pseudo-selectors filter or select from multiple matches.

### Position Pseudo-Selectors

#### `:first`

Select the first match:

```bash
desktop query notepad "@button:first"
```

#### `:last`

Select the last match:

```bash
desktop query notepad "@button:last"
```

#### `:nth(N)`

Select the Nth match (0-indexed):

```bash
desktop query notepad "@tab:nth(0)"    # First tab
desktop query notepad "@tab:nth(2)"    # Third tab
```

### State Pseudo-Selectors

#### `:enabled`

Select only enabled elements:

```bash
desktop query notepad "@button:enabled"
```

#### `:disabled`

Select only disabled elements:

```bash
desktop query notepad "@button:disabled"
```

#### `:visible`

Select only visible elements (not offscreen):

```bash
desktop query notepad "@input:visible"
```

### Combining Pseudo-Selectors

You can chain pseudo-selectors:

```bash
desktop query notepad "@button:enabled:first"
desktop query altium "@input:visible:last"
```

## Spatial Selectors

Spatial selectors find elements based on their position relative to other elements.

### `~below(selector)`

Find elements below a reference element:

```bash
desktop query notepad "~below('@text \"File name\"') @input"
```

Finds the input field below the "File name" label.

### `~above(selector)`

Find elements above a reference element:

```bash
desktop query notepad "~above(@button 'Save') @input"
```

### `~near(selector)`

Find elements near a reference element:

```bash
desktop query notepad "~near(@text 'Name') @input"
```

### Spatial Examples

Find input below label:
```bash
desktop do notepad type "~below('@text \"Username\"') @input" --value "admin"
```

Find button near text:
```bash
desktop click altium "~near('@text \"Project\"') @button"
```

## CSS-Style Selectors

The lower-level `find-element` command supports CSS-style selectors for advanced use cases.

### By Element Type

```bash
desktop find-element notepad "Button"
desktop find-element notepad "Edit"
```

### By ID

```bash
desktop find-element notepad "Button#save"
desktop find-element notepad "#editor"
```

### By Attribute

Match by name attribute with wildcards:

```bash
desktop find-element notepad "[name='Save']"           # Exact
desktop find-element notepad "[name~='*Save*']"        # Substring
desktop find-element notepad "Button[name~='*OK*']"    # Type + attribute
```

## Combining Selectors

You can combine different selector types for precise targeting.

### Role + State

```bash
desktop query notepad "@button:enabled"
desktop query altium "@input:visible:first"
```

### Role + Name + Pseudo

```bash
desktop query vscode "@tab 'main.rs':first"
desktop query notepad "@button 'Save*':enabled"
```

### Spatial + Role + State

```bash
desktop query notepad "~below('@text \"Name\"') @input:enabled"
```

## Common Patterns

### Finding Buttons

Simple button:
```bash
desktop query notepad "@button 'Save'"
```

Button with wildcard:
```bash
desktop query notepad "@button 'Save*'"
```

First enabled button:
```bash
desktop query notepad "@button:enabled:first"
```

### Finding Inputs

Named input:
```bash
desktop query notepad "@input 'File name'"
```

All inputs:
```bash
desktop query notepad "@input" --all
```

Input below label:
```bash
desktop query notepad "~below('@text \"Username\"') @input"
```

### Finding Tabs

Specific tab:
```bash
desktop query vscode "@tab 'main.rs'"
```

Second tab:
```bash
desktop query browser "@tab:nth(1)"
```

### Finding Menu Items

Top-level menu:
```bash
desktop query notepad "@menuitem 'File'"
```

Submenu item:
```bash
desktop query notepad "@menuitem 'Save As'"
```

### Finding Tree Items

Specific node:
```bash
desktop query explorer "@treeitem 'Documents'"
```

Nested node:
```bash
desktop query explorer "@treeitem 'Downloads':visible"
```

### Finding List Items

Specific item:
```bash
desktop query notepad "@listitem 'document1.txt'"
```

All visible items:
```bash
desktop query notepad "@listitem:visible" --all
```

## Debugging Selectors

If your selector doesn't work:

1. **Use `dump-tree` to explore structure:**
   ```bash
   desktop dump-tree notepad --depth 10
   ```

2. **Use `summary` to see available elements:**
   ```bash
   desktop summary notepad
   ```

3. **Try broader selectors first:**
   ```bash
   desktop query notepad "@button" --all
   ```

4. **Check for wildcards:**
   ```bash
   desktop query notepad "@button 'Save*'"
   ```

5. **Verify element is visible and enabled:**
   ```bash
   desktop query notepad "@button:visible:enabled" --all
   ```

## Best Practices

1. **Prefer role selectors over CSS selectors** - They're more readable and LLM-friendly
2. **Use automation IDs when available** - They're stable across UI changes
3. **Use wildcards judiciously** - Too broad and you'll match unintended elements
4. **Combine state filters** - `:enabled:visible` ensures elements are actionable
5. **Use spatial selectors for ambiguous UIs** - When multiple elements have same name
6. **Start broad, then narrow** - Use `--all` to see all matches, then add filters
7. **Check visibility** - Offscreen elements exist but aren't actionable

## Troubleshooting

### "Element not found"

- Element might be offscreen - try scrolling first
- Name might not match exactly - try wildcards
- Element might be disabled - remove `:enabled` filter
- Element might be in different window - check window targeting

### "Multiple matches"

- Add pseudo-selectors like `:first` or `:nth(N)`
- Use more specific name matching
- Add state filters like `:enabled` or `:visible`
- Use spatial selectors to narrow by position
- Use automation ID if available

### "Selector matches wrong element"

- Use `--all` to see all matches
- Make name matching more specific (remove wildcards)
- Add state filters
- Use spatial selectors
- Check element hierarchy with `dump-tree`
