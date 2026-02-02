# Introduction

**desktop-cli** is a cross-platform desktop automation tool optimized for LLM agents. It provides programmatic control over desktop applications through native accessibility APIs.

## What is desktop-cli?

desktop-cli enables automated interaction with desktop applications by exposing their UI elements through a unified command-line interface. Instead of pixel-based automation or scripting, it uses platform-native accessibility APIs to query and control applications semantically.

## Key Features

- **LLM-Optimized Output**: Compact, categorized UI summaries designed to maximize signal-to-noise ratio for AI agents
- **Enhanced Query Language**: Intuitive `@role` syntax for finding UI elements (e.g., `@button "Save"`, `@input:enabled`)
- **Smart Window Targeting**: Target windows by name, title, index, or let the CLI auto-disambiguate using element selectors
- **Semantic Role Detection**: Automatic classification of UI elements (button, input, menu, text, etc.)
- **Spatial Queries**: Find elements by position relative to other elements (e.g., `~below("Label") @input`)
- **Cross-Platform Support**: Consistent API across Windows, Linux, and macOS

## Platform Support

| Platform | Automation API | Status |
|----------|----------------|--------|
| **Windows** | UI Automation (UIA) | ✅ Full support |
| **Linux** | AT-SPI2 + X11 | ✅ Full support |
| **macOS** | Cocoa Accessibility | 🚧 Active development |

## Use Cases

desktop-cli is designed for:

- **LLM Agents**: AI assistants that need to control desktop applications programmatically
- **Automation Scripts**: Cross-platform automation workflows driven by accessibility APIs
- **Testing Tools**: Accessibility-based UI testing without brittle pixel coordinates
- **Complex Applications**: Control CAD tools, IDEs, design software, and other complex desktop apps

## Quick Example

```bash
# List all open windows
desktop windows

# Get a compact summary of an application's UI
desktop summary notepad

# Click a button using semantic queries
desktop click notepad "@button 'Save'"

# Type into an input field
desktop type notepad "@input" --value "Hello World"

# Smart disambiguation: finds the right window automatically
desktop click vscode "@button 'Run'"
```

## Example Output

When you run `desktop summary notepad`, you get LLM-optimized output like:

```
# Notepad

## Actions
[b1] button: "Save" (click)
[b2] button: "Cancel" (click)
[i1] input: "File name" (type)

## Navigation
[m1] menu: "File" (click)
[m2] menu: "Edit" (click)

Stats: 45 total, 32 visible, 8 actionable
```

## Architecture

desktop-cli uses platform-native APIs for maximum reliability:

- **Windows**: UI Automation (UIA) for element trees and SendInput for actions
- **Linux**: AT-SPI2 for accessibility, X11 for window management, enigo for input
- **macOS**: Cocoa Accessibility (AXUIElement) for element trees, CGEvent for input

All platforms expose the same command-line interface, with platform differences handled transparently.

## Next Steps

- [Installation](installation/linux.md): Get desktop-cli installed on your platform
- [Tutorials](tutorials/basics.md): Learn the basics with hands-on examples
- [Reference](reference/commands.md): Explore all available commands and options
- [Contributing](contributing.md): Help improve desktop-cli
