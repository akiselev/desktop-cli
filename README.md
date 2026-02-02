# Desktop CLI

[![CI](https://github.com/akiselev/desktop-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/akiselev/desktop-cli/actions/workflows/ci.yml)
[![Documentation](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://akiselev.github.io/desktop-cli/)
[![License: GPL-3.0-only](https://img.shields.io/badge/license-GPL--3.0--only-green)](LICENSE)

A cross-platform desktop automation CLI optimized for LLM agents. Control desktop applications through native accessibility APIs on Windows, Linux, and macOS.

## Platform Support

| Platform | Accessibility API | Input Simulation | Status |
|----------|------------------|------------------|--------|
| Windows  | UI Automation (UIA) | SendInput | Stable |
| Linux    | AT-SPI2 + X11 | enigo (libxdo) | Stable |
| macOS    | Cocoa Accessibility | enigo (CGEvent) | In Development |

## Quick Start

```bash
# Install
cargo install desktop-cli

# List windows
desktop windows

# Get a compact UI summary (optimized for LLM consumption)
desktop summary notepad
```

## Features

- **Cross-Platform**: Native accessibility APIs on Windows, Linux, and macOS
- **LLM-Optimized Output**: Compact, categorized UI summaries that maximize signal-to-noise ratio
- **Enhanced Query Language**: Intuitive `@role` selector syntax designed for AI agents
- **Smart Window Targeting**: Target windows by name, title, index, or let the CLI auto-disambiguate using element selectors
- **Semantic Role Detection**: Automatic classification of UI elements (button, input, menu, etc.)
- **Spatial Queries**: Find elements by position relative to other elements

## Usage

```bash
# Discover windows
desktop windows --exe firefox

# Get UI state (use after every action)
desktop summary :1

# Find elements
desktop query notepad "@button 'Save'"

# Interact
desktop click notepad "@button 'Save'"
desktop type notepad "#editor" --value "Hello World"
desktop keys notepad "ctrl+s"
desktop scroll notepad down --amount 5

# Combined action
desktop do notepad click "@button 'Save'"
```

## Window Targeting

| Query | Description |
|-------|-------------|
| `:1`, `:2` | Window by index |
| `notepad` | Match by executable name |
| `title:PCB` | Match by window title |
| `hwnd:0x1234` | Match by HWND |
| `pid:12345` | Match by process ID |
| `title:*Draft*` | Wildcard matching |

When multiple windows match, the CLI tries the element selector on each and auto-selects the right one.

## For LLM Agents

Desktop CLI is designed as a tool for LLM agents to control desktop applications:

1. **Start with `desktop windows`** to discover available windows
2. **Use `desktop summary`** after every action to understand UI state changes
3. **Use `@role` selectors** (`@button "Save"`, `@input:first`) for reliable element targeting
4. **Use `desktop do`** for combined find-and-act operations
5. **Use `--json` output** for machine-readable responses

See [AGENT.md](AGENT.md) for detailed LLM agent instructions.

## Documentation

Full documentation is available at [akiselev.github.io/desktop-cli](https://akiselev.github.io/desktop-cli/).

- [Installation](https://akiselev.github.io/desktop-cli/installation/linux.html)
- [Tutorials](https://akiselev.github.io/desktop-cli/tutorials/basics.html)
- [Command Reference](https://akiselev.github.io/desktop-cli/reference/commands.html)
- [Selector Reference](https://akiselev.github.io/desktop-cli/reference/selectors.html)

## Development

```bash
# Build
cargo build

# Unit tests
cargo test --lib

# Linux E2E tests (Docker)
docker build -t desktop-cli-test .
docker run --rm --init desktop-cli-test
```

## License

GPL-3.0-only
