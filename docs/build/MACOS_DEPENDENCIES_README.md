# macOS Dependencies

**Purpose**: One-time setup of development tools for building the Blazor Agile Board desktop application on macOS.

**Time Estimate**: ~10-15 minutes (downloads + installations)

**Minimum Requirements**:
- macOS 11.0 (Big Sur) or later
- ~5GB free disk space
- Admin access (for Xcode Command Line Tools)
- Internet connection

---

## Quick Reference

After completing this setup, you should have:
- ✅ Xcode Command Line Tools
- ✅ Rust 1.93.0+ and Cargo
- ✅ .NET SDK 10.0
- ✅ Protocol Buffers Compiler (protoc)
- ✅ Tauri CLI
- ✅ SQLx CLI (for database migrations)
- ✅ Just (task runner for build automation)
- ✅ Environment variables configured

---

## 1. Xcode Command Line Tools

**Required for**: C/C++ compilation, system libraries

**Installation**:
```bash
xcode-select --install
```

A dialog will appear. Click **"Install"** and wait ~5-10 minutes.

**Verification**:
```bash
xcode-select -p
```
**Expected output**: `/Library/Developer/CommandLineTools`

**Troubleshooting**:
- If already installed: "command line tools are already installed"
- If command fails: Download manually from [Apple Developer](https://developer.apple.com/download/all/)

---

## 2. Rust & Cargo

**Required for**: Tauri backend compilation

**Installation** (Official rustup installer - fast, no Homebrew):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

When prompted:
- Select option **1** (default installation)
- Installation takes ~3-5 minutes

**Important**: Restart your terminal after installation (or run `source $HOME/.cargo/env`)

**Verification**:
```bash
rustc --version
cargo --version
```
**Expected output**: Version 1.93.0 or later for both

**Updating Rust**:
```bash
# Update rustup itself (the installer/manager)
rustup self update

# Update Rust toolchain to latest stable
rustup update stable

# Verify new version
rustc --version
```

**Troubleshooting**:
- `command not found: cargo` → Restart terminal or check `~/.cargo/bin` is in PATH
- Check for available updates: `rustup check`

---

## 3. .NET SDK 10.0

**Required for**: Blazor frontend compilation

**Installation**:
1. Visit: https://dotnet.microsoft.com/download/dotnet/10.0
2. Click **"macOS"** tab
3. Download **".NET SDK 10.0.x - macOS Installer"** (.pkg file)
4. Run the downloaded .pkg file
5. Follow installer prompts (~2 minutes)

**Verification**:
```bash
dotnet --version
```
**Expected output**: `10.0.x` (e.g., 10.0.0, 10.0.1)

**Troubleshooting**:
- If `dotnet: command not found` → Check `/usr/local/share/dotnet` is in PATH
- Add to PATH manually: `export PATH="/usr/local/share/dotnet:$PATH"` in `~/.zshrc` or `~/.bash_profile`

---

## 4. Protocol Buffers Compiler (protoc)

**Required for**: Compiling `.proto` files for frontend/backend communication

**What it does**: Protocol Buffers (protobuf) is the binary serialization format used for WebSocket communication between the Blazor frontend and Rust backend. The `protoc` compiler generates C# and Rust code from `proto/messages.proto`.

**Installation** (Pre-built Binary - RECOMMENDED):

This is the fastest method (30 seconds) and avoids Homebrew's source compilation.

```bash
# Download latest pre-built binary
cd ~/Downloads
curl -LO https://github.com/protocolbuffers/protobuf/releases/download/v29.2/protoc-29.2-osx-universal_binary.zip

# Extract to a permanent location
mkdir -p ~/tools
unzip protoc-29.2-osx-universal_binary.zip -d ~/tools/protoc

# Add to PATH in your shell profile
echo 'export PATH="$HOME/tools/protoc/bin:$PATH"' >> ~/.zshrc

# Apply changes (or restart terminal)
source ~/.zshrc
```

**For bash users**, replace `~/.zshrc` with `~/.bash_profile` in the commands above.

**Verification**:
```bash
protoc --version
```
**Expected output**: `libprotoc 29.2.0` (or the version you downloaded)

**Alternative: Homebrew** (WARNING: May build from source - very slow):
```bash
# Only use if you specifically need Homebrew management
brew install protobuf
```
⚠️ **Note**: Homebrew may compile from source, taking 10-20 minutes. The pre-built binary above is much faster.

**Latest Version Check**:
Visit [Protocol Buffers releases](https://github.com/protocolbuffers/protobuf/releases) to get the latest version number. Replace `29.2` in the download URL with the current version.

**CRITICAL First-Time Setup**:

The `pm-proto` crate outputs generated Rust code to `src/generated/`. **This directory must exist before the first build**, otherwise you'll encounter:
```
Failed to compile protobuf definitions: Os { code: 2, kind: NotFound, message: "No such file or directory" }
```

Create it with:
```bash
# From repository root
mkdir -p backend/crates/pm-proto/src/generated
```

**Why?** The build script (`build.rs`) uses `prost_build` to compile `.proto` files, but it doesn't create the output directory automatically.

**This is a one-time setup** - once created, the directory persists and future builds work automatically.

**Why it's needed**:
- The frontend's `ProjectManagement.Core` project uses the `Grpc.Tools` NuGet package
- `Grpc.Tools` invokes `protoc` during build to generate C# classes from `proto/messages.proto`
- Without `protoc`, frontend builds will fail with "protoc not found" errors

**Troubleshooting**:
- `protoc: command not found` → Check PATH is set: `echo $PATH | grep protoc`
- Restart terminal after adding to PATH (or run `source ~/.zshrc`)
- Frontend build fails with "protoc not found" → Check `PROTOC` environment variable (see section 8)
- Permission denied → Run `chmod +x ~/tools/protoc/bin/protoc`
- Version too old → Download newer release and extract to `~/tools/protoc` (overwrites old version)

---

## 5. Tauri CLI

**Required for**: Building and running the desktop application

**Installation** (via Cargo - no Node.js needed):
```bash
cargo install tauri-cli
```

This compiles the Tauri CLI from source. Takes ~2-3 minutes (one-time compilation, cached afterward).

**Verification**:
```bash
cargo tauri --version
```
**Expected output**: `tauri-cli 1.x.x` or later

**Troubleshooting**:
- Compilation errors → Ensure Xcode Command Line Tools are installed
- `cargo: command not found` → Ensure Rust installation completed (see step 2)

---

## 6. SQLx CLI

**Required for**: Database migrations and compile-time SQL verification

**What it does**: SQLx CLI manages database migrations and validates SQL queries at compile time. This project uses SQLite with SQLx migrations in `backend/crates/pm-db/migrations/`.

**Installation** (SQLite-only, no PostgreSQL/MySQL dependencies):
```bash
cargo install sqlx-cli --no-default-features --features sqlite
```

This installs only SQLite support, making it faster and lighter. Takes ~2-3 minutes.

**Verification**:
```bash
sqlx --version
```
**Expected output**: `sqlx-cli 0.8.x` or later

**Common commands** (you'll use these):
```bash
# Run migrations (from backend/ directory)
sqlx migrate run --database-url sqlite:.pm/data.db

# Create new migration
sqlx migrate add create_new_table

# Revert last migration
sqlx migrate revert --database-url sqlite:.pm/data.db
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
```bash
cargo install just
```

Takes ~1-2 minutes.

**Verification**:
```bash
just --version
```
**Expected output**: `just 1.x.x` or later

**Common commands** (from project root):
```bash
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
- `just: command not found` → Ensure `~/.cargo/bin` is in PATH
- See project's `justfile` for all available tasks

---

## 8. Environment Variable Configuration

**Required for**: Ensuring `Grpc.Tools` can find the `protoc` compiler during .NET builds

**What this does**: The `Grpc.Tools` NuGet package (used in `ProjectManagement.Core`) needs to locate the `protoc` executable to compile `.proto` files. While it usually auto-detects `protoc` from PATH, explicitly setting the `PROTOC` environment variable ensures reliable builds, especially with the `.slnx` solution format.

**Setup Instructions**:

Add the following to your shell profile (`~/.zshrc` for zsh or `~/.bash_profile` for bash):

**If you installed protoc via pre-built binary** (recommended method):
```bash
# Protocol Buffers compiler for .NET builds
export PROTOC="$HOME/tools/protoc/bin/protoc"
```

**If you installed protoc via Homebrew**:
```bash
# Protocol Buffers compiler for .NET builds
export PROTOC="$(brew --prefix protobuf)/bin/protoc"
```

**Apply the changes**:
```bash
# For zsh (macOS default)
source ~/.zshrc

# For bash
source ~/.bash_profile
```

**Verification**:
```bash
echo $PROTOC
```
**Expected output**: Full path to protoc binary (e.g., `/opt/homebrew/opt/protobuf/bin/protoc`)

```bash
$PROTOC --version
```
**Expected output**: `libprotoc 3.x.x` or later

**Why this is needed**:
- The frontend uses `ProjectManagement.slnx` (XML-based solution format)
- `Grpc.Tools` sometimes fails to auto-detect `protoc` with `.slnx` solutions
- Explicitly setting `PROTOC` prevents "protoc not found" build errors
- This env var is checked during `dotnet build` and `just build-frontend`

**Troubleshooting**:
- Build fails with "protoc not found" → Verify `$PROTOC` is set: `echo $PROTOC`
- `$PROTOC` is empty → Ensure you ran `source ~/.zshrc` (or restarted terminal)
- Still fails → Check protoc is installed: `which protoc`
- Homebrew path incorrect → Run `brew --prefix protobuf` to get correct path

---

## Complete Environment Verification

Run all verification commands together:

```bash
echo "=== Xcode Command Line Tools ==="
xcode-select -p

echo "\n=== Rust & Cargo ==="
rustc --version
cargo --version

echo "\n=== .NET SDK ==="
dotnet --version

echo "\n=== Protocol Buffers Compiler ==="
protoc --version

echo "\n=== Tauri CLI ==="
cargo tauri --version

echo "\n=== SQLx CLI ==="
sqlx --version

echo "\n=== Just Task Runner ==="
just --version

echo "\n=== Environment Variables ==="
echo "PROTOC: $PROTOC"
```

**All commands should succeed** with version numbers matching the requirements above.

---

## Common Issues

### "xcrun: error: invalid active developer path"
**Solution**: Install Xcode Command Line Tools (step 1)

### Cargo/Rust commands not found after installation
**Solution**: Restart terminal or run `source $HOME/.cargo/env`

### .NET SDK version shows older version
**Solution**:
- Ensure you downloaded SDK (not Runtime)
- Multiple SDKs can coexist; check: `dotnet --list-sdks`

### Tauri CLI installation fails with "linker error"
**Solution**: Ensure Xcode Command Line Tools are fully installed and active

### Frontend build fails with "protoc not found" or "Grpc.Tools failed"
**Solution**:
1. Verify protoc is installed: `protoc --version`
2. Set `PROTOC` environment variable (see section 8)
3. Restart terminal after setting env var
4. If still fails, check `echo $PROTOC` shows the correct path

### `PROTOC` environment variable not set after adding to shell profile
**Solution**:
- Restart terminal completely (close and reopen)
- Or run `source ~/.zshrc` (or `source ~/.bash_profile` for bash)
- Verify with `echo $PROTOC` - should show full path to protoc

---

## Next Steps

Dependencies installed? Proceed to:
- **Development builds**: [MACOS_DEV_BUILD_README.md](MACOS_DEV_BUILD_README.md)
- **Production builds**: [MACOS_PROD_BUILD_README.md](MACOS_PROD_BUILD_README.md)

---

## Maintenance

**Updating dependencies**:
```bash
# Update Rust (two-step process)
rustup self update      # Update rustup itself
rustup update stable    # Update Rust toolchain

# Update .NET SDK
# Download new installer from microsoft.com

# Update protoc (if using pre-built binary)
# 1. Download new version from https://github.com/protocolbuffers/protobuf/releases
# 2. Extract to ~/tools/protoc (overwrites old version)
# Or via Homebrew (if you used that method):
brew upgrade protobuf

# Update Cargo tools
cargo install tauri-cli --force
cargo install sqlx-cli --force --no-default-features --features sqlite
cargo install just --force
```

**Checking for updates**:
```bash
rustup check            # Shows available Rust updates
dotnet --list-sdks      # Shows installed .NET versions
cargo install --list    # Shows installed Cargo tools
```

---

**Last Updated**: 2026-02-11
