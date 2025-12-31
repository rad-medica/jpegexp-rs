# VSCode Configuration Optimizations

This document describes the optimizations applied to the workspace for building and debugging.

## Build Optimizations

### Cargo Configuration (`.cargo/config.toml`)
- **Incremental compilation**: Enabled for faster rebuilds
- **Parallel compilation**: Uses all available CPU cores
- **Optimized dev profile**: Dependencies are optimized (opt-level 3) while your code remains unoptimized (opt-level 0) for faster iteration
- **Test profile**: Slightly optimized (opt-level 1) for faster test execution
- **Benchmark profile**: Fully optimized with LTO for accurate benchmarks

### Cargo.toml Profiles
- **Dev profile**: Optimized dependencies, unoptimized main code for fast debug builds
- **Test profile**: Slightly optimized for faster test runs while maintaining debug info

## Debugging Optimizations

### Launch Configurations (`.vscode/launch.json`)
- **Pre-launch tasks**: All debug configurations now build before launching
- **Environment variables**: 
  - `RUST_BACKTRACE=1` or `full` for better error traces
  - `RUST_TEST_NOCAPTURE=1` for test output visibility
- **Integrated terminal**: All debug sessions use integrated terminal for better output

### Debug Configurations Available:
1. **Debug CLI (jpegexp)** - Debug the main CLI tool
2. **Debug CLI with args** - Debug with custom input/output files (prompts for paths)
3. **Debug Tests** - Debug all tests
4. **Debug Current Test** - Debug a specific test (prompts for test name)
5. **Debug Bench (bench_idct)** - Debug the benchmark binary
6. **Debug Library (attach)** - Attach to a running process

## Rust Analyzer Optimizations

### Settings (`.vscode/settings.json`)
- **Separate target directory**: `target/rust-analyzer` prevents conflicts with regular builds
- **Inlay hints**: Enabled for types, parameters, and chaining hints
- **Auto-import**: Enabled for better developer experience
- **Import granularity**: Grouped by module for cleaner imports
- **Clippy on save**: Runs with `-D warnings` to catch issues early

## Task Optimizations

### Build Tasks (`.vscode/tasks.json`)
- **Default build task**: Set to debug build (Ctrl+Shift+B)
- **Problem matchers**: All Rust tasks use `$rustc` for proper error highlighting
- **Environment variables**: Test tasks include `RUST_BACKTRACE` for better error output
- **Presentation**: Optimized reveal settings for better workflow

## Performance Tips

1. **First build**: May take longer, but subsequent builds are incremental
2. **rust-analyzer**: Uses separate target directory to avoid build conflicts
3. **Dependency optimization**: Dependencies are optimized even in dev mode for faster builds
4. **Parallel compilation**: Uses all CPU cores by default

## Troubleshooting

### Debugging not working?
- Ensure CodeLLDB extension is installed (`vadimcn.vscode-lldb`)
- Check that the binary exists: `target/debug/jpegexp.exe`
- Verify pre-launch task completed successfully

### Slow builds?
- Check `.cargo/config.toml` has `incremental = true`
- Ensure you're using the optimized dev profile
- Consider using `cargo build --release` only when needed

### rust-analyzer conflicts?
- The separate target directory (`target/rust-analyzer`) should prevent conflicts
- If issues persist, restart rust-analyzer: `Ctrl+Shift+P` → "rust-analyzer: Restart server"

