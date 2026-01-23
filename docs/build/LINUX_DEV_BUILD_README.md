# Linux Development Build

**Purpose**: Quick development workflow with hot reload for rapid iteration on Linux.

**Time**: ~30 seconds for initial build, ~1-5 seconds for incremental rebuilds

**Prerequisites**: [LINUX_DEPENDENCIES_README.md](LINUX_DEPENDENCIES_README.md) completed

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
- ✅ WebView DevTools accessible (F12 or right-click → Inspect)

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
pkg-config --modversion gtk+-3.0
pkg-config --modversion webkit2gtk-4.1
```

If any command fails, see [LINUX_DEPENDENCIES_README.md](LINUX_DEPENDENCIES_README.md).

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
    Finished dev [unoptimized + debuginfo] target(s) in 2m 48s
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
3. Save the file (Ctrl+S)
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
# Stop the app (Ctrl+C or close window)
rm -rf desktop/.pm/data.db
cargo tauri dev  # Recreates DB with fresh migrations
```

**Inspect database**:
```bash
# SQLite CLI should be pre-installed on most Linux distros
# If not: sudo apt install sqlite3 (Debian/Ubuntu)

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
- **F12** opens DevTools
- Or right-click anywhere → Select **"Inspect Element"**

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
- Rust debugging with `gdb` or `lldb`
- Better panic stack traces
- Profiling with `perf`, `valgrind`, or `heaptrack`

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

[target.x86_64-unknown-linux-gnu]
# Use mold linker (fastest) - install: sudo apt install mold
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

# Alternative: lld linker - install: sudo apt install lld
# rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

**Install mold linker** (highly recommended for speed):
```bash
# Ubuntu/Debian
sudo apt install mold

# Arch Linux
sudo pacman -S mold

# Fedora
sudo dnf install mold
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

### Use sccache for Faster Builds

```bash
# Install sccache (caches compiled crates)
cargo install sccache

# Configure in ~/.cargo/config.toml
[build]
rustc-wrapper = "/home/user/.cargo/bin/sccache"
```

---

## Troubleshooting

### "Port already in use"

**Symptom**: `cargo tauri dev` fails with port conflict

**Solution**:
```bash
# Find process using port 8080
lsof -i :8080
# OR
ss -tulpn | grep :8080

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
# Stop the app (Ctrl+C)
# Check for zombie processes
ps aux | grep pm-server
kill -9 <PID>

# Restart
cargo tauri dev
```

### Hot Reload Not Working

**Symptom**: Changes to Blazor files don't trigger refresh

**Solution**:
- Ensure file is saved (Ctrl+S)
- Check terminal for build errors
- Hard refresh: Ctrl+R or F5 in the app
- Restart `cargo tauri dev`

### "error: linker `cc` not found"

**Symptom**: Missing C compiler during Rust compilation

**Solution**:
```bash
# Ubuntu/Debian
sudo apt install build-essential

# Fedora
sudo dnf groupinstall "Development Tools"

# Arch Linux
sudo pacman -S base-devel
```

### "Package 'gtk+-3.0' not found"

**Symptom**: Missing GTK development libraries

**Solution**: Install system dependencies (see [LINUX_DEPENDENCIES_README.md](LINUX_DEPENDENCIES_README.md))

```bash
# Ubuntu/Debian
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev
```

### WebView DevTools Not Appearing

**Symptom**: F12 or right-click menu missing "Inspect Element"

**Solution**:
- Development mode enables DevTools by default
- Check `desktop/src-tauri/tauri.conf.json` → `"devPath"` is set
- Restart `cargo tauri dev`

### "Rust analyzer is slow"

**Symptom**: VS Code Rust extension lags

**Solution**:
```bash
# Build once to populate target/
cargo build

# Restart Rust analyzer in VS Code
# Ctrl+Shift+P → "Rust Analyzer: Restart Server"

# Or reduce rust-analyzer CPU usage
# Edit .vscode/settings.json:
{
  "rust-analyzer.checkOnSave.command": "check",
  "rust-analyzer.cargo.buildScripts.enable": false
}
```

### Display/Wayland Issues

**Symptom**: App window doesn't appear or crashes

**Solution**:
```bash
# Force X11 backend (if on Wayland)
GDK_BACKEND=x11 cargo tauri dev

# Or force Wayland
GDK_BACKEND=wayland cargo tauri dev

# Check current session
echo $XDG_SESSION_TYPE
```

---

## Development URLs

With the app running:

**Backend WebSocket**: `ws://127.0.0.1:8080/ws`
**Backend Health**: `http://127.0.0.1:8080/health`
**Frontend DevTools**: F12 or right-click in app → Inspect Element

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

### Neovim / Vim

**Recommended plugins**:
- `neovim/nvim-lspconfig` with rust-analyzer
- `OmniSharp/omnisharp-vim` for C#
- `:LspInstall rust_analyzer`

### Debugging

**VS Code launch configuration** (`.vscode/launch.json`):
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

**GDB debugging**:
```bash
# Build with debug symbols
cargo build

# Debug with gdb
gdb desktop/src-tauri/target/debug/blazor-agile-board
(gdb) run
(gdb) bt  # backtrace on crash
```

---

## Shell Aliases

Add to `~/.bashrc` or `~/.zshrc`:

```bash
# Aliases for common tasks
alias btd='cd ~/path/to/blazor-agile-board/desktop && cargo tauri dev'
alias btb='cd ~/path/to/blazor-agile-board/desktop && cargo tauri build'
alias btest='cd ~/path/to/blazor-agile-board/backend && cargo test --workspace'
alias ftest='cd ~/path/to/blazor-agile-board/frontend && dotnet test'

# Fast clean
bclean() {
    cd ~/path/to/blazor-agile-board
    cargo clean
    cd frontend && dotnet clean
    cd ..
}
```

**Reload shell**: `source ~/.bashrc`

---

## Desktop Environment Considerations

### GNOME

- System tray may require extension: `gnome-shell-extension-appindicator`
- Install: `sudo apt install gnome-shell-extension-appindicator`

### KDE Plasma

- System tray works out of the box
- No additional configuration needed

### XFCE / MATE / Cinnamon

- System tray works out of the box
- No additional configuration needed

### i3 / Sway (Tiling WMs)

- System tray requires status bar with tray support
- Recommended: `waybar` (Wayland) or `i3status` + `i3bar` (X11)

---

## Performance Profiling

### CPU Profiling with perf

```bash
# Install perf
sudo apt install linux-tools-common linux-tools-generic  # Ubuntu/Debian
sudo dnf install perf  # Fedora

# Profile app
cargo build --release
perf record --call-graph dwarf ./desktop/src-tauri/target/release/blazor-agile-board

# View results
perf report
```

### Memory Profiling with heaptrack

```bash
# Install heaptrack
sudo apt install heaptrack  # Ubuntu/Debian
sudo dnf install heaptrack  # Fedora

# Profile app
heaptrack ./desktop/src-tauri/target/release/blazor-agile-board

# View results
heaptrack_gui heaptrack.blazor-agile-board.*.gz
```

### Valgrind Memory Check

```bash
# Install valgrind
sudo apt install valgrind

# Check for memory leaks
cargo build
valgrind --leak-check=full ./desktop/src-tauri/target/debug/blazor-agile-board
```

---

## Next Steps

**Ready for production builds?** See [LINUX_PROD_BUILD_README.md](LINUX_PROD_BUILD_README.md)

**Manual testing?** See [../TESTING.md](../../desktop/TESTING.md)

**CI/CD setup?** Development builds are local-only (production builds are for distribution)

---

**Last Updated**: 2026-01-23
