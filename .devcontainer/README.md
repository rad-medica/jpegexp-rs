# DevContainer Configuration

This directory contains the DevContainer configuration for developing jpegexp-rs in GitHub Codespaces or locally with VS Code.

## What's Included

### Base Image
- **Microsoft DevContainers Rust** (Debian Bullseye)
- Pre-configured Rust toolchain with cargo

### Additional Features
- **Python 3.11** - For running test scripts
- **Git** - Version control
- **GitHub CLI** - GitHub integration

### VS Code Extensions
- **rust-analyzer** - Rust language support
- **Even Better TOML** - TOML file support
- **Crates** - Cargo.toml dependency management
- **CodeLLDB** - Rust debugging
- **Python** - Python language support
- **Pylance** - Python type checking

### Installed Tools
- `rustfmt` - Code formatting
- `clippy` - Rust linter
- `rust-src` - Rust source code (for rust-analyzer)
- `cargo-edit` - Manage Cargo.toml dependencies
- `cargo-watch` - Watch for changes and rebuild
- `cargo-audit` - Security vulnerability scanning

### Python Packages
- `numpy` - Numerical computing
- `pillow` - Image processing
- `imagecodecs` - Standard codec implementations for testing

## Usage

### GitHub Codespaces
1. Go to the repository on GitHub
2. Click the green "Code" button
3. Select "Codespaces" tab
4. Click "Create codespace on [branch]"
5. Wait for the environment to build (first time may take 5-10 minutes)
6. Start coding!

### VS Code with Dev Containers Extension
1. Install the "Dev Containers" extension in VS Code
2. Open the repository folder
3. VS Code will prompt to "Reopen in Container"
4. Click "Reopen in Container"
5. Wait for the build to complete

### Manual Setup
If the post-create script fails, you can manually run:

```bash
# Install Rust components
rustup component add rustfmt clippy rust-src

# Install cargo tools
cargo install cargo-edit cargo-watch cargo-audit

# Install Python dependencies
pip install --user numpy pillow imagecodecs

# Build the project
cargo build --release
```

## Development Workflow

```bash
# Build in release mode
cargo build --release

# Run tests
cargo test

# Run linter
cargo clippy

# Format code
cargo fmt

# Watch for changes and rebuild
cargo watch -x build

# Run comprehensive codec tests
python3 tests/comprehensive_test.py

# Run CLI tool
cargo run --bin jpegexp -- help
```

## Troubleshooting

### Build Takes Too Long
The first build caches all dependencies and can take 5-10 minutes. Subsequent builds will be much faster.

### Python Tests Fail
Ensure Python dependencies are installed:
```bash
pip install --user numpy pillow imagecodecs
```

### Rust Analyzer Not Working
Try reloading VS Code or rebuilding the project:
```bash
cargo clean
cargo build
```

## Performance Notes

- The `target/` directory is mounted as a volume for faster builds
- Rust analyzer uses clippy for additional checks
- Auto-formatting is enabled on save for Rust files

## Customization

You can customize the devcontainer by editing `.devcontainer/devcontainer.json`:
- Add more VS Code extensions
- Change Python version
- Add additional features
- Modify environment variables
