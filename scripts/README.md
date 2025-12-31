# Build Scripts

PowerShell scripts for common development tasks.

## Available Scripts

### `build.ps1`
Build the project in various modes.

```powershell
.\scripts\build.ps1          # Build debug (default)
.\scripts\build.ps1 debug     # Build debug
.\scripts\build.ps1 release   # Build release
.\scripts\build.ps1 all       # Build debug + release + Python bindings
.\scripts\build.ps1 cli       # Build CLI binary (release)
.\scripts\build.ps1 python    # Build Python bindings
```

### `test.ps1`
Run tests.

```powershell
.\scripts\test.ps1            # Run all tests (default)
.\scripts\test.ps1 all        # Run Rust + Python tests
.\scripts\test.ps1 unit       # Run Rust unit tests only
.\scripts\test.ps1 verbose    # Run tests with output
```

### `check.ps1`
Run code quality checks.

```powershell
.\scripts\check.ps1           # Run all checks (default)
.\scripts\check.ps1 all       # Format + Clippy + Tests
.\scripts\check.ps1 fmt        # Format check only
.\scripts\check.ps1 clippy    # Clippy only
.\scripts\check.ps1 test      # Tests only
```

### `clean.ps1`
Clean build artifacts.

```powershell
.\scripts\clean.ps1           # Clean everything (default)
.\scripts\clean.ps1 all        # Clean Cargo + Python artifacts
.\scripts\clean.ps1 cargo     # Clean Cargo artifacts only
.\scripts\clean.ps1 python    # Clean Python artifacts only
```

### `run.ps1`
Build and run the CLI tool.

```powershell
.\scripts\run.ps1                    # Show help
.\scripts\run.ps1 decode -i file.jpg -o output.raw
.\scripts\run.ps1 encode -i input.raw -o output.jls -w 512 -h 512 -c jpegls
```

## VSCode Integration

The project includes VSCode tasks and launch configurations:

- **Tasks**: Press `Ctrl+Shift+P` → "Tasks: Run Task" → Select a task
- **Debug**: Press `F5` or use the Debug panel to select a configuration

See `.vscode/tasks.json` and `.vscode/launch.json` for details.

