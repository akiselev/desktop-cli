# Showcase: Automating VS Code

This tutorial walks through automating Visual Studio Code with desktop-cli, demonstrating real-world usage for LLM agents.

> **Prerequisites**: VS Code must be installed and running. This tutorial uses static example output and is not auto-generated.

## Step 1: Find the VS Code Window

```bash
desktop windows --exe code
```

Example output:
```
[:1] code - Welcome - Visual Studio Code (hwnd:0x2a00042, pid:4521)
[:2] code - main.rs - desktop-cli - Visual Studio Code (hwnd:0x2a00088, pid:4521)
```

Target a specific window by title:

```bash
desktop summary "title:main.rs"
```

## Step 2: Open the Command Palette

```bash
desktop keys code "ctrl+shift+p"
```

```
Keys sent successfully
```

Verify the palette opened:

```bash
desktop summary code --roles input
```

Example output:
```json
{
  "window": "Visual Studio Code",
  "actions": [
    {"ref_id": "i1", "role": "input", "label": "Command Palette", "action": "type"}
  ]
}
```

## Step 3: Open a File via Command Palette

Type into the command palette to open a file:

```bash
desktop type code "@input 'Command Palette'" --value "Open File"
```

```
Text typed successfully
```

Then press Enter to execute:

```bash
desktop keys code "return"
```

## Step 4: Navigate to a Specific Line

Use Ctrl+G to open the "Go to Line" dialog:

```bash
desktop keys code "ctrl+g"
```

Type the line number:

```bash
desktop type code "@input" --value "42"
desktop keys code "return"
```

## Step 5: Verify via Status Bar

Query the status bar to confirm the cursor position:

```bash
desktop query code "@statusbar" --all
```

Or get a summary focused on the editor area:

```bash
desktop summary code --roles input,text --region 0,600,1920,100
```

## Complete Workflow

Here's the full sequence an LLM agent would use:

```bash
# 1. Find VS Code
desktop windows --exe code

# 2. Get initial state
desktop summary "title:main.rs"

# 3. Open command palette
desktop keys "title:main.rs" "ctrl+shift+p"

# 4. Search for a command
desktop type "title:main.rs" "@input" --value "Toggle Word Wrap"
desktop keys "title:main.rs" "return"

# 5. Verify the action
desktop summary "title:main.rs"
```

## Tips for LLM Agents

- **Always check state after actions**: Use `desktop summary` after each interaction to verify the UI responded as expected.
- **Use title targeting**: When multiple VS Code windows are open, use `title:filename` to target the right one.
- **Key combos**: VS Code uses many keyboard shortcuts. Common ones:
  - `ctrl+shift+p` — Command Palette
  - `ctrl+p` — Quick Open (files)
  - `ctrl+g` — Go to Line
  - `ctrl+shift+f` — Find in Files
  - `ctrl+backtick` — Toggle Terminal
- **Escape to dismiss**: If a dialog is in the way, `desktop keys code "escape"` dismisses it.
