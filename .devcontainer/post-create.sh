#!/bin/bash
set -e

echo "🚀 Setting up jpegexp-rs development environment..."

# Install Rust toolchain components
echo "📦 Installing Rust components..."
rustup component add rustfmt clippy rust-src

# Install cargo tools
echo "🔧 Installing cargo tools..."
cargo install cargo-edit cargo-watch cargo-audit || true

# Install Python dependencies for testing
echo "🐍 Installing Python dependencies..."
pip install --user numpy pillow imagecodecs

# Pre-build the project to cache dependencies
echo "🏗️  Pre-building project (this may take a few minutes)..."
cargo build --release || echo "⚠️  Initial build failed, but you can retry later"

# Run tests to verify setup
echo "🧪 Running initial tests..."
cargo test --lib || echo "⚠️  Some tests failed, but environment is ready"

echo "✅ Development environment setup complete!"
echo ""
echo "Quick start commands:"
echo "  cargo build --release       # Build in release mode"
echo "  cargo test                  # Run tests"
echo "  cargo clippy                # Run linter"
echo "  cargo run --bin jpegexp     # Run CLI tool"
echo "  python3 tests/comprehensive_test.py  # Run comprehensive codec tests"
echo ""
echo "Happy coding! 🦀"
