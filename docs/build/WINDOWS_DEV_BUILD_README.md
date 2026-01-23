# Windows Development Build

**Purpose**: Quick development workflow with hot reload for rapid iteration on Windows.

**Time**: ~30 seconds for initial build, ~1-5 seconds for incremental rebuilds

**Prerequisites**: [WINDOWS_DEPENDENCIES_README.md](WINDOWS_DEPENDENCIES_README.md) completed

---

## Quick Start

```powershell
# Clone repository (if not already done)
git clone https://github.com/your-org/blazor-agile-board.git
cd blazor-agile-board\desktop

# Run development build
cargo tauri dev
```

The app will launch with:
- ✅ Hot reload enabled (Blazor changes auto-refresh)
- ✅ Debug symbols included (PDB files)
- ✅ Faster compilation (unoptimized)
- ✅ Development logging enabled
- ✅ WebView DevTools accessible (F12 or right-click → Inspect)

---

## First-Time Setup

### 1. Clone Repository

```powershell
git clone https://github.com/your-org/blazor-agile-board.git
cd blazor-agile-board
```

### 2. Verify Dependencies

```powershell
rustc --version   # Should be 1.93.0+
dotnet --version  # Should be 10.0.x
cargo tauri --version
```

If any command fails, see [WINDOWS_DEPENDENCIES_README.md](WINDOWS_DEPENDENCIES_README.md).

### 3. Initial Build

```powershell
cd desktop
cargo tauri dev
```

**First build takes ~3-7 minutes** (compiles all dependencies, one-time cost).

**Expected output**:
```
    Compiling pm-core v0.1.0
    Compiling pm-db v0.1.0
    ...
    Finished dev [unoptimized + debuginfo] target(s) in 3m 45s
    Running frontend\ProjectManagement.Wasm
    ...
Opening app...
```

The desktop application window will open automatically.

---

## Development Workflow

### Hot Reload (Blazor)

1. Keep `cargo tauri dev` running
2. Edit any `.razor`, `.cs`, or `.css` file in `frontend\`
3. Save the file (Ctrl+S)
4. **App auto-refreshes** within 1-2 seconds

**Example**:
```powershell
# Terminal 1: Keep this running
cd desktop
cargo tauri dev

# Terminal 2: Edit files
cd frontend\ProjectManagement.Components\Pages
# Edit Home.razor, save
# → App refreshes automatically
```

### Rust Backend Changes

If you modify Rust code in `desktop\src-tauri\` or `backend\`:

1. Save your changes
2. Tauri detects the change and **auto-recompiles**
3. App restarts automatically (preserves app state via SQLite)

**Incremental rebuilds**: ~5-15 seconds (only changed crates recompile)

### Database Changes

Development database location:
```
desktop\.pm\data.db
```

**Reset database** (for testing migrations):
```powershell
# Stop the app (Alt+F4 or close window)
Remove-Item -Recurse -Force desktop\.pm\data.db
cargo tauri dev  # Recreates DB with fresh migrations
```

**Inspect database**:
```powershell
# Install SQLite CLI if needed
# Download from https://www.sqlite.org/download.html (sqlite-tools-win-x64)

# Open database
sqlite3.exe desktop\.pm\data.db
sqlite> .tables
sqlite> SELECT * FROM pm_work_items;
sqlite> .quit
```

---

## Development Features

### WebView DevTools

With the app running:
- **F12** opens DevTools
- Or right-click anywhere → Select **"Inspect"**

**Use for**:
- Debugging Blazor components
- Network tab (WebSocket messages)
- Console logs
- Performance profiling

### Backend Logs

Logs are written to:
```
desktop\.pm\logs\app.log
```

**View logs** (live in PowerShell):
```powershell
Get-Content desktop\.pm\logs\app.log -Wait -Tail 50
```

**Log levels**: Controlled in `desktop\.pm\config.toml`:
```toml
[logging]
level = "debug"  # trace, debug, info, warn, error
```

### Debug Symbols

Dev builds include full debug symbols (PDB files) for:
- Debugging with Visual Studio
- Better panic stack traces
- Profiling with Windows Performance Analyzer

---

## Common Development Tasks

### Clean Build (Fresh Start)

```powershell
# Clean Rust artifacts
cargo clean

# Clean .NET artifacts
cd ..\frontend
dotnet clean

# Rebuild
cd ..\desktop
cargo tauri dev
```

### Run Tests

```powershell
# Backend tests (use in-memory databases, no setup needed)
cd backend
cargo test --workspace

# Frontend tests
cd ..\frontend
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
- See `backend\crates\pm-db\README.md` for the workflow
- TL;DR: You'll need to create `.sqlx-test\` and run `cargo sqlx prepare`

### Update Dependencies

```powershell
# Update Rust dependencies
cd backend
cargo update

# Update .NET dependencies
cd ..\frontend
dotnet restore
```

### Format Code

```powershell
# Rust formatting
cd backend
cargo fmt --all

# Check Rust code quality
cargo clippy --all-targets --all-features

# .NET formatting (if using dotnet-format)
cd ..\frontend
dotnet format
```

---

## Performance Tips

### Faster Incremental Builds

Add to `%USERPROFILE%\.cargo\config.toml`:
```toml
[build]
# Use all CPU cores
jobs = 8  # Adjust to your CPU core count

# Faster linker (Windows)
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

### Reduce Rebuild Times

**Modular editing**:
- Frontend changes → Only Blazor recompiles (~1-2 seconds)
- Tauri changes → Only `desktop\src-tauri` recompiles (~5-10 seconds)
- Backend changes → Only affected crates recompile (~5-20 seconds)

**Avoid touching**:
- `Cargo.toml` (forces full rebuild)
- Proto files (regenerates all proto code)

### Parallel Testing

```powershell
# Run tests in parallel
cargo test --workspace -- --test-threads=8
```

### Disable Windows Defender (Optional)

**Significantly speeds up builds** by excluding project directories:

```powershell
# Run PowerShell as Administrator
Add-MpPreference -ExclusionPath "$env:USERPROFILE\.cargo"
Add-MpPreference -ExclusionPath "C:\path\to\blazor-agile-board"
```

**Security note**: Only exclude trusted project directories.

---

## Troubleshooting

### "Port already in use"

**Symptom**: `cargo tauri dev` fails with port conflict

**Solution**:
```powershell
# Find process using port 8080
netstat -ano | findstr :8080

# Kill the process
taskkill /PID <PID> /F

# Or change port in desktop\.pm\config.toml
[server]
port = 8081
```

### "Frontend build failed"

**Symptom**: .NET compilation errors

**Solution**:
```powershell
# Clean and restore
cd frontend
dotnet clean
dotnet restore
dotnet build

# Check for errors
cd ..\desktop
cargo tauri dev
```

### "Database is locked"

**Symptom**: SQLite database locked error

**Solution**:
```powershell
# Stop the app (Alt+F4)
# Check for zombie processes
tasklist | findstr pm-server
taskkill /IM pm-server.exe /F

# Restart
cargo tauri dev
```

### Hot Reload Not Working

**Symptom**: Changes to Blazor files don't trigger refresh

**Solution**:
- Ensure file is saved (Ctrl+S)
- Check terminal for build errors
- Hard refresh: Ctrl+R in the app
- Restart `cargo tauri dev`

### "LINK: fatal error LNK1181: cannot open input file"

**Symptom**: Linker fails during Rust compilation

**Solution**:
- Ensure Visual Studio Build Tools installed (see [WINDOWS_DEPENDENCIES_README.md](WINDOWS_DEPENDENCIES_README.md))
- Use **Developer PowerShell for VS 2022** instead of regular PowerShell
- Restart PowerShell after installing build tools

### WebView2 Not Found

**Symptom**: "WebView2 runtime not installed"

**Solution**:
- Windows 11: Run Windows Update
- Manual install: https://developer.microsoft.com/en-us/microsoft-edge/webview2/

### "Rust analyzer is slow"

**Symptom**: VS Code Rust extension lags

**Solution**:
```powershell
# Build once to populate target\
cargo build

# Restart Rust analyzer in VS Code
# Ctrl+Shift+P → "Rust Analyzer: Restart Server"
```

### Long Path Errors

**Symptom**: "The filename or extension is too long"

**Solution** (run PowerShell as Administrator):
```powershell
New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" -Name "LongPathsEnabled" -Value 1 -PropertyType DWORD -Force
```

Restart computer after enabling.

---

## Development URLs

With the app running:

**Backend WebSocket**: `ws://127.0.0.1:8080/ws`
**Backend Health**: `http://127.0.0.1:8080/health`
**Frontend DevTools**: F12 or right-click in app → Inspect

**Change backend port**: Edit `desktop\.pm\config.toml`

---

## Editor Integration

### Visual Studio Code (Recommended)

**Extensions**:
- `rust-lang.rust-analyzer` - Rust language support
- `ms-dotnettools.csharp` - C# support
- `tauri-apps.tauri-vscode` - Tauri integration

**Workspace settings** (`.vscode\settings.json`):
```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy",
  "dotnet.defaultSolution": "frontend/ProjectManagement.sln"
}
```

### Visual Studio 2022

1. Open `frontend\ProjectManagement.sln` for .NET development
2. For Rust: Use VS Code with rust-analyzer (better experience)

### Debugging

**VS Code launch configuration** (`.vscode\launch.json`):
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "cppvsdbg",
      "request": "launch",
      "name": "Debug Tauri",
      "program": "${workspaceFolder}\\desktop\\src-tauri\\target\\debug\\blazor-agile-board.exe",
      "cwd": "${workspaceFolder}\\desktop",
      "preLaunchTask": "cargo build"
    }
  ]
}
```

---

## PowerShell Profile Tips

Add to `$PROFILE` (create if doesn't exist):

```powershell
# Aliases for common tasks
function btd { cd C:\path\to\blazor-agile-board\desktop; cargo tauri dev }
function btb { cd C:\path\to\blazor-agile-board\desktop; cargo tauri build }
function btest { cd C:\path\to\blazor-agile-board\backend; cargo test --workspace }
function ftest { cd C:\path\to\blazor-agile-board\frontend; dotnet test }

# Fast clean
function bclean {
    cd C:\path\to\blazor-agile-board
    cargo clean
    cd frontend
    dotnet clean
    cd ..
}
```

**Reload profile**: `. $PROFILE`

---

## Next Steps

**Ready for production builds?** See [WINDOWS_PROD_BUILD_README.md](WINDOWS_PROD_BUILD_README.md)

**Manual testing?** See [..\TESTING.md](..\..\desktop\TESTING.md)

**CI/CD setup?** Development builds are local-only (production builds are for distribution)

---

**Last Updated**: 2026-01-23
