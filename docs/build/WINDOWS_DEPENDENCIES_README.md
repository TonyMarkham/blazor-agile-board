# Windows Dependencies

**Purpose**: One-time setup of development tools for building the Blazor Agile Board desktop application on Windows.

**Time Estimate**: ~15-25 minutes (downloads + installations)

**Minimum Requirements**:
- Windows 10 version 1809 (build 17763) or later
- Windows 11 (any version)
- ~8GB free disk space
- Admin access
- Internet connection

---

## Quick Reference

After completing this setup, you should have:
- ✅ Microsoft C++ Build Tools
- ✅ WebView2 Runtime (usually pre-installed on Windows 11)
- ✅ Rust 1.93.0+ and Cargo
- ✅ .NET SDK 10.0
- ✅ Tauri CLI
- ✅ SQLx CLI (for database migrations)
- ✅ Just (task runner for build automation)

---

## 1. Microsoft C++ Build Tools

**Required for**: Rust compilation, native Windows libraries

**Installation**:

**Option A: Visual Studio Build Tools (Recommended - Smaller)**

1. Download: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
2. Scroll to **"Tools for Visual Studio"** → Download **"Build Tools for Visual Studio 2022"**
3. Run the installer
4. In the installer, select **"Desktop development with C++"** workload
5. Click **Install** (~2-3 GB download, 10-15 minutes)

**Option B: Full Visual Studio Community**

1. Download: https://visualstudio.microsoft.com/vs/community/
2. Run installer
3. Select **"Desktop development with C++"** workload
4. Click **Install** (~7+ GB download, 20-30 minutes)

**Verification**:
Open **Developer Command Prompt for VS 2022** (search in Start menu) and run:
```cmd
cl
```
**Expected output**: Microsoft C/C++ compiler version information (not "command not found")

**Troubleshooting**:
- If `cl` not found → Ensure "Desktop development with C++" was selected
- Restart required after installation

---

## 2. WebView2 Runtime

**Required for**: Tauri's embedded browser component

**Check if already installed** (likely on Windows 11):
```powershell
Get-ItemProperty -Path "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -Name pv -ErrorAction SilentlyContinue
```

If output shows a version number, WebView2 is installed. **Skip to step 3.**

**Installation** (if not installed):
1. Download: https://developer.microsoft.com/en-us/microsoft-edge/webview2/#download-section
2. Choose **"Evergreen Standalone Installer"** (x64)
3. Run `MicrosoftEdgeWebview2Setup.exe`
4. Installation takes ~1 minute

**Verification**:
Run the PowerShell check command above, or look for `msedgewebview2.exe` in:
```
C:\Program Files (x86)\Microsoft\EdgeWebView\Application\
```

---

## 3. Rust & Cargo

**Required for**: Tauri backend compilation

**Installation** (Official rustup installer):

1. Download: https://rustup.rs/
2. Click **"Download rustup-init.exe (64-bit)"**
3. Run `rustup-init.exe`
4. When prompted, select option **1** (default installation)
5. Installation takes ~3-5 minutes

**Important**: Close and reopen your terminal/PowerShell after installation.

**Verification**:
```powershell
rustc --version
cargo --version
```
**Expected output**: Version 1.93.0 or later for both

**Updating Rust**:
```powershell
# Update rustup itself (the installer/manager)
rustup self update

# Update Rust toolchain to latest stable
rustup update stable

# Verify new version
rustc --version
```

**Troubleshooting**:
- `cargo: command not found` → Restart PowerShell or add `%USERPROFILE%\.cargo\bin` to PATH
- Missing linker errors during cargo build → Install C++ Build Tools (step 1)
- Check for available updates: `rustup check`

---

## 4. .NET SDK 10.0

**Required for**: Blazor frontend compilation

**Installation**:
1. Visit: https://dotnet.microsoft.com/download/dotnet/10.0
2. Click **"Windows"** tab
3. Download **".NET SDK 10.0.x - Windows x64 Installer"** (.exe file)
4. Run the installer
5. Follow prompts (~2 minutes)

**Verification**:
```powershell
dotnet --version
```
**Expected output**: `10.0.x` (e.g., 10.0.0, 10.0.1)

**Troubleshooting**:
- If `dotnet: command not found` → Restart PowerShell
- Check installation: `dotnet --list-sdks`
- Multiple SDKs can coexist; ensure 8.0.x is listed

---

## 5. Tauri CLI

**Required for**: Building and running the desktop application

**Installation** (via Cargo - no Node.js needed):
```powershell
cargo install tauri-cli
```

This compiles the Tauri CLI from source. Takes ~3-5 minutes (one-time compilation, cached afterward).

**Verification**:
```powershell
cargo tauri --version
```
**Expected output**: `tauri-cli 1.x.x` or later

**Troubleshooting**:
- Compilation errors → Ensure C++ Build Tools installed (step 1)
- Linker errors → Restart PowerShell after installing Visual Studio tools
- `cargo: command not found` → Ensure Rust installation completed (step 3)

---

## 6. SQLx CLI

**Required for**: Database migrations and compile-time SQL verification

**What it does**: SQLx CLI manages database migrations and validates SQL queries at compile time. This project uses SQLite with SQLx migrations in `backend\crates\pm-db\migrations\`.

**Installation** (SQLite-only, no PostgreSQL/MySQL dependencies):
```powershell
cargo install sqlx-cli --no-default-features --features sqlite
```

This installs only SQLite support, making it faster and lighter. Takes ~3-5 minutes.

**Verification**:
```powershell
sqlx --version
```
**Expected output**: `sqlx-cli 0.8.x` or later

**Common commands** (you'll use these):
```powershell
# Run migrations (from backend\ directory)
sqlx migrate run --database-url sqlite:.pm\data.db

# Create new migration
sqlx migrate add create_new_table

# Revert last migration
sqlx migrate revert --database-url sqlite:.pm\data.db
```

**Troubleshooting**:
- If installed with wrong features, reinstall: `cargo install sqlx-cli --force --no-default-features --features sqlite`
- Migration errors → Check database path is correct
- Compile-time verification requires `DATABASE_URL` env var (optional for this project)

---

## 7. Just (Task Runner)

**Required for**: Build automation and development workflows

**What it does**: Just is a command runner (like Make, but better). This project uses a `justfile` to automate common tasks like `just dev` (run development build) and `just build` (production build).

**Installation**:
```powershell
cargo install just
```

Takes ~1-2 minutes.

**Verification**:
```powershell
just --version
```
**Expected output**: `just 1.x.x` or later

**Common commands** (from project root):
```powershell
# List all available tasks
just --list

# Run development build
just dev

# Run production build
just build

# Run backend tests
just test-backend

# Run frontend tests
just test-frontend

# Clean all build artifacts
just clean
```

**Why Just instead of npm scripts?**
- No Node.js dependency
- Cross-platform (works on macOS, Windows, Linux)
- Simpler syntax than Make
- Native to Rust ecosystem

**Troubleshooting**:
- `just: command not found` → Ensure `%USERPROFILE%\.cargo\bin` is in PATH
- Restart PowerShell after installation
- See project's `justfile` for all available tasks

---

## Complete Environment Verification

Run all verification commands together (PowerShell):

```powershell
Write-Host "`n=== C++ Build Tools ===" -ForegroundColor Green
cl 2>&1 | Select-String "Microsoft"

Write-Host "`n=== WebView2 Runtime ===" -ForegroundColor Green
Get-ItemProperty -Path "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -Name pv -ErrorAction SilentlyContinue | Select-Object -ExpandProperty pv

Write-Host "`n=== Rust & Cargo ===" -ForegroundColor Green
rustc --version
cargo --version

Write-Host "`n=== .NET SDK ===" -ForegroundColor Green
dotnet --version

Write-Host "`n=== Tauri CLI ===" -ForegroundColor Green
cargo tauri --version

Write-Host "`n=== SQLx CLI ===" -ForegroundColor Green
sqlx --version

Write-Host "`n=== Just Task Runner ===" -ForegroundColor Green
just --version
```

**All commands should succeed** with version numbers matching the requirements above.

---

## Common Issues

### "cl is not recognized as an internal or external command"
**Solution**:
- Install Visual Studio Build Tools with "Desktop development with C++" (step 1)
- Use **Developer Command Prompt for VS 2022** instead of regular PowerShell

### "error: linker `link.exe` not found" during cargo build
**Solution**:
- Install C++ Build Tools (step 1)
- Restart PowerShell after installation
- Ensure you selected "Desktop development with C++" workload

### WebView2 runtime not detected
**Solution**:
- Windows 11: Usually pre-installed, run Windows Update
- Manual install: Download from Microsoft (step 2)

### "Access denied" or permission errors
**Solution**: Run PowerShell as Administrator for global installations

### Cargo commands fail with "SSL certificate" errors
**Solution**: Corporate firewall/proxy issue
- Set `CARGO_HTTP_CAINFO` environment variable to your CA bundle
- Or disable SSL verification (not recommended): `CARGO_HTTP_CHECK_REVOKE=false`

---

## Next Steps

Dependencies installed? Proceed to:
- **Development builds**: [WINDOWS_DEV_BUILD_README.md](WINDOWS_DEV_BUILD_README.md)
- **Production builds**: [WINDOWS_PROD_BUILD_README.md](WINDOWS_PROD_BUILD_README.md)

---

## Maintenance

**Updating dependencies**:
```powershell
# Update Rust (two-step process)
rustup self update      # Update rustup itself
rustup update stable    # Update Rust toolchain

# Update .NET SDK
# Download new installer from microsoft.com

# Update Cargo tools
cargo install tauri-cli --force
cargo install sqlx-cli --force --no-default-features --features sqlite
cargo install just --force
```

**Checking for updates**:
```powershell
rustup check            # Shows available Rust updates
dotnet --list-sdks      # Shows installed .NET versions
cargo install --list    # Shows installed Cargo tools
```

---

## Windows-Specific Notes

**PowerShell Execution Policy**:
If you get "script execution is disabled" errors:
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

**Long Path Support** (recommended for Rust builds):
```powershell
# Run as Administrator
New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" -Name "LongPathsEnabled" -Value 1 -PropertyType DWORD -Force
```

**Windows Defender Exclusions** (optional, speeds up builds):
- Exclude `%USERPROFILE%\.cargo` from real-time scanning
- Exclude project directory from real-time scanning

---

**Last Updated**: 2026-01-23
