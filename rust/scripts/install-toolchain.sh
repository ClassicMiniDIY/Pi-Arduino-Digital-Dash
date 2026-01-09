#!/bin/bash
#
# Arduino Digital Dash - Rust Toolchain Installation Script
#
# Installs all dependencies needed to build the Arduino firmware and Pi receiver:
# - Rust (stable + nightly)
# - AVR toolchain (gcc, libc, avrdude)
# - ravedude (automated flashing)
#
# Usage: ./install-toolchain.sh
#

set -e  # Exit on error

echo "========================================"
echo "Arduino Digital Dash - Toolchain Setup"
echo "========================================"
echo ""

# Detect OS
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macos"
else
    echo "❌ Unsupported OS: $OSTYPE"
    echo "This script supports Linux and macOS only."
    exit 1
fi

echo "Detected OS: $OS"
echo ""

# ========================================
# Step 1: Install Rust
# ========================================
echo "Step 1: Installing Rust..."
if command -v rustc &> /dev/null; then
    echo "✓ Rust is already installed ($(rustc --version))"
else
    echo "Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo "✓ Rust installed successfully"
fi
echo ""

# ========================================
# Step 2: Install Rust Nightly
# ========================================
echo "Step 2: Installing Rust nightly toolchain..."
rustup install nightly
rustup component add rust-src --toolchain nightly
echo "✓ Rust nightly installed with rust-src component"
echo ""

# ========================================
# Step 3: Install AVR Toolchain
# ========================================
echo "Step 3: Installing AVR toolchain..."
if [[ "$OS" == "macos" ]]; then
    # macOS (Homebrew)
    if ! command -v brew &> /dev/null; then
        echo "❌ Homebrew not found. Please install Homebrew first:"
        echo "   /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
        exit 1
    fi

    echo "Installing AVR toolchain via Homebrew..."
    brew tap osx-cross/avr || true
    brew install avr-gcc avrdude
    echo "✓ AVR toolchain installed (macOS)"

elif [[ "$OS" == "linux" ]]; then
    # Linux (detect package manager)
    if command -v apt-get &> /dev/null; then
        echo "Installing AVR toolchain via apt..."
        sudo apt-get update
        sudo apt-get install -y gcc-avr avr-libc avrdude
        echo "✓ AVR toolchain installed (Debian/Ubuntu)"

    elif command -v pacman &> /dev/null; then
        echo "Installing AVR toolchain via pacman..."
        sudo pacman -S --noconfirm avr-gcc avr-libc avrdude
        echo "✓ AVR toolchain installed (Arch Linux)"

    elif command -v dnf &> /dev/null; then
        echo "Installing AVR toolchain via dnf..."
        sudo dnf install -y avr-gcc avr-libc avrdude
        echo "✓ AVR toolchain installed (Fedora/RHEL)"

    else
        echo "❌ Unknown package manager. Please install manually:"
        echo "   - gcc-avr"
        echo "   - avr-libc"
        echo "   - avrdude"
        exit 1
    fi
fi
echo ""

# ========================================
# Step 4: Install ravedude
# ========================================
echo "Step 4: Installing ravedude (automated flashing tool)..."
cargo install ravedude --quiet
echo "✓ ravedude installed"
echo ""

# ========================================
# Step 5: Verify Installation
# ========================================
echo "Step 5: Verifying installation..."
echo ""

echo "Rust version:"
rustc --version
echo ""

echo "Rust nightly:"
rustup toolchain list | grep nightly || echo "❌ Nightly not found"
echo ""

echo "AVR GCC version:"
avr-gcc --version | head -n1
echo ""

echo "avrdude version:"
avrdude -? 2>&1 | head -n1 || echo "✓ avrdude installed"
echo ""

echo "ravedude:"
if command -v ravedude &> /dev/null; then
    echo "✓ ravedude installed at $(which ravedude)"
else
    echo "❌ ravedude not found in PATH"
    echo "   You may need to add ~/.cargo/bin to your PATH:"
    echo "   export PATH=\"\$HOME/.cargo/bin:\$PATH\""
fi
echo ""

# ========================================
# Step 6: Post-Installation Notes
# ========================================
echo "========================================"
echo "✓ Installation Complete!"
echo "========================================"
echo ""
echo "Next steps:"
echo ""
echo "1. Build Arduino firmware:"
echo "   cd crates/arduino-firmware"
echo "   cargo build --release"
echo ""
echo "2. Flash to Arduino:"
echo "   cargo run --release"
echo ""
echo "3. Build Pi receiver:"
echo "   cd ../pi-receiver"
echo "   cargo build --release"
echo ""
echo "4. Query Arduino:"
echo "   cargo run -- --port /dev/ttyACM0 query"
echo ""
echo "For detailed instructions, see:"
echo "  - docs/BUILDING.md"
echo "  - docs/HARDWARE.md"
echo "  - docs/PROTOCOL.md"
echo ""

# Linux-specific: Add user to dialout group
if [[ "$OS" == "linux" ]]; then
    echo "Linux users: You may need to add your user to the 'dialout' group:"
    echo "  sudo usermod -a -G dialout \$USER"
    echo "  (then log out and back in)"
    echo ""
fi

echo "Happy building! 🚀"
