# Multi-stage Dockerfile for Linux AT-SPI2 testing
# Builder stage: Compile Rust project
# Runtime stage: Test with Xvfb + D-Bus + AT-SPI2

# =============================================================================
# BUILDER STAGE
# =============================================================================
FROM rust:1.84 AS builder

WORKDIR /app

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn dummy() {}" > src/lib.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# Copy actual source
COPY src ./src
COPY tests ./tests

# Build release binary and tests
RUN cargo build --release && \
    cargo build --release --tests

# =============================================================================
# RUNTIME STAGE
# =============================================================================
FROM ubuntu:22.04 AS runtime

# Prevent interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install AT-SPI2, X11, D-Bus, GTK3 runtime and build dependencies
# at-spi2-core: AT-SPI2 registry service
# libatk1.0-0, libatk-bridge2.0-0: ATK accessibility bridge
# libgtk-3-0: GTK3 runtime for test app
# xvfb, dbus-x11: Virtual X11 and D-Bus integration
# libx11-6, libxcb1: X11 client libraries
# gcc, make, pkg-config, libgtk-3-dev: Build tools for GTK test app
RUN apt-get update && apt-get install -y --no-install-recommends \
    at-spi2-core \
    libatk1.0-0 \
    libatk-bridge2.0-0 \
    libgtk-3-0 \
    libgtk-3-dev \
    xvfb \
    dbus-x11 \
    libx11-6 \
    libxcb1 \
    gcc \
    make \
    pkg-config \
    ca-certificates \
    dconf-cli \
    gsettings-desktop-schemas \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy built artifacts from builder
COPY --from=builder /app/target/release/desktop /app/
COPY --from=builder /app/target/release/deps/desktop_cli-* /app/tests/

# Copy test fixtures
COPY tests/fixtures /app/tests/fixtures

# Build GTK test app
RUN cd /app/tests/fixtures/gtk_test_app && make

# Enable accessibility (required for AT-SPI2)
# Run in dbus-run-session to have proper session bus
RUN mkdir -p /root/.config/dconf

# Environment for AT-SPI2 and GTK accessibility
ENV GTK_MODULES=gail:atk-bridge
ENV GTK_A11Y=atspi
ENV NO_AT_BRIDGE=0

# Test runner script that sets up Xvfb + D-Bus + accessibility
COPY <<'EOF' /app/run-tests.sh
#!/bin/bash
set -e

# Start D-Bus session and run tests inside it
exec dbus-run-session -- bash -c '
    # Enable accessibility
    gsettings set org.gnome.desktop.interface toolkit-accessibility true 2>/dev/null || true

    # Run the test binary
    exec "$@"
' _ "$@"
EOF
RUN chmod +x /app/run-tests.sh

# Default: run tests with Xvfb (auto-selects display) and single-threaded (avoids races)
ENTRYPOINT ["xvfb-run", "-a", "/app/run-tests.sh"]
CMD ["./tests/desktop_cli-*", "--test-threads=1"]
