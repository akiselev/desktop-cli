# Desktop CLI

[![CI](https://github.com/akiselev/desktop-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/akiselev/desktop-cli/actions/workflows/ci.yml)
[![Documentation](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://akiselev.github.io/desktop-cli/)
[![License: GPL-3.0-only](https://img.shields.io/badge/license-GPL--3.0--only-green)](LICENSE)

> **Experimental.** The command and selector interfaces may change while platform support is still being developed.

A cross-platform desktop automation CLI built on native accessibility APIs for Windows, Linux, and macOS. It exposes compact summaries, structured output, selectors, and input actions suitable for scripts and other automation.

## Platform support

| Platform | Accessibility API | Input Simulation | Status |
|----------|------------------|------------------|--------|
| Windows  | UI Automation (UIA) | SendInput | Stable |
| Linux    | AT-SPI2 + X11 | enigo (libxdo) | Stable |
| macOS    | Cocoa Accessibility | enigo (CGEvent) | In Development |

## Quick start

```bash
# Install
cargo install desktop-cli

# List windows
desktop windows

# Get a compact UI summary
desktop summary notepad
```

## Features

- **Cross-platform accessibility**: native accessibility APIs on Windows, Linux, and macOS
- **Compact structured output**: categorized UI summaries plus JSON output for programmatic use
- **Selector language**: `@role` syntax for finding UI elements
- **Window targeting**: target windows by name, title, index, or element selector
- **Semantic role detection**: classify UI elements such as buttons, inputs, and menus
- **Spatial queries**: find elements by position relative to other elements

## Usage

```bash
# Discover windows
desktop windows --exe firefox

# Get UI state
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

## Window targeting

| Query | Description |
|-------|-------------|
| `:1`, `:2` | Window by index |
| `notepad` | Match by executable name |
| `title:PCB` | Match by window title |
| `hwnd:0x1234` | Match by HWND |
| `pid:12345` | Match by process ID |
| `title:*Draft*` | Wildcard matching |

When multiple windows match, the CLI tries the element selector on each and auto-selects the right one.

## Automation workflow

A typical programmatic workflow is:

1. Use `desktop windows` to discover available windows.
2. Use `desktop summary` after actions to inspect UI state changes.
3. Use `@role` selectors such as `@button "Save"` or `@input:first` for element targeting.
4. Use `desktop do` for combined find-and-act operations.
5. Use `--json` for machine-readable responses.

See [AGENT.md](AGENT.md) for additional automation guidance.

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
