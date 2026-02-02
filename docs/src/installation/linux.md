# Linux Installation

## Prerequisites

### Rust Toolchain

Install Rust via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### System Dependencies

desktop-cli uses **enigo** for input simulation, which requires `libxdo-dev`:

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install libxdo-dev

# Fedora
sudo dnf install libxdo-devel

# Arch
sudo pacman -S xdotool
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

## AT-SPI2 Setup

desktop-cli uses **AT-SPI2** (Assistive Technology Service Provider Interface) for accessibility. Most modern Linux desktop environments (GNOME, KDE Plasma, XFCE) have AT-SPI2 enabled by default.

### Verify AT-SPI2 is Running

```bash
dbus-send --session --print-reply \
  --dest=org.a11y.Bus \
  /org/a11y/bus \
  org.freedesktop.DBus.Peer.Ping
```

If this command succeeds (returns without error), AT-SPI2 is running.

### Enable AT-SPI2 (if needed)

If AT-SPI2 is not running or applications don't expose accessibility information:

```bash
# Set GTK accessibility environment variables
export GTK_MODULES=gail:atk-bridge
export GTK_A11Y=atspi

# Add to ~/.bashrc or ~/.zshrc to make permanent
echo 'export GTK_MODULES=gail:atk-bridge' >> ~/.bashrc
echo 'export GTK_A11Y=atspi' >> ~/.bashrc
```

Then restart your applications for the changes to take effect.

## Verify It Works

```bash
# List all windows (requires X11 display)
desktop windows

# Get summary of an application
desktop summary firefox

# Click a button
desktop click firefox "@button 'New Tab'"
```

## Common Issues

### "Failed to connect to AT-SPI bus"

This means AT-SPI2 is not running. Make sure:
1. You're running in an X11 session (not just a TTY)
2. Your display environment variable is set: `echo $DISPLAY`
3. AT-SPI2 service is running (it starts automatically in most desktop environments)

### "Cannot find libxdo.so"

You need to install `libxdo-dev`:

```bash
sudo apt-get install libxdo-dev  # Ubuntu/Debian
```

Then rebuild desktop-cli:

```bash
cargo clean
cargo build --release
```

### Applications Don't Expose UI Elements

Some applications disable accessibility by default. For GTK applications:

```bash
GTK_MODULES=gail:atk-bridge GTK_A11Y=atspi your-app
```

For Qt applications, accessibility is usually enabled by default, but you can verify with:

```bash
export QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1
your-qt-app
```

## Next Steps

- [Tutorials](../tutorials/basics.md): Learn how to use desktop-cli
- [Commands Reference](../reference/commands.md): Explore all available commands
