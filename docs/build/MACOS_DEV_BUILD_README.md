# macOS Development Build

**Purpose**: Quick development workflow with hot reload for rapid iteration on macOS.

**Time**: ~30 seconds for initial build, ~1-5 seconds for incremental rebuilds

**Prerequisites**: [MACOS_DEPENDENCIES_README.md](MACOS_DEPENDENCIES_README.md) completed

---

## Quick Start

```bash
# Clone repository (if not already done)
git clone https://github.com/your-org/blazor-agile-board.git
cd blazor-agile-board/desktop

# Run development build
cargo tauri dev
```

The app will launch with:
- ✅ Hot reload enabled (Blazor changes auto-refresh)
- ✅ Debug symbols included
- ✅ Faster compilation (unoptimized)
- ✅ Development logging enabled
- ✅ WebView DevTools accessible (right-click → Inspect Element)

---

## First-Time Setup

### 1. Clone Repository

```bash
git clone https://github.com/your-org/blazor-agile-board.git
cd blazor-agile-board
```

### 2. Verify Dependencies

```bash
rustc --version   # Should be 1.93.0+
dotnet --version  # Should be 10.0.x
cargo tauri --version
```

If any command fails, see [MACOS_DEPENDENCIES_README.md](MACOS_DEPENDENCIES_README.md).

### 3. Initial Build

```bash
cd desktop
cargo tauri dev
```

**First build takes ~2-5 minutes** (compiles all dependencies, one-time cost).

**Expected output**:
```
    Compiling pm-core v0.1.0
    Compiling pm-db v0.1.0
    ...
    Finished dev [unoptimized + debuginfo] target(s) in 2m 34s
    Running frontend/ProjectManagement.Wasm
    ...
Opening app...
```

The desktop application window will open automatically.

---

## Development Workflow

### Hot Reload (Blazor)

1. Keep `cargo tauri dev` running
2. Edit any `.razor`, `.cs`, or `.css` file in `frontend/`
3. Save the file
4. **App auto-refreshes** within 1-2 seconds

**Example**:
```bash
# Terminal 1: Keep this running
cd desktop
cargo tauri dev

# Terminal 2: Edit files
cd frontend/ProjectManagement.Components/Pages
# Edit Home.razor, save
# → App refreshes automatically
```

### Rust Backend Changes

If you modify Rust code in `desktop/src-tauri/` or `backend/`:

1. Save your changes
2. Tauri detects the change and **auto-recompiles**
3. App restarts automatically (preserves app state via SQLite)

**Incremental rebuilds**: ~5-10 seconds (only changed crates recompile)

### Database Changes

Development database location:
```
desktop/.pm/data.db
```

**Reset database** (for testing migrations):
```bash
# Stop the app (Cmd+Q)
rm -rf desktop/.pm/data.db
cargo tauri dev  # Recreates DB with fresh migrations
```

**Inspect database**:
```bash
# Install SQLite CLI if needed
brew install sqlite  # or download from sqlite.org

# Open database
sqlite3 desktop/.pm/data.db
sqlite> .tables
sqlite> SELECT * FROM pm_work_items;
sqlite> .quit
```

---

## Development Features

### WebView DevTools

With the app running:
1. Right-click anywhere in the app
2. Select **"Inspect Element"**
3. Chrome DevTools open

**Use for**:
- Debugging Blazor components
- Network tab (WebSocket messages)
- Console logs
- Performance profiling

### Backend Logs

Logs are written to:
```
desktop/.pm/logs/app.log
```

**View logs** (live):
```bash
tail -f desktop/.pm/logs/app.log
```

**Log levels**: Controlled in `desktop/.pm/config.toml`:
```toml
[logging]
level = "debug"  # trace, debug, info, warn, error
```

### Debug Symbols

Dev builds include full debug symbols for:
- Rust debugging with `lldb` or VS Code
- Better panic stack traces
- Profiling with Instruments.app

---

## Common Development Tasks

### Clean Build (Fresh Start)

```bash
# Clean Rust artifacts
cargo clean

# Clean .NET artifacts
cd ../frontend
dotnet clean

# Rebuild
cd ../desktop
cargo tauri dev
```

### Run Tests

```bash
# Backend tests (use in-memory databases, no setup needed)
cd backend
cargo test --workspace

# Frontend tests
cd ../frontend
dotnet test

# Specific test file
dotnet test --filter "FullyQualifiedName~UserIdentityServiceTests"
```

**Note on Backend Tests:**
- Backend tests use **in-memory SQLite databases** with migrations applied automatically
- You do NOT need to set up a test database or run migrations manually
- Tests work immediately after cloning the project

**Advanced Development:**
- Adding new migrations or SQL queries requires regenerating the SQLx query cache
- See `backend/crates/pm-db/README.md` for the workflow
- TL;DR: You'll need to create `.sqlx-test/` and run `cargo sqlx prepare`

### Update Dependencies

```bash
# Update Rust dependencies
cd backend
cargo update

# Update .NET dependencies
cd ../frontend
dotnet restore
```

### Format Code

```bash
# Rust formatting
cd backend
cargo fmt --all

# Check Rust code quality
cargo clippy --all-targets --all-features

# .NET formatting (if using dotnet-format)
cd ../frontend
dotnet format
```

---

## Performance Tips

### Faster Incremental Builds

Add to `~/.cargo/config.toml`:
```toml
[build]
# Use all CPU cores
jobs = 8  # Adjust to your CPU core count

[profile.dev]
# Faster linker (macOS)
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

### Reduce Rebuild Times

**Modular editing**:
- Frontend changes → Only Blazor recompiles (~1-2 seconds)
- Tauri changes → Only `desktop/src-tauri` recompiles (~5 seconds)
- Backend changes → Only affected crates recompile (~5-15 seconds)

**Avoid touching**:
- `Cargo.toml` (forces full rebuild)
- Proto files (regenerates all proto code)

### Parallel Testing

```bash
# Run tests in parallel
cargo test --workspace -- --test-threads=8
```

---

## Troubleshooting

### "Port already in use"

**Symptom**: `cargo tauri dev` fails with port conflict

**Solution**:
```bash
# Find process using port 8080
lsof -i :8080

# Kill the process
kill -9 <PID>

# Or change port in desktop/.pm/config.toml
[server]
port = 8081
```

### "Frontend build failed"

**Symptom**: .NET compilation errors

**Solution**:
```bash
# Clean and restore
cd frontend
dotnet clean
dotnet restore
dotnet build

# Check for errors
cd ../desktop
cargo tauri dev
```

### "Database is locked"

**Symptom**: SQLite database locked error

**Solution**:
```bash
# Stop the app (Cmd+Q)
# Check for zombie processes
ps aux | grep pm-server
kill -9 <PID>

# Restart
cargo tauri dev
```

### Hot Reload Not Working

**Symptom**: Changes to Blazor files don't trigger refresh

**Solution**:
- Ensure file is saved (Cmd+S)
- Check terminal for build errors
- Hard refresh: Cmd+R in the app
- Restart `cargo tauri dev`

### "Rust analyzer is slow"

**Symptom**: VS Code Rust extension lags

**Solution**:
```bash
# Build once to populate target/
cargo build

# Restart Rust analyzer in VS Code
# Cmd+Shift+P → "Rust Analyzer: Restart Server"
```

### WebView DevTools Not Appearing

**Symptom**: Right-click menu missing "Inspect Element"

**Solution**: Development mode enables DevTools by default. If missing:
- Check `desktop/src-tauri/tauri.conf.json`
- Ensure `"devPath"` is set (not `"distDir"`)
- Restart `cargo tauri dev`

---

## Development URLs

With the app running:

**Backend WebSocket**: `ws://127.0.0.1:8080/ws`
**Backend Health**: `http://127.0.0.1:8080/health`
**Frontend DevTools**: Right-click in app → Inspect Element

**Change backend port**: Edit `desktop/.pm/config.toml`

---

## Editor Integration

### VS Code (Recommended)

**Extensions**:
- `rust-lang.rust-analyzer` - Rust language support
- `ms-dotnettools.csharp` - C# support
- `tauri-apps.tauri-vscode` - Tauri integration

**Workspace settings** (`.vscode/settings.json`):
```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy",
  "dotnet.defaultSolution": "frontend/ProjectManagement.sln"
}
```

### Debugging

**Launch configuration** (`.vscode/launch.json`):
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Tauri",
      "cargo": {
        "args": ["build", "--manifest-path=desktop/src-tauri/Cargo.toml"]
      },
      "cwd": "${workspaceFolder}/desktop"
    }
  ]
}
```

---

## Next Steps

**Ready for production builds?** See [MACOS_PROD_BUILD_README.md](MACOS_PROD_BUILD_README.md)

**Manual testing?** See [../TESTING.md](../../desktop/TESTING.md)

**CI/CD setup?** Development builds are local-only (production builds are for distribution)

---

**Last Updated**: 2026-01-23
