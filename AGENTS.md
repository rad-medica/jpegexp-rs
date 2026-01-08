# Agent Guidelines for jpegexp-rs

This document provides essential instructions, commands, and standards for AI agents working on this codebase.

## 🚀 Quick Start & Verification

### Build & Test
- **Build**: `cargo build` (Debug) or `cargo build --release` (Release)
- **Run All Tests**: `cargo test --release` (Release mode recommended for codec performance)
- **Run Specific Test**: `cargo test --release --test <test_file> <test_fn_name> -- --nocapture`
  - Example: `cargo test --release --test final_interop test_grayscale_interop`
- **Run Long Tests**: `cargo test --release -- --ignored` (Runs exhaustive benchmarks)
- **Lint**: `cargo clippy -- -D warnings`
- **Format**: `cargo fmt` (Always run before committing)

### Scripts
- **Windows**: Use PowerShell scripts in `scripts/` (e.g., `scripts/test.ps1`).
- **Interop**: `scripts/test_interop.sh` runs binary compatibility checks against OpenJPEG/OpenHTJ2K.

## 🏗️ Codebase Structure
- `src/`: Core library code.
  - `jpeg1/`, `jpegls/`, `jpeg2000/`: Codec-specific implementations.
  - `dicom/`: DICOM encapsulation logic.
- `tests/`: Integration tests, unit tests, and regression suites organized by category.
- `tests/fixtures/`: Centralized test data and reference images.
- `tests/unit/`, `tests/integration/`, `tests/regression/`, `tests/interop/`: Categorized Rust tests.
- `tests/scripts/`: Python utility scripts.
- `tests/notebooks/`: Jupyter notebooks for analysis.
- `benches/`: Criterion benchmarks.
- `libs/`: External reference implementations (OpenJPEG, OpenHTJ2K, CharLS, libjpeg-turbo) source and binaries.
- `libs/bin/`: Centralized location for external executables.
- `docs/`: Comprehensive documentation and project status.

## 📝 Coding Standards

### Rust Style
- **Formatting**: Strictly follow `rustfmt` (4 spaces indent).
- **Naming**: `snake_case` for functions/variables/modules, `PascalCase` for types/traits/enums.
- **Imports**: Group imports at the top. Prefer `use crate::...` for internal modules.
- **Safety**: `#![forbid(unsafe_code)]` where possible. Isolate `unsafe` blocks with justification comments.

### Error Handling
- Use `Result<T, JpeglsError>` for fallible operations.
- Define errors in `src/error.rs` using `thiserror`.
- Avoid `unwrap()`/`expect()` in library code. Propagate errors.
- Use `?` operator for clean error propagation.

### Documentation
- **Public API**: All public structs, enums, and functions MUST have `///` doc comments.
- **Examples**: Include usage examples in doc comments where appropriate.
- **Updates**: Update `docs/status.md` and `docs/todo.md` when completing major tasks.

## 🧪 Testing Guidelines
- **Unit Tests**: Place in `mod tests` at the bottom of the source file or in `tests/unit/`.
- **Integration Tests**: Place in `tests/integration/`. Use `tests/fixtures/` for test data.
- **Interop**: When modifying encoders, verify output against reference decoders (OpenJPEG/CharLS) in `tests/interop/`.
- **Regression**: If fixing a bug, add a regression test case in `tests/regression/`.

## ⚠️ Critical Constraints
- **Performance**: This is a high-performance codec library. Avoid unnecessary allocations in hot loops. Use buffer pooling or in-place operations where feasible.
- **Compliance**: Adhere strictly to ISO standards (JPEG 1, LS, 2000). Refer to `docs/compliance/` for details.
- **No Std**: Ensure core logic supports `no_std` if required (currently `std` is used, but keep dependencies minimal).

## 🤖 Agent Workflow
1. **Analyze**: Check `docs/status.md` and `docs/todo.md` before starting.
2. **Plan**: outline changes.
3. **Implement**: Write code + unit tests.
4. **Verify**: Run related tests + `clippy` + `fmt`.
5. **Update Docs**: Reflect changes in documentation.
