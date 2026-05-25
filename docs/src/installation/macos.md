# macOS Installation

## Prerequisites

### Rust Toolchain

Install Rust via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Xcode Command Line Tools (Optional)

Most dependencies are handled by Cargo, but having Xcode Command Line Tools can help:

```bash
xcode-select --install
```

## Installation

### Option 1: Install from Crates.io

```bash
cargo install desktop-cli
```

### Option 2: Build from Source

```bash
git clone https://github.com/akiselev/desktop-cli.git
cd desktop-cli
cargo build --release

# Binary will be at target/release/desktop
sudo cp target/release/desktop /usr/local/bin/
```

## Verify Installation

```bash
desktop --version
```

## Accessibility Permissions

**IMPORTANT**: macOS requires explicit permission for applications to use accessibility APIs.

### Grant Permissions

1. Open **System Preferences** (or **System Settings** on macOS 13+)
2. Navigate to **Privacy & Security** → **Accessibility**
3. Click the **lock icon** to make changes (enter your password)
4. Click the **+** button and add your terminal application:
   - **Terminal.app** (if using built-in Terminal)
   - **iTerm.app** (if using iTerm2)
   - Or whichever terminal you're using

Alternatively, if you run `desktop` without permissions, macOS will show a permission prompt automatically.

### Verify Permissions

```bash
desktop windows
```

If you see a list of windows, permissions are granted. If you see an error like:

```
Error: Accessibility permissions not granted
```

Then you need to grant permissions as described above.

## Verify It Works

```bash
# List all windows
desktop windows

# Get summary of an application
desktop summary safari

# Click a button
desktop click safari "@button 'New Tab'"
```

## macOS Support Status

macOS support is **in active development**. Current status:

| Feature | Status |
|---------|--------|
| Window listing | ✅ Working |
| Element tree queries | ✅ Working |
| Click/Type actions | ✅ Working |
| Keyboard shortcuts | ✅ Working |
| Screenshots | 🚧 In progress |
| Advanced selectors | 🚧 In progress |

Some complex applications may have limited accessibility support. Native macOS applications (Safari, TextEdit, Finder) generally work well, while some third-party apps may have incomplete accessibility APIs.

## Common Issues

### "Accessibility permissions not granted"

Grant accessibility permissions in System Preferences as described above. You need to:
1. Add your terminal app to Accessibility list
2. Ensure the checkbox is **checked**
3. Restart your terminal after granting permissions

### "Cannot find window"

Some applications don't expose window information through accessibility APIs. Try:
1. Making sure the window is not minimized
2. Checking if the application is actively running
3. Using `desktop windows` to see what's available

### "Element not found"

macOS accessibility support varies by application:
- **Native apps** (Safari, TextEdit, Mail): Usually have excellent accessibility
- **Electron apps** (VS Code, Slack): Good accessibility support
- **Legacy apps**: May have limited or no accessibility support

## Supported macOS Versions

- ✅ macOS 14 (Sonoma)
- ✅ macOS 13 (Ventura)
- ✅ macOS 12 (Monterey)
- ⚠️ macOS 11 (Big Sur): Should work but not regularly tested

## Next Steps

- [Tutorials](../tutorials/basics.md): Learn how to use desktop-cli
- [Commands Reference](../reference/commands.md): Explore all available commands
- [Contributing](../contributing.md): Help improve macOS support
